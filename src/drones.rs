use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use wg_2024::controller::Command;
use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};

pub struct KrustyCrapDrone {
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
            select_biased! {
                recv(self.sim_contr_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        match command {
                            Command::AddChannel(id, sender) => self.add_channel(id, sender),
                            Command::RemoveChannel(_) => {}
                            Command::Crash => {}
                        }
                    }
                },
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match packet.pack_type {
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
                            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request, packet.session_id),
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response, packet.session_id),
                        }
                    }
                },
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

    fn handle_flood_request(&mut self, flood_request: FloodRequest, session_id: u64) {
        // flood ID has already been received
        if self.flood_ids.contains(&flood_request.flood_id) {
            self.send_flood_response(flood_request.path_trace, flood_request.flood_id, session_id);
            return;
        }

        // flood ID has not yet been received
        self.flood_ids.insert(flood_request.flood_id);

        if let Some(sender_id) = self.get_prev_node_id(&flood_request.path_trace) {
            let neighbors = self.get_neighbors_except(sender_id);

            if !neighbors.is_empty() {
                let mut new_path_trace = flood_request.path_trace.clone();
                new_path_trace.push((self.id, NodeType::Drone));

                // forwarding the FloodRequest to all neighbors except the sender
                for sender in neighbors {
                    let packet = Packet {
                        pack_type: PacketType::FloodRequest(
                            FloodRequest {
                                path_trace: new_path_trace.clone(),
                                ..flood_request
                            }),
                        routing_header: SourceRoutingHeader {   // isn't important since it's FloodRequest
                            hop_index: 0,
                            hops: vec![],
                        },
                        session_id,
                    };
                    if let Err(e) = sender.send(packet) {
                        eprintln!("Failed to send packet: {:?}", e);
                    }
                }
            } else {
                // sending a FloodResponse back to the sender
                self.send_flood_response(flood_request.path_trace, flood_request.flood_id, session_id);
            }
        }
    }

    fn send_flood_response(&self, path_trace: Vec<(NodeId, NodeType)>, flood_id: u64, session_id: u64) {
        let mut new_path_trace = path_trace.clone();
        new_path_trace.push((self.id, NodeType::Drone));

        if let Some (prev_node_id) = self.get_prev_node_id(&path_trace) {
            if let Some(sender) = self.get_sender_of(prev_node_id) {

                let reversed_path = new_path_trace
                    .iter()
                    .map(|(node_id, _)| *node_id)
                    .rev()
                    .collect();

                let packet = Packet {
                    pack_type: PacketType::FloodResponse(
                        FloodResponse {
                            flood_id,
                            path_trace: new_path_trace,
                        }),
                    routing_header: SourceRoutingHeader {
                        hop_index: 1,
                        hops: reversed_path,
                    },
                    session_id,
                };

                if let Err(e) = sender.send(packet) {
                    eprintln!("Failed to send packet: {:?}", e);
                }
            }
        }

    }

    fn get_prev_node_id(&self, path_trace: &[(NodeId, NodeType)]) -> Option<NodeId> {
        path_trace.last().map(|(node_id, _)| *node_id)
    }

    fn get_sender_of(&self, prev_node_id: NodeId) -> Option<&Sender<Packet>> {
        self.packet_send.get(&prev_node_id)
    }

    fn get_neighbors_except(&self, exclude_id: NodeId) -> Vec<&Sender<Packet>> {
        self.packet_send
            .iter()
            .filter(|(&node_id, _)| node_id != exclude_id)
            .map(|(_, sender)| sender)
            .collect()
    }

    fn handle_flood_response(&mut self, flood_response: FloodResponse, session_id: u64) {
        self.send_flood_response(flood_response.path_trace, flood_response.flood_id, session_id);
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
}
