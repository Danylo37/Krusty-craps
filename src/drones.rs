use crossbeam_channel::{select, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use wg_2024::controller::Command;
use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};

struct KrustyCrapDrone {
    id: NodeId,
    sim_contr_send: Sender<Command>,
    sim_contr_recv: Receiver<Command>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    flood_ids: HashSet<u64>,
}

impl Drone for KrustyCrapDrone {
    fn new(options: DroneOptions) -> Self {
        Self {
            id: options.id,
            sim_contr_send: options.sim_contr_send,
            sim_contr_recv: options.sim_contr_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr,
            packet_send: options.packet_send,
            flood_ids: HashSet::new(),
        }
    }

    fn run(&mut self) {
        loop {
            select! {
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match packet.pack_type {
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
                            PacketType::FloodRequest(mut flood_request) => self.handle_flood_request(&mut flood_request, packet.clone()),
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
                        }
                    }
                },
                recv(self.sim_contr_recv) -> command_res => {
                    if let Ok(_command) = command_res {
                            todo!();
                    }
                }
            }
        }
    }
}

impl KrustyCrapDrone {

    fn handle_nack(&mut self, _nack: Nack) {
        todo!()
    }

    fn handle_ack(&mut self, _ack: Ack) {
        todo!()
    }

    fn handle_fragment(&mut self, _fragment: Fragment) {
        todo!()
    }

    fn handle_flood_request(&mut self, flood_request: &mut FloodRequest, prev_packet: Packet) {
        // adding this drone to path_trace of the flood
        flood_request.path_trace.push((self.id, NodeType::Drone));

        // flood ID has already been received
        if self.flood_ids.contains(&flood_request.flood_id) {

            // getting the previous node index from path_trace
            let last_index = flood_request.path_trace.len() - 2;

            // getting the ID of the previous node from path_trace
            if let Some((last_node_id, _)) = flood_request.path_trace.get(last_index) {
                // checking if there's a sender channel for the previous node
                if let Some(sender) = self.packet_send.get(last_node_id) {
                    self.send_flood_response_to_prev_node(sender, flood_request, prev_packet.session_id);
                }
            }
            return;
        }

        // flood ID has not yet been received
        self.flood_ids.insert(flood_request.flood_id);

        // getting the previous node index from path_trace
        let last_index = flood_request.path_trace.len() - 2;

        // getting the ID of the previous node from path_trace
        let sender_id = if let Some((node_id, _)) =
            flood_request.path_trace.get(last_index) { *node_id }
        else { return };

        // getting neighbors without the sender
        let neighbors: Vec<_> = self.packet_send
            .iter()
            .filter(|(&node_id, _)| node_id != sender_id)
            .collect();

        if !neighbors.is_empty() {
            // forwarding the FloodRequest to all neighbors except the sender
            for (&neighbor_id, sender) in neighbors {
                let packet = Packet {
                    pack_type: PacketType::FloodRequest(flood_request.clone()),
                    routing_header: Default::default(), // I guess routing_header isn't important since it's FloodRequest
                    session_id: prev_packet.session_id,
                };
                sender.send(packet).unwrap();
            }
        } else {
            // sending a FloodResponse back to the sender if there is no neighbors
            if let Some(sender) = self.packet_send.get(&sender_id) {
                self.send_flood_response_to_prev_node(sender, flood_request, prev_packet.session_id);
            }
        }
    }

    fn send_flood_response_to_prev_node(&self, sender: &Sender<Packet>, flood_request: &FloodRequest, session_id: u64) {
        // reversing path_trace for the routing header
        let reversed_path = flood_request.path_trace
            .iter()
            .map(|(node_id, _)| *node_id)
            .rev()
            .collect();

        // creating the FloodResponse packet
        let packet = Packet {
            pack_type: PacketType::FloodResponse(
                FloodResponse {
                    flood_id: flood_request.flood_id,
                    path_trace: flood_request.path_trace.clone()}),
            routing_header: SourceRoutingHeader {
                hop_index: 1,
                hops: reversed_path,
            },
            session_id,
        };

        // sending the FloodResponse packet to the previous node
        sender.send(packet).unwrap();
    }

    fn handle_flood_response(&mut self, _flood_response: FloodResponse) {
        todo!()
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
}
