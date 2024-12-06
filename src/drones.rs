use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::HashMap;
use rand::Rng;

use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};

pub struct KrustyCrapDrone {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    floods: HashMap<NodeId, u64>,
    crashing_behavior: bool,
}

impl Drone for KrustyCrapDrone {
    fn new(
        id: NodeId,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        pdr: f32,
    ) -> Self {
        Self {
            id,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            pdr,
            floods: HashMap::new(),
            crashing_behavior: false,
        }
    }

    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command);
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                },
            }
            if self.crashing_behavior && self.packet_recv.is_empty() {
                break;
            }
        }
    }
}

impl KrustyCrapDrone {
    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type {
            PacketType::Nack(nack) => self.handle_nack(nack, packet.routing_header, packet.session_id),
            PacketType::Ack(ack) => self.handle_ack(ack, packet.routing_header, packet.session_id),
            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment, packet.routing_header, packet.session_id),
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request, packet.session_id),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response, packet.routing_header, packet.session_id),
        }
    }

    fn handle_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(id, sender) => self.add_sender(id, sender),
            DroneCommand::RemoveSender(id) => self.remove_sender(id),
            DroneCommand::SetPacketDropRate(pdr) => self.pdr = pdr,
            DroneCommand::Crash => self.crashing_behavior = true,
        }
    }

    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }

    fn remove_sender(&mut self, id: NodeId) {
        self.packet_send.remove(&id);
    }

    fn handle_nack(
        &mut self,
        nack: Nack,
        mut routing_header: SourceRoutingHeader,
        session_id: u64)
    {
        // Increment the hop index in the routing header to reflect progress through the route
        routing_header.increase_hop_index();

        // Create a new Nack packet using the updated routing header, session ID, and nack
        let packet = Packet::new_nack(routing_header.clone(), session_id, nack);

        // Attempt to send the Nack packet to the next hop in the route
        self.send_to_next(packet, routing_header);
    }

    fn handle_ack(
        &mut self,
        ack: Ack,
        mut routing_header: SourceRoutingHeader,
        session_id: u64)
    {
        // Increment the hop index in the routing header to reflect progress through the route
        routing_header.increase_hop_index();

        // Create a new Ack packet using the updated routing header, session ID and ack fragment_index
        let packet = Packet::new_ack(routing_header.clone(), session_id, ack.fragment_index);

        // Attempt to send the Ack packet to the next hop in the route
        self.send_to_next(packet, routing_header);
    }

    fn handle_fragment(
        &mut self,
        fragment: Fragment,
        mut routing_header:
        SourceRoutingHeader,
        session_id: u64)
    {
        // Check if the drone is in a crashing state
        // If so, send a Nack
        if self.crashing_behavior {
            // TODO: Send Nack (ErrorInRouting(self.id))
            return;
        }

        // Retrieve the current hop from the routing header
        // If it doesn't exist, send a Nack
        let Some(current_hop_id) = routing_header.current_hop() else {
            // TODO: Send Nack (UnexpectedRecipient(self.id))
            return;
        };
        // If the current hop isn't the drone's ID, send a Nack
        if self.id != current_hop_id {
            // TODO: Send Nack (UnexpectedRecipient(self.id))
            return;
        }

        // Retrieve the next hop from the routing header
        // If it doesn't exist, send a Nack
        let Some(next_hop_id) = routing_header.next_hop() else {
            // TODO: Send Nack (DestinationIsDrone)
            return;
        };

        // Attempt to find the sender for the next hop
        // If the sender isn't found, send a Nack
        let Some(sender) = self.packet_send.get(&next_hop_id) else {
            // TODO: Send Nack (ErrorInRouting(next_hop_id))
            return;
        };

        // Simulate packet drop based on the PDR
        // If the random number is less than PDR, drop the packet (send a Nack)
        if rand::rng().random_range(0.0..1.0) < self.pdr {
            // TODO: Send Nack (Dropped)
            return;
        }

        // Increment the hop index in the routing header to reflect progress through the route
        routing_header.increase_hop_index();

        // Create a new Fragment packet using the updated routing header, session ID and fragment
        let packet = Packet::new_fragment(routing_header, session_id, fragment);

        // Attempt to send the updated fragment packet to the next hop
        if let Err(_) = sender.send(packet) {
            // TODO: idk what to do
        }
    }

    fn handle_flood_request(&mut self, mut flood_request: FloodRequest, session_id: u64) {
        flood_request.increment(self.id, NodeType::Drone);

        let flood_id = flood_request.flood_id;
        let initiator_id = flood_request.initiator_id;

        // Flood ID has already been received from this flood initiator
        if self.floods.contains_key(&initiator_id) &&
            self.floods.get(&initiator_id).unwrap().to_owned() == flood_id {
            let response = flood_request.generate_response(session_id);
            self.send_flood_response(response, flood_request.path_trace);
            return;
        }

        // Flood ID has not yet been received from this flood initiator
        self.floods.insert(initiator_id, flood_id);

        if let Some(sender_id) = self.get_prev_node_id(&flood_request.path_trace) {
            let neighbors = self.get_neighbors_except(sender_id);

            if !neighbors.is_empty() {
                self.forward_flood_request(neighbors, flood_request, session_id);
            } else {
                let response = flood_request.generate_response(session_id);
                self.send_flood_response(response, flood_request.path_trace);
            }
        } else {
            // TODO: idk what to do
        }
    }

    fn send_flood_response(&self, response: Packet, path_trace: Vec<(NodeId, NodeType)>) {
        // Getting the previous node from path_trace
        let Some(prev_node_id) = self.get_prev_node_id(&path_trace) else {
            // TODO: Send the packet through the simulation controller
            return;
        };

        // Getting the send channel for the previous node
        let Some(sender) = self.packet_send.get(&prev_node_id) else {
            // TODO: Send the packet through the simulation controller
            return;
        };

        // Sending a package
        if let Err(_) = sender.send(response) {
            // TODO: Send the packet through the simulation controller
        }
    }

    fn get_prev_node_id(&self, path_trace: &[(NodeId, NodeType)]) -> Option<NodeId> {
        match path_trace.len() {
            0 => {
                None
            }
            1 => {
                None
            }
            _ => {
                let last = path_trace.last().unwrap();
                if last.0 != self.id {
                    Some(last.0)
                } else {
                    Some(path_trace[path_trace.len() - 2].0)
                }
            }
        }
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
        for sender in neighbors {
            let routing_header = SourceRoutingHeader::empty_route();
            let packet = Packet::new_flood_request(routing_header, session_id, request.clone());

            // Sending a package
            if let Err(_) = sender.send(packet) {
                // TODO: idk what to do
            }
        }
    }

    fn handle_flood_response(
        &mut self,
        flood_response: FloodResponse,
        mut routing_header: SourceRoutingHeader,
        session_id: u64)
    {
        // Updating hop_index
        routing_header.increase_hop_index();

        // Creating a new FloodResponse package
        let packet = Packet::new_flood_response(routing_header.clone(), session_id, flood_response.clone());

        self.send_to_next(packet, routing_header);
    }

    fn get_sender_of_next(&self, routing_header: SourceRoutingHeader) -> Option<&Sender<Packet>> {
        let Some(current_hop_id) = routing_header.current_hop() else {
            return None;
        };

        if self.id != current_hop_id {
            return None;
        }

        let Some(next_hop_id) = routing_header.next_hop() else {
            return None;
        };

        self.packet_send.get(&next_hop_id)
    }

    fn send_to_next(&self, packet: Packet, routing_header: SourceRoutingHeader) {
        let Some(sender) = self.get_sender_of_next(routing_header) else {
            // TODO: Send the packet through the simulation controller
            return;
        };

        // Sending a package
        if let Err(_) = sender.send(packet) {
            // TODO: Send the packet through the simulation controller
        }
    }
}
