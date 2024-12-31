use std::{
    io::empty,
    hash::Hash,
    collections::{HashMap, HashSet},
    thread, vec,
    string::String,
};
use serde::{Deserialize, Serialize};
use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{debug, error, info, warn};
use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{
        Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType,
        FRAGMENT_DSIZE,
    },
};
use crate::general_use::{ClientId,
                         FloodId,
                         ServerId,
                         SessionId,
                         FragmentIndex,
                         ClientCommand,
                         ClientEvent,
                         Message,
                         NotSentType,
                         PacketStatus,
                         Query,
                         Response,
                         ServerType,
                         Speaker,
                         File,
                         UsingTimes,
};

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
pub trait FragmentAssembler{
    fn handle_fragments_in_buffer_with_checking_status<T: Serialize>(&mut self);  //when you run

    ///principal methods
    fn get_total_n_fragments(&mut self, session_id: SessionId) -> Option<u64>;
    fn reassemble_fragments_in_buffer<T: Serialize + for<'de> Deserialize<'de>>(&mut self) -> T;

}

///todo! subtraits for receiving packets:
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
    fn reassemble_fragments_in_buffer<T: Serialize + for<'de> Deserialize<'de>>(&mut self) -> T;

    ///auxiliary functions
    fn get_total_n_fragments(&mut self, session_id: SessionId) -> Option<u64>;

}

///todo! command handler
pub trait CommandHandler{
    fn handle_controller_command(&mut self, command: ClientCommand);
}

pub trait ServerQuery{
    fn register_to_server(&mut self, server_id: NodeId);
    ///todo! other Query operation functions

    ///auxiliary functions
    fn discovered_servers(&mut self) -> HashSet<ServerId>;
}




