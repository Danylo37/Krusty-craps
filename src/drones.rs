use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};

pub struct KrustyCrapDrone {
    id: NodeId,
    controller_send: Sender<NodeEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    flood_ids: HashSet<u64>,
}

impl Drone for KrustyCrapDrone {
    fn new(options: DroneOptions) -> Self {
        Self {
            id: options.id,
            controller_send: options.controller_send,
            controller_recv: options.controller_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr,
            packet_send: HashMap::new(),
            flood_ids: HashSet::new(),
        }
    }

    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        if let DroneCommand::Crash = command {
                            println!("drone {} crashed", self.id);
                            break;
                        }
                        self.handle_command(command);
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                },
            }
        }
    }
}

impl KrustyCrapDrone {
    fn handle_packet(&mut self, packet: Packet) {
        match packet.pack_type {
            PacketType::Nack(nack) => self.handle_nack(nack),
            PacketType::Ack(ack) => self.handle_ack(ack),
            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request, packet.session_id),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response, packet.routing_header, packet.session_id),
        }
    }
    fn handle_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(id, sender) => self.add_channel(id, sender),
            DroneCommand::SetPacketDropRate(pdr) => self.pdr = pdr,
            DroneCommand::Crash => unreachable!(),
        }
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }

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
        let mut new_path_trace = flood_request.path_trace.clone();
        new_path_trace.push((self.id, NodeType::Drone));

        // flood ID has already been received
        if self.flood_ids.contains(&flood_request.flood_id) {
            self.send_flood_response(new_path_trace, flood_request.flood_id, session_id);
            return;
        }

        // flood ID has not yet been received
        self.flood_ids.insert(flood_request.flood_id);

        if let Some(sender_id) = self.get_prev_node_id(&new_path_trace) {
            let neighbors = self.get_neighbors_except(sender_id);

            if !neighbors.is_empty() {
                self.forward_flood_request(neighbors, flood_request, new_path_trace, session_id);
            } else {
                self.send_flood_response(new_path_trace, flood_request.flood_id, session_id);
            }
        } else {
            eprintln!("Could not determine previous node for FloodRequest");
        }
    }

    fn send_flood_response(
        &self,
        path_trace: Vec<(NodeId, NodeType)>,
        flood_id: u64,
        session_id: u64
    ) {
        // Getting the previous node from path_trace
        let Some(prev_node_id) = self.get_prev_node_id(&path_trace) else {
            eprintln!("No previous node found in path_trace for flood_id {}", flood_id);
            return;
        };

        // Getting the send channel for the previous node
        let Some(sender) = self.get_sender_of(prev_node_id) else {
            eprintln!("No sender found for node_id {}", prev_node_id);
            return;
        };

        let reversed_path = path_trace
            .iter()
            .map(|(node_id, _)| *node_id)
            .rev()
            .collect();

        let packet = Packet {
            pack_type: PacketType::FloodResponse(
                FloodResponse {
                    flood_id,
                    path_trace,
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

    fn get_prev_node_id(&self, path_trace: &[(NodeId, NodeType)]) -> Option<NodeId> {
        match path_trace.len() {
            0 => {
                eprintln!("Path trace is empty!");
                None
            }
            1 => {
                eprintln!("Path trace has only one node, assuming no previous node.");
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

    fn forward_flood_request(
        &self,
        neighbors: Vec<&Sender<Packet>>,
        flood_request: FloodRequest,
        new_path_trace: Vec<(NodeId, NodeType)>,
        session_id: u64,
    ) {
        for sender in neighbors {
            let packet = Packet {
                pack_type: PacketType::FloodRequest(FloodRequest {
                    path_trace: new_path_trace.clone(),
                    ..flood_request
                }),
                routing_header: SourceRoutingHeader { hop_index: 0, hops: vec![] },
                session_id,
            };
            if let Err(e) = sender.send(packet) {
                eprintln!("Failed to forward FloodRequest: {:?}", e);
            }
        }
    }


    fn handle_flood_response(
        &mut self,
        flood_response: FloodResponse,
        mut routing_header: SourceRoutingHeader,
        session_id: u64,
    ) {
        // Getting the previous node from path_trace
        let Some(prev_node_id) = self.get_prev_node_id(&flood_response.path_trace) else {
            eprintln!("No previous node found in path_trace for flood_id {}", flood_response.flood_id);
            return;
        };

        // Getting the send channel for the previous node
        let Some(sender) = self.get_sender_of(prev_node_id) else {
            eprintln!("No sender found for node_id {}", prev_node_id);
            return;
        };

        // Updating hop_index
        routing_header.hop_index += 1;

        // Creating a new FloodResponse package
        let packet = Packet {
            pack_type: PacketType::FloodResponse(flood_response),
            routing_header,
            session_id,
        };

        // Sending a package
        if let Err(e) = sender.send(packet) {
            eprintln!("Failed to send FloodResponse packet: {:?}", e);
        }
    }
}
