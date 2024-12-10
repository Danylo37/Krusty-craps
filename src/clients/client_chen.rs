use crate::general_use::{ClientCommand, ClientEvent, Query};
use std::vec;
use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{info, debug, error, warn};
use std::collections::{HashMap, HashSet};
use serde_json::ser::State::Empty;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{NackType, NodeType};
use wg_2024::{
    network::NodeId,
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType},
};
use std::sync::{Arc, Mutex};

pub struct ClientChen {
    id: NodeId ,                                   //general node id
    type_: NodeType ,
    connected_drone_ids: Vec<NodeId>, //node ids (of the drones) which the client is connected
    packet_send: HashMap<NodeId, Sender<Packet>>, //each NodeId <--> Sender to the connected_node with NodeId = node1_id
    //so we have different Senders of packets, each is maybe a clone of a Sender<Packet> that
    //sends to a same receiver of a Node
    packet_recv: Receiver<Packet>, //Receiver of this Client, it's unique, the senders to this Receiver are clones of each others.
    controller_send: Sender<ClientEvent>, //the Receiver<ClientEvent> will be in Simulation Controller
    controller_recv: Receiver<ClientCommand>, //the Sender<ClientCommand> will be in Simulation Controller
    path_traces: HashMap<NodeId, HashSet<Vec<(NodeId, NodeType)>>>, //it is maybe a map that for each destination we have a set of paths that //leads to them
    flood_id: u64,
    session_id: Arc<Mutex<u64>>,
}
impl ClientChen {
    pub fn new(
        id: NodeId,
        type_: NodeType,
        connected_drone_ids: Vec<NodeId>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
        path_traces: HashMap<NodeId, HashSet<Vec<(NodeId, NodeType)>>>,
        session_id: Arc<Mutex<u64>>,
    ) -> Self {
        Self {
            id,
            type_,
            connected_drone_ids,
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,
            path_traces,
            flood_id: 0,   //initial value to be 0 for every new client
            session_id,
        }
    }

    pub fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        match command {
                            ClientCommand::AddSender(node_id, sender) => {   //add a sender to a node with id = node_id
                                self.packet_send.insert(node_id, sender);
                            }
                            ClientCommand::RemoveSender(node_id) => {
                                self.packet_send.remove(&node_id);
                            }
                            ClientCommand::AskTypeTo(NodeId) => {
                                self.ask_server_type(NodeId)
                            },
                            ClientCommand::StartFlooding => {
                                self.do_flooding();
                            }
                            ,

                        }
                    }
                },
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match packet.pack_type {
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
                            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request),
                            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
                        }
                    }
                },
            }
        }
    }

    ///////////////////////HANDLING RECEIVING PACKETS
    //handling ack/nack
    pub fn handle_nack(&mut self, nack: Nack) {
        let type_of_nack = nack.nack_type;
        match type_of_nack {
            NackType::ErrorInRouting(node_id) => self.handle_error_in_routing(node_id),
            NackType::DestinationIsDrone => self.handle_destination_is_drone(),
            NackType::Dropped => self.handle_packdrop(),
            NackType::UnexpectedRecipient(node_id) => self.handle_unexpected_recipient(node_id),
        }
    }

    pub fn handle_error_in_routing(&mut self, node_id: NodeId) {
        self.removal_node(node_id);
        self.compute_new_route_to(node_id);
    }

    pub fn removal_node(&mut self, node_id: NodeId) {
        for (_, paths_to_node) in self.path_traces.iter_mut() {
            paths_to_node.retain(|path| !path.iter().any(|&(n, _)| n == node_id));
        }
    }

    pub fn handle_destination_is_drone(&mut self) {
        info!("Invalid destination, change destination");
    }

    pub fn handle_pack_dropped(&mut self) {
        //send again the packet
    }

    pub fn handle_unexpected_recipient(&mut self, node_id: NodeId) {

    }

    pub fn handle_ack(&mut self, ack: Ack) {}

    pub fn handle_fragment(&mut self, fragment: Fragment) {}

    pub fn handle_flood_request(&mut self, request: FloodRequest) {}

    pub fn handle_flood_response(&mut self, response: FloodResponse) {}

    pub fn send_packet_to_connected_drone(&mut self, node_id: NodeId, packet: Packet) {
        if self.connected_drone_ids.contains(&node_id) {
            if let Some(sender) = self.packet_send.get_mut(&node_id) {
                match sender.send(packet) {
                    Ok(_) => debug!("Packet sent to node {}", node_id),
                    Err(err) => error!("Failed to send packet to node {}: {}", node_id, err),
                }
            } else {
                warn!("No sender channel found for node {}", node_id);
            }
        }
    }

    pub fn send_packet_to_node(&mut self, node_id: NodeId, packet: Packet) {
    }
    pub fn register_to_server(&mut self, server_node_id: NodeId) {
    }

    pub fn do_flooding(&mut self) {
        // Initialize the flood request with the current flood_id, id, and node type
        let flood_request = FloodRequest::initialize(self.flood_id, self.id, NodeType::Client);

        // Acquire the lock on the session_id and get its value
        let session_id = self.session_id.lock().unwrap().clone();

        // Prepare the packet with the current session_id and flood_request
        let packet = Packet::new_flood_request(SourceRoutingHeader::empty_route(), session_id, flood_request);

        // Send the packet to each connected drone
        for &node_id in self.connected_drone_ids.iter() {
            self.send_packet_to_connected_drone(node_id, packet.clone()); // assuming `send_packet_to_connected_drone` takes a cloned packet
        }
    }
    pub fn update_routing(&mut self) {
        // Increment the flood_id
        self.flood_id += 1;

        // Lock the session_id and increment it
        let mut session_id = self.session_id.lock().unwrap();
        *session_id += 1;

        //send a flood request to the connected_drones
        self.do_flooding();
    }

    pub fn update_routing_info(&mut self, node_id: NodeId) {

    }


}
