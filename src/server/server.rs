//I think i know quite good now

use std::collections::{HashMap, HashSet};
use crossbeam_channel::{select_biased, Receiver, Sender};
use std::fmt::Debug;

use wg_2024::{
    network::{NodeId,SourceRoutingHeader},
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType},
};

use crate::general_use::{ServerCommand, ServerEvent, Message, ServerType};

#[derive(Debug)]
pub struct Server{
    //Basic data
    pub id: NodeId,
    pub server_type: ServerType,
    pub connected_drone_ids: Vec<NodeId>,
    pub flood_ids: HashSet<u64>,

    //Channels
    pub to_controller_event: Sender<ServerEvent>,
    pub from_controller_command: Receiver<ServerCommand>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,

    //Server data
    pub list_users: Vec<NodeId>,
    pub reassembling_messages: HashMap<u64,Vec<u8>>,
    pub messages: Vec<Message>,
}

///    Node Functions:
/// new
/// run
/// handle_flood_request
/// send_flood_response
/// handle_nack
/// handle_ack
/// handle_fragment

//     Server Functions
// discovery
// reassemble_fragment
// send_back_type



///Communication Server functions
pub trait CommunicationServer{
    fn add_user(&mut self, client_id: NodeId);
    fn get_list(&self) -> Vec<NodeId>;
    fn forward_list(&self);
    fn get_message();
    fn forward_message();
    fn forward_content();
}

///Content Server functions
pub trait ContentServer{

}

impl Server{
    pub fn new(
            id: NodeId,
            server_type: ServerType,
            connected_drone_ids: Vec<NodeId>,
            to_controller_event: Sender<ServerEvent>,
            from_controller_command: Receiver<ServerCommand>,
            packet_recv: Receiver<Packet>,
            packet_send: HashMap<NodeId, Sender<Packet>>,
            list_users: Vec<NodeId>,
    ) -> Self{
        Server{
            id,
            server_type,
            connected_drone_ids,
            to_controller_event,
            from_controller_command,
            packet_recv,
            packet_send,
            list_users,
            flood_ids: Default::default(),
            reassembling_messages: Default::default(),
            messages: Vec::new(),
        }
    }
    pub fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.from_controller_command) -> command_res => {
                    if let Ok(command) = command_res {
                        match command {
                            ServerCommand::AddSender(id, sender) => {
                                self.packet_send.insert(id, sender);

                            }
                            ServerCommand::RemoveSender(id) => {
                                self.packet_send.remove(&id);
                            }
                        }
                    }
                },
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match packet.pack_type {
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment, packet.routing_header ,packet.session_id),
                            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request, packet.session_id),
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response, packet.session_id),
                        }
                    }
                },
            }
        }
    }

    pub fn discovery(&self){

        let flood_request = FloodRequest::initialize(
            self.generate_unique_flood_id(), // Replace with your ID generation
            self.id,
            NodeType::Server,
        );

        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            self.generate_unique_session_id(),
            flood_request.clone(),
        );

        for sender in self.packet_send.values() {

            if let Err(e) = sender.send(packet.clone()) {
                eprintln!("Failed to send FloodRequest: {:?}", e);
            }
        }
    }

    fn handle_flood_request(&mut self, flood_request: FloodRequest, session_id: u64) {}
    fn send_flood_response(){}

    fn handle_nack(&mut self, nack: Nack) {}

    fn send_nack(&self, p0: Nack, p1: SourceRoutingHeader, p2: u64) {
        todo!()
    }

    fn handle_ack(&mut self, _ack: Ack) {
        todo!()
    }
    fn send_ack(){}

    pub fn give_type_back(&self){
        let server_type = self.server_type;
        let server_type_string = serde_json::to_string(&server_type).unwrap();
        let n_fragments = server_type_string.clone().as_bytes().len();
        let fragment = Fragment::from_string(0, n_fragments as u64, server_type_string);
        Self::send_message(Self::create_message(PacketType::MsgFragment(fragment), Vec::new(), 0));
    }

    //Handling messages?
    fn create_message(pack_type: PacketType, route: Vec<NodeId>, session_id: u64 )->Packet{
        Packet{
            pack_type,
            routing_header: Self::create_source_routing(route),
            session_id,
        }
        // Remember fragment.from_string()
    }

    fn send_message(packet0: Packet){}

    fn create_source_routing(route: Vec<NodeId>) -> SourceRoutingHeader {
        SourceRoutingHeader {
            hop_index: 1,
            hops: route,
        }
    }

    fn handle_fragment(&mut self, fragment: Fragment, routing_header: SourceRoutingHeader, session_id: u64) {
        // 1. Packet Verification
        if self.id != routing_header.hops[routing_header.hop_index] {
            // Send Nack (UnexpectedRecipient)
            let nack = Nack {
                fragment_index: fragment.fragment_index,
                nack_type: NackType::UnexpectedRecipient(self.id),
            };
            self.send_nack(nack, routing_header.get_reversed(), session_id);
            return;
        }

        // 2. Fragment Reassembly
        let message_key = (session_id, routing_header.hops); // Assuming src_id is the first hop

        // Entry for this message exists?
        if let Some(reassembling_message) = self.reassembling_messages.get_mut(&message_key.0) {
            let offset = fragment.fragment_index as usize * 128;

            // Check for valid fragment index and length
            if offset + fragment.length as usize > reassembling_message.capacity()
                || fragment.length as usize > 128 && fragment.fragment_index != fragment.total_n_fragments - 1
            {
                // Send Nack and potentially clear the reassembling message
                // ... error handling logic ...
                return;
            }

            // Copy data to the correct offset in the vector.
            reassembling_message[offset..offset + fragment.length as usize].copy_from_slice(&fragment.data[..fragment.length as usize]);

            // Check if all fragments have been received
            if reassembling_message.len() == reassembling_message.capacity() {
                // Message reassembled!
                // 3. Message Processing
                let reassembled_data = reassembling_message.clone(); // Take ownership of the data
                self.reassembling_messages.remove(&message_key.0); // Remove from map
                self.process_reassembled_message(reassembled_data, session_id, routing_header.hops.clone());

                // Send Ack
                let ack = Ack { fragment_index: fragment.fragment_index };
                self.send_ack(ack, routing_header.get_reversed(), session_id);
            }
        } else {
            // New message, create a new entry in the HashMap.
            let mut reassembling_message = vec![0; fragment.total_n_fragments as usize * 128];
            reassembling_message[0..fragment.length as usize].copy_from_slice(&fragment.data[..fragment.length as usize]);
            self.reassembling_messages.insert(message_key, reassembling_message);
        }
    }

    fn process_reassembled_message(&mut self, data: Vec<u8>, session_id: u64, src_id: Vec<NodeId>) {
        match String::from_utf8(data){
            Result::Ok(data_string) => {
                match serde_json::from_str(&data_string) {
                    Result::Ok(Message) => self.messages.push(Message),
                    Result::Err(_) => {panic!("Damn, not the right struct")}
                }
            }
            Result::Err(e) => println!("Dio porco, {:?}", e),
        }
        println!("We did it");
    }
}

impl CommunicationServer for Server{
    fn add_user(&mut self, client_id: NodeId) {
        self.list_users.push(client_id);
    }

    fn get_list(&self) -> Vec<NodeId> {
        self.list_users.clone()
    }

    fn forward_list(&self) {}

    fn get_message() {
        todo!()
    }

    fn forward_message() {
        todo!()
    }

    fn forward_content() {
        todo!()
    }
}
