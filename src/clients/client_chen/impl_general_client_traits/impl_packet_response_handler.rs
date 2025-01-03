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
        info!("Drone is crashed or or sender not found {}", node_id);
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::RoutingError));
        //if using get the mapping the destination of the packet (identified by nack_packet.session_id-1, nack.fragment_index)
        //we have None then the current flooding isn't finished for that destination then we need to wait for the flood response
        //to come such that the routing table is updated.
        //otherwise if we already have we need to do a flooding to update all the routes to that node.
        if self.communication.routing_table.get(&(self.storage.output_buffer.get(&*(nack_packet.session_id-1, nack.fragment_index)))) != None{
            self.do_flooding();
        } else{
            //wait
        }
        //the post-part of the handling is done in the handling flooding response
    }

    fn handle_destination_is_drone(&mut self, nack_packet: Packet, nack: Nack) {
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::DroneDestination));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }
    fn handle_pack_dropped(&mut self, nack_packet: Packet, nack: Nack) {
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::Dropped));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }

    fn handle_unexpected_recipient(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack) {
        info!("unexpected recipient found {}", node_id);
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::BeenInWrongRecipient));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }
}