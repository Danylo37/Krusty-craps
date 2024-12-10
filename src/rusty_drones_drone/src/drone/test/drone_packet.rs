#![cfg(test)]
use crate::testing_utils::data::new_test_fragment_packet;
use crate::testing_utils::data::*;
use crate::testing_utils::DroneOptions;

use crate::drone::test::{simple_drone_with_exit, simple_drone_with_two_exit};
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use wg_2024::packet::NackType::{Dropped, ErrorInRouting, UnexpectedRecipient};
use wg_2024::packet::Packet;

fn basic_single_hop_test(
    packet: Packet,
    expected_packet: Packet,
    crashing: bool,
    pdr: f32,
    node_id: NodeId,
    exit: NodeId,
) -> DroneOptions {
    let (options, mut drone, packet_exit) = simple_drone_with_exit(node_id, pdr, exit);

    drone.handle_packet(&packet, crashing);
    assert_eq!(expected_packet, packet_exit.try_recv().unwrap());

    options
}

fn basic_single_hop_test_fail(
    packet: Packet,
    crashing: bool,
    pdr: f32,
    node_id: NodeId,
    exit: NodeId,
) -> DroneOptions {
    let (options, mut drone, packet_exit) = simple_drone_with_exit(node_id, pdr, exit);

    drone.handle_packet(&packet, crashing);
    assert!(packet_exit.try_recv().is_err());

    options
}

#[test]
fn test_drone_packet_forward() {
    let packet = new_test_fragment_packet(&[10, 11, 12], 5);
    let expected_packet = new_forwarded(&packet);

    let options = basic_single_hop_test(packet, expected_packet.clone(), false, 0.0, 11, 12);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
}

#[test]
fn test_drone_packet_forward_to_none() {
    let packet = new_test_fragment_packet(&[10, 11, 12], 5);
    let expected_packet = new_test_nack(&[11, 10], ErrorInRouting(12), 5, 1);

    let options =
        basic_single_hop_test(packet.clone(), expected_packet.clone(), false, 0.0, 11, 10);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_packet_forward_crash() {
    let packet = new_test_fragment_packet(&[10, 11, 12], 5);
    let expected_packet = new_test_nack(&[11, 10], ErrorInRouting(11), 5, 1);

    let options = basic_single_hop_test(packet.clone(), expected_packet.clone(), true, 0.0, 11, 10);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_packet_forward_nack() {
    let packet = new_test_nack(&[10, 11, 12], Dropped, 5, 1);
    let expected_packet = new_forwarded(&packet);

    let options =
        basic_single_hop_test(packet.clone(), expected_packet.clone(), false, 0.0, 11, 12);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_packet_forward_nack_crashing() {
    let packet = new_test_nack(&[10, 11, 12], Dropped, 5, 1);
    let expected_packet = new_forwarded(&packet);

    let options = basic_single_hop_test(packet.clone(), expected_packet.clone(), true, 0.0, 11, 12);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_packet_forward_nack_pdr_max() {
    let packet = new_test_nack(&[10, 11, 12], Dropped, 5, 1);
    let expected_packet = new_forwarded(&packet);

    let options =
        basic_single_hop_test(packet.clone(), expected_packet.clone(), false, 1.0, 11, 12);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_packet_nack_to_nothing_shortcut() {
    let packet = new_test_nack(&[10, 11, 12], Dropped, 5, 1);

    let options = basic_single_hop_test_fail(packet.clone(), false, 1.0, 11, 10);
    options.assert_expect_drone_event(DroneEvent::ControllerShortcut(new_forwarded(&packet)));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_packet_dropped() {
    let packet = new_test_fragment_packet(&[10, 11, 12], 5);
    let expected = new_test_nack(&[11, 10], Dropped, 5, 1);

    let (options, mut drone, packet_exit, _) = simple_drone_with_two_exit(11, 1.0, 10, 12);
    drone.handle_packet(&packet, false);
    assert_eq!(expected, packet_exit.try_recv().unwrap());

    options.assert_expect_drone_event(DroneEvent::PacketDropped(packet));
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_packet_error_in_routing() {
    let packet = new_test_fragment_packet(&[10, 11, 12], 5);
    let expected_packet = new_test_nack(&[11, 10], ErrorInRouting(12), 5, 1);

    let options = basic_single_hop_test(packet, expected_packet.clone(), false, 0.0, 11, 10);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
}

#[test]
fn test_drone_packet_unexpected_recepient() {
    let packet = new_test_fragment_packet(&[10, 100, 12], 5);
    let expected_packet = new_test_nack(&[11, 10], UnexpectedRecipient(11), 5, 1);

    let options = basic_single_hop_test(packet, expected_packet.clone(), false, 0.0, 11, 10);
    options.assert_expect_drone_event(DroneEvent::PacketSent(expected_packet));
}
