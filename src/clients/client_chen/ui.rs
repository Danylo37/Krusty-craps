use crate::clients::client_chen::prelude::*;
use crate::ui_traits::{crossbeam_to_tokio_bridge, Monitoring};
use crate::clients::client_chen::{ClientChen, CommandHandler, FragmentsHandler, PacketsReceiver, Router, Sending};
use rmp_serde::encode::to_vec;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::interval;

/*
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
    //message pack
    fn run_with_monitoring(
        &mut self,
        sender_to_gui: mpsc::Sender<Vec<u8>>,
    ) -> impl Future<Output = ()> + Send {
        async move {
            // Create tokio mpsc channels for receiving controller commands and packets
            let (controller_tokio_tx, mut controller_tokio_rx) = mpsc::channel(32);
            let (packet_tokio_tx, mut packet_tokio_rx) = mpsc::channel(32);

            // Spawn the bridge function for controller commands
            let controller_crossbeam_rx = self.communication_tools.controller_recv.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(controller_crossbeam_rx, controller_tokio_tx));

            // Spawn the bridge function for incoming packets
            let packet_crossbeam_rx = self.communication_tools.packet_recv.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(packet_crossbeam_rx, packet_tokio_tx));

            let mut interval = interval(std::time::Duration::from_millis(10));
            loop {
                select! {
                    biased;
                    // Handle periodic tasks
                    _ = interval.tick() => {
                        eprintln!("Handling periodic tasks"); // Debug
                        // Handle fragments and send packet
                        self.handle_fragments_in_buffer_with_checking_status();
                        self.send_packets_in_buffer_with_checking_status(); // This can use crossbeam's send directly
                        self.update_routing_checking_status();

                        // Transform the routing table
                        let transformed_routing_table: HashMap<NodeId, Vec<Vec<NodeId>>> = self
                            .communication
                            .routing_table
                            .iter()
                            .map(|(node_id, routes)| {
                                let paths: Vec<Vec<NodeId>> = routes
                                    .keys()
                                    .cloned()
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
                                eprintln!("Sending msg encoded");
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

                    // Handle incoming packets from the tokio mpsc channel
                    packet_res = packet_tokio_rx.recv() => {
                        if let Some(packet) = packet_res {
                            self.handle_received_packet(packet);
                        } else {
                            eprintln!("Error receiving packet");
                        }
                    },
                    // Handle controller commands from the tokio mpsc channel
                    command_res = controller_tokio_rx.recv() => {
                        if let Some(command) = command_res {
                            self.handle_controller_command(command);
                        } else {
                            eprintln!("Error receiving controller command");
                        }
                    },

                }
            }
        }
    }


}*/


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


impl Monitoring for ClientChen{

    //json
    fn run_with_monitoring(
        &mut self,
        sender_to_gui: mpsc::Sender<String>,
    ) -> impl Future<Output = ()> + Send {
        async move {
            // Create tokio mpsc channels for receiving controller commands and packets
            let (controller_tokio_tx, mut controller_tokio_rx) = mpsc::channel(32);
            let (packet_tokio_tx, mut packet_tokio_rx) = mpsc::channel(32);

            // Spawn the bridge function for controller commands
            let controller_crossbeam_rx = self.communication_tools.controller_recv.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(controller_crossbeam_rx, controller_tokio_tx));

            // Spawn the bridge function for incoming packets
            let packet_crossbeam_rx = self.communication_tools.packet_recv.clone();
            tokio::spawn(crossbeam_to_tokio_bridge(packet_crossbeam_rx, packet_tokio_tx));

            let mut interval = interval(std::time::Duration::from_millis(10));
            loop {
                select! {
                    biased;
                    // Handle periodic tasks
                    _ = interval.tick() => {
                        //eprintln!("Handling periodic tasks"); // Debug
                        // Handle fragments and send packet
                        self.handle_fragments_in_buffer_with_checking_status();
                        self.send_packets_in_buffer_with_checking_status(); // This can use crossbeam's send directly
                        self.update_routing_checking_status();

                        let transformed_routing_table: HashMap<NodeId, Vec<Vec<NodeId>>> = self
                            .communication
                            .routing_table
                            .iter()
                            .map(|(node_id, routes)| {
                                let paths: Vec<Vec<NodeId>> = routes
                                    .keys()
                                    .cloned()
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
                        let json_string = serde_json::to_string(&display_data).unwrap();
                        if sender_to_gui.send(json_string).await.is_err() {
                            //eprintln!("Error sending data for Node {}", self.metadata.node_id);
                            break; // Exit loop if sending fails
                        }
                    },

                    // Handle incoming packets from the tokio mpsc channel
                    packet_res = packet_tokio_rx.recv() => {
                        if let Some(packet) = packet_res {
                            self.handle_received_packet(packet);
                        } else {
                            //eprintln!("Error receiving packet");
                        }
                    },
                    // Handle controller commands from the tokio mpsc channel
                    command_res = controller_tokio_rx.recv() => {
                        if let Some(command) = command_res {
                            self.handle_controller_command(command);
                        } else {
                            //eprintln!("Error receiving controller command");
                        }
                    },

                }
            }
        }
    }
}