use wg_2024::{
    network::SourceRoutingHeader,
    packet::{Fragment, Packet, PacketType, FloodRequest}
};
use wg_2024::packet::NodeType;

pub fn create_sample_packet() -> Packet {
    Packet {
        pack_type: PacketType::MsgFragment(Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12, 21],
        },
        session_id: 1,
    }
}

pub fn create_flood_request_packet() -> Packet {
    Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec![(1, NodeType::Client)],
        }),
        routing_header: SourceRoutingHeader::empty_route(),
        session_id: 1,
    }
}
