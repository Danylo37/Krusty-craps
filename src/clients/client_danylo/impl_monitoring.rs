use crate::general_use::{ClientId, FloodId, Message, ServerId, ServerType, SessionId};
use crate::ui_traits::{crossbeam_to_tokio_bridge, Monitoring};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::interval;
use wg_2024::network::NodeId;
use super::ChatClientDanylo;

#[derive(Debug, Serialize)]
pub struct ChatClientDisplayData {
    // Client metadata
    node_id: NodeId,
    node_type: String,

    // Used IDs
    flood_ids: Vec<FloodId>,
    session_ids: Vec<SessionId>,

    // Connections
    neighbours: HashSet<NodeId>,
    discovered_servers: HashMap<ServerId, ServerType>,
    registered_communication_servers: HashMap<ServerId, bool>,
    available_clients: HashMap<ServerId, Vec<ClientId>>,

    // Inbox
    received_messages: Vec<(ClientId, Message)>,
}


impl Monitoring for ChatClientDanylo {
    fn run_with_monitoring(
        &mut self,
        sender_to_gui: mpsc::Sender<String>,
    ) -> impl Future<Output = ()> + Send {
        async move {
            // Create tokio mpsc channels for receiving controller commands and packets
            let (controller_tokio_tx, mut controller_tokio_rx) = mpsc::channel(32);
            let (packet_tokio_tx, mut packet_tokio_rx) = mpsc::channel(32);

            // Spawn the bridge function for controller commands
            let controller_crossbeam_rx = self.controller_recv.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(controller_crossbeam_rx, controller_tokio_tx));

            // Spawn the bridge function for incoming packets
            let packet_crossbeam_rx = self.packet_recv.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(packet_crossbeam_rx, packet_tokio_tx));

            let mut interval = interval(std::time::Duration::from_millis(10));
            loop {
                select! {
                    biased;
                    // Handle periodic tasks
                    _ = interval.tick() => {
                        //eprintln!("Handling periodic tasks"); // Debug
                        // Handle fragments and send packet
                        // self.handle_fragments_in_buffer_with_checking_status();  // todo what is this
                        // self.send_packets_in_buffer_with_checking_status(); // This can use crossbeam's send directly    // todo what is this
                        // self.update_routing_checking_status();   // todo what is this

                        // Create the ChatClientDisplayData struct
                        let display_data = ChatClientDisplayData {
                            node_id: self.id,
                            node_type: "ChatClientDanylo".to_string(),
                            flood_ids: self.flood_ids.clone(),
                            session_ids: self.session_ids.clone(),
                            neighbours: self.packet_send.keys().cloned().collect(),
                            discovered_servers: self.servers.clone(),
                            registered_communication_servers: self.is_registered.clone(),
                            available_clients: self.clients.clone(),
                            received_messages: self.inbox.clone(),
                        };

                        // Serialize the DisplayData to MessagePack binary
                        let json_string = serde_json::to_string(&display_data).unwrap();
                        if sender_to_gui.send(json_string).await.is_err() {
                            //eprintln!("Error sending data for Node {}", self.metadata.node_id);
                            break; // Exit loop if sending fails
                        }
                    },

                    // Handle incoming packets from the tokio mpsc channel
                    packet_res = packet_tokio_rx.recv() => {
                        if let Some(packet) = packet_res {
                            self.handle_packet(packet);
                        } else {
                            //eprintln!("Error receiving packet");
                        }
                    },
                    // Handle controller commands from the tokio mpsc channel
                    command_res = controller_tokio_rx.recv() => {
                        if let Some(command) = command_res {
                            self.handle_command(command);
                        } else {
                            //eprintln!("Error receiving controller command");
                        }
                    },

                }
            }
        }
    }
}