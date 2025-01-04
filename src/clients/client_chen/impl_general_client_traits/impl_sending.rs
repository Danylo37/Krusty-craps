use crate::clients::client_chen::{ClientChen, PacketCreator, Router, Sending};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
use crate::general_use::NotSentType::ToBeSent;

impl Sending for ClientChen {
    fn send_packets_in_buffer_with_checking_status(&mut self){
        for &(session_id, fragment_index) in self.storage.output_buffer.keys(){
            let packet = self.storage.output_packet_disk.get(&(session_id, fragment_index)).unwrap().clone();
            self.packets_status_sending_actions(packet, self.storage.packets_status.get(&(session_id, fragment_index)).unwrap().clone());
        }
    }

    ///principal sending methods
    fn send(&mut self, packet: Packet) {
        if let Some(next_hop) = packet.routing_header.next_hop() {
            self.send_packet_to_connected_node(next_hop, packet);
        } else {
            // When there is no next hop, do nothing, the packet needs to be dropped.
            eprintln!("No next hop available for packet: {:?}", packet);
        }
    }

    fn send_events(&mut self, client_event: ClientEvent){
        self.communication_tools.controller_send.send(client_event).expect("Client event not successfully sent");
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
        let path_trace = self.hops_to_path_trace(packet.clone().routing_header.hops);

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


        if self.communication.connected_nodes_ids.contains(&target_node_id) {
            if let Some(sender) = self.communication_tools.packet_send.get_mut(&target_node_id) {
                match sender.send(packet.clone()) {
                    Ok(_) => {
                        debug!("Packet sent to node {}", target_node_id);
                        self.storage.packets_status.insert((packet.session_id, 0), PacketStatus::InProgress);
                    },
                    Err(err) => {
                        error!("Failed to send packet to node {}: {}", target_node_id, err);
                        self.storage.packets_status.insert((packet.session_id, 0), PacketStatus::NotSent(ToBeSent));
                    }
                }
            } else {
                warn!("No sender channel found for node {}", target_node_id);
                self.storage.packets_status.insert((packet.session_id, 0), PacketStatus::NotSent(ToBeSent));
            }
        }
    }

    ///auxiliary methods
    ///
    fn packets_status_sending_actions(&mut self, packet: Packet, packet_status: PacketStatus) {
        let destination = self.get_packet_destination(&packet.clone());

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
                if let Some(routes) = self.communication.routing_table.get_mut(&destination) {
                    if !routes.is_empty() {
                        self.send(packet);
                    } else {
                        // The packet is already enqueued for later processing when routes are updated
                    }
                } else {
                    // The packet is already enqueued for later processing when routes are updated
                }
            }
            NotSentType::ToBeSent | NotSentType::Dropped => {
                if let Some(routes) = self.communication.routing_table.get(&destination) {
                    if !routes.is_empty() {
                        self.send(packet);
                    } else {
                        // Log a warning since we expected to have routes but don't
                        warn!("Attempted to send packet to {} but no routes found", destination);
                    }
                } else {
                    // Log a warning since the destination is not in the routing table
                    warn!("Destination {} not found in routing table", destination);
                }
            }
            NotSentType::BeenInWrongRecipient => {
                //in the handle nack we send a client_event to the simulation controller that some senders are messed up

                //todo()! here we just wait (let the packet be stored in the buffer) until some command from the simulation
                //controller to send again the packet when the senders are again fixed.

            }
            NotSentType::DroneDestination => {
                //remove the packet from the buffer, and let it drop.
                self.storage.output_buffer.remove(&(packet.session_id, match packet.pack_type{
                    PacketType::MsgFragment(fragment) => fragment.fragment_index,
                    _ => 0,
                }));
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