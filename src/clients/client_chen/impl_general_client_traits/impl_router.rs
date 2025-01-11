use crate::clients::client_chen::{ClientChen, NodeInfo, PacketCreator, Router, Sending, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;

impl Router for ClientChen {
    ///main method of for discovering the routing
    fn do_flooding(&mut self) {
        // New ids for the flood and new session because of the flood response packet
        self.status.flood_id += 1;
        self.status.session_id += 1;

        self.communication.routing_table.clear();
        self.network_info.topology.clear();

        // Initialize the flood request with the current flood_id, id, and node type
        let flood_request = FloodRequest::initialize(self.status.flood_id, self.metadata.node_id, NodeType::Client);

        // Prepare the packet with the current session_id and flood_request
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            self.status.session_id,
            flood_request,
        );

        // Collect the connected node IDs into a temporary vector
        let connected_nodes: Vec<_> = self.communication.connected_nodes_ids.iter().cloned().collect();

        // Send the packet to each connected node
        for &node_id in &connected_nodes {
            self.send_packet_to_connected_node(node_id, packet.clone()); // Assuming `send_packet_to_connected_node` takes a cloned packet
        }
    }

    fn update_routing_for_server(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId, NodeType)>) {
        //ask some necessary queries to the server, to get the communicable clients as early as possible.
        self.storage.irresolute_path_traces.remove(&destination_id);
        let hops = self.get_hops_from_path_trace(path_trace);
        self.send_query(destination_id, Query::AskType);
        self.send_query(destination_id, Query::AskListClients);   //for this query the content server will eventually not drop a response.
        self.communication.routing_table
            .entry(destination_id)
            .or_insert_with(HashMap::new)
            .insert(hops, 0);   //why using times is 0, because we do the flooding when all the routing table is cleared.
    }

    //this needs to be done also in the free time when the registration to the server is successfully done,
    //so you check the flood_responses in the input_packet_disk.
    /// Protocol for update routing for the clients:
    /// 1. do the flooding as always and obviously in the first flooding the servers will not be registered.
    /// therefore we can not send anything to clients even having gotten the flood responses.
    /// 2. critical moment is when you receive the acceptance of the registered client, and you want
    /// to send some messages to the clients, then you try to send the packet, and put into the
    /// buffer, if there are no routes the status of the packet will be not sent because of no routes
    /// so we need to check to the flood response (of current flood id) from the client, if
    /// it is inside the input packet disk then we will filter those routes and update the valid
    /// routes in the routing table, otherwise we need to wait for the flood response to arrive.  (IMPLEMENTED)
    ///
    /// Alternative protocol: just need the routes to the servers and don't update routing for the client
    /// such that in order to send a packet to a client, you send a query to a server
    /// which the client is registered to:
    /// SendPacketTo(client_id, packet), and the server will process the packet
    /// and send it to the client.
    /// (drawback: this alternative you need to send to the server
    /// instead of sending to the client directly)                     (STILL TO IMPLEMENT)
    ///
    /// (advantage: you don't need to memorize that much of routing)
    fn update_routing_for_client(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId, NodeType)>) {
        let hops = self.get_hops_from_path_trace(path_trace.clone());
        //update the routes only if the routes are valid
        if self.check_if_exists_registered_communication_server_intermediary_in_route(hops) {
            //if exists a registered communication server that it is an intermediary between two clients then it will be ok
            self.storage.irresolute_path_traces.remove(&destination_id);
            let hops = self.get_hops_from_path_trace(path_trace);
            self.communication.routing_table
                .entry(destination_id)
                .or_insert_with(HashMap::new)
                .insert(hops, 0);
        } else {
            //the path_trace is still irresolute, that's a clever way
        }
    }

    fn update_routing_checking_status(&mut self) {
        let mut updates = Vec::new();

        // Collect the updates needed
        for (&destination_id, traces) in &self.storage.irresolute_path_traces {
            if let Some((_, destination_type)) = traces.last() {
                updates.push((destination_id, *destination_type, traces.clone()));
            } else {
                eprintln!("No traces found for destination: {:?}", destination_id);
            }
        }

        // Apply the updates
        for (destination_id, destination_type, path_trace) in updates {
            match destination_type {
                NodeType::Server => {
                    self.update_routing_for_server(destination_id, path_trace);
                }
                NodeType::Client => {
                    self.update_routing_for_client(destination_id, path_trace);
                }
                _ => {
                    // Handle other cases (e.g., NodeType::Drone)
                    eprintln!("Unhandled node type: {:?}", destination_type);
                }
            }
        }
    }

    ///auxiliary function
    fn check_if_exists_registered_communication_server_intermediary_in_route(&mut self, route: Vec<NodeId>) -> bool {
        for &server_id in self.communication.registered_communication_servers.keys() {
            if route.contains(&server_id) {
                return true;
            }
        }
        false
    }

    fn check_if_exists_route_contains_server(&mut self, server_id: ServerId, destination_id: ClientId) -> bool {
        for route in self.communication.routing_table.get(&destination_id).unwrap() {
            if route.0.contains(&server_id) {
                return true;
            }
        }
        false
    }

    fn get_flood_response_initiator(&mut self, flood_response: FloodResponse) -> NodeId {
        flood_response.path_trace.last().map(|(id, _)| *id).unwrap()
    }

    fn update_topology_entry_for_server(&mut self, initiator_id: NodeId, server_type: ServerType) {
        if let SpecificInfo::ServerInfo(server_info) = &mut self
            .network_info
            .topology
            .entry(initiator_id)
            .or_insert(NodeInfo::default()) // Use `or_insert` here
            .specific_info
        {
            server_info.server_type = server_type;
        }
    }

}



