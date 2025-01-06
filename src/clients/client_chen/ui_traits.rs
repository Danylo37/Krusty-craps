use crate::clients::client_chen::prelude::*;
use eframe::egui;
use crate::clients::client_chen::ClientChen;
impl eframe::App for ClientChen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel for general information
        egui::TopBottomPanel::top("Top Panel").show(ctx, |ui| {
            ui.heading(format!("Monitoring Client: Node ID {}", self.metadata.node_id));
            ui.label(format!("Node Type: {:?}", self.metadata.node_type));
        });

        // Central panel for detailed monitoring
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.collapsing("Status", |ui| {
                ui.label(format!("Flood ID: {}", self.status.flood_id));
                ui.label(format!("Session ID: {}", self.status.session_id));
            });

            ui.collapsing("Communication Info", |ui| {
                ui.label(format!("Connected Nodes: {:?}", self.communication.connected_nodes_ids));
                ui.label(format!("Routing Table Size: {}", self.communication.routing_table.len()));
                ui.label(format!(
                    "Registered Content Servers: {}",
                    self.communication.registered_content_servers.len()
                ));
            });

            ui.collapsing("Storage Info", |ui| {
                ui.label(format!(
                    "Fragments in Buffer: {}",
                    self.storage.fragment_assembling_buffer.len()
                ));
                ui.label(format!("Input Packets: {}", self.storage.input_packet_disk.len()));
                ui.label(format!("Output Packets: {}", self.storage.output_packet_disk.len()));
                ui.label(format!(
                    "Chat Messages: {}",
                    self.storage.message_chat.keys().len()
                ));
                ui.label(format!("Files Stored: {}", self.storage.file_storage.len()));
            });

            ui.collapsing("Network Info", |ui| {
                ui.label(format!("Topology Size: {}", self.network_info.topology.len()));
            });
        });
    }
}