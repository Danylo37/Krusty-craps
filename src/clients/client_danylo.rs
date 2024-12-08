use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType, NodeType},
};

use crate::general_use::{ClientCommand, ClientEvent, Query};

pub struct ClientDanylo {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<ClientEvent>,
    controller_recv: Receiver<ClientCommand>,
    session_ids: Vec<u64>,
    flood_ids: Vec<u64>,
    floods: HashMap<NodeId, HashSet<u64>>,
    topology: HashMap<NodeId, HashSet<NodeId>>,
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
                        match packet.pack_type {
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
                            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request),
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
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

    fn handle_flood_request(&self, _flood_request: FloodRequest) {
        todo!()
    }

    fn handle_flood_response(&self, _flood_response: FloodResponse) {
        todo!()
    }

    fn discovery(&mut self) {
        let flood_id = self.flood_ids.last().unwrap().to_owned() + 1;
        let flood_request = FloodRequest::initialize(
            flood_id,
            self.id,
            NodeType::Client,
        );
        self.flood_ids.push(flood_id);

        let session_id = self.flood_ids.last().unwrap().to_owned() + 1;
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            session_id,
            flood_request,
        );
        self.session_ids.push(session_id);

        for sender in self.packet_send.values() {
            if let Err(e) = sender.send(packet.clone()) {
                eprintln!("Failed to send FloodRequest: {:?}", e);
            }
        }
    }

    fn request_server_type(&self, server_id: NodeId) {
        let query = Query::AskType;
        let ask_type_query_string = serde_json::to_string(&query).unwrap();

        let query_in_vec_bytes = ask_type_query_string.as_bytes();
        let length_response = query_in_vec_bytes.len();

        //Counting fragments
        let mut n_fragments = length_response / 128+1;
        if n_fragments == 0 {
            n_fragments -= 1;
        }
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
