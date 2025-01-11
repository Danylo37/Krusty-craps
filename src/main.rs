//Mod components
mod network_initializer;
mod servers;

mod simulation_controller;
mod general_use;
mod ui;

mod clients;
pub mod ui_traits;
mod connecting_websocket;

use serde::{Deserialize, Serialize};
use futures_util::{SinkExt, StreamExt};
use crossbeam_channel;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio::sync::mpsc;


#[tokio::main]
async fn main() {
    let url = "ws://localhost:8000";

    println!("Connecting to {}", url);
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connected to WebSocket");

    let (mut write, _) = ws_stream.split();

    // Create an asynchronous channel for communication between clients and WebSocket writer task
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1000);

    // Network initializer instance
    let mut my_net = network_initializer::NetworkInitializer::new(tx.clone());
    my_net.initialize_from_file("input.toml");

    // Spawn a task for writing messages to the WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(err) = write.send(Message::Binary(msg)).await {
                eprintln!("Error writing to WebSocket: {:?}", err);
                break;
            }
        }
    });


    // Keep the program running to simulate clients sending data
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    println!("Client program terminated.");
}
/*
use std::collections::{HashMap, HashSet};
use crossbeam_channel::unbounded;
use wg_2024::network::NodeId;
use crate::clients::client_chen::{ClientChen, NodeType};
//Usages
//use crate::network_initializer::NetworkInitializer;


fn monitoring_clients(){
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

*/


/*
fn main() {
    let mut my_net = network_initializer::NetworkInitializer::new();
    my_net.initialize_from_file("input.toml");
}*/