use std::collections::{HashMap, HashSet};
use crossbeam_channel::{select, Receiver, Sender};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NodeType, Packet, PacketType};

struct ClientDanylo {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    topology: HashMap<NodeId, HashSet<NodeId>>
}

impl ClientDanylo {
    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            packet_send,
            packet_recv,
            topology: HashMap::new(),
        }
    }

    fn run(&mut self) {
        loop {
            select! {
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        match packet.pack_type {
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
                            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request),
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response, packet.routing_header, packet.session_id),
                        }
                    }
                }
            }
        }
    }

    fn handle_ack(&mut self, _ack: Ack) {
        todo!()
    }

    fn handle_nack(&mut self, _nack: Nack) {
        todo!()
    }

    fn handle_fragment(&mut self, _fragment: Fragment) {
        todo!()
    }

    fn start_flooding(&mut self) {
        todo!()
    }

    fn handle_flood_request(&mut self, _flood_request: FloodRequest) {
        todo!()
    }

    fn handle_flood_response(
        &mut self,
        flood_response: FloodResponse,
        mut routing_header: SourceRoutingHeader,
        session_id: u64,
    )
    {
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

    fn serialize_message(&mut self) {
        todo!()
    }

    fn fragment_message(&mut self) {
        todo!()
    }

    fn reassemble_message(&mut self) {
        todo!()
    }
}
