use serde::Serialize;

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{Fragment, Packet, PacketType},
};

#[derive(Clone, Debug)]
/// ###### Represents a message that is fragmented into smaller pieces for transmission.
pub struct MessageFragments {
    fragments: Vec<Fragment>,
    last_fragment_index: usize,
    session_id: u64,
    route: Vec<NodeId>,
}

impl MessageFragments {
    /// ###### Creates a new `MessageFragments` with the given session ID and route.
    pub fn new(session_id: u64, route: Vec<NodeId>) -> MessageFragments {
        Self {
            fragments: Vec::new(),
            last_fragment_index: 0,
            session_id,
            route,
        }
    }

    /// ###### Serializes the provided data and splits it into smaller fragments for sending.
    pub fn create_message_of<T: Serialize>(&mut self, data: T) -> bool {
        let serialized_message = match serde_json::to_string(&data) {
            Ok(string) => string,
            Err(_) => return false,
        };

        self.fragments = self.fragment(&serialized_message);
        true
    }

    /// ###### Splits a serialized message into fragments of a fixed size.
    pub fn fragment(&mut self, serialized_msg: &str) -> Vec<Fragment> {
        let serialized_msg_in_bytes = serialized_msg.as_bytes();
        let length_response = serialized_msg_in_bytes.len();

        let n_fragments = (length_response + 127) / 128;
        (0..n_fragments)
            .map(|i| {
                let start = i * 128;
                let end = ((i + 1) * 128).min(length_response);
                let fragment_data = &serialized_msg[start..end];
                Fragment::from_string(i as u64, n_fragments as u64, fragment_data.to_string())
            })
            .collect()
    }

    /// ###### Retrieves the packet for the specified fragment index.
    pub fn get_fragment_packet(&self, fragment_index: usize) -> Option<Packet> {
        if let Some(fragment) = self.fragments.get(fragment_index).cloned() {
            let hops = self.route.clone();
            let routing_header = SourceRoutingHeader {
                hop_index: 0,
                hops,
            };

            let packet = Packet {
                routing_header,
                session_id: self.session_id,
                pack_type: PacketType::MsgFragment(fragment),
            };

            Some(packet)
        } else {
            None
        }
    }

    /// ###### Increments the index of the last processed or sent fragment.
    pub fn increment_last_index(&mut self) {
        self.last_fragment_index += 1;
    }

    /// ###### Retrieves the route for the message fragments.
    pub fn get_route(&self) -> &Vec<NodeId> {
        &self.route
    }

    /// ###### Updates the route for the message fragments.
    pub fn update_route(&mut self, route: Vec<NodeId>) {
        self.route = route;
    }
}
