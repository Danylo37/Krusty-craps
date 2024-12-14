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

//use super::content::TEXT;
const TEXT: [&str; 6] = [
    r#"Alcuni versi di Leopardi:
Ma perchè dare al sole,
Perchè reggere in vita
Chi poi di quella consolar convenga?
Se la vita è sventura,
Perchè da noi si dura?
Intatta luna, tale
E’ lo stato mortale.
Ma tu mortal non sei,
E forse del mio dir poco ti cale."#,

    "Una banana #Media[banana]",

    "Non scegliere questo testo #Media[do_not_search_this_media]",

    r#"Phrases by Lillo:
- a lack of belief in free will is the antidote to hate and judgement
- il disordine è tale finche non viene ordinato
- if you have to ask if you’re a member of a group, you’re probably not."#,

    r#"One of the best panoramas are next to us,
just walk up on a mountain,
sit in the middle of the forest and watch at the Sparkling snow #Media[sparkling_snow]"#,

    r#"Bigfoot Sighting Report
Location: Dense forest near Willow Creek, California
Date and Time: December 12, 2024, 4:45 PM

Image: #Media[big_foot]

Report:
While hiking along an isolated trail, approximately 5 miles from the nearest road, I encountered an unusual figure standing roughly 50 yards away in a clearing.
The figure was enormous, standing between 7 and 8 feet tall, with broad shoulders and a heavily muscled frame.
Its body appeared to be covered in dark, shaggy hair, likely black or very dark brown, and it moved with a distinct upright, bipedal gait."#,
];
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
}
