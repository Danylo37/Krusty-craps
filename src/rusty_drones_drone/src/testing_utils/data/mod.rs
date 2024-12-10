#![cfg(test)]
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, Fragment, Nack, NackType, NodeType, Packet};

pub fn new_test_fragment_packet(routing_vector: &[NodeId], session_id: u64) -> Packet {
    Packet::new_fragment(
        SourceRoutingHeader::with_first_hop(routing_vector.to_vec()),
        session_id,
        Fragment::from_string(0, 1, String::new()),
    )
}

pub fn new_test_nack(
    hops: &[NodeId],
    nack_type: NackType,
    session_id: u64,
    hop_index: usize,
) -> Packet {
    Packet::new_nack(
        SourceRoutingHeader::new(hops.to_vec(), hop_index),
        session_id,
        Nack {
            fragment_index: 0,
            nack_type,
        },
    )
}

pub fn new_forwarded(packet: &Packet) -> Packet {
    let mut packet = packet.clone();
    packet.routing_header.increase_hop_index();
    packet
}

pub fn new_flood_request(
    session_id: u64,
    flood_id: u64,
    initiator_id: NodeId,
    include_itself: bool,
) -> Packet {
    Packet::new_flood_request(
        SourceRoutingHeader::new(vec![], 0),
        session_id,
        FloodRequest {
            flood_id,
            initiator_id,
            // Stupid assumption for testing
            path_trace: if include_itself {
                vec![(initiator_id, NodeType::Client)]
            } else {
                vec![]
            },
        },
    )
}

pub fn new_flood_request_with_path(
    session_id: u64,
    flood_id: u64,
    initiator_id: NodeId,
    path: &[(NodeId, NodeType)],
) -> Packet {
    Packet::new_flood_request(
        SourceRoutingHeader::new(vec![], 0),
        session_id,
        FloodRequest {
            flood_id,
            initiator_id,
            path_trace: path.to_vec(),
        },
    )
}
