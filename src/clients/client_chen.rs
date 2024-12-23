use std::{
    collections::{HashMap, HashSet},
    thread, vec,
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
use wg_2024::packet::NackType::UnexpectedRecipient;
use crate::general_use::{ClientCommand, ClientEvent, NotSentType, PacketStatus, Query};

pub type SessionId = u64;
pub type FloodId = u64;
pub type FragmentIndex = u64;



pub struct ClientChen {
    //data
    node_id: NodeId,
    node_type: NodeType,
    flood_id: FloodId,
    session_id: SessionId,

    //communication
    server_registered: HashSet<NodeId>, //All the servers registered by the client
    connected_nodes_ids: Vec<NodeId>, //I think better HashSet<(NodeId, NodeType)>, but given that we can ask for type.
    packet_send: HashMap<NodeId, Sender<Packet>>, //each connected_node_id is mapped with a sender to the connected_node
    packet_recv: Receiver<Packet>, //Receiver of this Client, it's unique, the senders to this receiver are clones of each others.
    controller_send: Sender<ClientEvent>, //the Receiver<ClientEvent> will be in Simulation Controller
    controller_recv: Receiver<ClientCommand>, //the Sender<ClientCommand> will be in Simulation Controller

    //storage
    routing_table: HashMap<NodeId, Vec<(NodeId, NodeType)>>, //it is maybe a map that for each destination we have a set of paths that //leads to them
    input_buffer: HashMap<(SessionId, Option<FragmentIndex>), Packet>, //temporary storage for the fragments to recombine,
    output_buffer: HashMap<(SessionId, Option<FragmentIndex>), Packet>, //temporary storage for the messages to send
    input_packet_disk: HashMap<(SessionId, Option<FragmentIndex>), Packet>, //storage of all the packets sent
    output_packet_disk: HashMap<(SessionId, Option<FragmentIndex>), Packet>, //storage of all the packets received
    packets_status: HashMap<(SessionId, Option<FragmentIndex>), PacketStatus>, //map every packet with the status
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
            //data
            node_id,
            node_type,
            flood_id: 0, //initial value to be 0 for every new client
            session_id: (node_id as u64) << 56, //e.g. just put the id of the client at the first 8 bits like 10100101 0000...

            //communication
            server_registered: HashSet::new(),
            connected_nodes_ids,
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,

            //storage
            routing_table: HashMap::new(),
            input_buffer: HashMap::new(),
            output_buffer: HashMap::new(),
            input_packet_disk: HashMap::new(),
            output_packet_disk: HashMap::new(),
            packets_status: HashMap::new(),
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
                self.send_packets_in_buffer_checking_status();
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

    /*
 pub fn send_packets_in_buffer(&mut self) {
     for &packet in self.output_buffer.values().map(|(packet,_)|packet) {
         self.send_with_routing(packet);
     }
 }*/



    pub fn packets_status_actions(&mut self, packet: Packet, packet_status: PacketStatus) { //the &mut self unnecessary
        match packet_status {
            PacketStatus::NotSent(not_sent_type) => {
                match not_sent_type {
                    NotSentType::RoutingError => {
                        let destination = Self::get_packet_destination(packet.clone());
                        if (self.if_current_flood_response_from_wanted_destination_is_received(self.flood_id, destination)) {
                            self.send_with_routing(packet.clone());
                        }
                    }
                    _ => {
                        //todo!

                    }
                }
            }
            _ => { //todo!}
            }
        }
    }


    pub fn send_packets_in_buffer_checking_status(&mut self) {
        ///
        for &(session_id, option_fragment_index) in self.output_buffer.keys(){
            let packet = self.output_packet_disk.get(&(session_id, option_fragment_index)).unwrap().clone();
            match option_fragment_index {
                Some(index) => {
                    // todo!
                },
                None => {
                    // todo!
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



    /// packets creation methods
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

    ///---------------------------------------------------------------------------------------------------------------------
    /// routing methods
    fn get_packet_destination(packet: Packet) -> NodeId {
        let destination = packet.routing_header.hops.last().unwrap().clone();
        destination
    }

    fn get_source_routing_header(&mut self, destination_id: NodeId) -> Option<SourceRoutingHeader> {
        if let Some(route) = self.routing_table.get(&destination_id) {
            let ids: Vec<_> = route.iter().map(|(id, _)| id.clone()).collect();
            let source_routing_header = SourceRoutingHeader::new(ids, 0);
            Some(source_routing_header)
        } else {
            None
        }
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
        self.input_buffer
            .values()
            .filter_map(|packet| match &packet.pack_type {
                PacketType::FloodResponse(flood_response) => Some(flood_response),
                _ => None,
            })
            .filter_map(|flood_response| {
                if self.get_flood_response_initiator(flood_response.clone()) == wanted_destination_id {
                    Some(flood_response.flood_id)
                } else {
                    None
                }
            })
            .collect()
    }
    fn if_current_flood_response_from_wanted_destination_is_received(&mut self, flood_id: u64, wanted_destination_id: NodeId) -> bool {
        self.filter_flood_responses_from_wanted_destination(wanted_destination_id).contains(&flood_id)
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

        // Send the packet to each connected drone
        for &node_id in self.connected_nodes_ids.iter() {
            self.send_packet_to_connected_node(node_id, packet.clone()); // assuming `send_packet_to_connected_node` takes a cloned packet
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
    fn handle_flood_response(&mut self, response: FloodResponse) {
        //destination client/server id
        let (destination_id, destination_type) = match response.path_trace.last().cloned() {
            // Clone it for ownership
            Some((destination_id, destination_type)) => (destination_id, destination_type),
            None => {
                println!("No elements in path_trace");
                return;
            }
        };
        if (destination_type != NodeType::Drone) {
            self.routing_table
                .insert(destination_id, response.path_trace);
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
            ClientCommand::AskTypeTo(target_node_id) => self.ask_server_type(target_node_id),
            ClientCommand::StartFlooding => {
                self.do_flooding();
            }
        }
    }
    fn handle_received_packet(&mut self, packet: Packet) {
        self.input_packet_disk.insert((self.session_id, match packet.pack_type{
            PacketType::MsgFragment(Some(fragment)) => Some(fragment.fragment_index),
            _ => None,
        }),
            packet.clone());

        match packet.clone().pack_type {
            PacketType::Nack(nack) => self.handle_nack(packet, nack),
            PacketType::Ack(ack) => self.handle_ack(packet, ack),
            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
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
        self.packets_status
        .entry((nack_packet.session_id - 1, Some(nack.fragment_index)))
        .and_modify(|status| *status = PacketStatus::NotSent(NotSentType::RoutingError))
        .or_insert(PacketStatus::NotSent(NotSentType::RoutingError));

        self.do_flooding();
    }

    fn handle_destination_is_drone(&mut self, nack_packet: Packet, nack: Nack) {
        self.packets_status
            .entry((nack_packet.session_id - 1, Some(nack.fragment_index)))
            .and_modify(|status| *status = PacketStatus::NotSent(NotSentType::DroneDestination))
            .or_insert(PacketStatus::NotSent(NotSentType::DroneDestination));

        //then we handle in the send_packets_in_buffer_checking_status
    }

    fn handle_pack_dropped(&mut self, nack_packet: Packet, nack: Nack) {
        self.packets_status
            .entry((nack_packet.session_id - 1, Some(nack.fragment_index)))
            .and_modify(|status| *status = PacketStatus::NotSent(NotSentType::Dropped))
            .or_insert(PacketStatus::NotSent(NotSentType::Dropped));

        //then we handle in the send_packets_in_buffer_checking_status
    }
    fn handle_unexpected_recipient(&mut self, node_id: NodeId, nack_packet: Packet, nack: Nack) {
        self.packets_status
            .entry((nack_packet.session_id - 1, Some(nack.fragment_index)))
            .and_modify(|status| *status = PacketStatus::NotSent(NotSentType::BeenInWrongRecipient))
            .or_insert(PacketStatus::NotSent(NotSentType::BeenInWrongRecipient));

        //then we handle in the send_packets_in_buffer_checking_status
    }



    ///usually send back an ack that contains the fragment_index and the session_id of the
    /// packet ack_packet of the ack will be the same session_id of the packet_arrived + 1
    ///so when I handle the ack I can recover the fragment packet doing
    /// packet_disk(ack_packet.session_id -1, Some(ack.fragment_index))
    fn handle_fragment(&mut self, fragment: Fragment) {
        //todo!
    }
    ///things that a client does
    fn register_to_server(&mut self, server_node_id: NodeId) {}
}
