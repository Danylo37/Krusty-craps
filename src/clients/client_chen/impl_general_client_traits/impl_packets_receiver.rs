use log::info;
use crate::clients::client_chen::{ClientChen, PacketsReceiver, PacketResponseHandler, FragmentsHandler, FloodingPacketsHandler};
use crate::clients::client_chen::prelude::*;

impl PacketsReceiver for ClientChen {
    fn handle_received_packet(&mut self, packet: Packet) {
        //insert the packets into the input disk
        self.decreasing_using_times_when_receiving_packet(&packet);
        self.storage.input_packet_disk.insert((self.status.session_id, match packet.pack_type{
            PacketType::MsgFragment(Some(fragment)) => {
                self.storage.fragment_assembling_buffer.insert((self.status.session_id, fragment.fragment_index), packet.clone());
                fragment.fragment_index
            },
            _ => 0, }), packet.clone());

        match packet.clone().pack_type {
            PacketType::Nack(nack) => self.handle_nack(packet, nack),
            PacketType::Ack(ack) => self.handle_ack(packet, ack),
            PacketType::MsgFragment(fragment) => self.handle_fragment(packet, fragment),
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(packet, flood_request),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(packet, flood_response),
        }
    }

    fn decreasing_using_times_when_receiving_packet(&mut self, packet: &Packet) {
        // Reverse the hops to get the correct order for path trace
        let hops = packet.routing_header.hops.iter().rev().collect::<Vec<_>>();
        let destination_id = *hops.last().unwrap().clone();
        let path_trace = hops
            .iter()
            .map(|&&node_id| {
                // Look up the node_id in edge_nodes, defaulting to NodeType::Drone if not found
                let node_type = self.communication.edge_nodes.get(&node_id).unwrap_or(&NodeType::Drone);
                (node_id, *node_type) // Return a tuple of (NodeId, NodeType)
            })
            .collect::<Vec<_>>();

        // Decrease `using_times` by 1 for the corresponding route
        if let Some(routes) = self.communication.routing_table.get_mut(&destination_id) {
            if let Some(using_times) = routes.get_mut(&path_trace) {
                *using_times -= 1;
            }
        }
    }
}



