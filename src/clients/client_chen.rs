use crate::general_use::{ClientCommand, ClientEvent, Query};
use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{info, debug, error, warn};
use std::collections::{HashMap, HashSet};
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::NackType;
use wg_2024::{
    network::NodeId,
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType},
};

pub struct ClientChen {
    id: NodeId,                                   //general node id
    connected_drone_ids: Vec<NodeId>, //node ids (of the drones) which the client is connected
    packet_send: HashMap<NodeId, Sender<Packet>>, //each NodeId <--> Sender to the node with NodeId = node1_id}
    //so we have different Senders of packets, each is maybe a clone of a Sender<Packet> that
    //sends to a same receiver of a Node
    packet_recv: Receiver<Packet>, //Receiver of this Client, it's unique, the senders to this Receiver are clones of each others.
    controller_send: Sender<ClientEvent>, //the Receiver<ClientEvent> will be in Simulation Controller
    controller_recv: Receiver<ClientCommand>, //the Sender<ClientCommand> will be in Simulation Controller
    topology: HashMap<NodeId, HashSet<NodeId>>, //it is a map that NodeId <--> set of neighbors of NodeId
    path_traces: HashMap<NodeId, HashSet<Vec<NodeId>>>, //it is maybe a map that for each destination we have a set of paths that
                                                        //leads to them
}
impl ClientChen {
    pub fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
        topology: HashMap<NodeId, HashSet<NodeId>>,
        path_traces: HashMap<NodeId, HashSet<Vec<NodeId>>>,
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,
            topology,
            path_traces,
        }
    }

    pub fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command_res => {
                    /*if let Ok(command) = command_res {
                        match command {
                            ClientCommand::AddSender(node_id, sender) => {   //add a sender to a node with id = node_id
                                self.packet_send.insert(node_id, sender);
                            }
                            ClientCommand::RemoveSender(node_id) => {
                                self.packet_send.remove(&node_id);
                            }
                            
                        }
                    }*/
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
            NackType::ErrorInRouting(node_id) => self.handle_error_in_routing(),
            NackType::DestinationIsDrone => self.handle_destination_is_drone(),
            NackType::Dropped => self.handle_packdrop(),
            NackType::UnexpectedRecipient(node_id) => self.handle_unexpected_recipient(),
        }
    }

    pub fn handle_error_in_routing(&mut self) {
        self.discover_new_route();
    }

    pub fn handle_destination_is_drone(&mut self) {
        info!("Invalid destination, change destination");
    }

    pub fn handle_packdrop(&mut self) {
        //send again the packet
    }

    pub fn handle_unexpected_recipient(&mut self) {}

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


    pub fn eliminate_drone(&mut self, drone_id: NodeId) {
        //remove from the connected drones if it is in the connected drones
        self.connected_drone_ids.retain(|&id| id != drone_id);

        //remove from the topology and from the hashsets of the other drones
        self.topology.remove(&drone_id);
        for (_, neighbors) in self.topology.iter_mut() {
            neighbors.remove(&drone_id);
        }

        //remove the sender to it if there is
        self.packet_send.remove(&drone_id);
    }

    pub fn discover_new_route(&mut self) {
        //send a flood request to the connected_drones
    }


}

