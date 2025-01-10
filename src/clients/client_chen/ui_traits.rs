use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::{ClientChen, CommandHandler, FragmentsHandler, PacketsReceiver, Router, Sending};

pub trait Monitoring{
    fn run_with_monitoring(&mut self, sender_to_gui:Sender<String>);
}

impl Monitoring for ClientChen{
    fn run_with_monitoring(&mut self, sender_to_gui: Sender<String>) {
        loop {
            select_biased! {
                recv(self.communication_tools.controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        self.handle_controller_command(command);
                    }
                },
                recv(self.communication_tools.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        self.handle_received_packet(packet);
                    }
                },
                default(std::time::Duration::from_millis(10)) => {
                    self.handle_fragments_in_buffer_with_checking_status();
                    self.send_packets_in_buffer_with_checking_status();
                    self.update_routing_checking_status();

                    let json_string = serde_json::to_string(&self).unwrap();
                    if sender_to_gui.send(json_string).is_err() {
                        eprintln!("Error sending data for Node {}", self.metadata.node_id);
                        break; // Exit loop if sending fails
                    }
                 },
            }
        }
    }
}