use crate::clients::client_chen::prelude::*;
pub trait Sending{
    fn send_packets_in_buffer_with_checking_status(&mut self);//when you run the client

    ///principal sending methods
    fn send(&mut self, packet: Packet);
    fn send_query(&mut self, server_id: ServerId, query: Query);

    fn send_packet_to_connected_node(&mut self, target_node_id: NodeId, packet: Packet);


    ///auxiliary methods
    fn packets_status_sending_actions(&mut self, packet: Packet, packet_status: PacketStatus);
    fn handle_sent_packet(&mut self, packet: Packet);
    fn handle_not_sent_packet(&mut self, packet: Packet, not_sent_type: NotSentType, destination: NodeId);
    fn update_packet_status(&mut self, session_id: SessionId, fragment_index: FragmentIndex, status: PacketStatus);

}


pub trait Router{
    ///main method of for discovering the routing
    fn do_flooding(&mut self);
    fn update_routing_for_server(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId,NodeType)>);
    fn update_routing_for_client(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId,NodeType)>);

    ///auxiliary function
    fn check_if_exists_registered_server_intermediary_in_route(&mut self, route: Vec<NodeId>) -> bool;
    fn check_if_exists_route_contains_server(&mut self, server_id: ServerId, destination_id: ClientId) -> bool;
    fn get_flood_response_initiator(&mut self, flood_response: FloodResponse) -> NodeId;
    fn filter_flood_responses_from_wanted_destination(&mut self, wanted_destination_id: NodeId) -> HashSet<u64>;
    fn if_current_flood_response_from_wanted_destination_is_received(&mut self, wanted_destination_id: NodeId) -> bool;


}

pub trait PacketCreator{
    ///creating fragment packet
    fn divide_string_into_slices(&mut self, string: String, max_slice_length: usize) -> Vec<String>;
    fn msg_to_fragments<T: Serialize>(&mut self, msg: T, destination_id: NodeId) -> Option<HashSet<Packet>>;
    ///creating ack packet
    fn create_ack_packet_from_receiving_packet(&mut self, packet: Packet) -> Packet;

    ///auxiliary methods
    fn get_packet_destination(packet: Packet) -> NodeId;
    fn get_hops_from_path_trace(&mut self, path_trace: Vec<(NodeId, NodeType)>) -> Vec<NodeId>;
    fn get_source_routing_header(&mut self, destination_id: NodeId) -> Option<SourceRoutingHeader>;

}


pub trait PacketsReceiver{
    fn handle_received_packet(&mut self, packet: Packet);
    fn decreasing_using_times_when_receiving_packet(&mut self, packet: &Packet);
}

pub trait PacketResponseHandler:PacketsReceiver{   //Ack Nack
    fn handle_ack(&mut self, ack_packet: Packet, ack: Ack);
    fn handle_nack(&mut self, nack_packet: Packet, nack: Nack);


    ///nack handling (we could do also a sub trait of a sub trait)
    fn handle_error_in_routing(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack);
    fn handle_destination_is_drone(&mut self, nack_packet: Packet, nack: Nack);
    fn handle_pack_dropped(&mut self, nack_packet: Packet, nack: Nack);
    fn handle_unexpected_recipient(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack);
}




pub trait FloodingPacketsHandler:PacketsReceiver{  //flood request/response
    fn handle_flood_request(&mut self, packet: Packet, request: FloodRequest);
    fn handle_flood_response(&mut self, packet: Packet, response: FloodResponse);

    ///auxiliary functions

}

pub trait FragmentsHandler:PacketsReceiver{ //message fragments
    fn handle_fragment(&mut self, msg_packet: Packet, fragment: Fragment);

    ///auxiliary functions
    fn get_total_n_fragments(&mut self, session_id: SessionId) -> Option<u64>;
    fn handle_fragments_in_buffer_with_checking_status<T: Serialize>(&mut self);  //when you run

    ///principal methods
    fn reassemble_fragments_in_buffer<T: Serialize + for<'de> Deserialize<'de>>(&mut self) -> T;

}

pub trait CommandHandler{
    fn handle_controller_command(&mut self, command: ClientCommand);
}

pub trait ServerQuery{
    fn register_to_server(&mut self, server_id: ServerId);
    fn unregister_from_server(&mut self, server_id: ServerId);
    fn ask_server_type(&mut self, server_id: ServerId);
    fn ask_list_clients(&mut self, server_id: ServerId);
    fn send_message_to_client(&mut self, server_id: ServerId, message: Message);
    fn ask_list_files(&mut self, server_id: ServerId);  //all the files that a server has, so not a specific file_ref (or file_index)
    fn ask_file(&mut self, server_id: ServerId, file_ref: u8);
    fn ask_media(&mut self, server_id: ServerId, media_ref: String);  //string is the reference found in the files
    ///auxiliary functions
    fn get_discovered_servers(&mut self) -> HashSet<ServerId>;
}


pub trait ClientEvents{
    fn message_sent_to_client(&mut self, message: Message);
    fn message_received_from_client(&mut self, message: Message);
    fn message_received_from_server(&mut self, message: Message);

}






