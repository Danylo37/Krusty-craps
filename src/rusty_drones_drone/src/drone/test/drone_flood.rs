#![cfg(test)]
use crate::drone::test::{simple_drone_with_exit, simple_drone_with_two_exit};
use crate::testing_utils::data::{new_flood_request, new_flood_request_with_path};
use wg_2024::controller::DroneEvent;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{FloodResponse, NodeType, Packet};

#[test]
fn test_drone_flood_req_inclusive() {
    let packet = new_flood_request(5, 7, 10, true);
    let expected =
        new_flood_request_with_path(5, 7, 10, &[(10, NodeType::Client), (11, NodeType::Drone)]);

    let (options, mut drone, packet_exit1, packet_exit2) =
        simple_drone_with_two_exit(11, 1.0, 12, 13);

    drone.handle_packet(&packet, false);
    assert_eq!(expected, packet_exit1.try_recv().unwrap());
    assert_eq!(expected, packet_exit2.try_recv().unwrap());

    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_flood_res_inclusive() {
    let packet = new_flood_request(5, 7, 10, true);
    let expected = Packet::new_flood_response(
        SourceRoutingHeader::new(vec![11, 10], 1),
        5,
        FloodResponse {
            flood_id: 7,
            path_trace: vec![(10, NodeType::Client), (11, NodeType::Drone)],
        },
    );

    let (options, mut drone, packet_exit) = simple_drone_with_exit(11, 1.0, 10);
    drone.handle_packet(&packet, false);
    drone.handle_packet(&packet, false);

    assert_eq!(expected.clone(), packet_exit.try_recv().unwrap());

    options.assert_expect_drone_event(DroneEvent::PacketSent(expected));
    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_flood_req() {
    let packet = new_flood_request(5, 7, 10, false);
    let expected = new_flood_request_with_path(5, 7, 10, &[(11, NodeType::Drone)]);

    let (options, mut drone, packet_exit1, packet_exit2) =
        simple_drone_with_two_exit(11, 1.0, 12, 13);

    drone.handle_packet(&packet, false);
    assert_eq!(expected, packet_exit1.try_recv().unwrap());
    assert_eq!(expected, packet_exit2.try_recv().unwrap());

    options.assert_expect_drone_event_fail();
}

#[test]
fn test_drone_flood_res() {
    let packet = new_flood_request(5, 7, 10, false);
    let expected = Packet::new_flood_response(
        SourceRoutingHeader::new(vec![11, 10], 1),
        5,
        FloodResponse {
            flood_id: 7,
            path_trace: vec![(11, NodeType::Drone)],
        },
    );

    let (options, mut drone, packet_exit) = simple_drone_with_exit(11, 1.0, 10);
    drone.handle_packet(&packet, false);
    drone.handle_packet(&packet, false);

    assert_eq!(expected.clone(), packet_exit.try_recv().unwrap());

    options.assert_expect_drone_event(DroneEvent::PacketSent(expected));
    options.assert_expect_drone_event_fail();
}
