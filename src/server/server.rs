//Server file, idk what i am doing (maybe a bit now)

use std::collections::{HashMap, HashSet};
use crossbeam_channel::{select, select_biased, Receiver, Sender};
use std::fmt::Debug;

use wg_2024::{
    network::NodeId,
    drone::Drone,
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType},
};
use wg_2024::controller::DroneCommand;
use wg_2024::network::SourceRoutingHeader;
use crate::general_use::{ServerCommand, ServerEvent};

#[derive(Debug)]
pub struct Server{
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
    pub to_controller_event: Sender<ServerEvent>,
    pub from_controller_command: Receiver<ServerCommand>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub list_users: Vec<NodeId>,
    pub flood_ids: HashSet<u64>,
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
            connected_drone_ids: Vec<NodeId>,
            to_controller_event: Sender<ServerEvent>,
            from_controller_command: Receiver<ServerCommand>,
            packet_recv: Receiver<Packet>,
            packet_send: HashMap<NodeId, Sender<Packet>>,
            list_users: Vec<NodeId>,
    ) -> Self{
        Server{
            id,
            connected_drone_ids,
            to_controller_event,
            from_controller_command,
            packet_recv,
            packet_send,
            list_users,
            flood_ids: Default::default(),
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
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment, packet.session_id),
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

    pub fn give_type_back(&self){}

    //Handling messages?
    fn create_message(pack_type: PacketType, route: Vec<NodeId>, session_id: u64 )->Packet{
        Packet{
            pack_type,
            routing_header: Self::create_source_routing(route),
            session_id,
        }
    }

    fn create_source_routing(route: Vec<NodeId>) -> SourceRoutingHeader{
        SourceRoutingHeader {
            hop_index: 1,
            hops: route,
        }
    }

    fn reassemble_fragment(){}

    fn handle_nack(&mut self, nack: Nack) {}

    fn handle_ack(&mut self, _ack: Ack) {
        todo!()
    }

    fn handle_fragment(&mut self, fragment: Fragment, session_id: u64) {}

}

impl CommunicationServer for Server{
    fn add_user(&mut self, client_id: NodeId) {
        self.list_users.push(client_id);
    }

    fn get_list(&self) -> Vec<NodeId> {
        self.list_users.clone()
    }

    fn forward_list(&self) {

    }

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
