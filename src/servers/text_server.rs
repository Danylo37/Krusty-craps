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

use super::server::TextServer as CharTrait;
use super::server::Server as MainTrait;

use super::content::TEXT;

#[derive(Debug)]
pub struct TextServer{

    //Basic data
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
    pub flood_ids: HashSet<u64>,
    pub reassembling_messages: HashMap<u64, Vec<u8>>,
    pub counter: (u64, u64),

    //Channels
    pub to_controller_event: Sender<ServerEvent>,
    pub from_controller_command: Receiver<ServerCommand>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,

    //Characteristic-Server fields
    pub content: Vec<String>,
}

impl TextServer{
    pub fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        to_controller_event: Sender<ServerEvent>,
        from_controller_command: Receiver<ServerCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        let content = Self::prepare_content();
        TextServer {
            id,
            connected_drone_ids,
            flood_ids: Default::default(),
            reassembling_messages: Default::default(),
            counter: (0, 0),

            to_controller_event,
            from_controller_command,
            packet_recv,
            packet_send,

            content: Vec::new(),
        }
    }
    fn prepare_content() -> Vec<String>{
        let mut content = TEXT.to_vec();

        let len = content.len();
        if len == 0 {
            return Vec::new();
        }else if len < 4 {
            // If the Vec has less than 4 elements, remove one element if possible
            content.pop();
            return content.into_iter().map(|value| value.to_string()).collect::<Vec<String>>();
        }

        // Calculate the step size for removing elements
        let step = len / 4;

        // Retain only the elements that aren't in the removal step
        content.into_iter()
            .enumerate()
            .filter(|(index, _)| (index + 1) % step != 0)
            .map(|(_, value)| value.to_string())
            .collect::<Vec<String>>()
    }
}

impl MainTrait for TextServer{
    fn get_id(&self) -> NodeId{ self.id }

    fn get_server_type(&self) -> ServerType{ ServerType::Text }
    fn get_flood_id(&mut self) -> u64{
        self.counter.0 += 1;
        self.counter.0
    }
    fn get_session_id(&mut self) -> u64{
        self.counter.1 += 1;
        self.counter.1
    }
    fn get_from_controller_command(&mut self) -> &mut Receiver<ServerCommand>{ &mut self.from_controller_command }

    fn get_packet_recv(&mut self) -> &mut Receiver<Packet>{ &mut self.packet_recv }
    fn get_packet_send(&mut self) -> &mut HashMap<NodeId, Sender<Packet>>{ &mut self.packet_send }
    fn get_packet_send_not_mutable(&self) -> &HashMap<NodeId, Sender<Packet>>{ &self.packet_send }
    fn get_reassembling_messages(&mut self) -> &mut HashMap<u64, Vec<u8>>{ &mut self.reassembling_messages }

    fn process_reassembled_message(&mut self, data: Vec<u8>, src_id: NodeId){
        match String::from_utf8(data.clone()) {
            Ok(data_string) => match serde_json::from_str(&data_string) {
                Ok(Query::AskType) => self.give_type_back(src_id),

                Ok(Query::AskListFiles) => self.give_list_back(src_id),
                Ok(Query::AskFile(file_id)) => self.give_file_back(src_id, file_id),

                Err(_) => {
                    panic!("Damn, not the right struct")
                }
                _ => {}
            },
            Err(e) => println!("Dio porco, {:?}", e),
        }
        println!("process reassemble finished");
    }
}

impl CharTrait for TextServer{
    fn give_list_back(&mut self, client_id: NodeId) {

        //Get list
        let list_files = self.content.clone();

        //Creating data to send
        let response = Response::ListFiles(list_files);

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

    fn give_file_back(&mut self, client_id: NodeId, file_id: u8) {

        //Get file
        let file:&String = self.content.get(file_id as usize).unwrap();

        //Creating data to send
        let response = Response::File(file.clone());

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
}