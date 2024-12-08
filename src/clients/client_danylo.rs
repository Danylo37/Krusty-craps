use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};

use wg_2024::{
    network::NodeId,
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType},
};
use wg_2024::network::SourceRoutingHeader;
use crate::general_use::{ClientCommand, ClientEvent, Query};

pub struct ClientDanylo {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<ClientEvent>,
    controller_recv: Receiver<ClientCommand>,
    topology: HashMap<NodeId, HashSet<NodeId>>,
    floods: HashMap<NodeId, HashSet<u64>>,
}

impl ClientDanylo {
    pub fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,
            topology: HashMap::new(),
            floods: HashMap::new(),
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
                        match packet.pack_type {
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
                            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request),
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
                        }
                    }
                },
            }
        }
    }

    fn discovery(&mut self) {}

    fn handle_ack(&self, _ack: Ack) {
        todo!()
    }

    fn handle_nack(&self, _nack: Nack) {
        todo!()
    }

    fn handle_fragment(&self, _fragment: Fragment) {
        todo!()
    }

    fn handle_flood_request(&self, _flood_request: FloodRequest) {
        todo!()
    }

    fn handle_flood_response(&self, _flood_response: FloodResponse) {
        todo!()
    }
    fn start_flooding(&self) {
        todo!()
    }

    fn request_server_type(&self, server_id: NodeId) {
        // ???????????????????????????????????
        let query = Query::AskType;
        let ask_type_query_string = serde_json::to_string(&query).unwrap();
        let n_fragments = ask_type_query_string.clone().as_bytes().len();
        let fragment = Fragment::from_string(0, n_fragments as u64, ask_type_query_string);

        let hop_index = 1;
        let hops = vec![1, 2, 3, server_id];
        let routing_header = SourceRoutingHeader {
            hop_index,
            hops: hops.clone(),
        };
        
        let packet = Packet {
            routing_header,
            session_id: 0,
            pack_type: PacketType::MsgFragment(fragment),
        };

        let next_hop = self.packet_send.get(&hops[1]).unwrap();
        next_hop.send(packet).unwrap();
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
