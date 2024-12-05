//Server file, idk what i am doing (maybe a bit now)

use std::collections::HashMap;
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
    pub controller_send: Sender<ServerEvent>,
    pub controller_recv: Receiver<ServerCommand>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub list_users: Vec<NodeId>,
}

pub trait CommunicationServer{
    fn add_user(&mut self, client_id: NodeId);
    fn get_list(&self) -> Vec<NodeId>;
    fn forward_list(&self);
    fn get_message();
    fn forward_message();
    fn forward_content();
}
pub trait ContentServer{

}

impl Server{
    pub fn new(
            id: NodeId,
            connected_drone_ids: Vec<NodeId>,
            controller_send: Sender<ServerEvent>,
            controller_recv: Receiver<ServerCommand>,
            packet_recv: Receiver<Packet>,
            packet_send: HashMap<NodeId, Sender<Packet>>
    ) -> Self{
        Server{
            id,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send: HashMap::new(),
            list_users: Vec::new(),
        }
    }
    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command_res => {
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
        let flood_request = FloodRequest{
            flood_id: 1,
            initiator_id: self.id,
            path_trace: Vec::new(),
        };
    }


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

    fn reassemble_fragment(){

    }


    fn handle_nack(&mut self, nack: Nack) {

    }

    fn handle_ack(&mut self, _ack: Ack) {
        todo!()
    }

    fn handle_fragment(&mut self, fragment: Fragment, session_id: u64) {

    }
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
