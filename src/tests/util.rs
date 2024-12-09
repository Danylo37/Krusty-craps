use std::time::Duration;
use wg_2024::{
    network::SourceRoutingHeader,
    packet::Packet
};
use wg_2024::packet::{FloodRequest, NodeType};

pub const TIMEOUT: Duration = Duration::from_millis(400);

pub fn create_flood_request_packet() -> Packet {
    Packet::new_flood_request(
        SourceRoutingHeader::empty_route(),
        2,
        FloodRequest {
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec![(1, NodeType::Client)],
        })
}