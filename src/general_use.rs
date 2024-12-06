use crossbeam_channel::Sender;
use wg_2024::{
    packet::Packet,
    network::NodeId,
};
use serde::{Deserialize, Serialize};
use serde_json;

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
    AddSender(NodeId, Sender<Packet>)
}
pub enum ClientEvent{

}

//To change probably
#[derive(Deserialize, Serialize, Copy, Debug)]
pub enum Query{
    AskType,
    AddClient(NodeId),
}

#[derive(Deserialize, Serialize, Copy, Debug)]
pub enum ServerType{
    Communication,
    Content,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Message{
    text: String,
}
