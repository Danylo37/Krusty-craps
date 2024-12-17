use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType},
};

use crate::general_use::{ClientCommand, ClientEvent, HashNodeType};

pub struct ClientDanylo {
    id: NodeId,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<ClientEvent>,
    controller_recv: Receiver<ClientCommand>,
    session_ids: Vec<u64>,
    flood_ids: Vec<u64>,
    floods: HashMap<NodeId, HashSet<u64>>,
    topology: HashMap<(NodeId, HashNodeType), HashSet<(NodeId, HashNodeType)>>,
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
            session_ids: vec![0],
            flood_ids: vec![0],
            floods: HashMap::new(),
            topology: HashMap::new(),
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
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
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

    fn handle_ack(&self, _ack: Ack) {
        todo!()
    }

    fn handle_nack(&self, _nack: Nack) {
        todo!()
    }

    fn handle_fragment(&self, _fragment: Fragment) {
        todo!()
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
            eprintln!("Error sending the packet to next hop.");
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
            let current = (path[i].0, HashNodeType(path[i].1));
            let next = (path[i + 1].0, HashNodeType(path[i + 1].1));

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
        let flood_id = self.flood_ids.last().unwrap() + 1;
        self.flood_ids.push(flood_id);

        let flood_request = FloodRequest::initialize(
            flood_id,
            self.id,
            NodeType::Client,
        );

        let session_id = self.flood_ids.last().unwrap() + 1;
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

    fn request_server_type(&self, _server_id: NodeId) {
        todo!()
    }

    fn request_files_list(&self) {
        todo!()
    }

    fn request_file(&self) {
        todo!()
    }

    fn request_media(&self) {
        todo!()
    }

    fn serialize_message(&self) {
        todo!()
    }

    fn fragment_message(&self) {
        todo!()
    }

    fn reassemble_message(&self) {
        todo!()
    }
}
