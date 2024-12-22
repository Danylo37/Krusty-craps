use std::collections::HashMap;
use crossbeam_channel::{unbounded, Sender};

use wg_2024::{network::NodeId, packet::Packet};

use crate::general_use::ServerType;

use super::{
    ui::start_ui,
    chat_client::ChatClient,
    client_danylo::ChatClientDanylo
};

pub fn test_ui() {
    let mut c1 = create_test_chat_client(1, "Danylo".to_string());
    c1.servers.insert(4, Some(ServerType::Communication));
    c1.servers.insert(5, Some(ServerType::Communication));

    start_ui(c1);
}

pub fn create_test_chat_client(id: NodeId, name: String) -> ChatClientDanylo {
    // Client with id 1
    let (_c1_send, c1_recv) = unbounded();

    // Drone with id 2
    let (d2_send, _d2_recv) = unbounded();
    // Drone with id 3
    let (d3_send, _d3_recv) = unbounded::<Packet>();

    // SC commands and events
    let (_sc_command_send, sc_command_recv) = unbounded();
    let (sc_event_send, _sc_event_recv) = unbounded();

    let packet_send: HashMap<NodeId, Sender<Packet>> = HashMap::from([(2, d2_send), (3, d3_send)]);

    ChatClientDanylo::new(
        id,
        name,
        packet_send,
        c1_recv,
        sc_event_send,
        sc_command_recv)
}