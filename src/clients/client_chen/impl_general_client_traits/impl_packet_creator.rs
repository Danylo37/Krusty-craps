use crate::clients::client_chen::{ClientChen, PacketCreator};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
impl PacketCreator for ClientChen{
    fn divide_string_into_slices(&mut self, string: String, max_slice_length: usize) -> Vec<String> {
        let mut slices = Vec::new();
        let mut start = 0;

        while start < string.len() {
            let end = std::cmp::min(start + max_slice_length, string.len());

            // Ensure we slice at a valid character boundary
            let valid_end = string
                .char_indices()
                .take_while(|&(idx, _)| idx <= end)
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(string.len());

            slices.push(string[start..valid_end].to_string());
            start = valid_end;
        }
        slices
    }
    fn msg_to_fragments<T: Serialize>(&mut self, msg: T, destination_id: NodeId) -> Option<HashSet<Packet>> {
        let serialized_msg = serde_json::to_string(&msg).unwrap();
        let mut fragments = HashSet::new(); //fragments are of type Packet
        let msg_slices = self.divide_string_into_slices(serialized_msg, FRAGMENT_DSIZE);
        let number_of_fragments = msg_slices.len();

        if let Some(source_routing_header) = self.get_source_routing_header(destination_id){
            self.status.session_id += 1;
            //the i is counted from 0 so it's perfect suited in our case
            for (i, slice) in msg_slices.into_iter().enumerate() {
                //Convert each slice of the message into the same format of the field data of the struct Fragment.
                let slice_bytes = slice.as_bytes();
                let fragment_data = {
                    let mut buffer = [0u8; FRAGMENT_DSIZE]; // Create a buffer with the exact required size
                    let slice_length = std::cmp::min(slice_bytes.len(), FRAGMENT_DSIZE); // Ensure no overflow
                    buffer[..slice_length].copy_from_slice(&slice_bytes[..slice_length]);
                    buffer
                };

                let fragment = Fragment {
                    fragment_index: i as u64,
                    total_n_fragments: number_of_fragments as u64,
                    length: slice.len() as u8, //Note u8 is 256 but actually "length <= FRAGMENT_DSIZE = 128"
                    data: fragment_data,       //Fragment data
                };

                let routing_header = source_routing_header.clone();
                let packet = Packet::new_fragment(routing_header, self.status.session_id, fragment);
                fragments.insert(packet);
            }
            Some(fragments)
        }else{
            None
        }
    }

    fn create_ack_packet_from_receiving_packet(&mut self, packet: Packet) -> Packet{
        let routing_header = SourceRoutingHeader{
            hop_index : 1,
            hops: packet.routing_header.hops.iter().rev().copied().collect(),   //when you can, use Copy trait instead of Clone trait, it's more efficient.
        };   //nope we need to use the same of which is arrived.
        let ack_packet = Packet::new_ack(routing_header,
                                         packet.session_id + 1,
                                         match packet.clone().pack_type{
                                             PacketType::MsgFragment(fragment)=> fragment.fragment_index,
                                             _=> 0,
                                        });

        ack_packet
    }

    fn get_packet_destination(&mut self, packet: &Packet) -> NodeId {
        let destination = packet.routing_header.destination().unwrap();
        destination
    }

    fn get_hops_from_path_trace(&mut self, path_trace: Vec<(NodeId, NodeType)>) -> Vec<NodeId> {
        // Transform the best path into a vector of NodeId and initialize hop_index to 1
        let hops = path_trace.iter().map(|&(node_id, _)| node_id).collect();
        hops
    }

    /// find best source routing header sort by the keys of OrderId and the UsageTimes, in order to improve the efficiency of
    fn get_source_routing_header(&mut self, destination_id: NodeId) -> Option<SourceRoutingHeader> {
        if let Some(routes) = self.communication.routing_table.get(&destination_id) {
            if let Some(min_using_times) = routes.values().min() {
                if let Some(best_path) = routes
                    .iter()
                    .filter(|(_, &using_times)| using_times == *min_using_times)
                    .map(|(path, _)| path)
                    .min_by_key(|path| path.len())
                {
                    // Transform the best path into a vector of NodeId and initialize hop_index to 1
                    let hops = self.get_hops_from_path_trace(best_path.clone());
                    return Some(SourceRoutingHeader::new(hops, 1));
                }
            }
        }
        None
    }

    fn hops_to_path_trace(&mut self, hops: Vec<NodeId>) -> Vec<(NodeId, NodeType)> {
            hops
            .iter()
            .map(|&node_id| {
                // Look up the node_id in edge_nodes, defaulting to NodeType::Drone if not found
                if let Some(node_info) = self.network_info.topology.get(&node_id) {
                    (node_id, node_info.node_type)
                } else {
                    (node_id, NodeType::Drone)
                }
            })
            .collect::<Vec<_>>()
    }
}