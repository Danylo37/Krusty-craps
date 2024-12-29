///--------------------------
///todo!
/// 1) handling the edge_nodes, also fill with edge_node -> flood_status(edge_node).
/// 2) maybe do a flooding to update those things when the clients starts to run.
/// 3) done that we can set the flooding status (InGoing/ Finished(flood_id)) so that when a valid flood_response
/// is received by all the edge_nodes (clients) through the servers so we will have:
/// -let m = number_of_server
/// -let n = number_of_clients(to which communicate)
/// then each client(as destination) has m number of routes.
/// only then we can set the flooding status to finished(flood_id).


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
                         ReceivedOrderId,
                         ClientCommand,
                         ClientEvent,
                         CommunicableType,
                         FloodReceivedStatusFromNode,
                         FloodStatus,
                         Message,
                         NotSentType,
                         PacketStatus,
                         Query,
                         Response,
                         ServerType,
                         Speaker,
                         File,
                         OrderId,
                         UsingTimes,
};


pub struct ClientChen {
    //client's data
    node_id: NodeId,
    node_type: NodeType,

    //status
    flood_id: FloodId,
    session_id: SessionId,
    packets_status: HashMap<(SessionId, Option<FragmentIndex>), PacketStatus>, //map every packet with the status of sending
    flood_status: FloodStatus,                                                 //status of the current flooding: still in progress or not
    flood_received_nodes: HashMap<NodeId, FloodReceivedStatusFromNode>,
    received_node_order_id: ReceivedOrderId,

    //communication info
    connected_nodes_ids: Vec<NodeId>, //I think better HashSet<(NodeId, NodeType)>, but given that we can ask for type.
    server_registered: HashMap<ServerId, Vec<ClientId>>, //All the servers registered by the client and it's respective list of client registered
                                                         //Notice that if the client is registered to the server then the Vec<ClientId> contains this client.

    servers: HashMap<ServerId, ServerType>,
    edge_nodes: HashMap<NodeId, NodeType>,  //All the nodes that are not drones, which are all the servers and all the clients
    communicable_nodes: HashMap<NodeId, CommunicableType>,
    routing_table: HashMap<NodeId, HashMap<Vec<(NodeId, NodeType)>, UsingTimes>>, //a map that for each destination we have a path according
                                                             //to the protocol
    //communication tools
    packet_send: HashMap<NodeId, Sender<Packet>>,       //each connected_node_id is mapped with a sender to the connected_node
    packet_recv: Receiver<Packet>,                      //Receiver of this Client, it's unique, the senders to this receiver are clones of each others.
    controller_send: Sender<ClientEvent>,               //the Receiver<ClientEvent> will be in Simulation Controller
    controller_recv: Receiver<ClientCommand>,           //the Sender<ClientCommand> will be in Simulation Controller

    //storage
    fragment_assembling_buffer: HashMap<(SessionId, Option<FragmentIndex>), Packet>, //temporary storage for the fragments to recombine,
    output_buffer: HashMap<(SessionId, Option<FragmentIndex>), Packet>,              //temporary storage for the messages to send
    input_packet_disk: HashMap<(SessionId, Option<FragmentIndex>), Packet>,         //storage of all the packets received
    output_packet_disk: HashMap<(SessionId, Option<FragmentIndex>), Packet>, //storage of all the packets sent
    message_chat: HashMap<ClientId, Vec<(Speaker, Message)>>,   //to store for each client you are chatting with, the messages in order
    file_storage: HashMap<ServerId, File>,    //created supposing the files are only received by media servers.
}
impl ClientChen {
    pub fn new(
        node_id: NodeId,
        node_type: NodeType,
        connected_nodes_ids: Vec<NodeId>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
    ) -> Self {
        Self {
            //client's data
            node_id,
            node_type,

            //status
            flood_id: 0, //initial value to be 0 for every new client
            session_id: (node_id as u64) << 56, //e.g. just put the id of the client at the first 8 bits like 10100101 0000... because the node_id is 8 bits and we use it as identifier
            packets_status: HashMap::new(),
            flood_status: FloodStatus::Finished(0),
            flood_received_nodes: HashMap::new(),   //are the nodes from which we received an acceptable flood response
            received_node_order_id: 0,
            //communication info
            connected_nodes_ids,
            servers: HashMap::new(),
            server_registered: HashMap::new(),
            edge_nodes: HashMap::new(),    //because for now the NodeType is not Hashable.
            communicable_nodes: HashMap::new(),
            routing_table: HashMap::new(),

            //communication tools
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,

            //storage
            fragment_assembling_buffer: HashMap::new(),
            output_buffer: HashMap::new(),
            input_packet_disk: HashMap::new(),
            output_packet_disk: HashMap::new(),
            message_chat: HashMap::new(),
            file_storage: HashMap::new(),


        }
    }

    pub fn run(&mut self) {
        loop {
            select_biased! {
            recv(self.controller_recv) -> command_res => {
                if let Ok(command) = command_res {
                    self.handle_controller_command(command);
                }
            },
            recv(self.packet_recv) -> packet_res => {
                if let Ok(packet) = packet_res {
                    self.handle_received_packet(packet);
                }
            },
            default(std::time::Duration::from_millis(10)) => {
                self.handle_fragments_in_buffer_with_checking_status();
                self.send_packets_in_buffer_with_checking_status();
            },
        }
        }
    }


    /// STRUCTURE OF THE CODE:
    ///-sending packets methods
    ///-packet creation methods
    ///-routing methods, contains also the flooding algorithm
    ///-various packets handling methods

    ///--------------------------------------------------------------------------------------------------------------------
    ///sending packets methods
    ///

    fn handle_not_sent_packet(&mut self, packet: Packet, not_sent_type: NotSentType, destination: NodeId) {
        ///     ToBeSent => send it,
        ///     Dropped => send it,
        ///     RoutingError => check the route to be updated and send it,
        ///     DroneDestination => just do nothing for now, we'll see how to handle it,
        ///     BeenInWrongRecipient => just do nothing for now, we'll see how to handle it,
        match not_sent_type {
            NotSentType::RoutingError => {
                if self.if_current_flood_response_from_wanted_destination_is_received(destination) {
                    self.send_with_routing(packet);
                }
            }
            NotSentType::ToBeSent | NotSentType::Dropped => {
                self.send_with_routing(packet);
            }
            _ => {}
        }
    }

    fn handle_sent_packet(&mut self, packet: Packet) {
        self.output_buffer.remove(&(
            packet.session_id,
            match packet.pack_type {
                PacketType::MsgFragment(fragment) => Some(fragment.fragment_index),
                _ => None,
            },
        ));
    }

    pub fn packets_status_actions(&mut self, packet: Packet, packet_status: PacketStatus) {
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

    pub fn send_packets_in_buffer_with_checking_status(&mut self) {
        for &(session_id, option_fragment_index) in self.output_buffer.keys(){
            let packet = self.output_packet_disk.get(&(session_id, option_fragment_index)).unwrap().clone();
            match option_fragment_index {
                Some(index) => {
                    self.packets_status_actions(packet, self.packets_status.get(&(session_id, Some(index))).unwrap().clone());
                },
                None => {
                    self.packets_status_actions(packet, self.packets_status.get(&(session_id, None)).unwrap().clone());
                }
            }
        }
    }

    pub fn send_packet_to_connected_node(&mut self, target_node_id: NodeId, packet: Packet) {
        if self.connected_nodes_ids.contains(&target_node_id) {
            if let Some(sender) = self.packet_send.get_mut(&target_node_id) {
                match sender.send(packet.clone()) {
                    Ok(_) => debug!("Packet sent to node {}", target_node_id),
                    Err(err) => error!("Failed to send packet to node {}: {}", target_node_id, err),
                }
            } else {
                warn!("No sender channel found for node {}", target_node_id);
            }
        }
    }

    pub fn send_with_routing(&mut self, packet: Packet) {
        self.send_packet_to_connected_node(packet.routing_header.current_hop().unwrap(), packet);
    }

    fn send_query(&mut self, server_id: ServerId, query: Query) {
        let messages = Self::msg_to_fragments(query, server_id);

        for message in messages {
            // Extract session ID and optional fragment index for the key
            let key = (
                message.session_id,
                if let PacketType::MsgFragment(fragment) = &message.pack_type {
                    Some(fragment.fragment_index)
                } else {
                    None
                },
            );

            // Insert the message into both buffers
            self.output_buffer.insert(key.clone(), message.clone());
            self.output_packet_disk.insert(key, message);
            self.packets_status.insert(key, PacketStatus::NotSent(NotSentType::ToBeSent));

            //the rest is controlled in the send with checking status.
        }
    }

    ///------------------------- packets methods
    pub fn divide_string_into_slices(string: String, max_slice_length: usize) -> Vec<String> {
        let mut slices = Vec::new();
        let mut start = 0;

        while start < string.len() {
            let end = std::cmp::min(start + max_slice_length, string.len());

            // Ensure we slice at a valid character boundary
            let valid_end = string
                .char_indices()
                .take_while(|&(idx, _)| idx <= end)
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(string.len());

            slices.push(string[start..valid_end].to_string());
            start = valid_end;
        }

        slices
    }
    pub fn msg_to_fragments<T: Serialize>(msg: T, destination_id: NodeId) -> HashSet<Packet> {
        let serialized_msg = serde_json::to_string(&msg).unwrap();
        let mut fragments = HashSet::new(); //fragments are of type Packet
        let msg_slices = Self::divide_string_into_slices(serialized_msg, FRAGMENT_DSIZE);
        let number_of_fragments = msg_slices.len();

        let source_routing_header = Self.get_source_routing_header(destination_id).unwrap();

        Self.session_id += 1;
        //the i is counted from 0 so it's perfect suited in our case
        for (i, slice) in msg_slices.into_iter().enumerate() {
            //Convert each slice of the message into the same format of the field data of the struct Fragment.
            let slice_bytes = slice.as_bytes();
            let fragment_data = {
                let mut buffer = [0u8; FRAGMENT_DSIZE]; // Create a buffer with the exact required size
                let slice_length = std::cmp::min(slice_bytes.len(), FRAGMENT_DSIZE); // Ensure no overflow
                buffer[..slice_length].copy_from_slice(&slice_bytes[..slice_length]);
                buffer
            };

            let fragment = Fragment {
                fragment_index: i as u64,
                total_n_fragments: number_of_fragments as u64,
                length: slice.len() as u8, //Note u8 is 256 but actually "length< <= FRAGMENT_DSIZE = 128"
                data: fragment_data,       //Fragment data
            };

            let routing_header = source_routing_header.clone();

            let packet = Packet::new_fragment(routing_header, Self.session_id, fragment);
            fragments.insert(packet);
        }
        fragments
    }

    fn update_packet_status(
        &mut self,
        session_id: SessionId,
        fragment_index: FragmentIndex,
        status: PacketStatus,
    ) {
        self.packets_status
            .entry((session_id, Some(fragment_index)))
            .and_modify(|current_status| *current_status = status.clone())
            .or_insert(status);
    }


    ///--------------------------------------------------------------------------------------------------------------
    /// reassembling received fragments


    fn get_total_n_fragments(&mut self) -> Option<u64> {
        self.fragment_assembling_buffer.keys().next().and_then(|first_key| {
            self.fragment_assembling_buffer.get(first_key).and_then(|first_fragment_packet| {
                match &(*first_fragment_packet).pack_type {
                    PacketType::MsgFragment(fragment) => Some(fragment.total_n_fragments),
                    _ => None,
                }
            })
        })
    }

    fn handle_fragments_in_buffer_with_checking_status<T: Serialize>(&mut self){
        if self.fragment_assembling_buffer.iter().len() as u64 == self.get_total_n_fragments().unwrap() {
            let first_key = self.fragment_assembling_buffer.keys().next().unwrap();
            let initiator_id = Self::get_packet_destination(self.fragment_assembling_buffer.get(first_key).unwrap().clone());
            let message:T = self.reassemble_fragments_in_buffer();
            match message{
                Response::ServerType(ServerType) => {
                    self.servers.insert(initiator_id, ServerType);
                }
                Response::ClientRegistered => {
                    self.server_registered.insert(initiator_id, vec![self.node_id]);
                }
                Response::MessageFrom(client_id, message) => {
                    self.message_chat
                        .entry(client_id)
                        .or_insert_with(Vec::new) // Initialize with an empty vector if absent
                        .push((Speaker::HimOrHer, message)); // Push the new message directly
                }
                Response::ListClients(list_users) => {
                    //note that in order to receive the list of clients the client is already registered to that server
                    for user_id in list_users{
                        self.communicable_nodes.insert(user_id, CommunicableType::Yes);
                    }
                },

                //todo! handle these files when lillo is ready.
                Response::ListFiles(list_files) => {}
                Response::File(file_path) => {}
                Response::Media(media) => {}
                Response::Err(error)=> {}
                _=> {}
            }
        }
        //todo! if for every server in self.server_registered and if for every clients in server_registered
        //we have a flood_response so map: ClientId-> FloodReceivedFromNode::Received(flood_id) && flood_id == self.flood_id
        //then the flood_status = Finished. To think again!
    }

    fn reassemble_fragments_in_buffer<T: Serialize + for<'de> Deserialize<'de>>(&mut self) -> T {
        let mut keys: Vec<(SessionId, Option<FragmentIndex>)> = self
            .fragment_assembling_buffer
            .keys()
            .cloned() // Get owned copies of keys
            .collect();

        // Sort the keys by fragment index (key.1)
        keys.sort_by_key(|key| key.1);

        // Initialize the serialized message
        let mut serialized_entire_msg = String::new();

        // Iterate through sorted keys to reassemble the message
        for key in keys {
            if let Some(associated_packet) = self.fragment_assembling_buffer.get(&key) {
                match &associated_packet.pack_type {
                    PacketType::MsgFragment(fragment) => {
                        // Safely push data into the serialized message
                        if let Ok(data_str) = std::str::from_utf8(&fragment.data) {
                            serialized_entire_msg.push_str(data_str);
                        } else {
                            panic!("Invalid UTF-8 sequence in fragment for key: {:?}", key);
                        }
                    }
                    _ => {
                        panic!("Unexpected packet type for key: {:?}", key);
                    }
                }
            } else {
                panic!("Missing packet for key: {:?}", key);
            }
        }

        // Deserialize the fully assembled string
        match serde_json::from_str(&serialized_entire_msg) {
            Ok(deserialized_msg) => deserialized_msg,
            Err(e) => panic!("Failed to deserialize message: {}", e),
        }
    }

    ///---------------------------------------------------------------------------------------------------------------------
    /// routing methods
    fn get_packet_destination(packet: Packet) -> NodeId {
        let destination = packet.routing_header.hops.last().unwrap().clone();
        destination
    }

    /// find best source routing header sort by the keys of OrderId and the UsageTimes,in order to improve the efficiency of
    fn get_source_routing_header(&mut self, destination_id: NodeId) -> Option<SourceRoutingHeader> {
        if let Some(routes) = self.routing_table.get(&destination_id) {
            if let Some(min_using_times) = routes.values().min() {
                if let Some(best_path) = routes
                    .iter()
                    .filter(|(_, &using_times)| using_times == *min_using_times)
                    .map(|(path, _)| path)
                    .min_by_key(|path| path.len())
                {
                    // Transform the best path into a vector of NodeId and initialize hop_index to 1
                    let hops = best_path.iter().map(|&(node_id, _)| node_id).collect();
                    return Some(SourceRoutingHeader::new(hops, 1));
                }
            }
        }
        None
    }



    fn removal_node(&mut self, node_id: NodeId) {
        for (_, paths_to_node) in self.routing_table.iter_mut() {
            paths_to_node.retain(|path| !path.iter().any(|&(n, _)| n == node_id));
        }
    }

    fn get_flood_response_initiator(&mut self, flood_response: FloodResponse) -> NodeId {
        flood_response.path_trace.last().map(|(id, _)| *id).unwrap()
    }
    fn filter_flood_responses_from_wanted_destination(&mut self, wanted_destination_id: NodeId) -> HashSet<u64> {
        self.input_packet_disk
            .values()
            .filter_map(|packet| match &packet.pack_type {
                PacketType::FloodResponse(flood_response) => Some(flood_response),
                _ => None,
            })
            .filter_map(|flood_response| {
                if self.get_flood_response_initiator(flood_response.clone()) == wanted_destination_id && flood_response.flood_id == self.flood_id {
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

    fn do_flooding(&mut self) {
        self.flood_id += 1;
        self.session_id += 1;

        // Initialize the flood request with the current flood_id, id, and node type
        let flood_request = FloodRequest::initialize(self.flood_id, self.node_id, NodeType::Client);

        // Prepare the packet with the current session_id and flood_request
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            self.session_id,
            flood_request,
        );

        // Send the packet to each connected node
        for &node_id in self.connected_nodes_ids.iter() {
            self.send_packet_to_connected_node(node_id, packet.clone()); //assuming `send_packet_to_connected_node` takes a cloned packet
        }
    }

    fn handle_flood_request(&mut self, mut request: FloodRequest) {
        //general idea: prepare a flood response and send it back.
        //1. create a flood response packet
        self.session_id += 1;
        request.path_trace.push((self.node_id, self.node_type));
        let mut response: Packet = request.generate_response(self.session_id);

        //hop_index is again set to 1 because you want to send back using the same path.
        response.routing_header.hop_index = 1;

        //2. send it back
        self.send_with_routing(response);
    }


    ///REMINDER:
    ///we assume that the server when receives a flood_request from a client/server, it both:
    ///1) sends back a flood_response to the client
    ///2) forwards the flood_request to his neighbors, (the flood_request stops until encounters a client).
    fn handle_flood_response(&mut self, response: FloodResponse) {
        //destination client/server id
        if let Some((destination_id, destination_type)) = response.path_trace.last().cloned() {
            if destination_type != NodeType::Drone {
                self.edge_nodes.insert(destination_id, destination_type);
                if self.routing_table.get(&destination_id).is_none() && response.flood_id == self.flood_id {
                    // Initialize a new HashMap for the destination_id if it doesn't exist
                    self.routing_table
                        .entry(destination_id)
                        .or_insert_with(HashMap::new)
                        .insert(response.path_trace.clone(), 0); // Insert the path_trace with an initial UsingTimes 0
                }
            }
            if destination_type == NodeType::Server {
                self.send_query(destination_id, Query::AskListClients);
            }
        }
    }



    ///-------------------------------------------------------------------------------------------------------------
    ///various packets handling methods
    fn handle_controller_command(&mut self, command: ClientCommand) {
        match command {
            ClientCommand::AddSender(target_node_id, sender) => {
                self.packet_send.insert(target_node_id, sender);
            }
            ClientCommand::RemoveSender(target_node_id) => {
                self.packet_send.remove(&target_node_id);
            }
        }
    }
    fn handle_received_packet(&mut self, packet: Packet) {
        //insert the packets into the input disk
        self.input_packet_disk.insert((self.session_id, match packet.pack_type{
            PacketType::MsgFragment(Some(fragment)) => {
                self.fragment_assembling_buffer.insert((self.session_id, Some(fragment.fragment_index)), packet.clone());
                Some(fragment.fragment_index)},
            _ => None,
        }),
            packet.clone());

        match packet.clone().pack_type {
            PacketType::Nack(nack) => self.handle_nack(packet, nack),
            PacketType::Ack(ack) => self.handle_ack(packet, ack),
            PacketType::MsgFragment(fragment) => self.handle_fragment(packet, fragment),
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
        }
    }

    fn handle_ack(&mut self, ack_packet: Packet, ack: Ack) {
        ///just remove the received packet from the output buffer
        let packet_id_pair = (ack_packet.session_id - 1, Some(ack.fragment_index));
        self.packets_status
            .entry((ack_packet.session_id - 1, Some(ack.fragment_index)))
            .and_modify(|status| *status = PacketStatus::Sent)
            .or_insert(PacketStatus::Sent);

        self.output_buffer.remove(&packet_id_pair);
    }

    ///----------------------------nack section
    fn handle_nack(&mut self, nack_packet: Packet, nack: Nack) {
        //change packet status
        //(nack_packet.session_id - 1, Some(nack.fragment_index))

        match nack.nack_type.clone() {
            NackType::ErrorInRouting(node_id) => {
                self.handle_error_in_routing(node_id, nack_packet, nack);
            }
            NackType::DestinationIsDrone => self.handle_destination_is_drone(nack_packet, nack),
            NackType::Dropped => self.handle_packdrop(nack_packet, nack),
            NackType::UnexpectedRecipient(node_id) => self.handle_unexpected_recipient(node_id, nack_packet, nack),
        }
    }

    ///handling various type of nack methods
    fn handle_error_in_routing(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack) {
        info!("Drone is crashed or or sender not found {}", node_id);
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::RoutingError));
        match self.flood_status {
            FloodStatus::Finished(flood_id) => {
                if (flood_id != self.flood_id) {
                    self.do_flooding();
                    self.flood_status = FloodStatus::InGoing;
                }
            },
            _ => {}
            ///do nothing, because in both case (when the flood response from the packet destination is received or not)
            /// we can wait till the FloodStatus is Finished, and then we try to resend again, then
            ///if it gives another nack, knowing that the FloodStatus is Finished, so we can
            ///do another flooding to update the better route.
        }
        //the post-part of the handling is done in the send_packets_in_buffer_checking_status
    }

    fn handle_destination_is_drone(&mut self, nack_packet: Packet, nack: Nack) {
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::DroneDestination));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }

    fn handle_pack_dropped(&mut self, nack_packet: Packet, nack: Nack) {
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::Dropped));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }

    fn handle_unexpected_recipient(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack) {
        info!("unexpected recipient found {}", node_id);
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::BeenInWrongRecipient));
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }

    ///usually send back an ack that contains the fragment_index and the session_id of the
    /// packet ack_packet of the ack will be the same session_id of the packet_arrived + 1
    ///so when I handle the ack I can recover the fragment packet doing
    /// packet_disk(ack_packet.session_id -1, Some(ack.fragment_index))


    ///create ack_packet and send it
    fn create_ack_packet(&mut self, destination_id: NodeId, session_id: SessionId, fragment_index: FragmentIndex) -> Packet{
        let routing_header = self.get_source_routing_header(destination_id);
        let ack_packet = Packet::new_ack(
            routing_header.unwrap(),
            session_id,
            fragment_index,
        );
        ack_packet
    }
    fn handle_fragment(&mut self, msg_packet: Packet,  fragment: Fragment) {
        self.input_packet_disk.insert((msg_packet.session_id, Some(fragment.fragment_index)), msg_packet.clone());
        self.fragment_assembling_buffer.insert((msg_packet.session_id, Some(fragment.fragment_index)), msg_packet.clone());

        let routing_header = SourceRoutingHeader{
            hop_index : 1,
            hops: msg_packet.routing_header.hops.iter().rev().copied().collect(),   //when you can, use Copy trait instead of Clone trait, it's more efficient.
        };
        let ack_packet = Packet::new_ack(routing_header, msg_packet.session_id + 1, fragment.fragment_index);
        self.output_packet_disk.insert((msg_packet.session_id, Some(fragment.fragment_index)), ack_packet.clone());
        self.output_buffer.insert((msg_packet.session_id, Some(fragment.fragment_index)), ack_packet);
    }

    ///things that a client does
    fn discovered_servers(&mut self) -> HashSet<ServerId> {
        let discovered_servers: HashSet<ServerId> = self
            .edge_nodes
            .keys()
            .filter(|node_id| matches!(self.edge_nodes.get(node_id), Some(NodeType::Server)))
            .cloned() // Clone just the filtered keys
            .collect();
        discovered_servers
    }
    fn register_to_server(&mut self, server_id: NodeId) {
        if self.discovered_servers().contains(&server_id) {
            //send a query of registration to the server
            self.send_query(server_id, Query::RegisterClient(self.node_id))  //we could avoid the self.node_id getting the initiator_id from packet
        }                                                                    //so we could omit the NodeId in the field in Query::RegisterClient(NodeId)
    }
}
