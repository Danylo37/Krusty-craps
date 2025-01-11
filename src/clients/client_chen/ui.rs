use crate::clients::client_chen::prelude::*;
use crate::ui_traits::Monitoring;
use crate::clients::client_chen::{ClientChen, CommandHandler, FragmentsHandler, PacketsReceiver, Router, Sending};
use tokio::sync::mpsc;
use rmp_serde::encode::to_vec;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
pub struct DisplayData {
    node_id: NodeId,
    node_type: String,
    flood_id: FloodId,
    session_id: SessionId,
    connected_node_ids: HashSet<NodeId>,
    registered_communication_servers: HashMap<ServerId, Vec<ClientId>>,
    registered_content_servers: HashSet<ServerId>,
    routing_table: HashMap<NodeId, Vec<Vec<NodeId>>>,
    message_chat: HashMap<ClientId, Vec<(Speaker, Message)>>,
}

impl Monitoring for ClientChen {
    async fn run_with_monitoring(&mut self, sender_to_gui: mpsc::Sender<Vec<u8>>) {
        loop {
            select_biased! {
                recv(self.communication_tools.controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        self.handle_controller_command(command);
                    } else {
                        eprintln!("Error receiving controller command");
                    }
                },
                recv(self.communication_tools.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        self.handle_received_packet(packet);
                    } else {
                        eprintln!("Error receiving packet");
                    }
                },
                default(std::time::Duration::from_millis(10)) => {
                    self.handle_fragments_in_buffer_with_checking_status::<Response>();
                    self.send_packets_in_buffer_with_checking_status();
                    self.update_routing_checking_status();

                    // Transform the routing table
                    let transformed_routing_table: HashMap<NodeId, Vec<Vec<NodeId>>> = self
                        .communication
                        .routing_table
                        .iter()
                        .map(|(node_id, routes)| {
                            let paths: Vec<Vec<NodeId>> = routes
                                .keys()
                                .map(|path| path.iter().map(|(node_id, _)| *node_id).collect())
                                .collect();
                            (*node_id, paths)
                        })
                        .collect();

                    // Create the DisplayData struct
                    let display_data = DisplayData {
                        node_id: self.metadata.node_id,
                        node_type: "ClientChen".to_string(),
                        flood_id: self.status.flood_id,
                        session_id: self.status.session_id,
                        connected_node_ids: self.communication.connected_nodes_ids.clone(),
                        registered_communication_servers: self.communication.registered_communication_servers.clone(),
                        registered_content_servers: self.communication.registered_content_servers.clone(),
                        routing_table: transformed_routing_table,
                        message_chat: self.storage.message_chat.clone(),
                    };

                    // Serialize the DisplayData to MessagePack binary
                    match to_vec(&display_data) {
                        Ok(encoded) => {
                            // Send the binary data over the WebSocket
                            if sender_to_gui.send(encoded).await.is_err() {
                                eprintln!("Error sending data for Node {}", self.metadata.node_id);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error serializing data: {:?}", e);
                        }
                    }
                },
            }
        }
    }
}