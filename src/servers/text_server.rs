use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap};
use std::fmt::Debug;
use std::future::Future;
use tokio::sync::mpsc;
use tokio::select;
use wg_2024::{
    network::{NodeId},
    packet::{
        Packet,
        PacketType,
    },
};
use crate::general_use::{Query, Response, ServerCommand, ServerEvent, ServerType};
use crate::ui_traits::{crossbeam_to_tokio_bridge, Monitoring};
use super::server::TextServer as CharTrait;
use super::server::Server as MainTrait;

type FloodId = u64;
type SessionId = u64;
#[derive(Debug)]
pub struct TextServer{

    //Basic data
    pub id: NodeId,

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
    pub content: Vec<String>,
}

impl TextServer{
    pub fn new(
        id: NodeId,
        content: Vec<String>,
        to_controller_event: Sender<ServerEvent>,
        from_controller_command: Receiver<ServerCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        TextServer {
            id,

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

            content,
        }
    }
}

impl Monitoring for TextServer {
    fn run_with_monitoring(
        &mut self,
        sender_to_gui: mpsc::Sender<Vec<u8>>,
    ) -> impl Future<Output = ()> + Send {
        async move {
            // Create tokio mpsc channels for receiving controller commands and packets
            let (controller_tokio_tx, mut controller_tokio_rx) = mpsc::channel(32);
            let (packet_tokio_tx, mut packet_tokio_rx) = mpsc::channel(32);

            // Spawn the bridge function for controller commands
            let controller_crossbeam_rx = self.from_controller_command.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(controller_crossbeam_rx, controller_tokio_tx));

            // Spawn the bridge function for incoming packets
            let packet_crossbeam_rx = self.packet_recv.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(packet_crossbeam_rx, packet_tokio_tx));

            loop {
                select! {
                    // Handle controller commands from the tokio mpsc channel
                    command_res = controller_tokio_rx.recv() => {
                        if let Some(command) = command_res {
                            match command {
                                ServerCommand::AddSender(id, sender) => {
                                    self.packet_send.insert(id, sender);
                                }
                                ServerCommand::RemoveSender(id) => {
                                    self.packet_send.remove(&id);
                                }
                            }
                        } else {
                            eprintln!("Error receiving controller command");
                        }
                    },
                    // Handle incoming packets from the tokio mpsc channel
                    packet_res = packet_tokio_rx.recv() => {
                        if let Some(packet) = packet_res {
                            match packet.pack_type {
                                PacketType::MsgFragment(fragment) => {
                                    self.handle_fragment(fragment, packet.routing_header, packet.session_id);
                                }
                                _ => {}
                            }
                        } else {
                            eprintln!("Error receiving packet");
                        }
                    },
                    // Handle periodic tasks
                    _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {
                        // Perform periodic tasks here
                    },
                }
            }
        }
    }
}

impl MainTrait for TextServer{
    fn get_id(&self) -> NodeId{ self.id }
    fn get_server_type(&self) -> ServerType{ ServerType::Text }

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
    fn get_sending_messages_not_mutable(&self) -> &HashMap<u64, (Vec<u8>, u8)>{ &self.sending_messages }


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
            Err(e) => println!("Argh, {:?}", e),
        }
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
