//Mod components
mod network_initializer;
mod servers;

mod simulation_controller;
mod general_use;
mod ui;

mod clients;
pub mod ui_traits;
mod connecting_websocket;

use std::thread;
use serde::{Deserialize, Serialize};
use futures_util::{SinkExt, StreamExt};
use crossbeam_channel;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio::sync::mpsc;


#[tokio::main]  //HERE YOU ARE EXPLICITLY USING MULTITHREADED RUNTIME WITH TOKIO SO SURE THAT YOU ARE
                //NOT RUNNING EVERYTHING IN ONE THREAD.
async fn main() {
    let url = "ws://localhost:8000";

    println!("Connecting to {}", url);
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connected to WebSocket");
    let (mut write, _) = ws_stream.split();

  
    // Create an asynchronous channel for communication between clients and WebSocket writer task
    let (tx, mut rx) = mpsc::channel::<String>(1000);

    // Network initializer instance
    let mut my_net = network_initializer::NetworkInitializer::new(tx.clone());
    my_net.initialize_from_file("input.toml");
    
    // Spawn a task for writing messages to the WebSocket
    tokio::spawn(async move {
        loop {
            while let Some(msg) = rx.recv().await {
                //eprintln!("I'm running"); // Debug
                if let Err(err) = write.send(Message::Text(msg)).await {
                    //eprintln!("Error writing to WebSocket: {:?}", err);
                    break;
                }
            }
        }
    });





    // Keep the program running to simulate clients sending data
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    //println!("Client program terminated.");
}

