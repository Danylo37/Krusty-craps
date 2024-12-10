#![cfg(test)]
mod drone_command;
mod drone_flood;
mod drone_packet;

use crate::testing_utils::{test_initialization_with_value, DroneOptions};
use crate::RustyDrone;
use crossbeam_channel::{unbounded, Receiver};
use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

fn simple_drone_with_exit(
    id: NodeId,
    pdr: f32,
    exit: NodeId,
) -> (DroneOptions, RustyDrone, Receiver<Packet>) {
    let (options, mut drone) = test_initialization_with_value(id, pdr);

    let (new_sender, new_receiver) = unbounded();
    drone.handle_commands(&DroneCommand::AddSender(exit, new_sender));

    (options, drone, new_receiver)
}

fn simple_drone_with_two_exit(
    id: NodeId,
    pdr: f32,
    exit1: NodeId,
    exit2: NodeId,
) -> (DroneOptions, RustyDrone, Receiver<Packet>, Receiver<Packet>) {
    let (options, mut drone) = test_initialization_with_value(id, pdr);

    let (new_sender1, new_receiver1) = unbounded();
    drone.handle_commands(&DroneCommand::AddSender(exit1, new_sender1));

    let (new_sender2, new_receiver2) = unbounded();
    drone.handle_commands(&DroneCommand::AddSender(exit2, new_sender2));

    (options, drone, new_receiver1, new_receiver2)
}

#[test]
fn test_drone_new() {
    let pdr = 0.3;
    let id = 5;
    let (options, drone) = test_initialization_with_value(id, pdr);

    assert_eq!(drone.id, id);
    assert!(drone.controller_send.same_channel(&options.controller_send));
    assert!(drone.controller_recv.same_channel(&options.controller_recv));
    assert!(drone.packet_recv.same_channel(&options.packet_recv));
    assert_eq!(drone.packet_send.len(), options.packet_send.len());
    assert_eq!(drone.pdr, pdr);
}
