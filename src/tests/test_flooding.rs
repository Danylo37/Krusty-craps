use std::collections::HashMap;
use std::thread;
use crossbeam_channel::unbounded;

use wg_2024::{
    drone::Drone,
    network::SourceRoutingHeader,
    packet::{Packet, PacketType, FloodRequest, NodeType},
};

use crate::tests::util::create_flood_request_packet;

pub(crate) fn generic_flood_request_forward<T: Drone + Send + 'static>() {
    // Client 1 channels
    let (c_send, _c_recv) = unbounded();
    // Server 21 channels
    let (s_send, s_recv) = unbounded();
    // Drone 11
    let (d11_send, d11_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // SC - needed to not make the drone crash
    let (_d_command_send, d_command_recv) = unbounded();

    // Drone 11
    let neighbours11 = HashMap::from([(12, d12_send.clone()), (1, c_send.clone())]);
    let mut drone11 = T::new(
        11,
        unbounded().0,
        d_command_recv.clone(),
        d11_recv.clone(),
        neighbours11,
        0.0,
    );
    // Drone 12
    let neighbours12 = HashMap::from([(11, d11_send.clone()), (21, s_send.clone())]);
    let mut drone12 = T::new(
        12,
        unbounded().0,
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

    let msg = create_flood_request_packet();

    // "Client" sends packet to the drone
    d11_send.send(msg.clone()).unwrap();

    assert_eq!(
        s_recv.recv().unwrap(),
        Packet {
            pack_type: PacketType::FloodRequest(FloodRequest {
                flood_id: 1,
                initiator_id: 1,
                path_trace: vec![(1, NodeType::Client), (11, NodeType::Drone), (12, NodeType::Drone)],
            }),
            routing_header: SourceRoutingHeader::empty_route(),
            session_id: 1,
        }
    );
}
