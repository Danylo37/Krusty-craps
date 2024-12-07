use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};

use wg_2024::{
    network::NodeId,
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType},
};
use wg_2024::network::SourceRoutingHeader;
use crate::general_use::{ClientCommand, ClientEvent, Query};

pub struct ClientChen{
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    controller_send: Sender<ClientEvent>,
    controller_recv: Receiver<ClientCommand>,
    topology: HashMap<NodeId, HashSet<NodeId>>,
    floods: HashMap<NodeId, HashSet<u64>>,
}
impl ClientChen {
    pub fn new(
                   id: NodeId,
                   connected_drone_ids: Vec<NodeId>,
                   packet_send: HashMap<NodeId, Sender<Packet>>,
                   packet_recv: Receiver<Packet>,
                   controller_send: Sender<ClientEvent>,
                   controller_recv: Receiver<ClientCommand>,
                   topology: HashMap<NodeId, HashSet<NodeId>>,
                   floods: HashMap<NodeId, HashSet<u64>>) -> self{

        self{

        }
    }
}

