use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::fmt::Debug;
//use eframe::egui::accesskit::NodeId as ui_node_id;
use wg_2024::{
    network::{NodeId},
    packet::{
        Packet
    },
};
use crate::general_use::{Message, Query, Response, ServerCommand, ServerEvent, ServerType};

use super::server::CommunicationServer as CharTrait;
use super::server::Server as MainTrait;

type FloodId = u64;
type SessionId = u64;

#[derive(Debug)]
pub struct CommunicationServer{

    //Basic data
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,

    //Fragment-related
    pub reassembling_messages: HashMap<SessionId, Vec<u8>>,
    pub sending_messages: HashMap<SessionId, (Vec<u8>, NodeId)>,

    //Flood-related
    pub clients: Vec<NodeId>,                                   // Available clients
    pub topology: HashMap<NodeId, Vec<NodeId>>,             // Nodes and their neighbours
    pub routes: HashMap<NodeId, Vec<NodeId>>,                   // Routes to the servers
    pub flood_ids: Vec<FloodId>,
    pub counter: (FloodId, SessionId),

    //Channels
    pub to_controller_event: Sender<ServerEvent>,
    pub from_controller_command: Receiver<ServerCommand>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,

    //Characteristic-Server fields
    pub list_users: Vec<NodeId>,
}

impl CommunicationServer{
    pub fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        to_controller_event: Sender<ServerEvent>,
        from_controller_command: Receiver<ServerCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        CommunicationServer {
            id,
            connected_drone_ids,

            reassembling_messages: Default::default(),
            sending_messages: Default::default(),

            clients: Default::default(),                                   // Available clients
            topology: Default::default(),
            routes: Default::default(),
            flood_ids: Default::default(),
            counter: (0, 0),

            to_controller_event,
            from_controller_command,
            packet_recv,
            packet_send,

            list_users: Vec::new(),
        }
    }
}

impl MainTrait for CommunicationServer{
    fn get_id(&self) -> NodeId{ self.id }
    fn get_server_type(&self) -> ServerType{ ServerType::Communication }

    fn get_session_id(&mut self) -> u64{
        self.counter.1 += 1;
        self.counter.1
    }

    fn get_flood_id(&mut self) -> u64{
        self.counter.0 += 1;
        self.counter.0
    }

    fn push_flood_id(&mut self, flood_id: FloodId){ self.flood_ids.push(flood_id); }
    fn get_clients(&mut self) -> &mut Vec<NodeId>{ &mut self.clients }
    fn get_topology(&mut self) -> &mut HashMap<NodeId, Vec<NodeId>>{ &mut self.topology }
    fn get_routes(&mut self) -> &mut HashMap<NodeId, Vec<NodeId>>{ &mut self.routes }

    fn get_from_controller_command(&mut self) -> &mut Receiver<ServerCommand>{ &mut self.from_controller_command }
    fn get_packet_recv(&mut self) -> &mut Receiver<Packet>{ &mut self.packet_recv }
    fn get_packet_send(&mut self) -> &mut HashMap<NodeId, Sender<Packet>>{ &mut self.packet_send }
    fn get_packet_send_not_mutable(&self) -> &HashMap<NodeId, Sender<Packet>>{ &self.packet_send }
    fn get_reassembling_messages(&mut self) -> &mut HashMap<u64, Vec<u8>>{ &mut self.reassembling_messages }
    fn get_sending_messages(&mut self) ->  &mut HashMap<u64, (Vec<u8>, u8)>{ &mut self.sending_messages }


    fn process_reassembled_message(&mut self, data: Vec<u8>, src_id: NodeId){
        match String::from_utf8(data.clone()) {
            Ok(data_string) => match serde_json::from_str(&data_string) {
                Ok(Query::AskType) => self.give_type_back(src_id),

                Ok(Query::RegisterClient(node_id)) => self.add_client(node_id),
                Ok(Query::AskListClients) => self.give_list_back(src_id),
                Ok(Query::SendMessageTo(node_id, message)) => self.forward_message_to(node_id, message),
                Err(_) => {
                    panic!("Damn, not the right struct")
                }
                _ => {}
            },
                Err(e) => println!("Argh, {:?}", e),
            }
    }

}

impl CharTrait for CommunicationServer {
    fn add_client(&mut self, client_id: NodeId) {
        self.list_users.push(client_id);

        let response = Response::ClientRegistered;

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
        let route: Vec<NodeId> = self.find_path_to(client_id);
        let header = Self::create_source_routing(route);

        // Generating ids
        let session_id = self.generate_unique_session_id();

        //Send fragments
        self.send_fragments(session_id, n_fragments,response_in_vec_bytes, header);
    }

    fn give_list_back(&mut self, client_id: NodeId) {

        //Get list
        let list_clients = self.list_users.clone();

        //Creating data to send
        let response = Response::ListClients(list_clients);

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
        let route: Vec<NodeId> = self.find_path_to(client_id);
        let header = Self::create_source_routing(route);

        // Generating ids
        let session_id = self.generate_unique_session_id();

        //Send fragments
        self.send_fragments(session_id, n_fragments,response_in_vec_bytes, header);

    }

    fn forward_message_to(&mut self, destination_id: NodeId, message: Message) {

        //Creating data to send
        let response = Response::MessageFrom(destination_id,message);

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
        let route: Vec<NodeId> = self.find_path_to(*self.list_users.get(&destination_id).unwrap());
        let header = Self::create_source_routing(route);

        // Generating fragment
        let session_id = self.generate_unique_session_id();

        self.send_fragments(session_id, n_fragments,response_in_vec_bytes, header);
    }
}

