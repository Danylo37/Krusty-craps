use std::collections::{HashMap, HashSet};
use crossbeam_channel::{unbounded, Sender};

use wg_2024::{network::NodeId, packet::Packet};

use crate::{
    clients::client::Client,
    general_use::ServerType,
};

use super::{
    client_danylo::ChatClientDanylo,
    ui::run_chat_client_ui
};

pub fn test_ui() {
    let mut c1 = create_test_chat_client(1);

    c1.topology.insert(2, HashSet::from_iter(vec![1, 4]));
    c1.topology.insert(3, HashSet::from_iter(vec![1, 5]));

    c1.servers.insert(4, ServerType::Communication);
    c1.servers.insert(5, ServerType::Communication);
    c1.is_registered.insert(4, false);
    c1.is_registered.insert(5, true);

    c1.servers.insert(6, ServerType::Media);
    c1.servers.insert(7, ServerType::Text);
    c1.servers.insert(8, ServerType::Undefined);

    c1.clients.push(9);
    c1.clients.push(10);
    c1.clients.push(11);

    c1.new_messages = 2;

    run_chat_client_ui(c1);
}

pub fn create_test_chat_client(id: NodeId) -> ChatClientDanylo {
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
        packet_send,
        c1_recv,
        sc_event_send,
        sc_command_recv)
}