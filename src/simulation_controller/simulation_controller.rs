use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::{NodeType, Packet, PacketType};
use crate::drones::KrustyCrapDrone;
use crate::general_use::{ClientCommand, ClientEvent, ServerCommand, ServerEvent};

pub struct SimulationState {
    pub nodes: HashMap<NodeId, NodeType>,
    pub topology: HashMap<NodeId, Vec<NodeId>>,
    pub packet_history: Vec<PacketInfo>,
    available_drone_types: Vec<String>,  // Store available drone types
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
    pub drone_event_receiver: Receiver<DroneEvent>,
    pub client_event_receiver: Receiver<ClientEvent>,
    pub server_event_receiver: Receiver<ServerEvent>,
    pub command_senders_drones: HashMap<NodeId, Sender<DroneCommand>>,
    pub command_senders_clients: HashMap<NodeId, Sender<ClientCommand>>,
    pub command_senders_servers: HashMap<NodeId, Sender<ServerCommand>>,
    pub drones: HashMap<NodeId, Box<dyn Drone + Send>>,
    pub packet_senders: HashMap<NodeId, Sender<Packet>>,


    // You might want to store clients and servers here as well if you
    // need to access them directly in the SimulationController
    // pub clients: HashMap<NodeId, ...>,                                                                TODO
    // pub servers: HashMap<NodeId, ...>,                                                                TODO

}


impl SimulationController {
    pub fn new(
        drone_event_receiver: Receiver<DroneEvent>,
        client_event_receiver: Receiver<ClientEvent>,
        server_event_receiver: Receiver<ServerEvent>,
        available_drone_types: Vec<String>,
    ) -> Self {
        Self {
            state: SimulationState {
                nodes: HashMap::new(),
                topology: HashMap::new(),
                packet_history: Vec::new(),
                available_drone_types, // Initialize available drone types
            },
            drone_event_receiver,
            command_senders_drones: HashMap::new(),
            client_event_receiver,
            command_senders_clients: HashMap::new(),
            server_event_receiver,
            command_senders_servers: HashMap::new(),
            drones: HashMap::new(),
            packet_senders: HashMap::new(),
        }
    }

    /// Runs the main simulation loop.
    /// This function continuously processes events, updates the GUI (not implemented), and sleeps briefly.
    pub fn run(&mut self) {  // Note: &mut self since we're modifying state directly
        loop {
            self.process_events();
            // GUI updates and user input...                                                            TODO
            sleep(Duration::from_millis(100));
        }
    }

    /// Registers a drone with the simulation controller.
    pub fn register_drone(&mut self, node_id: NodeId, command_sender: Sender<DroneCommand>) {
        self.command_senders_drones.insert(node_id, command_sender);
    }

    pub fn register_server(&mut self, node_id: NodeId, command_sender: Sender<ServerCommand>) {
        self.command_senders_servers.insert(node_id, command_sender);
    }

    pub fn register_client(&mut self, node_id: NodeId, command_sender: Sender<ClientCommand>) {
        self.command_senders_clients.insert(node_id, command_sender);
    }

    /// Spawns a new drone.
    pub fn create_drone(
        &mut self,
        drone_id: NodeId,
        event_sender: Sender<DroneEvent>,
        command_receiver: Receiver<DroneCommand>,
        packet_receiver: Receiver<Packet>,
        connected_nodes: Vec<NodeId>,
        pdr: f32,
    ) {

        let drone_type_name = self.state.available_drone_types.pop().unwrap_or_else(|| {
            println!("No more specific drone types available. Using default.");
            "default_drone".to_string() //
        });

        // Create packet senders for connected nodes:
        let packet_senders: HashMap<NodeId, Sender<Packet>> = connected_nodes
            .into_iter()
            .filter_map(|id| {
                self.packet_senders.get(&id).cloned().map(|sender| (id, sender))
            })
            .collect();


        let drone: Box<dyn Drone + Send> = match drone_type_name.as_str() {
            "KrustyCrapDrone" => Box::new(KrustyCrapDrone::new(drone_id, event_sender, command_receiver, packet_receiver, packet_senders, pdr)),
            // Add other drone types here
            _ => Box::new(KrustyCrapDrone::new(drone_id, event_sender, command_receiver, packet_receiver, packet_senders, pdr)),
        };

        self.drones.insert(drone_id, drone); // Store the drone
    }


    /// Processes incoming events from drones.
    /// This function handles `PacketSent`, `PacketDropped`, and `ControllerShortcut` events.
    fn process_packet_sent_events(&mut self) {
        loop {
            match self.drone_event_receiver.try_recv() {
                Ok(DroneEvent::PacketSent(packet)) => self.handle_packet_sent(packet),
                Ok(_) => continue,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("Drone event channel disconnected!"),
            }
        }
    }

    fn process_packet_dropped_events(&mut self) {
        loop {
            match self.drone_event_receiver.try_recv() {
                Ok(DroneEvent::PacketDropped(packet)) => self.handle_packet_dropped(packet),
                Ok(_) => continue,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("Drone event channel disconnected!"),
            }
        }
    }

    fn process_controller_shortcut_events(&mut self) {
        loop {
            match self.drone_event_receiver.try_recv() {
                Ok(DroneEvent::ControllerShortcut(packet)) => {
                    match packet.pack_type {
                        PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                            // Get destination from packet
                            if let Some(destination) = self.get_destination_from_packet(&packet) {
                                // Determine the correct command sender based on destination node type
                                let command_sender = if self.command_senders_drones.contains_key(&destination) {
                                    // Destination is a drone
                                    // Send directly between drones
                                    if let Some(sender) = self.packet_senders.get(&destination){
                                        Some(sender.clone())

                                    }else{
                                        None
                                    }

                                } else if self.command_senders_clients.contains_key(&destination) {
                                    // Destination is a client
                                    // Send to client via its packet sender

                                    if let Some(sender) = self.packet_senders.get(&destination){
                                        Some(sender.clone())
                                    }else{
                                        None
                                    }

                                } else if self.command_senders_servers.contains_key(&destination) {
                                    // Destination is a server
                                    if let Some(sender) = self.packet_senders.get(&destination){
                                        Some(sender.clone())
                                    }else{
                                        None
                                    }
                                } else {
                                    None
                                };

                                if let Some(sender) = command_sender {
                                    if let Err(e) = sender.send(packet.clone()) {
                                        eprintln!("Failed to send packet to destination {}: {:?}", destination, e);
                                    }
                                } else {
                                    eprintln!("Destination {} not found or invalid node type", destination);
                                }
                            } else {
                                eprintln!("Could not determine destination for ControllerShortcut");
                            }
                        }
                        _ => eprintln!("Unexpected packet type in ControllerShortcut: {:?}", packet.pack_type),
                    }
                }
                Ok(_) => continue, // ignore other events
                Err(TryRecvError::Empty) => break,          // Break on Empty
                Err(TryRecvError::Disconnected) => panic!("Event channel disconnected!"),
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
        packet.routing_header.hops.last().copied() // Safe way to get the last element
    }

    /// Handles `PacketSent` events, adding packet information to the history.
    fn handle_packet_sent(&mut self, packet: Packet) {
        let destination = self.get_destination_from_packet(&packet).unwrap_or(255); // Provide default if None

        self.state.packet_history.push(PacketInfo {
            source: self.get_source_from_packet(&packet),
            destination,  // Use unwrapped or default destination
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
            NodeType::Client => { // Similar error handling for clients and servers
                if let Some(command_sender) = self.command_senders_clients.get(&node_id) {
                    if let Err(e) = command_sender.send(ClientCommand::AddSender(connected_node_id, sender)) {
                        eprintln!("Failed to send AddSender command to client {}: {:?}", node_id, e);
                    }
                } else {
                    eprintln!("Client {} not found in controller", node_id);
                }
            }
            NodeType::Server => {
                if let Some(command_sender) = self.command_senders_servers.get(&node_id) {
                    if let Err(e) = command_sender.send(ServerCommand::AddSender(connected_node_id, sender)) {
                        eprintln!("Failed to send AddSender command to server {}: {:?}", node_id, e);
                    }
                } else {
                    eprintln!("Server {} not found in controller", node_id);
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
    pub fn crash_drone(&mut self, drone_id: NodeId) {
        if let Some(command_sender) = self.command_senders_drones.get(&drone_id) {
            if let Err(e) = command_sender.send(DroneCommand::Crash) { // Error handling
                eprintln!("Failed to send Crash command to drone {}: {:?}", drone_id, e);
            }
        } else {
            eprintln!("Drone {} not found in controller", drone_id);
        }
    }
}


// Example usage in network_initializer
/*

// ... in network initialization ...
let (event_sender, event_receiver) = unbounded();
let mut sim_controller = SimulationController::new(event_receiver);

// Spawn drones (example):
for i in 0..5 {  // Or however many drones
    let drone_id = i;
    sim_controller.spawn_drone::<KrustyCrapDrone>(drone_id); // Replace MyDrone with your Drone type
}

// Start the simulation controller on a separate thread
thread::spawn(move || {
    sim_controller.run();
});

// ... rest of network initialization

*/
