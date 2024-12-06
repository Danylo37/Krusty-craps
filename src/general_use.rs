use crossbeam_channel::Sender;
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
    AddSender(NodeId, Sender<Packet>)
}
pub enum ClientEvent{
    
}
