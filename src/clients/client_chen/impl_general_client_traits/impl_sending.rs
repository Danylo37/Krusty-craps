use crate::clients::client_chen::{ClientChen, PacketCreator, Router, Sending};
use crate::clients::client_chen::prelude::*;
impl Sending for ClientChen {
    fn send_packets_in_buffer_with_checking_status(&mut self){
        for &(session_id, fragment_index) in self.storage.output_buffer.keys(){
            let packet = self.storage.output_packet_disk.get(&(session_id, fragment_index)).unwrap().clone();
            self.packets_status_sending_actions(packet, self.storage.packets_status.get(&(session_id, fragment_index)).unwrap().clone());
        }
    }

    ///principal sending methods
    fn send(&mut self, packet: Packet) {    //send with routing
        self.send_packet_to_connected_node(packet.routing_header.current_hop().unwrap(), packet);
    }

    fn send_query(&mut self, server_id: ServerId, query: Query) {
        if let Some(messages) = self.msg_to_fragments(query.clone(), server_id) {
            for message in messages {
                // if you send the messages, it automatically updates the buffers and status and packet disk
                self.send(message);
            }
        } else {
            // Log or handle the case where no fragments are returned
            warn!("No fragments returned for query: {:?}", query);
        }
    }
    fn send_packet_to_connected_node(&mut self, target_node_id: NodeId, packet: Packet) {
        let path_trace = packet.routing_header.hops
            .iter()
            .map(|&node_id| {
                // Look up the node_id in edge_nodes, defaulting to NodeType::Drone if not found
                let node_type = self.communication.edge_nodes.get(&node_id).unwrap_or(&NodeType::Drone);
                (node_id, *node_type) // Return a tuple of (NodeId, NodeType)
            })
            .collect::<Vec<_>>();

        // Increase `using_times` by 1 for the corresponding route
        if let Some(routes) = self.communication.routing_table.get_mut(&packet.routing_header.destination().unwrap()) {
            if let Some(using_times) = routes.get_mut(&path_trace) { //there are always some using times because it is initialized to 0 for every route
                *using_times += 1;
            }
        }

        //insert the packet in output buffer, output_packet_dist, packet_status
        self.storage.output_buffer.insert((packet.session_id, match packet.clone().pack_type{
            PacketType::MsgFragment(fragment) => fragment.fragment_index,
            _=> 0,
        }), packet.clone());

        self.storage.output_packet_disk.insert((packet.session_id, 0), packet.clone());
        self.storage.packets_status.insert((packet.session_id, 0), PacketStatus::InProgress);

        if self.communication.connected_nodes_ids.contains(&target_node_id) {
            if let Some(sender) = self.communication_tools.packet_send.get_mut(&target_node_id) {
                match sender.send(packet.clone()) {
                    Ok(_) => debug!("Packet sent to node {}", target_node_id),
                    Err(err) => error!("Failed to send packet to node {}: {}", target_node_id, err),
                }
            } else {
                warn!("No sender channel found for node {}", target_node_id);
            }
        }
    }

    ///auxiliary methods
    ///
    fn packets_status_sending_actions(&mut self, packet: Packet, packet_status: PacketStatus) {
        let destination = Self::get_packet_destination(packet.clone());

        match packet_status {
            PacketStatus::NotSent(not_sent_type) => {
                self.handle_not_sent_packet(packet, not_sent_type, destination);
            }
            PacketStatus::Sent => {
                self.handle_sent_packet(packet);
            }
            _ => {
                //do nothing, just wait
            }
        }
    }

    fn handle_sent_packet(&mut self, packet: Packet) {
        self.storage.output_buffer.remove(&(
            packet.session_id,
            match packet.pack_type {
                PacketType::MsgFragment(fragment) => fragment.fragment_index,
                _ => 0,
            },
        ));
    }
    fn handle_not_sent_packet(&mut self, packet: Packet, not_sent_type: NotSentType, destination: NodeId) {
        match not_sent_type {
            NotSentType::RoutingError => {
                if self.if_current_flood_response_from_wanted_destination_is_received(destination) {
                    self.send(packet);
                }
            }
            NotSentType::ToBeSent | NotSentType::Dropped => {
                self.send(packet);
            }
            _ => { //we 'll see how to handle them
            }
        }
    }
    fn update_packet_status(
        &mut self,
        session_id: SessionId,
        fragment_index: FragmentIndex,
        status: PacketStatus,
    ) {
        self.storage.packets_status
            .entry((session_id, fragment_index))
            .and_modify(|current_status| *current_status = status.clone())
            .or_insert(status);
    }
}