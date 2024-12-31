///--------------------------
///todo!
/// 1) maybe do a flooding to update those things when the clients starts to run.
/// 2) protocol communication between the client and simulation controller
/// 3) modularity of the project
/// 4) function of updating the topology
/// 5) testing
/// Note: when you send the packet with routing the hop_index is increased in the receiving by a drone
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


pub struct ClientChen {
    //client's data
    node_id: NodeId,
    node_type: NodeType,

    //status
    flood_id: FloodId,
    session_id: SessionId,
    packets_status: HashMap<(SessionId, FragmentIndex), PacketStatus>, //map every packet with the status of sending

    //communication info
    connected_nodes_ids: Vec<NodeId>, //I think better HashSet<(NodeId, NodeType)>, but given that we can ask for type.
    server_registered: HashMap<ServerId, Vec<ClientId>>, //All the servers registered by the client and it's respective list of client registered
                                                         //Notice that if the client is registered to the server then the Vec<ClientId> contains this client.
    servers: HashMap<ServerId, ServerType>,  //All the servers.
    edge_nodes: HashMap<NodeId, NodeType>,  //All the nodes that are not drones, which are all the servers and all the clients discovered
    communicable_nodes: HashSet<NodeId>,   //Thanks that the NodeId type is u8 so it is Hashable
    routing_table: HashMap<NodeId, HashMap<Vec<(NodeId, NodeType)>, UsingTimes>>, //a map that for each destination we have a path according
                                                             //to the protocol
    //communication tools
    packet_send: HashMap<NodeId, Sender<Packet>>,       //each connected_node_id is mapped with a sender to the connected_node
    packet_recv: Receiver<Packet>,                      //Receiver of this Client, it's unique, the senders to this receiver are clones of each others.
    controller_send: Sender<ClientEvent>,               //the Receiver<ClientEvent> will be in Simulation Controller
    controller_recv: Receiver<ClientCommand>,           //the Sender<ClientCommand> will be in Simulation Controller

    //storage: alternative structure for packet storage: HashMap<SessionId, HashMap<FragmentIndex, Packet>>
    fragment_assembling_buffer: HashMap<(SessionId, FragmentIndex), Packet>, //temporary storage for the fragments to recombine,
    output_buffer: HashMap<(SessionId, FragmentIndex), Packet>,              //temporary storage for the messages to send
    input_packet_disk: HashMap<(SessionId, FragmentIndex), Packet>,         //storage of all the packets received
    output_packet_disk: HashMap<(SessionId, FragmentIndex), Packet>, //storage of all the packets sent

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

            //communication info
            connected_nodes_ids,
            servers: HashMap::new(),
            server_registered: HashMap::new(),
            edge_nodes: HashMap::new(),    //because for now the NodeType is not Hashable.
            communicable_nodes: HashSet::new(),
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
        match not_sent_type {
            NotSentType::RoutingError => {
                if self.if_current_flood_response_from_wanted_destination_is_received(destination) {
                    self.send(packet);
                }
            }
            NotSentType::ToBeSent | NotSentType::Dropped => {
                self.send(packet);
            }
            _ => { //we 'll see how to handle them
            }
        }
    }

    fn handle_sent_packet(&mut self, packet: Packet) {
        self.output_buffer.remove(&(
            packet.session_id,
            match packet.pack_type {
                PacketType::MsgFragment(fragment) => fragment.fragment_index,
                _ => 0,
            },
        ));
    }

    pub fn packets_status_sending_actions(&mut self, packet: Packet, packet_status: PacketStatus) {
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
        for &(session_id, fragment_index) in self.output_buffer.keys(){
            let packet = self.output_packet_disk.get(&(session_id, fragment_index)).unwrap().clone();
            self.packets_status_sending_actions(packet, self.packets_status.get(&(session_id, fragment_index)).unwrap().clone());
        }
    }

    pub fn send_packet_to_connected_node(&mut self, target_node_id: NodeId, packet: Packet) {
        let path_trace = packet.routing_header.hops
            .iter()
            .map(|&node_id| {
                // Look up the node_id in edge_nodes, defaulting to NodeType::Drone if not found
                let node_type = self.edge_nodes.get(&node_id).unwrap_or(&NodeType::Drone);
                (node_id, *node_type) // Return a tuple of (NodeId, NodeType)
            })
            .collect::<Vec<_>>();

        // Increase `using_times` by 1 for the corresponding route
        if let Some(routes) = self.routing_table.get_mut(&packet.routing_header.destination().unwrap()) {
            if let Some(using_times) = routes.get_mut(&path_trace) { //there are always some using times because it is initialized to 0 for every route
                *using_times += 1;
            }
        }

        //insert the packet in output buffer, output_packet_dist, packet_status
        self.output_buffer.insert((packet.session_id, match packet.clone().pack_type{
            PacketType::MsgFragment(fragment) => fragment.fragment_index,
            _=> 0,
        }), packet.clone());

        self.output_packet_disk.insert((packet.session_id, 0), packet.clone());
        self.packets_status.insert((packet.session_id, 0), PacketStatus::InProgress);

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

    pub fn send(&mut self, packet: Packet) {    //send with routing
        self.send_packet_to_connected_node(packet.routing_header.current_hop().unwrap(), packet);
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

    ///------------------------- packets methods
    pub fn divide_string_into_slices(&mut self, string: String, max_slice_length: usize) -> Vec<String> {
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

    pub fn msg_to_fragments<T: Serialize>(&mut self, msg: T, destination_id: NodeId) -> Option<HashSet<Packet>> {
        let serialized_msg = serde_json::to_string(&msg).unwrap();
        let mut fragments = HashSet::new(); //fragments are of type Packet
        let msg_slices = self.divide_string_into_slices(serialized_msg, FRAGMENT_DSIZE);
        let number_of_fragments = msg_slices.len();

        if let Some(source_routing_header) = Self.get_source_routing_header(destination_id){
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
            Some(fragments)
        }else{
            None
        }
    }

    fn update_packet_status(
        &mut self,
        session_id: SessionId,
        fragment_index: FragmentIndex,
        status: PacketStatus,
    ) {
        self.packets_status
            .entry((session_id, fragment_index))
            .and_modify(|current_status| *current_status = status.clone())
            .or_insert(status);
    }

    fn create_ack_packet_from_receiving_packet(&mut self, packet: Packet) -> Packet{
        let routing_header = SourceRoutingHeader{
            hop_index : 1,
            hops: packet.routing_header.hops.iter().rev().copied().collect(),   //when you can, use Copy trait instead of Clone trait, it's more efficient.
        };   //nope we need to use the same of which is arrived.
        let ack_packet = Packet::new_ack(routing_header,
                                         packet.session_id + 1,
                                         match packet.clone().pack_type{
                                             PacketType::MsgFragment(fragment)=> fragment.fragment_index,
                                             _=> 0,
                                         });

        ack_packet
    }

    ///--------------------------------------------------------------------------------------------------------------
    /// reassembling received fragments

    fn get_total_n_fragments(&mut self, session_id: SessionId) -> Option<u64> {
        self.fragment_assembling_buffer
            .iter()
            .find_map(|(&(session, _), packet)| {
                if session == session_id {
                    if let PacketType::MsgFragment(fragment) = &packet.pack_type {
                        return Some(fragment.total_n_fragments);
                    }
                }
                None
            })
    }
    fn handle_fragments_in_buffer_with_checking_status<T: Serialize>(&mut self) {
        let sessions: HashMap<SessionId, Vec<FragmentIndex>> = self
            .fragment_assembling_buffer
            .keys()
            .fold(HashMap::new(), |mut acc, &(session_id, fragment_id)| {
                acc.entry(session_id).or_insert_with(Vec::new).push(fragment_id);
                acc
            });

        for (session_id, fragments) in sessions {
            if let Some(total_n_fragments) = self.get_total_n_fragments(session_id) {
                if fragments.len() as u64 == total_n_fragments {
                    // Get the first fragment for the session
                    let first_key = self.fragment_assembling_buffer
                        .keys()
                        .find(|&&(session, _)| session == session_id)
                        .unwrap();

                    // Use the corresponding packet
                    let first_packet = self.fragment_assembling_buffer.get(first_key).unwrap();

                    let initiator_id = Self::get_packet_destination(first_packet.clone());
                    let message: T = self.reassemble_fragments_in_buffer();

                    match message {
                        Response::ServerType(ServerType) => {
                            self.servers.insert(initiator_id, ServerType);
                        }
                        Response::ClientRegistered => {
                            self.server_registered.insert(initiator_id, vec![self.node_id]);
                        }
                        Response::MessageFrom(client_id, message) => {
                            self.message_chat
                                .entry(client_id)
                                .or_insert_with(Vec::new)
                                .push((Speaker::HimOrHer, message));
                        }
                        Response::ListClients(list_users) => {
                            for user_id in list_users {
                                self.communicable_nodes.insert(user_id);
                            }
                        }
                        Response::ListFiles(_) | Response::File(_) | Response::Media(_) => {
                            // Placeholder for file/media handling
                        }
                        Response::Err(error) => {
                            eprintln!("Error received: {:?}", error);
                        }
                        _ => {
                            // Handle other cases if necessary
                        }
                    }
                }
            }
        }
    }

    fn reassemble_fragments_in_buffer<T: Serialize + for<'de> Deserialize<'de>>(&mut self) -> T {
        let mut keys: Vec<(SessionId, FragmentIndex)> = self
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
        let destination = packet.routing_header.destination().unwrap();
        destination
    }

    /// find best source routing header sort by the keys of OrderId and the UsageTimes, in order to improve the efficiency of
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
                    let hops = self.get_hops_from_path_trace(best_path.clone());
                    return Some(SourceRoutingHeader::new(hops, 1));
                }
            }
        }
        None
    }
    fn get_hops_from_path_trace(&mut self, path_trace: Vec<(NodeId, NodeType)>) -> Vec<NodeId> {
        // Transform the best path into a vector of NodeId and initialize hop_index to 1
        let hops = path_trace.iter().map(|&(node_id, _)| node_id).collect();
        hops
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
        //new ids for the flood and new session because of the flood response packet
        self.flood_id += 1;
        self.session_id += 1;

        self.routing_table.clear();
        self.communicable_nodes.clear();


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

    fn handle_flood_request(&mut self, packet: Packet ,mut request: FloodRequest) {
        //store in the input packet disk
        self.input_packet_disk.insert((packet.session_id, 0), packet);   //because this packet is not any fragment packet.
        //general idea: prepare a flood response and send it back.
        self.session_id += 1;
        request.path_trace.push((self.node_id, self.node_type));
        let mut response: Packet = request.generate_response(self.session_id);

        //hop_index is again set to 1 because you want to send back using the same path.
        response.routing_header.hop_index = 1;

        //2. send it back
        if let Some(routes) = self.routing_table.get(&response.clone().routing_header.destination().unwrap()){
            if !routes.is_empty(){
                self.send(response.clone());
                //so it is a direct response and doesn't need to wait in the buffer, because
                //of the select biased function is prioritizing this send in respect to the others.
                //then if this flood response is not received by any one then there will be a nack of this packet sent back from a drone
                //then we need to handle it.
                }
            } else{ //if we don't have routes
                //we insert regardless thanks to the non repeat mechanism of the hashmaps
                self.packets_status.insert((response.session_id, 0), PacketStatus::NotSent(NotSentType::ToBeSent));
                self.output_buffer.insert((response.session_id, 0), response.clone());
            }
        }



    ///REMINDER:
    ///we assume that the server when receives a flood_request from a client/server, it both:
    ///1) sends back a flood_response to the client
    ///2) forwards the flood_request to his neighbors, (the flood_request stops until encounters a client).
    fn handle_flood_response(&mut self, packet: Packet, response: FloodResponse) {
        self.input_packet_disk.insert((packet.session_id, 0), packet);
        if let Some((destination_id, destination_type)) = response.path_trace.last().cloned() {
            // Ignore drones or mismatched flood IDs
            if destination_type == NodeType::Drone || response.flood_id != self.flood_id {
                return;
            }
            // Update edge nodes
            self.edge_nodes.insert(destination_id, destination_type);

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

    fn update_routing_for_server(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId,NodeType)>) {
        //update the communicable_nodes
        self.communicable_nodes.insert(destination_id);  //regardless for the server
        self.send_query(destination_id, Query::AskType);
        self.send_query(destination_id, Query::AskListClients);
        self.routing_table
            .entry(destination_id)
            .or_insert_with(HashMap::new)
            .insert(path_trace.clone(),0);   //why using times is 0, because we do the flooding when all the routing table is cleared.
    }

    fn update_routing_for_client(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId,NodeType)>) {
        let hops = self.get_hops_from_path_trace(path_trace.clone());
        //update the routes only if the routes are valid
        if self.check_if_exists_registered_server_intermediary_in_route(hops) {
            self.routing_table
                .entry(destination_id)
                .or_insert_with(HashMap::new)
                .insert(path_trace, 0);
            self.communicable_nodes.insert(destination_id); //regardless if before there is
        }
    }

    fn check_if_exists_registered_server_intermediary_in_route(&mut self, route: Vec<NodeId>) -> bool {
        for &server_id in self.server_registered.keys(){
            if route.contains(&server_id) {
                return true;
            }
        }
        false
    }

    fn check_if_exists_route_contains_server(&mut self, server_id: ServerId, destination_id: ClientId) -> bool {
        for route in self.routing_table.get(&destination_id).unwrap() {
            if route.0.contains(*server_id){
                return true;
            }
        }
        false
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
        self.decreasing_using_times_when_receiving_packet(&packet);
        self.input_packet_disk.insert((self.session_id, match packet.pack_type{
            PacketType::MsgFragment(Some(fragment)) => {
                self.fragment_assembling_buffer.insert((self.session_id, fragment.fragment_index), packet.clone());
                fragment.fragment_index
            },
            _ => 0, }), packet.clone());

        match packet.clone().pack_type {
            PacketType::Nack(nack) => self.handle_nack(packet, nack),
            PacketType::Ack(ack) => self.handle_ack(packet, ack),
            PacketType::MsgFragment(fragment) => self.handle_fragment(packet, fragment),
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(packet, flood_request),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(packet, flood_response),
        }
    }

    ///FOR THE SERVER:
    /// notice the increasing using_times when sending packet is done in sending to connected nodes in edge nodes
    ///note that just the drones don't need to do that, the servers need to do that but only the part of the route
    ///that cuts the nodes before the server.
    ///and when the server receives an ack_packet directed to the original sender_client then the server
    ///decreases the using times of the first part of the route and increases the second part of the route
    ///contained in the source routing header of the ack_packet.
    fn decreasing_using_times_when_receiving_packet(&mut self, packet: &Packet) {
        // Reverse the hops to get the correct order for path trace
        let hops = packet.routing_header.hops.iter().rev().collect::<Vec<_>>();
        let destination_id = *hops.last().unwrap().clone();
        let path_trace = hops
            .iter()
            .map(|&&node_id| {
                // Look up the node_id in edge_nodes, defaulting to NodeType::Drone if not found
                let node_type = self.edge_nodes.get(&node_id).unwrap_or(&NodeType::Drone);
                (node_id, *node_type) // Return a tuple of (NodeId, NodeType)
            })
            .collect::<Vec<_>>();

        // Decrease `using_times` by 1 for the corresponding route
        if let Some(routes) = self.routing_table.get_mut(&destination_id) {
            if let Some(using_times) = routes.get_mut(&path_trace) {
                *using_times -= 1;
            }
        }
    }

    /// Handles ACK packets.
    fn handle_ack(&mut self, ack_packet: Packet, ack: Ack) {
        // Remove the fragment/packet corresponding to the ACK
        let packet_key = (ack_packet.session_id - 1, ack.fragment_index);
        self.packets_status
            .entry(packet_key)
            .and_modify(|status| *status = PacketStatus::Sent)
            .or_insert(PacketStatus::Sent);
        self.output_buffer.remove(&packet_key);
    }

    /// Handles NACK packets.
    fn handle_nack(&mut self, nack_packet: Packet, nack: Nack) {
        // Handle specific NACK types
        match nack.nack_type.clone() {
            NackType::ErrorInRouting(node_id) =>  self.handle_error_in_routing(node_id, nack_packet, nack),
            NackType::DestinationIsDrone => self.handle_destination_is_drone(nack_packet, nack),
            NackType::Dropped => self.handle_packdrop(nack_packet, nack),
            NackType::UnexpectedRecipient(node_id) => self.handle_unexpected_recipient(node_id, nack_packet, nack),
        }
    }

    ///handling various type of nack methods
    fn handle_error_in_routing(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack) {
        info!("Drone is crashed or or sender not found {}", node_id);
        self.update_packet_status(nack_packet.session_id-1, nack.fragment_index, PacketStatus::NotSent(NotSentType::RoutingError));
        //if using get the mapping the destination of the packet (identified by nack_packet.session_id-1, nack.fragment_index)
        //we have None then the current flooding isn't finished for that destination then we need to wait for the flood response
        //to come such that the routing table is updated.
        //otherwise if we already have we need to do a flooding to update all the routes to that node.
        if self.routing_table.get(&(self.output_buffer.get(&*(nack_packet.session_id-1, nack.fragment_index)))) != None{
            self.do_flooding();
        } else{
            //wait
        }
        //the post-part of the handling is done in the handling flooding response
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

    fn handle_fragment(&mut self, msg_packet: Packet, fragment: Fragment) {
        self.decreasing_using_times_when_receiving_packet(&msg_packet);
        self.input_packet_disk.insert((msg_packet.session_id, fragment.fragment_index), msg_packet.clone());
        self.fragment_assembling_buffer.insert((msg_packet.session_id, fragment.fragment_index), msg_packet.clone());
        //send an ack instantly
        let ack_packet = self.create_ack_packet_from_receiving_packet(msg_packet.clone());
        self.send(ack_packet);
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
