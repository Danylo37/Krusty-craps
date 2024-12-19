use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use serde::Serialize;
use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType},
};
use crate::general_use::{ClientCommand, ClientEvent, Query};

pub struct ClientDanylo {
    id: NodeId,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<ClientEvent>,
    controller_recv: Receiver<ClientCommand>,
    session_ids: Vec<u64>,
    flood_ids: Vec<u64>,
    floods: HashMap<NodeId, HashSet<u64>>,
    topology: HashMap<NodeId, HashSet<NodeId>>,
    routes: HashMap<NodeId, Vec<NodeId>>,
    packets_to_send: HashMap<u64, Vec<Packet>>,
    fragments_to_reassemble: HashMap<u64, Vec<Fragment>>,
    last_fragment_indexes: HashMap<u64, usize>,
}

impl ClientDanylo {
    pub fn new(
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
            session_ids: Vec::new(),
            flood_ids: Vec::new(),
            floods: HashMap::new(),
            topology: HashMap::new(),
            routes: HashMap::new(),
            packets_to_send: HashMap::new(),
            fragments_to_reassemble: HashMap::new(),
            last_fragment_indexes: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
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
                },
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match packet.pack_type.clone() {
                            PacketType::Ack(ack) => self.handle_ack(ack.fragment_index, packet.session_id),
                            PacketType::Nack(nack) => self.handle_nack(nack, packet.session_id),
                            PacketType::MsgFragment(fragment) => {
                                self.send_ack(fragment.fragment_index, packet.session_id, packet.routing_header);
                                self.handle_fragment(fragment, packet.session_id)
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
                },
            }
        }
    }

    fn handle_ack(&mut self, fragment_index: u64, session_id: u64) {
        if let Some(packets) = self.packets_to_send.get_mut(&session_id) {
            let next_index = fragment_index as usize + 1;

            if next_index < packets.len() {
                // Get the next fragment and send it
                let next_fragment = packets[next_index].clone();
                self.send_to_next_hop(next_fragment);
                self.last_fragment_indexes.insert(session_id, next_index);

            } else {
                // All fragments are acknowledged; remove session
                self.packets_to_send.remove(&session_id);
            }
        } else {
            eprintln!("Error: session_id {} not found", session_id);
        }
    }

    fn handle_nack(&mut self, nack: Nack, session_id: u64) {
        match nack.nack_type {
            NackType::ErrorInRouting(_)
            | NackType::DestinationIsDrone
            | NackType::UnexpectedRecipient(_) => {
                self.discovery();
                self.resend_with_new_route();
            }
            NackType::Dropped => self.resend_last_packet(session_id),
        }
    }

    fn resend_last_packet(&self, session_id: u64) {
        let last_fragment_index = *self.last_fragment_indexes.get(&session_id).unwrap();
        let last_fragment = self.packets_to_send.get(&session_id).unwrap()[last_fragment_index].clone();
        self.send_to_next_hop(last_fragment);
    }

    fn handle_fragment(&mut self, fragment: Fragment, session_id: u64) {
        // Get or create a vector of fragments for the current session_id
        let fragments = self.fragments_to_reassemble.entry(session_id).or_insert_with(Vec::new);

        // Add the current fragment
        fragments.push(fragment.clone());

        // If this is the last fragment, we reassemble the message
        if fragment.fragment_index == fragment.total_n_fragments - 1 {
            self.reassemble_message(session_id);
        }
    }

    fn send_ack(&self, fragment_index: u64, session_id: u64, routing_header: SourceRoutingHeader) {
        let ack = Packet::new_ack(routing_header, session_id, fragment_index);
        self.send_to_next_hop(ack)
    }

    fn handle_flood_request(&mut self, mut flood_request: FloodRequest, session_id: u64) {
        // Add current drone to the flood request's path trace
        flood_request.increment(self.id, NodeType::Drone);

        let flood_id = flood_request.flood_id;
        let initiator_id = flood_request.initiator_id;

        // Check if the flood ID has already been received from this flood initiator
        if self.floods.get(&initiator_id).map_or(false, |ids| ids.contains(&flood_id)) {
            // Generate and send the flood response
            let response = flood_request.generate_response(session_id);
            self.send_to_next_hop(response);
            return;
        }

        // If Flood ID has not yet been received from this flood initiator
        self.floods
            // Use the 'entry' method to get access to the entry with the key 'initiator_id'
            .entry(initiator_id)
            // If the entry doesn't exist, create a new 'HashSet' using 'or_insert_with'
            .or_insert_with(HashSet::new)
            // Insert 'flood_id' into the found or newly created 'HashSet'
            .insert(flood_id);

        // Check if there's a previous node (sender) in the flood path
        // If the sender isn't found, print an error
        let Some(sender_id) = self.get_prev_node_id(&flood_request.path_trace) else {
            eprintln!("There's no previous node in the flood path.");
            return;
        };

        // Get all neighboring nodes except the sender
        let neighbors = self.get_neighbors_except(sender_id);

        // If there are neighbors, forward the flood request to them
        if !neighbors.is_empty() {
            self.forward_flood_request(neighbors, flood_request, session_id);
        } else {
            // If no neighbors, generate and send a response instead
            let response = flood_request.generate_response(session_id);
            self.send_to_next_hop(response);
        }
    }

    fn get_sender_of_next(&self, routing_header: SourceRoutingHeader) -> Option<&Sender<Packet>> {
        // Attempt to retrieve the current hop ID from the routing header
        // If it is missing, return `None` as we cannot proceed without it
        let Some(current_hop_id) = routing_header.current_hop() else {
            return None;
        };

        // Check if the current hop ID matches this drone's ID
        // If it doesn't match, return `None` because this drone is not the expected recipient
        if self.id != current_hop_id {
            return None;
        }

        // Attempt to retrieve the next hop ID from the routing header
        // If it is missing, return `None` as there is no valid destination to send the packet to
        let Some(next_hop_id) = routing_header.next_hop() else {
            return None;
        };

        // Use the next hop ID to look up the associated sender in the `packet_send` map
        // Return a reference to the sender if it exists, or `None` if not found
        self.packet_send.get(&next_hop_id)
    }

    fn send_to_next_hop(&self, mut packet: Packet) {
        // Attempt to find the sender for the next hop
        let Some(sender) = self.get_sender_of_next(packet.routing_header.clone()) else {
            eprintln!("There is no sender to the next hop.");
            return;
        };

        // Increment the hop index in the routing header to reflect progress through the route
        packet.routing_header.increase_hop_index();

        // Attempt to send the updated fragment packet to the next hop
        if sender.send(packet).is_err() {
            eprintln!("Error sending the packet to next hop.");
        }
    }

    fn get_prev_node_id(&self, path_trace: &Vec<(NodeId, NodeType)>) -> Option<NodeId> {
        if path_trace.len() > 1 {
            return Some(path_trace[path_trace.len() - 2].0);
        }
        None
    }

    fn get_neighbors_except(&self, exclude_id: NodeId) -> Vec<&Sender<Packet>> {
        self.packet_send
            .iter()
            .filter(|(&node_id, _)| node_id != exclude_id)
            .map(|(_, sender)| sender)
            .collect()
    }

    fn forward_flood_request(
        &self,
        neighbors: Vec<&Sender<Packet>>,
        request: FloodRequest,
        session_id: u64)
    {
        // Iterate over each neighbor
        for sender in neighbors {
            // Create an empty routing header, because this is unnecessary in flood request
            let routing_header = SourceRoutingHeader::empty_route();
            // Create a new FloodRequest
            let packet = Packet::new_flood_request(routing_header, session_id, request.clone());

            // Attempt to send the updated fragment packet to the next hop.
            if sender.send(packet.clone()).is_err() {
                eprintln!("Error sending the packet to the neighbor.");
            }
        }
    }

    fn handle_flood_response(&mut self, flood_response: FloodResponse) {
        let path = &flood_response.path_trace;

        for i in 0..path.len() - 1 {
            let current = path[i].0;
            let next = path[i + 1].0;

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

    fn discovery(&mut self) {
        let flood_id = self.flood_ids.last().map_or(1, |last| last + 1);
        self.flood_ids.push(flood_id);

        let flood_request = FloodRequest::initialize(
            flood_id,
            self.id,
            NodeType::Client,
        );

        let session_id = self.session_ids.last().map_or(1, |last| last + 1);
        self.session_ids.push(session_id);

        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            session_id,
            flood_request,
        );

        for sender in self.packet_send.values() {
            if let Err(e) = sender.send(packet.clone()) {
                eprintln!("Failed to send FloodRequest: {:?}", e);
            }
        }
    }

    fn request_server_type(&mut self, server_id: NodeId) {
        self.create_and_send_message(Query::AskType, server_id);
    }

    fn find_route_to(&self, server_id: NodeId) -> Option<Vec<NodeId>> {
        let mut queue: VecDeque<(NodeId, Vec<NodeId>)> = VecDeque::new();
        let mut visited: HashSet<NodeId> = HashSet::new();

        queue.push_back((self.id, vec![self.id]));

        while let Some((current, path)) = queue.pop_front() {
            if current == server_id {
                return Some(path);
            }

            visited.insert(current);

            if let Some(neighbors) = self.topology.get(&current) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor);
                        queue.push_back((neighbor, new_path));
                    }
                }
            }
        }
        None
    }

    fn request_files_list(&mut self, server_id: NodeId) {
        self.create_and_send_message(Query::AskListFiles, server_id);
    }

    fn request_file(&mut self, server_id: NodeId, file: u8) {
        self.create_and_send_message(Query::AskFile(file), server_id);
    }

    fn request_media(&mut self, server_id: NodeId, media: String) {
        self.create_and_send_message(Query::AskMedia(media), server_id);
    }

    fn create_and_send_message<T: Serialize>(&mut self, message: T, server_id: NodeId) {
        let serialized_message = match serde_json::to_string(&message) {
            Ok(string) => string,
            Err(e) => {
                eprintln!("Failed to serialize message: {}", e);
                return;
            }
        };

        let fragments = self.fragment_message(&serialized_message);

        let hops = if let Some(route) = self.routes.get(&server_id) {
            route.clone()
        } else if let Some(route) = self.find_route_to(server_id) {
            self.routes.insert(server_id, route.clone());
            route
        } else {
            eprintln!("There is no routes to the server with id {}", server_id);
            return;
        };

        let routing_header = SourceRoutingHeader {
            hop_index: 0,
            hops: hops.clone(),
        };

        let session_id = self.session_ids.last().map_or(1, |last| last + 1);
        self.session_ids.push(session_id);

        self.packets_to_send.insert(session_id, Vec::new());
        for fragment in fragments {
            let packet = Packet {
                routing_header: routing_header.clone(),
                session_id,
                pack_type: PacketType::MsgFragment(fragment),
            };
            self.packets_to_send.get_mut(&session_id).unwrap().push(packet);
        }

        self.send_to_next_hop(self.packets_to_send.get(&session_id).unwrap().get(0).unwrap().clone());
        self.last_fragment_indexes.insert(session_id, 0);
    }

    fn fragment_message(&self, serialized_msg: &str) -> Vec<Fragment> {
        let serialized_msg_in_bytes = serialized_msg.as_bytes();
        let length_response = serialized_msg_in_bytes.len();

        let n_fragments = (length_response + 127) / 128;
        (0..n_fragments)
            .map(|i| {
                let start = i * 128;
                let end = ((i + 1) * 128).min(length_response);
                let fragment_data = &serialized_msg[start..end];
                Fragment::from_string(i as u64, n_fragments as u64, fragment_data.to_string())
            })
            .collect()
    }

    fn reassemble_message(&self, _session_id: u64) {
        todo!()
    }
}

struct Message {
    fragments_to_send: Vec<Fragment>,
    last_fragment_index: usize,
    session_id: u64,
    route: Vec<NodeId>,
}

impl Message {
    pub fn new(session_id: u64, route: Vec<NodeId>) -> Message {
        Self {
            fragments_to_send: Vec::new(),
            last_fragment_index: 0,
            session_id,
            route,
        }
    }

    pub fn create_message_of<T: Serialize>(&mut self, data: T) {
        let serialized_message = match serde_json::to_string(&data) {
            Ok(string) => string,
            Err(e) => {
                eprintln!("Failed to serialize message {}", e);
                return;
            }
        };
        self.fragments_to_send = self.fragment(&serialized_message);
    }

    pub fn fragment(&mut self, serialized_msg: &str) -> Vec<Fragment> {
        let serialized_msg_in_bytes = serialized_msg.as_bytes();
        let length_response = serialized_msg_in_bytes.len();

        let n_fragments = (length_response + 127) / 128;
        (0..n_fragments)
            .map(|i| {
                let start = i * 128;
                let end = ((i + 1) * 128).min(length_response);
                let fragment_data = &serialized_msg[start..end];
                Fragment::from_string(i as u64, n_fragments as u64, fragment_data.to_string())
            })
            .collect()
    }

    pub fn update_route(&mut self, route: Vec<NodeId>) {
        self.route = route;
    }

    pub fn reassemble(&mut self) {
        todo!()
    }

    pub fn get_next_fragment_packet(&mut self) -> Option<Packet> {
        let Some(fragment) = self.fragments_to_send.get(self.last_fragment_index).cloned(); {
            let hops = self.route.clone();
            let routing_header = SourceRoutingHeader {
                hop_index: 0,
                hops: hops.clone(),
            };

            let packet = Packet {
                routing_header: routing_header.clone(),
                session_id: self.session_id,
                pack_type: PacketType::MsgFragment(fragment),
            };
            Some(packet)
        };
        None
    }

    pub fn increment_last_fragment_index(&mut self) {
        self.last_fragment_index += 1;
    }
}
