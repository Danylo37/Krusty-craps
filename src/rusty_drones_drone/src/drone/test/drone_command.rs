#![cfg(test)]
use crate::testing_utils::test_initialization;
use crossbeam_channel::unbounded;
use wg_2024::controller::DroneCommand;
use wg_2024::packet::Packet;

#[test]
fn test_drone_command_crash() {
    let (_options, mut drone) = test_initialization();

    assert!(drone.handle_commands(&DroneCommand::Crash));
}

#[test]
fn test_drone_command_set_packet_drop_rate() {
    let (_options, mut drone) = test_initialization();

    let pdr = 0.123;
    assert!(!drone.handle_commands(&DroneCommand::SetPacketDropRate(pdr)));
    assert_eq!(drone.pdr, pdr);
}

#[test]
fn test_drone_command_add_sender() {
    let (_options, mut drone) = test_initialization();

    let node_id = 42;
    let (packet_send, _) = unbounded::<Packet>();

    assert!(!drone.handle_commands(&DroneCommand::AddSender(node_id, packet_send.clone())));
    assert!(drone.packet_send.contains_key(&node_id));
}

#[test]
fn test_drone_command_add_duplicated_sender() {
    let (_options, mut drone) = test_initialization();

    let node_id = 42;
    let (packet_send1, _) = unbounded::<Packet>();
    let (packet_send2, _) = unbounded::<Packet>();

    assert!(!drone.handle_commands(&DroneCommand::AddSender(node_id, packet_send1.clone())));
    assert!(drone.packet_send.contains_key(&node_id));
    assert!(packet_send1.same_channel(drone.packet_send.get(&node_id).unwrap()));

    assert!(!drone.handle_commands(&DroneCommand::AddSender(node_id, packet_send2.clone())));
    assert!(drone.packet_send.contains_key(&node_id));
    assert!(packet_send2.same_channel(drone.packet_send.get(&node_id).unwrap()));
}

#[test]
fn test_drone_command_remove_sender() {
    let (_options, mut drone) = test_initialization();

    let node_id = 42;
    let (packet_send, _) = unbounded::<Packet>();

    assert!(!drone.handle_commands(&DroneCommand::AddSender(node_id, packet_send.clone())));
    assert!(drone.packet_send.contains_key(&node_id));

    assert!(!drone.handle_commands(&DroneCommand::RemoveSender(node_id)));
    assert!(!drone.packet_send.contains_key(&node_id));
}

#[test]
fn test_drone_command_remove_not_existent_sender() {
    let (_options, mut drone) = test_initialization();

    let node_id = 42;

    assert!(!drone.packet_send.contains_key(&node_id));
    assert!(!drone.handle_commands(&DroneCommand::RemoveSender(node_id)));
    assert!(!drone.packet_send.contains_key(&node_id));
}
