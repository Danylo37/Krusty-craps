//I say i did a good job damn lillo

use std::fmt::{Display, Formatter};
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

use wg_2024::{
    network::NodeId,
    packet::Packet,
};
use crate::clients::client_chen::FloodId;

pub type Message = String;

///packet sending status
#[derive(Debug, Clone)]
#[cfg_attr(feature = "debug", derive(PartialEq))]
pub enum NotSentType{
    ToBeSent,
    Dropped,
    RoutingError,
    DroneDestination,
    BeenInWrongRecipient,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "debug", derive(PartialEq))]
pub enum PacketStatus{
    Sent,                   //Successfully sent packet, that is with ack received
    NotSent(NotSentType),   //Include the packet not successfully sent, that is nack received
    InProgress,             //When we have no ack or nack confirmation
}

///flood status

#[derive(Debug, Clone)]
#[cfg_attr(feature = "debug", derive(PartialEq))]
pub enum FloodStatus{
    InGoing,
    Finished(FloodId),
}

/// From controller to Server
#[derive(Debug, Clone)]
pub enum ServerCommand {
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>)
}

#[derive(Debug, Clone)]
pub enum FloodReceivedStatusFromNode {
    Received(FloodId),   //if the response of the current flood is received
    NotReceived,         //when we initialize
}

#[derive(Debug, Clone)]
pub enum CommunicableType {
    Yes,
    No,
}

///Server-Controller
pub enum ServerEvent {
}

/// From controller to Client
pub enum ClientCommand {
    //Controller functions
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>),

    //Client behaviour
    AskTypeTo(NodeId),
    StartFlooding,
}

pub enum ClientEvent {

}

//Queries (Client -> Server)
#[derive(Deserialize, Serialize, Debug)]
pub enum Query{
    //Common-shared
    AskType,

    //To Communication Server
    AddUser(NodeId),
    AskListUsers,
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
    UserAdded,
    MessageFrom(NodeId, Message),
    ListUsers(Vec<NodeId>),

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
#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq)]
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
