use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    drone::Drone,
    network::NodeId,
    packet::{NodeType, Packet, PacketType}
};
use crate::general_use::{ClientCommand, ClientEvent,
                         ServerCommand, ServerEvent, ServerType, ClientType};

pub struct SimulationState {
    pub nodes: HashMap<NodeId, NodeType>,
    pub topology: HashMap<NodeId, Vec<NodeId>>,
    pub packet_history: Vec<PacketInfo>,
}


#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub source: NodeId,
    pub destination: NodeId,
    pub packet_type: PacketType,
    pub dropped: bool,
}


pub struct SimulationController {
    pub state: SimulationState,
    pub drone_event_sender: Sender<DroneEvent>,
    pub drone_event_receiver: Receiver<DroneEvent>,
    pub client_event_sender: Sender<ClientEvent>,
    pub client_event_receiver: Receiver<ClientEvent>,
    pub server_event_sender: Sender<ServerEvent>,
    pub server_event_receiver: Receiver<ServerEvent>,
    pub command_senders_drones: HashMap<NodeId, Sender<DroneCommand>>,
    pub command_senders_clients: HashMap<NodeId, (Sender<ClientCommand>, ClientType)>,
    pub command_senders_servers: HashMap<NodeId, (Sender<ServerCommand>, ServerType)>,
    pub packet_senders: HashMap<NodeId, Sender<Packet>>,
}


impl SimulationController {
    pub fn new(
        drone_event_sender: Sender<DroneEvent>,
        drone_event_receiver: Receiver<DroneEvent>,
        client_event_sender: Sender<ClientEvent>,
        client_event_receiver: Receiver<ClientEvent>,
        server_event_sender: Sender<ServerEvent>,
        server_event_receiver: Receiver<ServerEvent>,
    ) -> Self {
        Self {
            state: SimulationState {
                nodes: HashMap::new(),
                topology: HashMap::new(),
                packet_history: Vec::new(),
            },
            command_senders_drones: HashMap::new(),
            command_senders_clients: HashMap::new(),
            command_senders_servers: HashMap::new(),
            drone_event_sender,
            drone_event_receiver,
            client_event_sender,
            client_event_receiver,
            server_event_sender,
            server_event_receiver,
            packet_senders: HashMap::new(),
        }
    }

    /// Runs the main simulation loop.
    /// This function continuously processes events, updates the GUI (not implemented), and sleeps briefly.
    pub fn run(&mut self) {  // Note: &mut self since we're modifying state directly
        loop {
            self.process_packet_sent_events();
            self.process_packet_dropped_events();
            self.process_controller_shortcut_events();
            // GUI updates and user input...                                                            TODO
            sleep(Duration::from_millis(100));
        }
    }

    /// Registers a drone with the simulation controller.
    pub fn register_drone(&mut self, node_id: NodeId, command_sender: Sender<DroneCommand>) {
        self.command_senders_drones.insert(node_id, command_sender);
    }

    pub fn register_server(&mut self, node_id: NodeId, command_sender: Sender<ServerCommand>, server_type: ServerType) {

        self.command_senders_servers.insert(node_id, (command_sender, server_type));
    }

    pub fn register_client(&mut self, node_id: NodeId, command_sender: Sender<ClientCommand>, client_type: ClientType) {
        self.command_senders_clients.insert(node_id, (command_sender, client_type));
    }

    /// Spawns a new drone.
    pub fn create_drone<T: Drone + Send + 'static>(&mut self,
                                                   drone_id: NodeId,
                                                   event_sender: Sender<DroneEvent>,
                                                   command_receiver: Receiver<DroneCommand>,
                                                   packet_receiver: Receiver<Packet>,
                                                   connected_nodes: HashMap<NodeId, Sender<Packet>>,
                                                   pdr: f32,
    ) -> Result<T, String> {

        let drone = T::new(
            drone_id,
            self.drone_event_sender.clone(),
            command_receiver,
            packet_receiver,
            connected_nodes,
            pdr,
        );
        Ok(drone)
    }

    /// Processes incoming events from drones.
    /// This function handles `PacketSent`, `PacketDropped`, and `ControllerShortcut` events.
    fn process_packet_sent_events(&mut self) {
        if let Ok(event) = self.drone_event_receiver.try_recv() {
            if let DroneEvent::PacketSent(packet) = event {
                self.handle_packet_sent(packet);
            }
        }
    }

    fn process_packet_dropped_events(&mut self) {
        if let Ok(event) = self.drone_event_receiver.try_recv() {
            if let DroneEvent::PacketDropped(packet) = event {
                self.handle_packet_dropped(packet);
            }
        }
    }

    fn process_controller_shortcut_events(&mut self) {
        if let Ok(event) = self.drone_event_receiver.try_recv() {  // Receive event or continue if none is available
            if let DroneEvent::ControllerShortcut(packet) = event {   // Check event type

                match packet.pack_type {
                    PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                        if let Some(destination) = self.get_destination_from_packet(&packet) {  // Try to get destination

                            // Determine where to send the packet based on the destination ID and node type
                            if self.command_senders_clients.contains_key(&destination) {          //If it's client

                                if let Some((client_sender, _)) = self.command_senders_clients.get(&destination) {
                                    if let Err(e) = client_sender.send(ClientCommand::ShortcutPacket(packet.clone())) {
                                        eprintln!("Error sending to client {}: {:?}", destination, e);
                                    }
                                } else {

                                    eprintln!("No sender found for client {}", destination);
                                }
                            } else if self.command_senders_servers.contains_key(&destination) {   // If it's server
                                if let Some((server_sender, _)) = self.command_senders_servers.get(&destination) {
                                    if let Err(e) = server_sender.send(ServerCommand::ShortcutPacket(packet.clone())) {
                                        eprintln!("Error sending to server {}: {:?}", destination, e);
                                    }
                                } else {
                                    eprintln!("No sender found for server {}", destination);
                                }
                            } else {
                                eprintln!("Invalid destination or unknown node type: {}", destination);
                            }
                        } else {
                            eprintln!("Could not determine destination for ControllerShortcut");
                        }
                    }
                    _ => eprintln!("Unexpected packet type in ControllerShortcut: {:?}", packet.pack_type),
                }
            }
        }
    }

    fn get_source_from_packet(&self, packet: &Packet) -> NodeId {
        if let Some(first_hop) = packet.routing_header.hops.first() {
            return *first_hop;
        }

        match &packet.pack_type {
            PacketType::MsgFragment(_) => {
                if packet.routing_header.hop_index == 1 {
                    *packet.routing_header.hops.first().unwrap()
                } else {
                    packet.routing_header.hops[packet.routing_header.hop_index - 2]
                }
            }
            PacketType::FloodRequest(flood_req) => flood_req.initiator_id,
            PacketType::FloodResponse(flood_res) => flood_res.path_trace.last().unwrap().0,
            PacketType::Ack(_) | PacketType::Nack(_) => 255,
        }
    }


    fn get_destination_from_packet(&self, packet: &Packet) -> Option<NodeId> {
        packet.routing_header.hops.last().copied()
    }

    /// Handles `PacketSent` events, adding packet information to the history.
    fn handle_packet_sent(&mut self, packet: Packet) {
        let destination = self.get_destination_from_packet(&packet).unwrap_or(255); // Provide default if None

        self.state.packet_history.push(PacketInfo {
            source: self.get_source_from_packet(&packet),
            destination,
            packet_type: packet.pack_type.clone(),
            dropped: false,
        });
    }

    /// Handles `PacketDropped` events, adding packet information to the history.
    fn handle_packet_dropped(&mut self, packet: Packet) {
        self.state.packet_history.push(PacketInfo {
            source: self.get_source_from_packet(&packet),
            destination: self.get_destination_from_packet(&packet).unwrap_or(255), // 255 is a valid default
            packet_type: packet.pack_type.clone(),
            dropped: true,
        });
    }

    pub fn add_sender(&mut self, node_id: NodeId, node_type: NodeType, connected_node_id: NodeId, sender: Sender<Packet>) {
        match node_type {
            NodeType::Drone => {
                if let Some(command_sender) = self.command_senders_drones.get(&node_id) {
                    if let Err(e) = command_sender.send(DroneCommand::AddSender(connected_node_id, sender)) {
                        eprintln!("Failed to send AddSender command to drone {}: {:?}", node_id, e);
                    }
                } else {
                    eprintln!("Drone {} not found in controller", node_id);
                }
            }
            NodeType::Client => {
                if let Some((command_sender, _)) = self.command_senders_clients.get(&node_id) {
                    if let Err(e) = command_sender.send(ClientCommand::AddSender(connected_node_id, sender.clone())) {
                        eprintln!("Failed to send AddSender command to client {}: {:?}", node_id, e);
                    }
                } else {
                    eprintln!("Client {} not found in controller", node_id);
                }
            }
            NodeType::Server => {

                if let Some((command_sender, _)) = self.command_senders_servers.get(&node_id) {
                    if let Err(e) = command_sender.send(ServerCommand::AddSender(connected_node_id, sender.clone())) {
                        eprintln!("Failed to send AddSender command to server {}: {:?}", node_id, e);
                    }
                } else {
                    eprintln!("Server {} not found in controller", node_id);
                }
            }
        }
    }

    pub fn remove_sender(&mut self, node_id: NodeId, node_type: NodeType, connected_node_id: NodeId) -> Result<(), String> {
        match node_type {
            NodeType::Drone => {
                if let Some(command_sender) = self.command_senders_drones.get(&node_id) {
                    if let Err(e) = command_sender.send(DroneCommand::RemoveSender(connected_node_id)) {  // Send command, return error if fails
                        return Err(format!("Failed to send RemoveSender command to drone {}: {:?}", node_id, e));
                    }
                    Ok(())

                } else {

                    Err(format!("Drone with ID {} not found", node_id))  // Return error if no sender
                }
            }
            NodeType::Client => {
                if let Some((command_sender, _)) = self.command_senders_clients.get(&node_id) {
                    if let Err(e) = command_sender.send(ClientCommand::RemoveSender(connected_node_id)) {
                        return Err(format!("Failed to send RemoveSender command to client {}: {:?}", node_id, e));
                    }
                    Ok(())
                } else {
                    Err(format!("Client with ID {} not found", node_id))
                }
            }
            NodeType::Server => {

                if let Some((command_sender, _)) = self.command_senders_servers.get(&node_id) {

                    if let Err(e) = command_sender.send(ServerCommand::RemoveSender(connected_node_id)) {
                        return Err(format!("Failed to send RemoveSender command to server {}: {:?}", node_id, e));
                    }
                    Ok(())
                } else {
                    Err(format!("Server with ID {} not found", node_id))
                }
            }
        }
    }

    pub fn set_packet_drop_rate(&mut self, drone_id: NodeId, pdr: f32) {
        if let Some(command_sender) = self.command_senders_drones.get(&drone_id) {
            if let Err(e) = command_sender.send(DroneCommand::SetPacketDropRate(pdr)) { // Error handling
                eprintln!("Failed to send SetPacketDropRate command to drone {}: {:?}", drone_id, e);
            }
        } else {
            eprintln!("Drone {} not found in controller", drone_id);
        }
    }

    /*- This function sends a Crash command to the specified drone_id.
It uses the command_senders map to find the appropriate sender channel.
*/
    pub fn request_drone_crash(&mut self, drone_id: NodeId) -> Result<(), String> {
        if let Some(command_sender) = self.command_senders_drones.get(&drone_id) {
            if let Err(e) = command_sender.send(DroneCommand::Crash) { // Error handling
                eprintln!("Failed to send Crash command to drone {}: {:?}", drone_id, e);
                return Err(format!("Failed to send Crash command to drone {}: {:?}", drone_id, e));
            }
            Ok(())
        } else {
            Err(format!("Drone {} not found in controller", drone_id))
        }
    }

    pub fn run_client_ui(&self, client_id: NodeId) -> Result<(), String> {
        if let Some((command_sender, _)) = self.command_senders_clients.get(&client_id) {
            if let Err(e) = command_sender.send(ClientCommand::RunUI) {
                return Err(format!("Failed to send RunUI command to client {}: {:?}", client_id, e));
            }
            Ok(())
        } else {
            Err(format!("Client with ID {} not found", client_id))
        }
    }

    pub fn get_list_clients(&self) -> Vec<(ClientType, NodeId)> {
        self.command_senders_clients
            .iter()
            .map(|(&id, (_, client_type))| (*client_type, id))
            .collect()
    }

    pub fn get_list_servers(&self) -> Vec<(ServerType, NodeId)> {
        self.command_senders_servers
            .iter()
            .map(|(&id, (_, server_type))| (*server_type, id))
            .collect()
    }

    pub fn request_known_servers(&mut self, client_id: NodeId) -> Result<(), String> {
        if let Some((client_command_sender, _)) = self.command_senders_clients.get(&client_id) {

            if client_command_sender.send(ClientCommand::GetKnownServers).is_err() { // No need to check error here, client should handle it
                return Err(format!("Client {} disconnected", client_id));//Return err if the client disconnected
            }

            //wait for KnownServers event
            let timeout = Duration::from_secs(1);

            //Using recv_client_event_timeout
            if let Some(event) = self.recv_client_event_timeout(timeout) {
                match event {
                    ClientEvent::KnownServers(servers) => {
                        self.update_known_servers(servers);
                        Ok(())
                    },
                    _ => Err("Unexpected client event".to_string()),
                }
            } else {
                Err(format!("Timeout waiting for KnownServers from client {}", client_id))
            }
        } else {
            Err(format!("Client with ID {} not found", client_id))
        }
    }


    fn recv_client_event_timeout(&self, timeout: Duration) -> Option<ClientEvent> {
        let start = std::time::Instant::now();

        loop {
            if let Ok(event) = self.client_event_receiver.try_recv() {
                return Some(event);
            }

            if start.elapsed() >= timeout {
                return None; // Timeout
            }

            sleep(Duration::from_millis(10));
        }

    }



    fn update_known_servers(&mut self, servers: Vec<(NodeId, ServerType, bool)>) {
        for (server_id, server_type, _) in servers { // Iterate over servers and their types

            if let Some((sender, _)) = self.command_senders_servers.get(&server_id) { // Check if server already exists in controller
                self.command_senders_servers.insert(server_id, (sender.clone(), server_type)); //Update server type
            } else {                                                                           //If server not found create it
                let (sender, receiver) = unbounded();                                           //Create channels for server
                self.command_senders_servers.insert(server_id, (sender, server_type));           //Insert server in controller
            }
        }
    }

    pub fn get_server_type(&self, node_id: NodeId) -> ServerType {
        self.command_senders_servers.get(&node_id).map(|(_, server_type)| *server_type).unwrap_or(ServerType::Undefined) // Default if not found
    }

    ///This is the function for asking the server it's type, given the id of the server
    pub fn ask_which_type(&self, client_id: NodeId, server_id: NodeId) -> Result<ServerType, String> {

        if let Some((client_command_sender, _)) = self.command_senders_clients.get(&client_id) {
            if let Err(e) = client_command_sender.send(ClientCommand::AskTypeTo(server_id)) {
                return Err(format!("Failed to send AskServerType command to client {}: {:?}", client_id, e));
            }
            return Ok(self.get_server_type(server_id)); // Return the server type
        }
        Err(format!("Client with id {} not found", client_id))
    }

    pub fn start_flooding_on_client(&self, client_id: NodeId) -> Result<(), String> {

        if let Some((client_command_sender, _)) = self.command_senders_clients.get(&client_id) {
            if let Err(e) = client_command_sender.send(ClientCommand::StartFlooding) {
                return Err(format!("Failed to send StartFlooding command to client {}: {:?}", client_id, e));
            }
            Ok(())
        } else {
            Err(format!("Client with ID {} not found", client_id))
        }
    }

    pub fn ask_server_type_with_client_id(&mut self, client_id: NodeId, server_id: NodeId) -> Result<(), String> {
        if let Some((client_command_sender, _)) = self.command_senders_clients.get(&client_id) {
            if let Err(e) = client_command_sender.send(ClientCommand::AskTypeTo(server_id)) {
                return Err(format!("Failed to send AskServerType command to client {}: {:?}", client_id, e));
            }
            Ok(())
        } else {
            Err(format!("Client with ID {} not found", client_id))
        }
    }
}