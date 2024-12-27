use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
    fmt::Debug,
    thread,
};
use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{info, debug, warn, error};
use serde::Serialize;

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType},
};

use crate::{
    general_use::{ClientCommand, ClientEvent, Message, Query, Response, ServerType},
    clients::client::Client
};

use super::message_fragments::MessageFragments;

pub struct ChatClientDanylo {
    // ID
    pub id: NodeId,                                             // Client ID

    // Channels
    pub packet_send: HashMap<NodeId, Sender<Packet>>,           // Neighbor's packet sender channels
    pub packet_recv: Receiver<Packet>,                          // Packet receiver channel
    pub controller_send: Sender<ClientEvent>,                   // Event sender channel
    pub controller_recv: Receiver<ClientCommand>,               // Command receiver channel

    // Servers and clients
    pub servers: HashMap<NodeId, ServerType>,                   // IDs and types of the available servers
    pub is_registered: HashMap<NodeId, bool>,                   // Registration status on servers
    pub clients: Vec<NodeId>,                                   // Available clients

    // Used IDs
    pub session_ids: Vec<u64>,                                  // Used session IDs
    pub flood_ids: Vec<u64>,                                    // Used flood IDs

    // Network
    pub topology: HashMap<NodeId, HashSet<NodeId>>,             // Nodes and their neighbours
    pub routes: HashMap<NodeId, Vec<NodeId>>,                   // Routes to the servers

    // Message queues
    pub messages_to_send: HashMap<u64, MessageFragments>,       // Queue of messages to be sent for different sessions
    pub fragments_to_reassemble: HashMap<u64, Vec<Fragment>>,   // Queue of fragments to be reassembled for different sessions

    // Inbox
    pub inbox: Vec<(NodeId, Message)>,                          // Messages with their senders
    pub new_messages: usize,                                    // Count of new messages

    // Response statuses
    pub response_received: bool,                                // Status of the last sent query
    pub last_response_time: Option<Instant>,                    // Time of the last response
    pub external_error: Option<String>,                         // Error message from server/drone
}

impl Client for ChatClientDanylo {
    fn new(
        id: NodeId,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
    ) -> Self {
        Self {
            id,
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,
            servers: HashMap::new(),
            is_registered: HashMap::new(),
            clients: Vec::new(),
            session_ids: Vec::new(),
            flood_ids: Vec::new(),
            topology: HashMap::new(),
            routes: HashMap::new(),
            messages_to_send: HashMap::new(),
            fragments_to_reassemble: HashMap::new(),
            inbox: Vec::new(),
            new_messages: 0,
            response_received: false,
            last_response_time: None,
            external_error: None,
        }
    }

    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        self.handle_command(command);
                    }
                },
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        self.handle_packet(packet);
                    }
                },
            }
        }
    }
}

impl ChatClientDanylo {
    /// ###### Handles incoming packets and delegates them to the appropriate handler based on the packet type.
    fn handle_packet(&mut self, packet: Packet) {
        debug!("Handling packet: {:?}", packet);

        match packet.pack_type.clone() {
            PacketType::Ack(ack) => self.handle_ack(ack.fragment_index, packet.session_id),
            PacketType::Nack(nack) => self.handle_nack(nack, packet.session_id),
            PacketType::MsgFragment(fragment) => {
                // Send acknowledgment for the received fragment
                self.send_ack(fragment.fragment_index, packet.session_id, packet.routing_header.clone());

                // Get the server ID from the routing header and handle the fragment
                let server_id = packet.routing_header.hops.first().unwrap();
                self.handle_fragment(fragment, packet.session_id, *server_id)
            }
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request, packet.session_id),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
        }
    }

    /// ###### Handles incoming commands.
    fn handle_command(&mut self, command: ClientCommand) {
        debug!("Handling command: {:?}", command);

        match command {
            ClientCommand::AddSender(id, sender) => {
                self.packet_send.insert(id, sender);
                info!("Added sender for node {}", id);
            }
            ClientCommand::RemoveSender(id) => {
                self.packet_send.remove(&id);
                info!("Removed sender for node {}", id);
            }
        }
    }

    /// ###### Handles the acknowledgment (ACK) for a given session and fragment.
    /// Processes the acknowledgment for a specific fragment in a session.
    /// If there are more fragments to send, it sends the next fragment.
    /// If all fragments are acknowledged, it removes the session from the queue.
    fn handle_ack(&mut self, fragment_index: u64, session_id: u64) {
        debug!("Handling ACK for session {} and fragment {}", session_id, fragment_index);

        // Retrieve the message fragments for the given session.
        let message = self.messages_to_send.get_mut(&session_id).unwrap();

        // Check if there is a next fragment to send.
        if let Some(next_fragment) = message.get_fragment_packet(fragment_index as usize) {
            // Prepare and send the next fragment if available.
            message.increment_last_index();
            match self.send_to_next_hop(next_fragment) {
                Ok(_) => info!("Sent next fragment for session {}", session_id),
                Err(err) => error!("Failed to send next fragment for session {}: {}", session_id, err),
            }
        } else {
            // All fragments are acknowledged; remove the session.
            self.messages_to_send.remove(&session_id);
            self.response_received = true;
            info!("All fragments acknowledged for session {}", session_id);
        }
    }

    /// ###### Handles the negative acknowledgment (NACK) for a given session.
    /// Processes the NACK for a specific session and takes appropriate action based on the NACK type.
    fn handle_nack(&mut self, nack: Nack, session_id: u64) {
        warn!("Handling NACK for session {}: {:?}", session_id, nack);

        match nack.nack_type {
            NackType::ErrorInRouting(id) => {
                // Set an external error indicating a routing error.
                self.external_error = Some(format!("ErrorInRouting; drone doesn't have neighbor with id {}", id));
            }
            NackType::DestinationIsDrone => {
                // Set an external error indicating the destination is a drone.
                self.external_error = Some("Error: DestinationIsDrone".to_string());
            }
            NackType::UnexpectedRecipient(recipient_id) => {
                // Set an external error indicating an unexpected recipient.
                self.external_error = Some(format!("Error: UnexpectedRecipient (node with id {})", recipient_id));
            }
            // Resend the last packet for the session.
            NackType::Dropped => self.resend_last_packet(session_id),
        }
    }

    /// ###### Resends the last packet for a given session.
    /// Retrieves the last fragment packet for the specified session and attempts to resend it.
    /// Logs the success or failure of the resend operation.
    fn resend_last_packet(&mut self, session_id: u64) {
        debug!("Resending last packet for session {}", session_id);

        let message = self.messages_to_send.get(&session_id).unwrap();
        let packet = message.get_fragment_packet(message.get_last_fragment_index()).unwrap();
        match self.send_to_next_hop(packet) {
            Ok(_) => info!("Resent last fragment for session {}", session_id),
            Err(err) => error!("Failed to resend last fragment for session {}: {}", session_id, err),
        }
    }

    /// ###### Handles a received message fragment.
    /// Adds the fragment to the collection for the session and checks if it is the last fragment.
    /// If it is the last fragment, reassembles the message and processes the server response.
    fn handle_fragment(&mut self, fragment: Fragment, session_id: u64, server_id: NodeId) {
        debug!("Handling fragment for session {}: {:?}", session_id, fragment);

        // Retrieve or create a vector to store fragments for the session.
        let fragments = self.fragments_to_reassemble.entry(session_id).or_insert_with(Vec::new);

        // Add the current fragment to the collection.
        fragments.push(fragment.clone());

        // Check if the current fragment is the last one in the sequence.
        if fragment.fragment_index == fragment.total_n_fragments - 1 {
            // Reassemble the fragments into a complete message and process it.
            let message = self.reassemble(session_id);
            self.handle_server_response(message, server_id);
        }
    }

    /// ###### Handles the server response.
    /// Processes the server response based on its type and takes appropriate actions.
    fn handle_server_response(&mut self, response: Option<Response>, server_id: NodeId) {
        debug!("Handling server response for server {}: {:?}", server_id, response);

        if let Some(response) = response {
            match response {
                Response::ServerType(server_type) => {
                    self.handle_server_type(server_id, server_type);
                },
                Response::ClientRegistered => {
                    self.handle_client_registered(server_id);
                }
                Response::ListClients(clients) => {
                    self.handle_clients_list(clients);
                }
                Response::MessageFrom(from, message) => {
                    info!("New message from {}: {:?}", from, &message);

                    self.inbox.insert(0, (from, message));
                    self.new_messages += 1;
                }
                Response::Err(error) => {
                    error!("Error received from server {}: {}", server_id, error);
                    self.external_error = Some(error);
                }
                _ => {}
            }
        }
    }

    /// ###### Handles the server type response.
    /// Updates the server type in the `servers` map and sets the registration status if the server is of type `Communication`
    /// and marks the response as received.
    fn handle_server_type(&mut self, server_id: NodeId, server_type: ServerType) {
        info!("Server type received successfully.");

        // Insert the server type into the servers map.
        self.servers.insert(server_id, server_type);

        // If the server is of type Communication, set the registration status to false.
        if server_type == ServerType::Communication {
            self.is_registered.insert(server_id, false);
        }

        // Mark the response as received.
        self.response_received = true;
    }

    /// ###### Handles the client registration response.
    /// Updates the registration status for the specified server and marks the response as received.
    fn handle_client_registered(&mut self, server_id: NodeId) {
        info!("Client registered successfully.");

        self.is_registered.insert(server_id, true);
        self.response_received = true;
    }

    /// ###### Handles the list of clients received from the server.
    /// Updates the list of available clients and marks the response as received.
    fn handle_clients_list(&mut self, clients: Vec<NodeId>) {
        info!("List of clients received successfully.");

        self.clients = clients;
        self.response_received = true;
    }

    /// ###### Waits for a response from the server.
    /// This function waits for a response from the server within a specified timeout period.
    /// If an external error is encountered or the timeout is reached, it returns an error.
    /// Otherwise, it resets the response status and returns `Ok(())`.
    pub fn wait_response(&mut self) -> Result<(), String> {
        let timeout = Duration::from_secs(1);
        let start_time = Instant::now();

        while !self.response_received {
            // Check for any external error and return it if found.
            if let Some(error) = self.external_error.take() {
                return Err(error);
            }

            // Check if the timeout has been reached.
            if start_time.elapsed() > timeout {
                return Err("Timeout waiting for server response".to_string());
            }

            // Sleep for a short duration before checking again.
            thread::sleep(Duration::from_millis(100));
        }

        // Reset the response status and return success.
        self.response_received = false;
        Ok(())
    }

    /// ###### Sends an acknowledgment (ACK) for a received fragment.
    /// Creates an ACK packet and sends it to the next hop.
    /// Logs the success or failure of the send operation.
    fn send_ack(&mut self, fragment_index: u64, session_id: u64, routing_header: SourceRoutingHeader) {
        let ack = Packet::new_ack(routing_header, session_id, fragment_index);

        // Attempt to send the ACK packet to the next hop.
        match self.send_to_next_hop(ack) {
            Ok(_) => {
                info!("ACK sent successfully for session {} and fragment {}", session_id, fragment_index);
            }
            Err(err) => {
                error!("Failed to send ACK for session {} and fragment {}: {}", session_id, fragment_index, err);
            }
        };
    }

    /// ###### Handles a flood request by adding the client to the path trace and generating a response.
    fn handle_flood_request(&mut self, mut flood_request: FloodRequest, session_id: u64) {
        debug!("Handling flood request for session {}: {:?}", session_id, flood_request);

        // Add client to the flood request's path trace.
        flood_request.increment(self.id, NodeType::Client);

        // Generate a response for the flood request.
        let response = flood_request.generate_response(session_id);

        // Send the response to the next hop.
        match self.send_to_next_hop(response) {
            Ok(_) => info!("FloodResponse sent successfully."),
            Err(err) => error!("Error sending FloodResponse: {}", err),
        }
    }

    /// ###### Sends the packet to the next hop in the route.
    ///
    /// Attempts to send the packet to the next hop in the route specified by the routing header.
    /// It retrieves the sender for the next hop, increments the hop index, and sends the packet.
    /// If successful, it logs the event and sends a `PacketSent` event to the simulation controller.
    fn send_to_next_hop(&mut self, mut packet: Packet) -> Result<(), String> {
        debug!("Sending packet to next hop: {:?}", packet);

        // Attempt to find the sender for the next hop.
        let Some(sender) = self.get_sender_of_next(packet.routing_header.clone()) else {
            return Err("No sender to the next hop.".to_string());
        };

        // Increment the hop index in the routing header to reflect progress through the route.
        packet.routing_header.increase_hop_index();

        // Attempt to send the updated fragment packet to the next hop.
        if sender.send(packet.clone()).is_err() {
            return Err("Error sending packet to next hop.".to_string());
        }

        // Send the 'PacketSent' event to the simulation controller
        self.send_event(ClientEvent::PacketSent(packet));

        Ok(())
    }

    /// ###### Sends the packet to the next hop and waits for a response.
    ///
    /// Attempts to send the packet to the next hop using the `send_to_next_hop` method.
    /// If the packet is successfully sent, it waits for a response from the server.
    fn send_to_next_hop_and_wait_response(&mut self, packet: Packet) -> Result<(), String> {
        match self.send_to_next_hop(packet) {
            Ok(_) => {
                // Wait for the response to the packet.
                debug!("Waiting for response to packet.");
                self.wait_response()
            }
            Err(err) => {
                Err(err)
            }
        }
    }

    /// ###### Retrieves the sender for the next hop in the routing header.
    ///
    /// This function attempts to retrieve the next hop ID from the routing header.
    /// If the next hop ID is missing, it returns `None` as there is no valid destination to send the packet to.
    /// It then uses the next hop ID to look up the associated sender in the `packet_send` map.
    /// Returns a reference to the sender if it exists, or `None` if not found.
    fn get_sender_of_next(&self, routing_header: SourceRoutingHeader) -> Option<&Sender<Packet>> {
        // Attempt to retrieve the next hop ID from the routing header.
        // If it is missing, return `None` as there is no valid destination to send the packet to.
        let Some(next_hop_id) = routing_header.next_hop() else {
            return None;
        };

        // Use the next hop ID to look up the associated sender in the `packet_send` map.
        // Return a reference to the sender if it exists, or `None` if not found.
        self.packet_send.get(&next_hop_id)
    }

    /// ###### Handles the flood response by updating routes and topology.
    ///
    /// This function processes the received `FloodResponse` by updating the routes and servers
    /// based on the path trace provided in the response. It also updates the network topology
    /// with the new path information and updates the time of the last response.
    fn handle_flood_response(&mut self, flood_response: FloodResponse) {
        debug!("Handling flood response: {:?}", flood_response);

        let path = &flood_response.path_trace;

        self.update_routes_and_servers(path);
        self.update_topology(path);

        self.last_response_time = Some(Instant::now());
    }

    /// ###### Updates the routes and servers based on the provided path.
    /// If the path leads to a server, it updates the routing table and the servers list.
    fn update_routes_and_servers(&mut self, path: &[(NodeId, NodeType)]) {
        if let Some((id, NodeType::Server)) = path.last() {
            if self
                .routes
                .get(id)
                .map_or(true, |prev_path| prev_path.len() > path.len())
            {
                // Add the server to the servers list with an undefined type.
                self.servers.insert(*id, ServerType::Undefined);

                // Update the routing table with the new, shorter path.
                self.routes.insert(
                    *id,
                    path.iter().map(|entry| entry.0.clone()).collect(),
                );
                info!("Updated route to server {}: {:?}", id, path);
            }
        }
    }

    /// ###### Updates the network topology based on the provided path.
    /// Adds connections between nodes in both directions.
    fn update_topology(&mut self, path: &[(NodeId, NodeType)]) {
        for i in 0..path.len() - 1 {
            let current = path[i].0;
            let next = path[i + 1].0;

            // Add the connection between the current and next node in both directions.
            self.topology
                .entry(current)
                .or_insert_with(HashSet::new)
                .insert(next);

            self.topology
                .entry(next)
                .or_insert_with(HashSet::new)
                .insert(current);
        }
        debug!("Updated topology with path: {:?}", path);
    }

    /// ###### Initiates the discovery process to find available servers and clients.
    /// Clears current data structures and sends a flood request to all neighbors.
    pub fn discovery(&mut self) -> Result<(), String> {
        info!("Starting discovery process");

        // Clear all current data structures related to clients, routes, servers, and topology.
        self.clients.clear();
        self.routes.clear();
        self.servers.clear();
        self.topology.clear();

        // Generate a new flood ID, incrementing the last one or starting at 1 if none exists.
        let flood_id = self.flood_ids.last().map_or(1, |last| last + 1);
        self.flood_ids.push(flood_id);

        // Create a new flood request initialized with the generated flood ID, the current node's ID, and its type.
        let flood_request = FloodRequest::initialize(
            flood_id,
            self.id,
            NodeType::Client,
        );

        // Generate a new session ID, incrementing the last one or starting at 1 if none exists.
        let session_id = self.session_ids.last().map_or(1, |last| last + 1);
        self.session_ids.push(session_id);

        // Create a new packet with the flood request and session ID.
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            session_id,
            flood_request,
        );

        // Attempt to send the flood request to all neighbors.
        for sender in &self.packet_send {
            if let Err(_) = sender.1.send(packet.clone()) {
                error!("Failed to send FloodRequest to the drone with id {}.", sender.0);
                return Err(format!("Failed to send FloodRequest to the drone with id {}.", sender.0));
            }
        }

        // Send the 'PacketSent' event to the simulation controller
        self.send_event(ClientEvent::PacketSent(packet));

        // Wait for the discovery process to complete.
        self.last_response_time = Some(Instant::now());
        self.wait_discovery_end(Duration::from_secs(1))
    }

    /// ###### Waits for the discovery process to complete within a specified timeout period.
    /// Returns an error if the timeout is reached.
    pub fn wait_discovery_end(&mut self, timeout: Duration) -> Result<(), String> {
        while let Some(last_response) = self.last_response_time {
            if Instant::now().duration_since(last_response) > timeout {
                info!("Discovery complete!");
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Discovery failed.".to_string())
    }

    /// ###### Requests the type of specified server.
    /// Sends a query to the server and waits for a response.
    pub fn request_server_type(&mut self, server_id: NodeId) -> Result<(), String> {
        info!("Requesting server type for server {}", server_id);

        let result = self.create_and_send_message(Query::AskType, server_id);

        match result {
            Ok(_) => {
                Ok(())
            }
            Err(err) => {
                error!("Failed to receive server type: {}", err);
                Err(err)
            },
        }
    }

    /// ###### Requests to register the client on a specified server.
    /// Sends a registration query to the server and waits for a response.
    pub fn request_to_register(&mut self, server_id: NodeId) -> Result<(), String> {
        info!("Requesting to register on server {}", server_id);

        let result = self.create_and_send_message(Query::RegisterClient(self.id), server_id);

        match result {
            Ok(_) => {
                Ok(())
            }
            Err(err) => {
                error!("Failed to register client: {}", err);
                Err(err)
            },
        }
    }

    /// ###### Requests the list of clients from a specified server.
    /// Sends a query to the server and waits for a response.
    pub fn request_users_list(&mut self, server_id: NodeId) -> Result<(), String> {
        info!("Requesting clients list from server {}", server_id);

        let result = self.create_and_send_message(Query::AskListClients, server_id);

        match result {
            Ok(_) => {
                Ok(())
            }
            Err(err) => {
                error!("Failed to get list of clients: {}", err);
                Err(err)
            },
        }
    }

    /// ###### Sends a message to a specified client via a specified server.
    /// Sends the message and waits for a response.
    pub fn send_message_to(&mut self, to: NodeId, message: Message, server_id: NodeId) -> Result<(), String> {
        info!("Sending message to client {} via server {}", to, server_id);

        let result = self.create_and_send_message(Query::SendMessageTo(to, message), server_id);

        match result {
            Ok(_) => {
                info!("Message sent successfully.");
                Ok(())
            }
            Err(err) => {
                error!("Failed to send message: {}", err);
                Err(err)
            },
        }
    }

    /// ###### Creates and sends a message to a specified server.
    /// Serializes the data, splits it into fragments, and sends the first fragment.
    fn create_and_send_message<T: Serialize + Debug>(&mut self, data: T, server_id: NodeId) -> Result<(), String> {
        debug!("Creating and sending message to server {}: {:?}", server_id, data);

        // Find or create a route.
        let hops = if let Some(route) = self.routes.get(&server_id) {
            route.clone()
        } else {
            return Err(format!("No routes to the server with id {server_id}"));
        };

        // Generate a new session ID.
        let session_id = self.session_ids.last().map_or(1, |last| last + 1);
        self.session_ids.push(session_id);

        // Create message (split the message into fragments) and send first fragment.
        let mut message = MessageFragments::new(session_id, hops);
        if message.create_message_of(data) {
            self.send_to_next_hop_and_wait_response(message.get_fragment_packet(0).unwrap())
        } else {
            Err("Failed to create message.".to_string())
        }
    }

    /// ###### Reassembles the fragments for a given session into a complete message.
    /// Returns the reassembled message or an error if reassembly fails.
    fn reassemble(&mut self, session_id: u64) -> Option<Response> {
        debug!("Reassembling message for session {}", session_id);

        // Retrieve the fragments for the given session.
        let fragments = match self.fragments_to_reassemble.get_mut(&session_id) {
            Some(fragments) => fragments,
            None => {
                error!("No fragments found for session {}", session_id);
                return None;
            },
        };

        // Ensure all fragments belong to the same message by checking the total number of fragments.
        let total_n_fragments = match fragments.first() {
            Some(first) => first.total_n_fragments,
            None => {
                error!("Fragment list is empty for session {}", session_id);
                return None;
            },
        };

        // Check if the number of fragments matches the expected total.
        if fragments.len() as u64 != total_n_fragments {
            error!(
                "Incorrect number of fragments for session {}: expected {}, got {}",
                session_id,
                total_n_fragments,
                fragments.len()
            );
            return None;
        }

        // Collect data from all fragments.
        let mut result = Vec::new();
        for fragment in fragments {
            result.extend_from_slice(&fragment.data[..fragment.length as usize]);
        }

        // Convert the collected data into a string.
        let reassembled_string = match String::from_utf8(result) {
            Ok(string) => string,
            Err(err) => {
                error!(
                    "Failed to convert data to string for session {}: {}",
                    session_id, err
                );
                return None;
            },
        };

        // Attempt to deserialize the string into an object.
        match serde_json::from_str(&reassembled_string) {
            Ok(deserialized) => Some(deserialized),
            Err(err) => {
                error!(
                    "Failed to deserialize JSON for session {}: {}",
                    session_id, err
                );
                None
            },
        }
    }

    /// ###### Sends an event to the simulation controller.
    /// Logs the success or failure of the send operation.
    fn send_event(&self, event: ClientEvent) {
        match event {
            ClientEvent::PacketSent(packet) => {
                match self.controller_send.send(ClientEvent::PacketSent(packet)) {
                    Ok(_) => info!("Sent 'PacketSent' event to controller"),
                    Err(_) => error!("Error sending 'PacketSent' event to controller"),
                }
            }
        }
    }
}
