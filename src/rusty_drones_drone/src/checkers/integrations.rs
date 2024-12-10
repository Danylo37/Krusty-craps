#![cfg(test)]

use crate::checkers::TIMEOUT;
use crate::testing_utils::data::{new_test_fragment_packet, new_test_nack};
use crate::testing_utils::Network;
use wg_2024::controller::DroneCommand;
use wg_2024::packet::NackType;

#[test]
fn test_drone_packet_1_hop() {
    let net = Network::create_and_run(3, &[(0, 1), (1, 2)], &[0, 2]);

    let mut packet = new_test_fragment_packet(&[0, 1, 2], 5);
    net.send_as_client(0, &packet).unwrap();

    let response = net.recv_as_client(2, TIMEOUT).unwrap();

    (&mut packet.routing_header).hop_index = 2;
    assert_eq!(packet, response);
}

#[test]
fn test_drone_packet_3_hop() {
    let net = Network::create_and_run(5, &[(0, 1), (1, 2), (2, 3), (3, 4)], &[0, 4]);

    let mut packet = new_test_fragment_packet(&[0, 1, 2, 3, 4], 5);
    net.send_as_client(0, &packet).unwrap();

    let response = net.recv_as_client(4, TIMEOUT).unwrap();

    (&mut packet.routing_header).hop_index = 4;
    assert_eq!(packet, response);
}

#[test]
fn test_drone_packet_3_hop_crash() {
    let net = Network::create_and_run(5, &[(0, 1), (1, 2), (2, 3), (3, 4)], &[0, 4]);

    net.send_as_simulation_controller_to(1, DroneCommand::Crash);
    let packet = new_test_fragment_packet(&[0, 1, 2, 3, 4], 5);

    net.send_as_client(0, &packet).unwrap();
    let response = net.recv_as_client(0, TIMEOUT).unwrap();

    let expected = new_test_nack(&[1, 0], NackType::ErrorInRouting(1), 5, 1);
    assert_eq!(expected, response);
}

#[test]
fn test_drone_packet_255_hop() {
    let net = Network::create_and_run(
        256,
        &(0..255).map(|i| (i, i + 1)).collect::<Vec<_>>(),
        &[0, 255],
    );

    let mut packet = new_test_fragment_packet(&(0..=255).collect::<Vec<_>>(), 5);
    net.send_as_client(0, &packet).unwrap();

    let response = net
        .recv_as_client(255, TIMEOUT * 100)
        .expect("Took too long or failed");
    (&mut packet.routing_header).hop_index = 255;
    assert_eq!(packet, response);
}

#[test]
fn test_drone_error_in_routing() {
    let net = Network::create_and_run(5, &[(0, 1), (1, 2)], &[0, 4]);

    let packet = new_test_fragment_packet(&[0, 1, 2, 4], 5);
    net.send_as_client(0, &packet).unwrap();

    let response = net.recv_as_client(0, TIMEOUT).unwrap();
    let expected = new_test_nack(&[2, 1, 0], NackType::ErrorInRouting(4), 5, 2);
    assert_eq!(expected, response);
}

#[test]
fn test_drone_destination_is_drone() {
    let net = Network::create_and_run(4, &[(0, 1), (1, 2), (2, 3)], &[0, 3]);

    let packet = new_test_fragment_packet(&[0, 1, 2], 5);
    net.send_as_client(0, &packet).unwrap();

    let response = net.recv_as_client(0, TIMEOUT).unwrap();
    let expected = new_test_nack(&[2, 1, 0], NackType::DestinationIsDrone, 5, 2);
    assert_eq!(expected, response);
}
