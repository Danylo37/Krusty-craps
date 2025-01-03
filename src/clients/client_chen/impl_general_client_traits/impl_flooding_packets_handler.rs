use crate::clients::client_chen::{ClientChen, ClientInformation, DroneBrand, DroneInformation, FloodingPacketsHandler, NodeInfo, Router, Sending, ServerInformation, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;

impl FloodingPacketsHandler for ClientChen {
    fn handle_flood_request(&mut self, packet: Packet, mut request: FloodRequest) {
        //store in the input packet disk
        self.storage.input_packet_disk.insert((packet.session_id, 0), packet);   //because this packet is not any fragment packet.
        //general idea: prepare a flood response and send it back.
        self.status.session_id += 1;
        request.path_trace.push((self.metadata.node_id, self.metadata.node_type));
        let mut response: Packet = request.generate_response(self.status.session_id);

        //hop_index is again set to 1 because you want to send back using the same path.
        //response.routing_header.hop_index = 0; this line is not needed because
        //it is already included in the generate_response function

        //2. send it back
        if let Some(routes) = self.communication.routing_table.get(&response.clone().routing_header.destination().unwrap()) {
            if !routes.is_empty() {
                self.send(response.clone());
                //so it is a direct response and doesn't need to wait in the buffer, because
                //of the select biased function is prioritizing this send in respect to the others.
                //then if this flood response is not received by any one then there will be a nack of this packet sent back from a drone
                //then we need to handle it.
            }
        } else { //if we don't have routes
            //we insert regardless thanks to the non repeat mechanism of the hashmaps
            self.storage.packets_status.insert((response.session_id, 0), PacketStatus::NotSent(NotSentType::ToBeSent));
            self.storage.output_buffer.insert((response.session_id, 0), response.clone());
        }
    }


    /// When you receive a flood response, you need first to update the topology with the elements of the path_traces
    /// everyone's connected_node_ids (using the hashset's methods).
    fn handle_flood_response(&mut self, packet: Packet, response: FloodResponse) {
        // Insert the packet into the input_packet_disk with session_id as key
        self.storage.input_packet_disk.insert((packet.session_id, 0), packet);

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