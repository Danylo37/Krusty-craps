use std::time::Duration;
use wg_2024::{
    network::SourceRoutingHeader,
    packet::{Fragment, Packet}
};
use wg_2024::packet::{FloodRequest, NodeType};

pub const TIMEOUT: Duration = Duration::from_millis(400);

/// Creates a sample packet for testing purposes. For convenience, using 1-10 for clients, 11-20 for drones and 21-30 for servers
pub fn create_sample_packet() -> Packet {
    Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12, 21],
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        }
    )
}

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