//I am a god

use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{
        Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType,
    },
};
use crate::general_use::{Message, Query, Response, ServerCommand, ServerEvent, ServerType};

///SERVER TRAIT
pub trait Server{
    fn get_from_controller_command(&mut self) -> &mut Receiver<ServerCommand>;
    fn get_packet_recv(&mut self) -> &mut Receiver<Packet>;
    fn get_packet_send(&mut self) -> &mut HashMap<NodeId, Sender<Packet>>;
    fn get_packet_send_not_mutable(&self) -> &HashMap<NodeId, Sender<Packet>>;
    fn get_id(&self) -> NodeId;
    fn get_reassembling_messages(&mut self) -> &mut HashMap<u64, Vec<u8>>;
    fn get_server_type(&self) -> ServerType;

    fn run(&mut self){
        loop {
            select_biased! {
                recv(self.get_from_controller_command()) -> command_res => {
                    if let Ok(command) = command_res {
                        match command {
                            ServerCommand::AddSender(id, sender) => {
                                self.get_packet_send().insert(id, sender);

                            }
                            ServerCommand::RemoveSender(id) => {
                                self.get_packet_send().remove(&id);
                            }
                        }
                    }
                },
                recv(self.get_packet_recv()) -> packet_res => {
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

    //FLOOD
    fn discover(&mut self) {
        let flood_request = FloodRequest::initialize(
            self.generate_unique_flood_id(), // Replace with your ID generation
            self.get_id(),
            NodeType::Server,
        );

        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            self.generate_unique_session_id(),
            flood_request.clone(),
        );

        for sender in self.get_packet_send().values() {
            if let Err(e) = sender.send(packet.clone()) {
                eprintln!("Failed to send FloodRequest: {:?}", e);
            }
        }
    }

    fn handle_flood_request(&mut self, flood_request: FloodRequest, session_id: u64){
        todo!();
    }
    fn handle_flood_response(&self, flood_response: FloodResponse, session_id: u64){
        todo!();
    }
    fn send_flood_response(){
        todo!();
    }

    //NACK
    fn handle_nack(&mut self, nack: Nack){
        todo!();
    }
    fn send_nack(&self, p0: Nack, p1: SourceRoutingHeader, p2: u64){
        todo!();
    }

    //ACK
    fn handle_ack(&mut self, _ack: Ack){
        todo!();
    }
    fn send_ack(&self, ack: Ack, routing_header: SourceRoutingHeader, session_id: u64) {
        let packet= Self::create_packet(PacketType::Ack(ack), routing_header, session_id);
        self.send_packet(packet);
        //It is not printing "Ack reveived in client"
    }

    //PACKET
    fn create_packet(pack_type: PacketType, routing_header: SourceRoutingHeader, session_id: u64, ) -> Packet {
        Packet {
            pack_type,
            routing_header,
            session_id,
        }
    }

    fn send_packet(&self, packet: Packet) {
        let first_carrier = self
            .get_packet_send_not_mutable()
            .get(&packet.routing_header.hops[1])
            .unwrap();
        first_carrier.send(packet).unwrap();
    }

    fn find_path_to(&self, destination_id: NodeId) -> Vec<NodeId>{
        todo!();
    }

    fn create_source_routing(route: Vec<NodeId>) -> SourceRoutingHeader{
        SourceRoutingHeader {
            hop_index: 1,
            hops: route,
        }
    }

    //FRAGMENT TO DECIDE IF IMPLEMENTING DEFAULT FOR EACH ONE

    fn handle_fragment(&mut self, fragment: Fragment, routing_header: SourceRoutingHeader, session_id: u64, ){
        // Packet Verification
        if self.get_id() != routing_header.hops[routing_header.hop_index] {
            // Send Nack (UnexpectedRecipient)
            let nack = Nack {
                fragment_index: fragment.fragment_index,
                nack_type: NackType::UnexpectedRecipient(self.get_id()),
            };
            self.send_nack(nack, routing_header.get_reversed(), session_id);
            return;
        }

        //Getting vec of data from fragment
        let mut data_to_add :Vec<u8> = fragment.data.to_vec();
        data_to_add.truncate(fragment.length as usize);

        ///Fragment reassembly
        // Check if it exists already
        if let Some(reassembling_message) = self.get_reassembling_messages().get_mut(&session_id) {
            let offset = reassembling_message.len();
            println!("Qua");

            // Check for valid fragment index and length
            ///????????To check the row below
            ///                 ///????????To check the row below
            if offset + fragment.length as usize > reassembling_message.capacity()
            {
                println!("Nack");
                // Send Nack and potentially clear the reassembling message
                // ... error handling logic ...
                return;

            }else{

                println!("Copy");
                // Copy data to the correct offset in the vector.
                reassembling_message.extend_from_slice(&data_to_add);

                // Check if all fragments have been received
                let reassembling_message_clone = reassembling_message.clone();
                println!("N fragments + current fragment length{}", fragment.total_n_fragments*128 + fragment.length as u64);
                self.if_all_fragments_received_process(&reassembling_message_clone, &fragment, session_id, routing_header);

            }

        }else {
            println!("Qui");
            //Check if it is only 1 fragment
            if !self.if_all_fragments_received_process(&data_to_add, &fragment, session_id, routing_header)
            {
                // New message, create a new entry in the HashMap.
                let mut reassembling_message = Vec::with_capacity(fragment.total_n_fragments as usize * 128);

                //Copying data
                reassembling_message.extend_from_slice(&data_to_add);

                //Inserting message for future fragments
                self.get_reassembling_messages()
                    .insert(session_id, reassembling_message.clone());
            }
        }
    }

    fn if_all_fragments_received_process(&mut self, message: &Vec<u8>, current_fragment: &Fragment, session_id: u64, routing_header: SourceRoutingHeader) -> bool {
        // Message Processing

        println!("Trying sending");
        println!("Message length {} n_fragments {} current.fragment length {}, fragment_index: {}", message.len(), current_fragment.total_n_fragments, current_fragment.length, current_fragment.fragment_index );
        if((message.len() as u64) == ((current_fragment.total_n_fragments-1)*128 + current_fragment.length as u64))
        {
            println!("Sending back");
            let reassembled_data = message.clone(); // Take ownership of the data
            self.get_reassembling_messages().remove(&session_id); // Remove from map
            self.process_reassembled_message(reassembled_data, routing_header.hops[0]);

            // Send Ack
            let ack = Ack {
                fragment_index: current_fragment.fragment_index,
            };
            self.send_ack(ack, routing_header.get_reversed(), session_id);
            return true;
        }
        false
    }
    fn process_reassembled_message(&mut self, data: Vec<u8>, src_id: NodeId);
    fn send_fragments(&self, session_id: u64, n_fragments: usize, response_in_vec_bytes: &[u8], header: SourceRoutingHeader) {

        for i in 0..n_fragments{

            //Preparing data of fragment
            let mut data:[u8;128] = [0;128];
            if((i+1)*128>response_in_vec_bytes.len()){
                data[0..response_in_vec_bytes.len()].copy_from_slice(&response_in_vec_bytes[i*128..response_in_vec_bytes.len()]);
            }else{
                data.copy_from_slice(&response_in_vec_bytes[i*128..(1+i)*128]);
            }

            //Generating fragment
            let fragment = Fragment::new(
                i as u64,
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


    //Common functions
    fn give_type_back(&self, src_id: NodeId){
        println!("We did it");

        //Get data
        let server_type = self.get_server_type();

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
        let session_id = self.generate_unique_session_id();

        //Send fragments
        self.send_fragments(session_id, n_fragments, response_in_vec_bytes, header);
    }

    fn generate_unique_flood_id(&self) -> u64 {
        1
    }
    fn generate_unique_session_id(&self) -> u64 {
        1
    }
}


///Communication Server functions
pub trait CommunicationServer {
    fn add_client(&mut self, nickname: String, client_id: NodeId);
    fn give_list_back(&self, client_id: NodeId);
    fn forward_message_to(&self, nickname: String, message: Message);
}

///Content Server functions
pub trait TextServer {

}

///Media server functions
pub trait MediaServer {

}

