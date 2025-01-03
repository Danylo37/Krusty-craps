use crate::clients::client_chen::{ClientChen, ClientInformation, DroneBrand, DroneInformation, FloodingPacketsHandler, NodeInfo, Router, Sending, ServerInformation, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;

impl FloodingPacketsHandler for ClientChen {
    fn handle_flood_request(&mut self, packet: Packet, mut request: FloodRequest) {
        // Store in the input packet disk (not a fragment).
        self.storage.input_packet_disk.insert((packet.session_id, 0), packet);

        // Prepare the flood response.
        self.status.session_id += 1;
        request.path_trace.push((self.metadata.node_id, self.metadata.node_type));
        let response = request.generate_response(self.status.session_id);

        // Try to find routes for the response.
        if let Some(destination_id) = response.routing_header.destination() {
            if let Some(routes) = self.communication.routing_table.get(&destination_id) {
                // If there are routes, send the response.
                if !routes.is_empty() {
                    self.send(response.clone());
                    // No need to buffer, it's a direct response.
                    return;  // We successfully sent the packet, no need to proceed.
                }
            }
        }

        // If no routes or empty routes, buffer the response.
        self.storage.packets_status.insert(
            (response.session_id, 0),
            PacketStatus::NotSent(NotSentType::ToBeSent),
        );
        self.storage.output_buffer.insert((response.session_id, 0), response.clone());
        self.storage.output_packet_disk.insert((response.session_id, 0), response);
    }

    /// When you receive a flood response, you need first to update the topology with the elements of the path_traces
    /// everyone's connected_node_ids (using the hashset's methods).
    fn handle_flood_response(&mut self, packet: Packet, response: FloodResponse) {

        if response.flood_id != self.status.flood_id{
            return;
        }
        // Insert the packet into the input_packet_disk with session_id as key
        self.storage.input_packet_disk.insert((packet.session_id, 0), packet);
        self.storage.irresolute_path_traces.insert(response.path_trace.clone());

        // Update the network topology and connect nodes in the path trace
        let mut path_iter = response.path_trace.iter().peekable();
        let mut previous_node: Option<NodeId> = None;

        while let Some(&(node_id, node_type)) = path_iter.next() {
            // Peek the next node in the path
            let next_node = path_iter.peek().map(|&(next_id, _)| next_id);

            // Get an entry for the node_id, initializing if it doesn't exist
            let entry = self.network_info.topology.entry(node_id).or_insert_with(|| {
                match node_type {
                    NodeType::Server => NodeInfo {
                        node_id,
                        node_type,
                        specific_info: SpecificInfo::ServerInfo(ServerInformation {
                            server_type: ServerType::Undefined,
                            connected_nodes_ids: HashSet::new(),
                        }),
                    },
                    NodeType::Client => NodeInfo {
                        node_id,
                        node_type,
                        specific_info: SpecificInfo::ClientInfo(ClientInformation {
                            connected_nodes_ids: HashSet::new(),
                        }),
                    },
                    NodeType::Drone => NodeInfo {
                        node_id,
                        node_type,
                        specific_info: SpecificInfo::DroneInfo(DroneInformation {
                            connected_nodes_ids: HashSet::new(),
                            drone_brand: DroneBrand::Undefined,
                        }),
                    },
                }
            });

            // Update the connected_nodes_ids with previous and next node
            if let SpecificInfo::ServerInfo(server_info) = &mut entry.specific_info {
                if let Some(prev) = previous_node {
                    server_info.connected_nodes_ids.insert(prev);
                }
                if let Some(&next) = next_node {
                    server_info.connected_nodes_ids.insert(next);
                }
            } else if let SpecificInfo::ClientInfo(client_info) = &mut entry.specific_info {
                if let Some(prev) = previous_node {
                    client_info.connected_nodes_ids.insert(prev);
                }
                if let Some(&next) = next_node {
                    client_info.connected_nodes_ids.insert(next);
                }
            } else if let SpecificInfo::DroneInfo(drone_info) = &mut entry.specific_info {
                if let Some(prev) = previous_node {
                    drone_info.connected_nodes_ids.insert(prev);
                }
                if let Some(&next) = next_node {
                    drone_info.connected_nodes_ids.insert(next);
                }
            }

            // Update the previous node
            previous_node = Some(node_id);
        }
        ///update the routing table
        if let Some((destination_id, destination_type)) = response.path_trace.last().cloned() {
            // Ignore drones or mismatched flood IDs
            if destination_type == NodeType::Drone || response.flood_id != self.status.flood_id {
                return;
            }

            // Update the routing table and communicable nodes
            match destination_type {
                NodeType::Server => {
                    self.update_routing_for_server(destination_id, response.path_trace);
                }
                NodeType::Client => {
                    self.update_routing_for_client(destination_id, response.path_trace);
                }
                _ => {}
            }
        }
    }
}