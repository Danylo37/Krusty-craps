//Mod components
mod network_initializer;
mod servers;

mod simulation_controller;
mod general_use;
mod ui;

mod clients;

use std::collections::{HashMap, HashSet};
use crossbeam_channel::unbounded;
use wg_2024::network::NodeId;
use crate::clients::client_chen::{ClientChen, NodeType};
//Usages
//use crate::network_initializer::NetworkInitializer;

fn main() {
    let (_packet_send_tx, packet_send_rx) = unbounded();
    let (client_send_tx, _controller_recv_rx) = unbounded();
    let (_controller_send_tx, client_recv_rx) = unbounded();
    let connected_nodes_ids: HashSet<NodeId> = HashSet::new();

    let client = ClientChen::new(
        1,
        NodeType::Client,
        connected_nodes_ids,
        HashMap::new(),
        packet_send_rx,
        client_send_tx,
        client_recv_rx,
    );

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "ClientChen Monitoring",
        options,
        Box::new(|_cc| Ok(Box::new(client))),
    ).unwrap();
}