use crate::clients::client_chen::{ClientChen, FloodingPacketsHandler, Router, Sending};
use crate::clients::client_chen::prelude::*;
impl FloodingPacketsHandler for ClientChen{
    fn handle_flood_request(&mut self, packet: Packet ,mut request: FloodRequest) {
        //store in the input packet disk
        self.storage.input_packet_disk.insert((packet.session_id, 0), packet);   //because this packet is not any fragment packet.
        //general idea: prepare a flood response and send it back.
        self.status.session_id += 1;
        request.path_trace.push((self.metadata.node_id, self.metadata.node_type));
        let mut response: Packet = request.generate_response(self.status.session_id);

        //hop_index is again set to 1 because you want to send back using the same path.
        response.routing_header.hop_index = 1;

        //2. send it back
        if let Some(routes) = self.communication.routing_table.get(&response.clone().routing_header.destination().unwrap()){
            if !routes.is_empty(){
                self.send(response.clone());
                //so it is a direct response and doesn't need to wait in the buffer, because
                //of the select biased function is prioritizing this send in respect to the others.
                //then if this flood response is not received by any one then there will be a nack of this packet sent back from a drone
                //then we need to handle it.
            }
        } else{ //if we don't have routes
            //we insert regardless thanks to the non repeat mechanism of the hashmaps
            self.storage.packets_status.insert((response.session_id, 0), PacketStatus::NotSent(NotSentType::ToBeSent));
            self.storage.output_buffer.insert((response.session_id, 0), response.clone());
        }
    }


    fn handle_flood_response(&mut self, packet: Packet, response: FloodResponse) {
        self.storage.input_packet_disk.insert((packet.session_id, 0), packet);
        if let Some((destination_id, destination_type)) = response.path_trace.last().cloned() {
            // Ignore drones or mismatched flood IDs
            if destination_type == NodeType::Drone || response.flood_id != self.status.flood_id {
                return;
            }
            // Update edge nodes
            self.communication.edge_nodes.insert(destination_id, destination_type);

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