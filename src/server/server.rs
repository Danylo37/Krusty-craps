//I am quite good now

use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{
        Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType,
    },
};

use crate::general_use::{Message, Query, Response, ServerCommand, ServerEvent, ServerType};

#[derive(Debug)]
pub struct Server {
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
    pub list_users: HashMap<String, NodeId>,
    pub reassembling_messages: HashMap<u64, Vec<u8>>,
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
pub trait CommunicationServer {
    fn add_client(&mut self, nickname: String, client_id: NodeId);
    fn give_list_back(&self, client_id: NodeId);
    fn forward_message_to(&self, nickname: String, message: Message);
}

///Content Server functions
pub trait ContentServer {}

impl Server {
    pub fn new(
        id: NodeId,
        server_type: ServerType,
        connected_drone_ids: Vec<NodeId>,
        to_controller_event: Sender<ServerEvent>,
        from_controller_command: Receiver<ServerCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        Server {
            id,
            server_type,
            connected_drone_ids,
            to_controller_event,
            from_controller_command,
            packet_recv,
            packet_send,
            list_users: HashMap::new(),
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

    ///FLOOD
    pub fn discovery(&self) {
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
    fn send_flood_response() {}

    ///NACK
    fn handle_nack(&mut self, nack: Nack) {}

    fn send_nack(&self, p0: Nack, p1: SourceRoutingHeader, p2: u64) {
        todo!()
    }

    ///ACK
    fn handle_ack(&mut self, _ack: Ack) {
        todo!()
    }
    fn send_ack() {}

    ///PACKET
    fn create_packet(
        pack_type: PacketType,
        routing_header: SourceRoutingHeader,
        session_id: u64,
    ) -> Packet {
        Packet {
            pack_type,
            routing_header,
            session_id,
        }
    }

    fn send_packet(&self, packet: Packet) {
        let first_carrier = self
            .packet_send
            .get(&packet.routing_header.hops[1])
            .unwrap();
        first_carrier.send(packet).unwrap();
    }

    ///HEADER
    fn create_source_routing(route: Vec<NodeId>) -> SourceRoutingHeader {
        SourceRoutingHeader {
            hop_index: 1,
            hops: route,
        }
    }

    ///FRAGMENT

    pub fn send_fragments(&self, id_fragment: u64, session_id: u64, n_fragments: usize, response_in_vec_bytes: &[u8], header: SourceRoutingHeader) {

        for i in 0..n_fragments{

            //Preparing data of fragment
            let data:[u8;128] = response_in_vec_bytes[i*128..(1+i)*128].try_into().unwrap();

            //Generating fragment
            let fragment = Fragment::new(
                id_fragment,
                n_fragments as u64,
                data,
            );

            //Generating packet
            let packet = Self::create_packet(
                PacketType::MsgFragment(fragment),
                header.clone(),
                session_id,
            );

            self.send_packet(packet);
        }
    }
    fn handle_fragment(
        &mut self,
        fragment: Fragment,
        routing_header: SourceRoutingHeader,
        session_id: u64,
    ) {
        // Packet Verification
        if self.id != routing_header.hops[routing_header.hop_index] {
            // Send Nack (UnexpectedRecipient)
            let nack = Nack {
                fragment_index: fragment.fragment_index,
                nack_type: NackType::UnexpectedRecipient(self.id),
            };
            self.send_nack(nack, routing_header.get_reversed(), session_id);
            return;
        }

        ///Fragment reassembly
        // Check if it exists already
        if let Some(reassembling_message) = self.reassembling_messages.get_mut(&session_id) {
            let offset = fragment.fragment_index as usize * 128;

            // Check for valid fragment index and length
            if offset + fragment.length as usize > reassembling_message.capacity()
                || fragment.length as usize > 128
                    && fragment.fragment_index != fragment.total_n_fragments - 1
            {
                // Send Nack and potentially clear the reassembling message
                // ... error handling logic ...
                return;
            }

            // Copy data to the correct offset in the vector.
            reassembling_message[offset..offset + fragment.length as usize]
                .copy_from_slice(&fragment.data[..fragment.length as usize]);

            // Check if all fragments have been received
            if reassembling_message.len() == reassembling_message.capacity() {
                /// Message reassembled!
                // Message Processing
                let reassembled_data = reassembling_message.clone(); // Take ownership of the data
                self.reassembling_messages.remove(&session_id); // Remove from map
                self.process_reassembled_message(reassembled_data, routing_header.hops[0]);

                // Send Ack
                let ack = Ack {
                    fragment_index: fragment.fragment_index,
                };
                self.send_ack(ack, routing_header.get_reversed(), session_id);
            }
        } else {
            // New message, create a new entry in the HashMap.
            let mut reassembling_message = vec![0; fragment.total_n_fragments as usize * 128];
            reassembling_message[0..fragment.length as usize]
                .copy_from_slice(&fragment.data[..fragment.length as usize]);
            self.reassembling_messages
                .insert(session_id, reassembling_message);
        }
    }

    fn process_reassembled_message(&mut self, data: Vec<u8>, src_id: NodeId) {
        match String::from_utf8(data.clone()) {
            Ok(data_string) => match serde_json::from_str(&data_string) {
                Ok(Query::AskType) => self.give_type_back(src_id),
                Ok(Query::AddClient(nickname, node_id)) => self.add_client(nickname, node_id),
                Ok(Query::AskListClients) => self.give_list_back(src_id),
                Ok(Query::SendMessageTo(nickname, message)) => {
                    self.forward_message_to(nickname, message)
                }
                Err(_) => {
                    panic!("Damn, not the right struct")
                }
                _ => {}
            },
            Err(e) => println!("Dio porco, {:?}", e),
        }
        println!("We did it");
    }

    ///Common functions
    pub fn give_type_back(&self, src_id: NodeId) {

        //Get data
        let server_type = self.server_type;

        //Serializing type
        let response_as_string = serde_json::to_string(&server_type).unwrap();
        let response_in_vec_bytes = response_as_string.as_bytes();
        let length_response = response_in_vec_bytes.len();

        //Counting fragments
        let mut n_fragments = length_response / 128+1;
        if n_fragments == 0 {
            n_fragments -= 1;
        }

        //Generating header
        let route = self.find_path_to(src_id);
        let header = Self::create_source_routing(route); //To fill

        // Generating ids
        let id_fragment = self.generate_unique_fragment_id();
        let session_id = self.generate_unique_session_id();

        //Send fragments
        self.send_fragments(id_fragment, session_id, n_fragments, response_in_vec_bytes, header);

    }
}

impl CommunicationServer for Server {
    fn add_client(&mut self, nickname: String, client_id: NodeId) {
        self.list_users.insert(nickname, client_id);
    }

    fn give_list_back(&self, client_id: NodeId) {

        //Get list
        let keys_list_clients = self.list_users.keys().cloned().collect();

        //Creating data to send
        let response = Response::ListUsers(keys_list_clients);

        //Serializing message to send
        let response_as_string = serde_json::to_string(&response).unwrap();
        let response_in_vec_bytes = response_as_string.as_bytes();
        let length_response = response_in_vec_bytes.len();

        //Counting fragments
        let mut n_fragments = length_response / 128+1;
        if n_fragments == 0 {
            n_fragments -= 1;
        }

        //Generating header
        let route: Vec<NodeId> = self.find_path_to(client_id); //To implement findpath
        let header = Self::create_source_routing(route); //To fill

        // Generating ids
        let id_fragment = self.generate_unique_fragment_id();
        let session_id = self.generate_unique_session_id();

        //Send fragments
        self.send_fragments(id_fragment, session_id, n_fragments,response_in_vec_bytes, header);

    }

    fn forward_message_to(&self, nickname: String, message: Message) {

        //Creating data to send
        let response = Response::MessageFrom(nickname.clone(),message);

        //Serializing message to send
        let response_as_string = serde_json::to_string(&response).unwrap();
        let response_in_vec_bytes = response_as_string.as_bytes();
        let length_response = response_in_vec_bytes.len();

        //Counting fragments
        let mut n_fragments = length_response / 128+1;
        if n_fragments == 0 {
            n_fragments -= 1;
        }

        //Generating header
        let route: Vec<NodeId> = self.find_path_to(self.list_users.get(&nickname)); //To implement findpath
        let header = Self::create_source_routing(route); //To fill

        // Generating fragment
        let id_fragment = self.generate_unique_fragment_id();
        let session_id = self.generate_unique_session_id();

        self.send_fragments(id_fragment, session_id, n_fragments,response_in_vec_bytes, header);
    }
}
