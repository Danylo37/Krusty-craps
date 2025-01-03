use crate::clients::client_chen::{ClientChen, PacketCreator, Router, Sending};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;

impl Router for ClientChen{
    ///main method of for discovering the routing
    fn do_flooding(&mut self) {
        //new ids for the flood and new session because of the flood response packet
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

        // Send the packet to each connected node
        for &node_id in self.communication.connected_nodes_ids.iter() {
            self.send_packet_to_connected_node(node_id, packet.clone()); //assuming `send_packet_to_connected_node` takes a cloned packet
        }
    }

    fn update_routing_for_server(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId,NodeType)>) {
        //ask some necessary queries to the server, to get the communicable clients as early as possible.
        self.send_query(destination_id, Query::AskType);
        self.send_query(destination_id, Query::AskListClients);
        self.communication.routing_table
            .entry(destination_id)
            .or_insert_with(HashMap::new)
            .insert(path_trace.clone(),0);   //why using times is 0, because we do the flooding when all the routing table is cleared.
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
    /// routes in the routing table, otherwise we need to wait for the flood response to arrive.
    ///
    /// Alternative protocol: just need the routes to the servers and don't update routing for the client
    /// such that in order to send a packet to a client, you send a query to a server
    /// which the client is registered to:
    /// SendPacketTo(client_id, packet), and the server will process the packet
    /// and send it to the client.
    /// (drawback: this alternative you need to send to the server
    /// instead of sending to the client directly)
    ///
    /// (advantage: you don't need to memorize that much of routing)
    fn update_routing_for_client(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId,NodeType)>) {
        let hops = self.get_hops_from_path_trace(path_trace.clone());
        //update the routes only if the routes are valid
        if self.check_if_exists_registered_server_intermediary_in_route(hops) {
            self.communication.routing_table
                .entry(destination_id)
                .or_insert_with(HashMap::new)
                .insert(path_trace, 0);
        }
    }

    ///auxiliary function
    fn check_if_exists_registered_server_intermediary_in_route(&mut self, route: Vec<NodeId>) -> bool {
        for &server_id in self.communication.server_registered.keys(){
            if route.contains(&server_id) {
                return true;
            }
        }
        false
    }

    fn check_if_exists_route_contains_server(&mut self, server_id: ServerId, destination_id: ClientId) -> bool {
        for route in self.communication.routing_table.get(&destination_id).unwrap() {
            if route.0.contains(*server_id){
                return true;
            }
        }
        false
    }

    fn get_flood_response_initiator(&mut self, flood_response: FloodResponse) -> NodeId {
        flood_response.path_trace.last().map(|(id, _)| *id).unwrap()
    }

    // Not that useful functions
    fn filter_flood_responses_from_wanted_destination(&mut self, wanted_destination_id: NodeId) -> HashSet<u64> {
        self.storage.input_packet_disk
            .values()
            .filter_map(|packet| match &packet.pack_type {
                PacketType::FloodResponse(flood_response) => Some(flood_response),
                _ => None,
            })
            .filter_map(|flood_response| {
                if self.get_flood_response_initiator(flood_response.clone()) == wanted_destination_id && flood_response.flood_id == self.status.flood_id {
                    Some(flood_response.flood_id)
                } else {
                    None
                }
            })
            .collect()
    }

    fn if_current_flood_response_from_wanted_destination_is_received(&mut self, wanted_destination_id: NodeId) -> bool {
        !self.filter_flood_responses_from_wanted_destination(wanted_destination_id).is_empty()
    }
}
