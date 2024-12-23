// TODO: add sending events to the controller and add logging

use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use serde::Serialize;

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType},
};

use crate::general_use::{ClientCommand, ClientEvent, Query, ServerType, Response, Message};

use super::{
    chat_client::ChatClient,
    fragments::MessageFragments,
};

pub struct ChatClientDanylo {
    pub id: NodeId,                                             // Client ID
    pub name: String,                                           // Client name
    pub packet_send: HashMap<NodeId, Sender<Packet>>,           // Neighbor's packet sender channels
    pub packet_recv: Receiver<Packet>,                          // Packet receiver channel
    pub controller_send: Sender<ClientEvent>,                   // Event sender channel
    pub controller_recv: Receiver<ClientCommand>,               // Command receiver channel
    pub servers: HashMap<NodeId, Option<ServerType>>,           // IDs of the available servers and their
    pub users: Vec<String>,                                     // Users
    pub session_ids: Vec<u64>,                                  // Used session IDs
    pub flood_ids: Vec<u64>,                                    // Used flood IDs
    pub floods: HashMap<NodeId, HashSet<u64>>,                  // Flood initiators and their flood IDs
    pub topology: HashMap<NodeId, HashSet<NodeId>>,             // Nodes and their neighbours
    pub routes: HashMap<NodeId, Vec<NodeId>>,                   // Routes to the servers
    pub messages_to_send: HashMap<u64, MessageFragments>,       // Queue of messages to be sent for different sessions
    pub fragments_to_reassemble: HashMap<u64, Vec<Fragment>>,   // Queue of fragments to be reassembled for different sessions
}

impl ChatClient for ChatClientDanylo {
    fn new(
        id: NodeId,
        name: String,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
    ) -> Self {
        Self {
            id,
            name,
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,
            servers: HashMap::new(),
            users: Vec::new(),
            session_ids: Vec::new(),
            flood_ids: Vec::new(),
            floods: HashMap::new(),
            topology: HashMap::new(),
            routes: HashMap::new(),
            messages_to_send: HashMap::new(),
            fragments_to_reassemble: HashMap::new(),
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
    /// ###### Handles incoming packets and dispatches them to the appropriate handler based on the packet type.
    ///
    /// ###### Arguments
    /// * `packet` - The incoming packet to be processed.
    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type.clone() {
            PacketType::Ack(ack) => self.handle_ack(ack.fragment_index, packet.session_id),
            PacketType::Nack(nack) => self.handle_nack(nack, packet.session_id),
            PacketType::MsgFragment(fragment) => {
                self.send_ack(fragment.fragment_index, packet.session_id, packet.routing_header.clone());
                let server_id = packet.routing_header.hops.last().unwrap();
                self.handle_fragment(fragment, packet.session_id, *server_id)
            }
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request, packet.session_id),
            PacketType::FloodResponse(flood_response) => {
                let initiator = flood_response.path_trace.first().unwrap().0;
                if initiator != self.id {
                    self.send_to_next_hop(packet);
                } else {
                    self.handle_flood_response(flood_response);
                }
            },
        }
    }

    /// ###### Handles a client command by performing the appropriate action based on the command type.
    ///
    /// ###### Arguments
    ///
    /// * `command` - The `ClientCommand` to be processed. It can be one of the following:
    ///   - `AddSender(id, sender)`: Adds a sender to the `packet_send` map with the given `id`.
    ///   - `RemoveSender(id)`: Removes a sender associated with the given `id` from the `packet_send` map.
    ///   - `AskTypeTo(server_id)`: Requests the type of the specified server using `server_id`.
    fn handle_command(&mut self, command: ClientCommand) {
        match command {
            ClientCommand::AddSender(id, sender) => {
                self.packet_send.insert(id, sender);
            }
            ClientCommand::RemoveSender(id) => {
                self.packet_send.remove(&id);
            }
            ClientCommand::AskTypeTo(server_id) => {
                self.request_server_type(server_id);
            }
            ClientCommand::StartFlooding => {
                self.discovery();
            }
        }
    }

    /// ###### Handles Ack packets by processing the specified fragment index and session ID.
    ///
    /// If the acknowledged fragment is not the last one, the next fragment is prepared and sent to the next hop.
    /// If all fragments have been acknowledged, the session is removed from the message queue.
    ///
    /// ###### Arguments
    /// * `fragment_index` - The index of the fragment that has been acknowledged.
    /// * `session_id` - The ID of the session associated with the acknowledgment.
    fn handle_ack(&mut self, fragment_index: u64, session_id: u64) {
        let message= self.messages_to_send.get_mut(&session_id).unwrap();

        if let Some(next_fragment) = message.get_fragment_packet(fragment_index as usize) {
            // Prepare and send the next fragment if available.
            message.increment_last_index();
            self.send_to_next_hop(next_fragment);
        } else {
            // All fragments are acknowledged; remove session
            self.messages_to_send.remove(&session_id);
        }
    }

    /// ###### Handles Nack packets by responding appropriately based on the NACK type.
    ///
    /// Depending on the Nack type, the method may trigger route discovery, resend the packet with a new route,
    /// or resend the last packet.
    ///
    /// ###### Arguments
    /// * `nack` - The Nack packet containing the type and details of the issue.
    /// * `session_id` - The ID of the session associated with the NACK.
    fn handle_nack(&mut self, nack: Nack, session_id: u64) {
        // For routing errors, drone destinations, or unexpected recipients, trigger discovery
        // and resend the packet using a new route.
        match nack.nack_type {
            NackType::ErrorInRouting(_)
            | NackType::DestinationIsDrone
            | NackType::UnexpectedRecipient(_) => {
                self.discovery();
                self.resend_with_new_route(session_id);
            }
            // For dropped packets, simply resend the last packet.
            NackType::Dropped => self.resend_last_packet(session_id),
        }
    }

    /// ###### Resends the last packet of a session to the next hop.
    ///
    /// This method retrieves the last fragment packet of the specified session and sends it to the next hop.
    ///
    /// ###### Arguments
    /// * `session_id` - The ID of the session whose last packet should be resent.
    fn resend_last_packet(&self, session_id: u64) {
        let message = self.messages_to_send.get(&session_id).unwrap();
        let packet = message.get_fragment_packet(message.get_last_fragment_index()).unwrap();
        self.send_to_next_hop(packet);
    }

    /// ###### Resends a packet with a new route after discovery.
    ///
    /// This method retrieves the message for the given session, finds a new route to the target server,
    /// updates the route in the message, and resends the last fragment packet. If no route is found,
    /// an error is logged.
    ///
    /// ###### Arguments
    /// * `session_id` - The ID of the session for which the packet should be resent.
    fn resend_with_new_route(&mut self, session_id: u64) {
        // Retrieve the server ID and last fragment index for the given session.
        let (server_id, last_index) = {
            let message = self.messages_to_send.get_mut(&session_id).unwrap();

            let server_id = *message.get_route().last().unwrap();
            (server_id, message.get_last_fragment_index())
        };

        // Attempt to find a new route to the server.
        let new_route = match self.find_route_to(server_id) {
            Some(route) => {
                self.routes.insert(server_id, route.clone());
                route
            },
            None => {
                eprintln!("No routes to the server with id {}", server_id);
                return;
            },
        };

        // Update the route in the message and attempt to resend the last fragment.
        let message = self.messages_to_send.get_mut(&session_id).unwrap();
        message.update_route(new_route);

        if let Some(fragment) = message.get_fragment_packet(last_index) {
            // Increment the last fragment index and send the fragment to the next hop.
            message.increment_last_index();
            self.send_to_next_hop(fragment);
        } else {
            eprintln!(
                "Failed to retrieve fragment at index {} for session {}",
                last_index, session_id
            );
        }
    }

    /// ###### Handles an incoming message fragment, storing it and reassembling the message if all fragments are received.
    ///
    /// This method adds the fragment to the collection of fragments for the specified session.
    /// If the fragment is the last one, the message is reassembled and processed.
    ///
    /// ###### Arguments
    /// * `fragment` - The incoming fragment to be processed.
    /// * `session_id` - The ID of the session associated with the fragment.
    fn handle_fragment(&mut self, fragment: Fragment, session_id: u64, server_id: NodeId) {
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

    /// ###### Handles the server's response based on the provided `Option<Response>` and updates the internal state accordingly.
    ///
    /// ###### Arguments
    ///
    /// * `response` - An optional `Response` from the server. If `None`, no action is taken.
    /// * `server_id` - The ID of the server that sent the response, used to identify which server the response is associated with.
    ///
    /// ###### Response Variants
    ///
    /// - `ServerType(server_type)`: If the `server_type` is `Communication`,
    ///   it inserts the `server_type` into the `servers` map for the given `server_id`.
    ///   Otherwise, it removes the `server_id` from the `servers` map.
    /// - `MessageFrom(from, message)`: Passes the message to the `handle_message` method for further processing.
    /// - `ListUsers(users)`: Updates the list of users with the provided `users`.
    /// - `ListFiles(_)`, `File(_)`, `Media(_)`: Logs a message indicating the client is not a web browser.
    /// - `Err(error)`: Prints the error message to the standard error stream.
    fn handle_server_response(&mut self, response: Option<Response>, server_id: NodeId) {
        if let Some(response) = response {
            match response {
                Response::ServerType(server_type) => {
                    if server_type == ServerType::Communication {
                        self.servers.insert(server_id, Some(server_type));
                    } else {
                        self.servers.remove(&server_id);
                    }
                },
                Response::MessageFrom(from, message) => {
                    self.handle_message(from, message);
                }
                Response::ListUsers(users) => {
                    self.users = users;
                }
                Response::ListFiles(_)
                | Response::File(_)
                | Response::Media(_) => {
                    println!("I'm not a web browser!!!");
                }
                Response::Err(error) => {
                    eprintln!("Occurred an error: {}", error);
                }
            }
        }
    }

    /// ###### Handles and prints a message received from a specified sender.
    ///
    /// ###### Arguments
    ///
    /// * `from` - A `String` representing the sender of the message.
    /// * `message` - A `String` containing the content of the message.
    ///
    /// This function prints the sender's name followed by the message content to the console.
    fn handle_message(&self, from: String, message: Message) {
        println!("Message from {}", from);
        println!("{}", message.text);
    }

    /// ###### Sends an Ack packet for a received fragment.
    ///
    /// This method creates an Ack packet for the specified fragment and session,
    /// using the provided routing header, and sends it to the next hop.
    ///
    /// ###### Arguments
    /// * `fragment_index` - The index of the fragment being acknowledged.
    /// * `session_id` - The ID of the session associated with the fragment.
    /// * `routing_header` - The routing information required to send the ACK packet.
    fn send_ack(&self, fragment_index: u64, session_id: u64, routing_header: SourceRoutingHeader) {
        let ack = Packet::new_ack(routing_header, session_id, fragment_index);
        self.send_to_next_hop(ack)
    }

    /// ###### Handles an incoming flood request by processing its path, ensuring uniqueness, and forwarding it to neighbors.
    ///
    /// This method adds the current node to the flood request's path trace, checks if the flood ID has already been
    /// processed from the same initiator, and either generates a response or forwards the request to neighboring nodes.
    ///
    /// ###### Arguments
    /// * `flood_request` - The flood request to be processed.
    /// * `session_id` - The ID of the session associated with the flood request.
    fn handle_flood_request(&mut self, mut flood_request: FloodRequest, session_id: u64) {
        // Add current drone to the flood request's path trace.
        flood_request.increment(self.id, NodeType::Drone);

        let flood_id = flood_request.flood_id;
        let initiator_id = flood_request.initiator_id;

        // Check if the flood ID has already been received from this flood initiator.
        if self.floods.get(&initiator_id).map_or(false, |ids| ids.contains(&flood_id)) {
            // Generate and send the flood response
            let response = flood_request.generate_response(session_id);
            self.send_to_next_hop(response);
            return;
        }

        // Record the flood ID for the initiator to prevent duplicate processing.
        self.floods
            .entry(initiator_id)
            .or_insert_with(HashSet::new)
            .insert(flood_id);

        // Retrieve the previous node (sender) from the flood request's path trace.
        let Some(sender_id) = self.get_prev_node_id(&flood_request.path_trace) else {
            eprintln!("There's no previous node in the flood path.");
            return;
        };

        // Get all neighboring nodes except the sender.
        let neighbors = self.get_neighbors_except(sender_id);

        // If there are neighbors, forward the flood request to them.
        if !neighbors.is_empty() {
            self.forward_flood_request(neighbors, flood_request);
        } else {
            // If no neighbors, generate and send a response instead.
            let response = flood_request.generate_response(session_id);
            self.send_to_next_hop(response);
        }
    }

    /// ###### Retrieves the sender for the next hop in the routing header.
    ///
    /// This method verifies that the client is the expected recipient of the packet and retrieves
    /// the sender associated with the next hop in the routing header. If any required information is missing
    /// or the client is not the intended recipient, `None` is returned.
    ///
    /// ###### Arguments
    /// * `routing_header` - The source routing header containing hop information.
    ///
    /// ###### Returns
    /// * `Option<&Sender<Packet>>` - A reference to the sender for the next hop, or `None` if unavailable.
    fn get_sender_of_next(&self, routing_header: SourceRoutingHeader) -> Option<&Sender<Packet>> {
        // Attempt to retrieve the current hop ID from the routing header.
        // If it is missing, return `None` as we cannot proceed without it.
        let Some(current_hop_id) = routing_header.current_hop() else {
            return None;
        };

        // Check if the current hop ID matches the client's ID.
        // If it doesn't match, return `None` because the client is not the expected recipient.
        if self.id != current_hop_id {
            return None;
        }

        // Attempt to retrieve the next hop ID from the routing header.
        // If it is missing, return `None` as there is no valid destination to send the packet to.
        let Some(next_hop_id) = routing_header.next_hop() else {
            return None;
        };

        // Use the next hop ID to look up the associated sender in the `packet_send` map.
        // Return a reference to the sender if it exists, or `None` if not found.
        self.packet_send.get(&next_hop_id)
    }

    /// ###### Sends a packet to the next hop in the route.
    ///
    /// This method retrieves the sender for the next hop, increments the hop index in the packet's routing header,
    /// and attempts to send the packet. If the sender is not found or the send operation fails, an error is logged.
    ///
    /// ###### Arguments
    /// * `packet` - The packet to be sent to the next hop.
    fn send_to_next_hop(&self, mut packet: Packet) {
        // Attempt to find the sender for the next hop.
        let Some(sender) = self.get_sender_of_next(packet.routing_header.clone()) else {
            eprintln!("There is no sender to the next hop.");
            return;
        };

        // Increment the hop index in the routing header to reflect progress through the route.
        packet.routing_header.increase_hop_index();

        // Attempt to send the updated fragment packet to the next hop.
        if sender.send(packet).is_err() {
            eprintln!("Error sending the packet to next hop.");
        }
    }

    /// ###### Retrieves the ID of the previous node in the path trace.
    ///
    /// This method checks if the path trace contains at least two nodes and returns the ID of the second-to-last node.
    /// If the path trace has fewer than two nodes, it returns `None`.
    ///
    /// ###### Arguments
    /// * `path_trace` - A vector containing the path trace as pairs of node IDs and their types.
    ///
    /// ###### Returns
    /// * `Option<NodeId>` - The ID of the previous node, or `None` if unavailable.
    fn get_prev_node_id(&self, path_trace: &Vec<(NodeId, NodeType)>) -> Option<NodeId> {
        if path_trace.len() > 1 {
            return Some(path_trace[path_trace.len() - 2].0);
        }
        None
    }

    /// ###### Retrieves all neighboring senders except the specified node ID.
    ///
    /// This method iterates through the `packet_send` map, filters out the sender associated with the `exclude_id`,
    /// and returns a vector of senders for the remaining neighbors.
    ///
    /// ###### Arguments
    /// * `exclude_id` - The ID of the node to be excluded from the list of neighbors.
    ///
    /// ###### Returns
    /// * `Vec<&Sender<Packet>>` - A vector of senders for all neighboring nodes except the excluded one.
    fn get_neighbors_except(&self, exclude_id: NodeId) -> Vec<&Sender<Packet>> {
        self.packet_send
            .iter()
            .filter(|(&node_id, _)| node_id != exclude_id)
            .map(|(_, sender)| sender)
            .collect()
    }

    /// ###### Forwards a flood request to the specified neighbors.
    ///
    /// This method iterates over the provided neighbors and sends the flood request to each one.
    /// A new routing header is created for each request, and the request is sent as a packet.
    /// If sending the packet fails, an error message is logged.
    ///
    /// ###### Arguments
    /// * `neighbors` - A vector of senders for the neighboring nodes to which the flood request will be forwarded.
    /// * `request` - The flood request to be forwarded.
    fn forward_flood_request(
        &self,
        neighbors: Vec<&Sender<Packet>>,
        request: FloodRequest)
    {
        // Iterate over each neighbor
        for sender in neighbors {
            // Create an empty routing header, because this is unnecessary in flood request
            let routing_header = SourceRoutingHeader::empty_route();
            // Create a new FloodRequest
            let packet = Packet::new_flood_request(routing_header, 0, request.clone());

            // Attempt to send the updated fragment packet to the next hop.
            if sender.send(packet.clone()).is_err() {
                eprintln!("Error sending the packet to the neighbor.");
            }
        }
    }

    /// ###### Handles a received flood response and updates the network topology.
    ///
    /// This method processes the path trace in the flood response, adding each pair of consecutive nodes to the topology
    /// to reflect the bidirectional connectivity between them. The topology is updated for both the current and next node
    /// in each step of the path trace.
    ///
    /// ###### Arguments
    /// * `flood_response` - The flood response containing the path trace to be processed.
    fn handle_flood_response(&mut self, flood_response: FloodResponse) {
        let path = &flood_response.path_trace;

        // Iterate through the path trace, excluding the last node.
        for i in 0..path.len() - 1 {
            let current = path[i].0;
            let next = path[i + 1].0;

            if path[i].1 == NodeType::Server {
                self.servers.insert(current, None);
            }
            if path[i + 1].1 == NodeType::Server {
                self.servers.insert(next, None);
            }

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
    }

    /// ###### Initiates a discovery process by sending a flood request to all neighboring nodes.
    ///
    /// This method clears the current topology and server IDs, generates a new flood ID, creates a flood request,
    /// and sends it to all available neighbors. It also generates a new session ID for the discovery process
    /// and logs any errors if the request fails to be sent.
    pub fn discovery(&mut self) {
        // Clear the current topology and server IDs.
        self.topology.clear();
        self.servers.clear();

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
        for sender in self.packet_send.values() {
            if let Err(e) = sender.send(packet.clone()) {
                eprintln!("Failed to send FloodRequest: {:?}", e);
            }
        }
    }

    /// ###### Sends a request to a server asking for its type.
    ///
    /// ###### Arguments
    /// * `server_id` - The ID of the server to which the request will be sent.
    pub fn request_server_type(&mut self, server_id: NodeId) {
        self.create_and_send_message(Query::AskType, server_id);
    }

    /// ###### Sends a request to add the current client to the specified server.
    ///
    /// ###### Arguments
    ///
    /// * `server_id` - The ID of the server to which the request is being sent.
    pub fn request_to_add_client(&mut self, server_id: NodeId) {
        self.create_and_send_message(Query::AddClient(self.name.clone(), self.id), server_id);
    }

    /// ###### Requests the server to provide a list of all clients.
    ///
    /// ###### Arguments
    ///
    /// * `server_id` - The ID of the server from which the list of clients is requested.
    pub fn request_list_clients(&mut self, server_id: NodeId) {
        self.create_and_send_message(Query::AskListClients, server_id);
    }

    /// ###### Sends a message to a specific client through the server.
    ///
    /// ###### Arguments
    ///
    /// * `server_id` - The ID of the server handling the message.
    /// * `to` - The recipient of the message (client's name).
    /// * `message` - The content of the message to be sent.
    pub fn send_message_to(&mut self, server_id: NodeId, to: String, message: Message) {
        self.create_and_send_message(Query::SendMessageTo(to, message), server_id);
    }

    /// ###### Creates and sends a serialized message to a specified server.
    ///
    /// This method finds or creates a route to the server, generates a new session ID, splits the message into fragments,
    /// and sends the first fragment to the next hop. If the route to the server is not available, an error is logged.
    ///
    /// ###### Arguments
    /// * `data` - The data to be serialized and sent as the message.
    /// * `server_id` - The ID of the server to which the message will be sent.
    fn create_and_send_message<T: Serialize>(&mut self, data: T, server_id: NodeId) {
        // Find or create a route.
        let hops = if let Some(route) = self.routes.get(&server_id) {
            route.clone()
        } else if let Some(route) = self.find_route_to(server_id) {
            self.routes.insert(server_id, route.clone());
            route
        } else {
            eprintln!("No routes to the server with id {}", server_id);
            return;
        };

        // Generate a new session ID.
        let session_id = self.session_ids.last().map_or(1, |last| last + 1);
        self.session_ids.push(session_id);

        // Create message (split the message into fragments) and send first fragment.
        let mut message = MessageFragments::new(session_id, hops);
        if message.create_message_of(data) {
            self.send_to_next_hop(message.get_fragment_packet(0).unwrap());
        } else {
            eprintln!("Failed to create message.");
        }
    }

    /// ###### Finds a route from the current node to the specified server using breadth-first search.
    ///
    /// This method explores the network topology starting from the current node, and returns the shortest path
    /// (in terms of hops) to the specified server if one exists. It uses a queue to explore nodes level by level,
    /// ensuring that the first valid path found is the shortest. If no path is found, it returns `None`.
    ///
    /// ###### Arguments
    /// * `server_id` - The ID of the server to which the route is being sought.
    ///
    /// ###### Returns
    /// * `Option<Vec<NodeId>>` - An optional vector representing the path from the current node to the server.
    /// If no route is found, `None` is returned.
    fn find_route_to(&self, server_id: NodeId) -> Option<Vec<NodeId>> {
        // Initialize a queue for breadth-first search and a set to track visited nodes.
        let mut queue: VecDeque<(NodeId, Vec<NodeId>)> = VecDeque::new();
        let mut visited: HashSet<NodeId> = HashSet::new();

        // Start from the current node with an initial path containing just the current node.
        queue.push_back((self.id, vec![self.id]));

        // Perform breadth-first search.
        while let Some((current, path)) = queue.pop_front() {
            // If the destination node is reached, return the path.
            if current == server_id {
                return Some(path);
            }

            // Mark the current node as visited.
            visited.insert(current);

            // Explore the neighbors of the current node.
            if let Some(neighbors) = self.topology.get(&current) {
                for &neighbor in neighbors {
                    // Only visit unvisited neighbors.
                    if !visited.contains(&neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor); // Extend the path to include the neighbor.
                        queue.push_back((neighbor, new_path)); // Add the neighbor to the queue.
                    }
                }
            }
        }
        None    // Return None if no path to the server is found.
    }

    /// ###### Reassembles the fragments of a message for the given session ID and attempts to deserialize the data.
    ///
    /// This method retrieves the fragments for the specified session, checks that the number of fragments matches
    /// the expected total, and combines the fragments into a single string. The string is then deserialized into
    /// an object (using JSON). If any errors occur during these steps, an error message is logged and `None` is returned.
    ///
    /// ###### Arguments
    /// * `session_id` - The ID of the session whose fragments are to be reassembled.
    ///
    /// ###### Returns
    /// * `Option<String>` - The deserialized message as a string if successful, or `None` if any error occurs.
    fn reassemble(&mut self, session_id: u64) -> Option<Response> {
        // Retrieve the fragments for the given session.
        let fragments = match self.fragments_to_reassemble.get_mut(&session_id) {
            Some(fragments) => fragments,
            None => {
                eprintln!("No fragments found for session {}", session_id);
                return None;
            },
        };

        // Ensure all fragments belong to the same message by checking the total number of fragments.
        let total_n_fragments = match fragments.first() {
            Some(first) => first.total_n_fragments,
            None => {
                eprintln!("Fragment list is empty for session {}", session_id);
                return None;
            },
        };

        // Check if the number of fragments matches the expected total.
        if fragments.len() as u64 != total_n_fragments {
            eprintln!(
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
                eprintln!(
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
                eprintln!(
                    "Failed to deserialize JSON for session {}: {}",
                    session_id, err
                );
                None
            },
        }
    }
}
