use crate::clients::client_chen::{ClientChen, PacketResponseHandler, Router, Sending};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;

impl PacketResponseHandler for ClientChen {
    fn handle_ack(&mut self, ack_packet: Packet, ack: Ack) {
        // Remove the fragment/packet corresponding to the ACK
        let packet_key = (ack_packet.session_id - 1, ack.fragment_index);
        self.storage.packets_status
            .entry(packet_key)
            .and_modify(|status| *status = PacketStatus::Sent)
            .or_insert(PacketStatus::Sent);
        self.storage.output_buffer.remove(&packet_key);
    }


    fn handle_nack(&mut self, nack_packet: Packet, nack: Nack) {
        // Handle specific NACK types
        match nack.nack_type.clone() {
            NackType::ErrorInRouting(node_id) =>  self.handle_error_in_routing(node_id, nack_packet, nack),
            NackType::DestinationIsDrone => self.handle_destination_is_drone(nack_packet, nack),
            NackType::Dropped => self.handle_packdrop(nack_packet, nack),
            NackType::UnexpectedRecipient(node_id) => self.handle_unexpected_recipient(node_id, nack_packet, nack),
        }
    }

    fn handle_error_in_routing(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack) {
        // Log a warning message indicating that there's an issue with the drone or sender not found.
        warn!("Drone is crashed or sender not found for node ID: {}", node_id);

        // Update the status of the packet to reflect that it has encountered a routing error.
        self.update_packet_status(nack_packet.session_id, nack.fragment_index, PacketStatus::NotSent(NotSentType::RoutingError));

        // Attempt to retrieve the destination of the packet from the output buffer using its session ID and fragment index.
        let packet_destination = if let Some(packet) = self.storage.output_buffer.get(&(nack_packet.session_id, nack.fragment_index)) {
            let packet_clone = packet.clone();
            self.get_packet_destination(&packet_clone) // Assuming the Packet struct has a 'destination' field that holds the target node ID.
        } else {
            // If the packet is not found, we can choose to return early or take another action.
            // Here, we'll simply return, as we cannot determine the destination without the packet.
            return; // Alternatively, you could panic or log an error here.
        };

        // Check whether the routing table contains a route to the packet's destination.
        match self.communication.routing_table.get(&packet_destination) {
            Some(routes) => {
                // If a route exists, then the route is wrong then perform a flooding to update the routes to this node
                if !routes.is_empty(){
                    self.do_flooding();
                }
            }
            None => {
                // If no route to the destination exists, then the flooding is still performing, then we can just wait
            }
        }

        // Any post-processing will be handled when the flooding response is received.
    }

    fn handle_destination_is_drone(&mut self, nack_packet: Packet, nack: Nack) {
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::DroneDestination));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }
    fn handle_packdrop(&mut self, nack_packet: Packet, nack: Nack) {
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::Dropped));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }

    fn handle_unexpected_recipient(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack) {
        info!("unexpected recipient found {}", node_id);
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::BeenInWrongRecipient));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }
}