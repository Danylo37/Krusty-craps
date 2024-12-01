use std::collections::{HashMap, HashSet};
use crossbeam_channel::{select, Receiver, Sender};
use wg_2024::network::{NodeId};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType};

struct ClientDanylo {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    packet_send: Sender<Packet>,
    packet_recv: Receiver<Packet>,
    topology: HashMap<NodeId, HashSet<NodeId>>
}

impl ClientDanylo {
    fn new(id: NodeId, connected_drone_ids: Vec<NodeId>, packet_send: Sender<Packet>, packet_recv: Receiver<Packet>) -> Self {
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
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
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

    fn handle_flood_response(&mut self, _flood_response: FloodResponse) {
        todo!()
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
