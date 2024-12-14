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

use super::server::MediaServer as CharTrait;
use super::server::Server as MainTrait;

#[derive(Debug)]
pub struct MediaServer{

    //Basic data
    pub id: NodeId,
    pub server_type: ServerType,
    pub connected_drone_ids: Vec<NodeId>,
    pub flood_ids: HashSet<u64>,
    pub reassembling_messages: HashMap<u64, Vec<u8>>,

    //Channels
    pub to_controller_event: Sender<ServerEvent>,
    pub from_controller_command: Receiver<ServerCommand>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
}

impl MediaServer{
    pub fn new(
        id: NodeId,
        server_type: ServerType,
        connected_drone_ids: Vec<NodeId>,
        to_controller_event: Sender<ServerEvent>,
        from_controller_command: Receiver<ServerCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        MediaServer {
            id,
            server_type,
            connected_drone_ids,
            flood_ids: Default::default(),
            reassembling_messages: Default::default(),

            to_controller_event,
            from_controller_command,
            packet_recv,
            packet_send,
        }
    }
}

impl MainTrait for MediaServer{
    fn process_reassembled_message(&mut self, data: Vec<u8>, src_id: NodeId){
        match String::from_utf8(data.clone()) {
            Ok(data_string) => match serde_json::from_str(&data_string) {
                Ok(Query::AskType) => self.give_type_back(src_id),
                Err(_) => {
                    panic!("Damn, not the right struct")
                }
                _ => {}
            },
            Err(e) => println!("Dio porco, {:?}", e),
        }
        println!("process reassemble finished");
    }

    fn get_from_controller_command(&mut self) -> &mut Receiver<ServerCommand>{ &mut self.from_controller_command }
    fn get_packet_recv(&mut self) -> &mut Receiver<Packet>{ &mut self.packet_recv }
    fn get_packet_send(&mut self) -> &mut HashMap<NodeId, Sender<Packet>>{ &mut self.packet_send }
    fn get_packet_send_not_mutable(&self) -> &HashMap<NodeId, Sender<Packet>>{ &self.packet_send }
    fn get_id(&self) -> NodeId{ self.id }
    fn get_reassembling_messages(&mut self) -> &mut HashMap<u64, Vec<u8>>{ &mut self.reassembling_messages }
    fn get_server_type(&self) -> ServerType{ self.server_type }
}