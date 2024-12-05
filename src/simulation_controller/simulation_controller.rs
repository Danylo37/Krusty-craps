use crossbeam_channel::{select_biased, unbounded, Receiver, Sender, TryRecvError};
use crossbeam_channel::RecvError;
use std::collections::HashMap;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::{NodeType, Packet, PacketType};
use crate::drones::KrustyCrapDrone;
use crate::general_use::{ClientCommand, ServerCommand};

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
    pub drone_command_sender: Sender<DroneCommand>,      // For sending commands TO drones
    pub drone_event_receiver: Receiver<DroneEvent>,        // For receiving events FROM drones
    pub client_command_sender: Sender<ClientCommand>,    // For sending commands TO clients
    pub client_event_receiver: Receiver<ClientEvent>,      //  For receiving events FROM clients  (you'll need to define a ClientEvent enum)              TODO
    pub server_command_sender: Sender<ServerCommand>,    // For sending commands TO servers
    pub server_event_receiver: Receiver<ServerEvent>,      // For receiving events FROM server (you'll need to define a ServerEvent enum)               TODO


    pub drones: HashMap<NodeId, Box<dyn Drone + Send>>,
    pub packet_senders: HashMap<NodeId, Sender<Packet>>,


    // You might want to store clients and servers here as well if you
    // need to access them directly in the SimulationController
    // pub clients: HashMap<NodeId, ...>,                                                                TODO
    // pub servers: HashMap<NodeId, ...>,                                                                TODO

}


impl SimulationController {
    pub fn new(
        drone_command_sender: Sender<DroneCommand>,
        drone_event_receiver: Receiver<DroneEvent>,
        client_command_sender: Sender<ClientCommand>,
        client_event_receiver: Receiver<ClientEvent>, //ClientEvent needs to be defined                           TODO
        server_command_sender: Sender<ServerCommand>,
        server_event_receiver: Receiver<ServerEvent>, //ServerEvent needs to be defined                           TODO
        available_drone_types: Vec<String>,

    ) -> Self {
        Self {
            state: SimulationState {
                nodes: HashMap::new(),
                topology: HashMap::new(),
                packet_history: Vec::new(),
                available_drone_types, // Initialize available drone types
            },
            drone_command_sender,
            drone_event_receiver,
            client_command_sender,
            client_event_receiver,
            server_command_sender,
            server_event_receiver,
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
            sleep(Duration::from_millis(100)); // No need for thread
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
        connected_nodes: Vec<NodeId>, // Provide connected nodes
        pdr: f32,
    ) {

        let drone_type_name = self.state.available_drone_types.pop().unwrap_or_else(|| {
            println!("No more specific drone types available. Using default.");
            "default_drone".to_string() // Or however you handle defaults
        });

        // Create packet senders for connected nodes:
        let packet_senders: HashMap<NodeId, Sender<Packet>> = connected_nodes
            .into_iter()
            .filter_map(|id| {
                self.packet_senders.get(&id).cloned().map(|sender| (id, sender))
            })
            .collect();


        let drone: Box<dyn Drone + Send> = match drone_type_name.as_str() {  // Correct as_str() usage
            "KrustyCrapDrone" => Box::new(KrustyCrapDrone::new(drone_id, event_sender, command_receiver, packet_receiver, packet_senders, pdr)), // Correct drone creation
            // Add other drone types here
            _ => Box::new(KrustyCrapDrone::new(drone_id, event_sender, command_receiver, packet_receiver, packet_senders, pdr)), // Default drone type                               TODO
        };

        self.drones.insert(drone_id, drone); // Store the drone
    }


        /// Processes incoming events from drones.
    /// This function handles `PacketSent`, `PacketDropped`, and `ControllerShortcut` events.
        fn process_packet_sent_events(&mut self) {
            loop {
                match self.event_receiver.try_recv() {
                    Ok(DroneEvent::PacketSent(packet)) => self.handle_packet_sent(packet),
                    Ok(_) => continue, // Ignore other events in this function
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("Event channel disconnected!"),
                }
            }
        }

    fn process_packet_dropped_events(&mut self) {
        loop {

            match self.event_receiver.try_recv() {
                Ok(DroneEvent::PacketDropped(packet)) => self.handle_packet_dropped(packet),
                Ok(_) => continue, // Ignore other events in this function
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("Event channel disconnected!"),
            }
        }
    }

    fn process_controller_shortcut_events(&mut self) {
        loop {
            match self.event_receiver.try_recv() {
                Ok(DroneEvent::ControllerShortcut(packet)) => { // Correct matching
                    // ... (Handle ControllerShortcut)
                },

                Ok(_) => continue, // Ignore other events in this function
                Err(TryRecvError::Empty) => break,
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
            PacketType::Ack(_) | PacketType::Nack(_) => 255, // Or handle differently if needed
        }
    }


    fn get_destination_from_packet(&self, packet: &Packet) -> Option<NodeId> {
        let routing_header = &packet.routing_header;
        // More robust handling of empty routing headers:
        routing_header.hops.last().copied()
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

    pub fn add_sender(&mut self, drone_id: NodeId, destination_id: NodeId, sender: Sender<Packet>) {
        if let Some(command_sender) = self.command_senders.get(&drone_id) {
            let _ = command_sender.send(DroneCommand::AddSender(destination_id, sender)); // Handle potential error here if needed          TODO
        } else {
            eprintln!("Drone {} not found", drone_id);
        }
    }


    pub fn set_packet_drop_rate(&mut self, drone_id: NodeId, pdr: f32) {
        if let Some(command_sender) = self.command_senders.get(&drone_id) {
            let _ = command_sender.send(DroneCommand::SetPacketDropRate(pdr)); // Handle potential error                                    TODO
        } else {
            eprintln!("Drone {} not found", drone_id);
        }
    }

        /*- This function sends a Crash command to the specified drone_id.
    It uses the command_senders map to find the appropriate sender channel.
*/
    pub fn crash_drone(&mut self, drone_id: NodeId) {
        if let Some(sender) = self.command_senders.get(&drone_id) {
            if sender.send(DroneCommand::Crash).is_err() {
                eprintln!("Failed to send crash command to drone {}", drone_id);
            }
        } else {
            eprintln!("Drone {} not found", drone_id);
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
