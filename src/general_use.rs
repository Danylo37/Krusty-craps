//I say i did a good job damn lillo

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

use wg_2024::{
    packet::Packet,
    network::NodeId,
};

/// From controller to Server
#[derive(Debug, Clone)]
pub enum ServerCommand {
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>)
}

///Server-Controller
pub enum ServerEvent{
}

/// From controller to Client
pub enum ClientCommand {
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>),
    AskTypeTo(NodeId),
}
pub enum ClientEvent{

}

//Queries (Client -> Server
#[derive(Deserialize, Serialize, Debug)]
pub enum Query{
    //Common-shared
    AskType,

    //To Communication Server
    AddClient(String, NodeId),
    AskListClients,
    SendMessageTo(String, Message),

    //To Content Server
    GetMedia, //not sure if i have to ask for which media before or whaT else
}

//Server -> Client
#[derive(Deserialize, Serialize, Debug)]
pub enum Response{
    //Common-shared
    ServerType(ServerType),

    //From Communication Server
    MessageFrom(String, Message),
    ListUsers(Vec<String>),

    //From Content Server
    GiveMedia,
}

///Material
#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub enum ServerType{
    Communication,
    Content,
}


#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Message{
    text: String,
}

