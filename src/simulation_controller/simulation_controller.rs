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
    pub command_senders: HashMap<NodeId, Sender<DroneCommand>>,
    pub event_receiver: Receiver<DroneEvent>,
}


impl SimulationController {
    ///  - Constructor for the SimulationController. It takes a Receiver<NodeEvent> as input, which is the channel through which the simulation controller will receive events from the nodes (drones, clients, servers) in simulation.
    /// - It initializes a new SimulationController struct. The state is initialized with empty collections (nodes, topology, packet history).
    /// The command_senders map is also initialized as empty. Importantly, the event_receiver field is initialized with the event_receiver that's passed in as a parameter.
    pub fn new(event_receiver: Receiver<DroneEvent>) -> Self {
        Self {
            state: SimulationState {
                nodes: HashMap::new(),
                topology: HashMap::new(),
                packet_history: Vec::new(),
            },
            command_senders: HashMap::new(),
            event_receiver,
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

    /// Spawns a new drone on a separate thread.
    pub fn spawn_drone<D: Drone + Send + 'static>(&mut self, drone_id: NodeId) {
        let (event_sender, event_receiver) = unbounded();
        let (command_sender, command_receiver) = unbounded();
        let (packet_sender, packet_receiver) = unbounded();
        let packet_send: HashMap<NodeId, Sender<Packet>> = HashMap::new();
        let pdr = 0.1; // Initial PDR value

        self.register_node(drone_id, command_sender); // Register the command sender *before* spawning the thread

        thread::spawn(move || {
            let mut drone = D::new(drone_id, event_sender, command_receiver, packet_receiver, packet_send, pdr);

            drone.run();
        });
    }

    /// This function registers a node with the simulation controller.
   /// It adds the node's command sender to the command_senders map, allowing the controller to send commands to that node.
    pub fn register_node(&mut self, node_id: NodeId, command_sender: Sender<DroneCommand>) {
        self.command_senders.insert(node_id, command_sender);
    }

        /// Processes incoming events from drones.
    /// This function handles `PacketSent`, `PacketDropped`, and `ControllerShortcut` events.
        fn process_events(&mut self) {
            loop {
                select_biased! {
                recv(self.event_receiver) -> event => match event {
                    Ok(event) => match event {
                        DroneEvent::PacketSent(packet) => self.handle_packet_sent(packet),
                        DroneEvent::PacketDropped(packet) => self.handle_packet_dropped(packet),
                        DroneEvent::ControllerShortcut(packet) => {
                            match packet.pack_type {
                                PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                                    if let Some(destination) = self.get_destination_from_packet(&packet) {
                                        if let Some(command_sender) = self.command_senders.get(&destination) {
                                            let command = DroneCommand::ControllerShortcut(packet);

                                            if let Err(e) = command_sender.send(command) {
                                                eprintln!("Failed to send controller shortcut: {:?}", e);
                                            }
                                        } else {
                                            eprintln!("No command sender found for destination: {:?}", destination);
                                        }
                                    } else {
                                        eprintln!("Could not determine destination for ControllerShortcut");
                                    }
                                }
                                _ => eprintln!("Unexpected packet type in ControllerShortcut"),
                            }
                        },
                    },
                    // Correct error handling:
                    Err(TryRecvError::Disconnected) => panic!("Event channel disconnected!"),  // Correct match
                    Err(TryRecvError::Empty) => break,          // Correct: Break on Empty
                _ => {}} // Removed unnecessary _ => {}
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
