use crate::clients::client_chen::{ClientChen, Router, Sending};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::Sending::*;
impl Router for ClientChen{
    ///main method of for discovering the routing
    fn do_flooding(&mut self) {
        //new ids for the flood and new session because of the flood response packet
        self.status.flood_id += 1;
        self.status.session_id += 1;

        self.communication.routing_table.clear();
        self.communication.communicable_nodes.clear();


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
        //update the communicable_nodes
        self.communication.communicable_nodes.insert(destination_id);  //regardless for the server
        self.send_query(destination_id, Query::AskType);
        self.send_query(destination_id, Query::AskListClients);
        self.communication.routing_table
            .entry(destination_id)
            .or_insert_with(HashMap::new)
            .insert(path_trace.clone(),0);   //why using times is 0, because we do the flooding when all the routing table is cleared.
    }
    fn update_routing_for_client(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId,NodeType)>) {
        let hops = self.get_hops_from_path_trace(path_trace.clone());
        //update the routes only if the routes are valid
        if self.check_if_exists_registered_server_intermediary_in_route(hops) {
            self.communication.routing_table
                .entry(destination_id)
                .or_insert_with(HashMap::new)
                .insert(path_trace, 0);
            self.communication.communicable_nodes.insert(destination_id); //regardless if before there is
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
