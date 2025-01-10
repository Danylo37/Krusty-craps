use std::fmt::{Display, Formatter};
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

use wg_2024::{
    network::NodeId,
    packet::Packet,
};
use crate::network_initializer::DroneBrand;

pub type Message = String;
pub type File = String;
pub type ServerId = NodeId;
pub type ClientId = NodeId;
pub type DroneId = NodeId;
pub type SessionId = u64;
pub type FloodId = u64;
pub type FragmentIndex = u64;
pub type UsingTimes = u64;  //to measure traffic of fragments in a path.

///packet sending status
#[derive(Debug, Clone)]
pub enum NotSentType{
    ToBeSent,
    Dropped,
    RoutingError,
    DroneDestination,
    BeenInWrongRecipient,
}

#[derive(Debug, Clone)]
pub enum Speaker{
    Me,
    HimOrHer,
}

#[derive(Debug, Clone)]
pub enum PacketStatus{
    Sent,                   //Successfully sent packet, that is with ack received
    NotSent(NotSentType),   //Include the packet not successfully sent, that is nack received
    InProgress,             //When we have no ack or nack confirmation
}

/// From controller to Server
#[derive(Debug, Clone)]
pub enum ServerCommand {
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>)
}

///Server-Controller
pub enum ServerEvent {
}

#[derive(Debug)]
/// From controller to Client
pub enum ClientCommand {
    //Controller functions
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>),
    RunUI,
    StartFlooding,
    AskTypeTo(ServerId),
    RequestText(NodeId),
    RequestMedia(NodeId),
}


#[derive(Debug, Clone)]
pub enum ClientEvent {
    PacketSent(Packet),
    SenderRemoved(NodeId),
    SenderAdded(NodeId),
    DoingFlood(FloodId),
    FloodIsFinished(FloodId),
}

//Queries (Client -> Server)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Query {
    //Common-shared
    AskType,

    //To Communication Server
    RegisterClient(NodeId),
    UnregisterClient(NodeId),
    AskListClients,
    SendMessageTo(NodeId, Message),

    //To Content Server
    //(Text)
    AskListFiles,
    AskFile(u8),
    //(Media)
    AskMedia(String), // String is the reference found in the files
}

//Server -> Client
#[derive(Deserialize, Serialize, Debug)]
pub enum Response {
    //Common-shared
    ServerType(ServerType),

    //From Communication Server
    ClientRegistered,
    MessageFrom(NodeId, Message),
    ListClients(Vec<NodeId>),

    //From Content Server
    //(Text)
    ListFiles(Vec<String>),
    File(String),
    //(Media)
    Media(String),

    //General Error
    Err(String)
}

///Material
#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub enum ServerType {
    Communication,
    Text,
    Media,
    Undefined,
}

impl Display for ServerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ServerType::Communication => "Communication",
            ServerType::Text => "Text",
            ServerType::Media => "Media",
            ServerType::Undefined => "Undefined",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash)]
pub enum ClientType {
    Chat,
    Web,
}
impl ClientType {
    // Returns an iterator over all variants of DroneBrand
    pub fn iter() -> impl Iterator<Item = ClientType> {
        [
            ClientType::Chat,
            ClientType::Web,
        ]
            .into_iter()
    }
}

impl Display for ClientType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
