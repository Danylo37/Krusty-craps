use crossbeam_channel::Sender;
use wg_network::NodeId;
use wg_packet::Packet;

/// From controller to Server
#[derive(Debug, Clone)]
pub enum ServerCommand {
    
}


/// From controller to Client
pub enum ClientCommand {
}
