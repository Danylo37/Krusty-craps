use std::collections::HashMap;
use std::thread;
use crossbeam_channel::unbounded;

use wg_2024::{
    drone::Drone,
    network::SourceRoutingHeader,
    packet::{Packet, PacketType, FloodRequest, NodeType},
};
use wg_2024::controller::DroneEvent;
use crate::tests::util::{create_flood_request_packet, TIMEOUT};

pub fn generic_flood_request_forward<T: Drone + Send + 'static>() {
    // Client 1 channels
    let (c_send, _c_recv) = unbounded();
    // Server 21 channels
    let (s_send, s_recv) = unbounded();
    // Drone 11
    let (d11_send, d11_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();

    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    // Drone 11
    let neighbours11 = HashMap::from([(12, d12_send.clone()), (1, c_send.clone())]);
    let mut drone11 = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d11_recv.clone(),
        neighbours11,
        0.0,
    );
    // Drone 12
    let neighbours12 = HashMap::from([(11, d11_send.clone()), (21, s_send.clone())]);
    let mut drone12 = T::new(
        12,
        d_event_send.clone(),
        d_command_recv.clone(),
        d12_recv.clone(),
        neighbours12,
        0.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone11.run();
    });

    thread::spawn(move || {
        drone12.run();
    });

    let flood_request = create_flood_request_packet();
    let expected_flood_request = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec![(1, NodeType::Client), (11, NodeType::Drone), (12, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader::empty_route(),
        session_id: 2,
    };

    let expected_flood_request_to_forward_1 = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec![(1, NodeType::Client), (11, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader::empty_route(),
        session_id: 2,
    };

    let expected_flood_request_to_forward_2 = expected_flood_request.clone();


    // "Client" sends packet to the drone
    d11_send.send(flood_request.clone()).unwrap();

    assert_eq!(
        s_recv.recv().unwrap(),
        expected_flood_request.clone()
    );

    assert_eq!(
        d_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(expected_flood_request_to_forward_1)
    );

    assert_eq!(
        d_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(expected_flood_request_to_forward_2)
    );
}
