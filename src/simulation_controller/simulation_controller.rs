use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, NodeEvent};
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
    pub event_receiver: Receiver<NodeEvent>,
}


impl SimulationController {
    /*  - Constructor for the SimulationController. It takes a Receiver<NodeEvent> as input, which is the channel through which the simulation controller will receive events from the nodes (drones, clients, servers) in simulation.
        - It initializes a new SimulationController struct. The state is initialized with empty collections (nodes, topology, packet history).
       The command_senders map is also initialized as empty. Importantly, the event_receiver field is initialized with the event_receiver that's passed in as a parameter.
*/
    pub fn new(event_receiver: Receiver<NodeEvent>) -> Self {
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

    /*- Main loop of the SimulationController.
    It runs indefinitely, calling process_events() and then pausing briefly (using sleep.
      - It takes &mut self because it modifies the state field (by adding packet information to the history).
*/
    pub fn run(&mut self) {  // Note: &mut self since we're modifying state directly
        loop {
            self.process_events();
            // GUI updates and user input...                                                            TODO
            sleep(Duration::from_millis(100)); // No need for thread
        }
    }

    /*- This function handles incoming events from the nodes in simulation
      - It continuously tries to receive events from the event_receiver using try_recv().
      - If an event is received (Ok(event)), it uses a match statement to determine the type of event and then processes it accordingly.
      - If no event is available (Err(TryRecvError::Empty)), it breaks out of the inner loop to avoid busy-waiting.
      - If the event channel is disconnected (Err(TryRecvError::Disconnected)), it panics, indicating a critical error.
      - Inside the match arms for PacketSent and PacketDropped, it calls helper functions to get the source and destination node IDs, adds a new PacketInfo entry to the packet_history, and then continues to the next event.
*/
    fn process_events(&mut self) { // &mut self
        loop {
            match self.event_receiver.try_recv() {
                Ok(event) => {
                    match event {  // No lock needed
                        NodeEvent::PacketSent(packet) => {
                            self.state.packet_history.push(PacketInfo { // Access state directly
                                source: self.get_source_from_packet(&packet),
                                destination: self.get_destination_from_packet(&packet),
                                packet_type: packet.pack_type.clone(),
                                dropped: false,
                            });
                        }
                        NodeEvent::PacketDropped(packet) => { // Same logic for PacketDropped
                            self.state.packet_history.push(PacketInfo { // Access state directly
                                source: self.get_source_from_packet(&packet),
                                destination: self.get_destination_from_packet(&packet),
                                packet_type: packet.pack_type.clone(),
                                dropped: true,
                            });
                        }
                        _ => (),
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("Event channel disconnected!"),
            }
        }
    }

    /*- This is a helper function to determine the source NodeId of a packet.
    The implementation logic depends on representing the source in packets (e.g., first hop in the routing header, or a dedicated field within the packet itself, or based on the packet type).
*/
    fn get_source_from_packet(&self, packet: &Packet) -> NodeId {
        // 1. If the source is ALWAYS the first hop in the hops vector:
        if let Some(first_hop) = packet.routing_header.hops.first() {
            return *first_hop;
        }

        // 2. If the source can be determined based on the PACKET TYPE:
        // (This is a more robust approach, especially if the first hop isn't
        // always the source, like in responses or acknowledgements)
        match &packet.pack_type {
            PacketType::MsgFragment(_) => {
                // In a message fragment, the source might be the first hop
                // (if it's the initial fragment) OR a previous hop.
                return if packet.routing_header.hop_index == 1 {
                    *packet.routing_header.hops.first().unwrap()
                } else {
                    packet.routing_header.hops[packet.routing_header.hop_index - 2]
                }
            }
            PacketType::FloodRequest(flood_req) => return flood_req.initiator_id,

            PacketType::FloodResponse(flood_res) => {
                return flood_res.path_trace.last().unwrap().0;
            }

            PacketType::Ack(_ack) => { /* logic based on 'ack' contents*/
                // Default value

            }
            PacketType::Nack(_nack) => { /* logic based on 'nack' contents*/
                // Default value

            }

        }
        255 // Default value if none of the above matches

    }

    /*- This is a helper function to determine the destination NodeId of a packet.*/
    fn get_destination_from_packet(&self, packet: &Packet) -> NodeId {
        let routing_header = &packet.routing_header;
        if routing_header.hops.is_empty() {
            // Handle the empty hops case.                                                              TODO handle the case where the hops vector might be empty.
            return 255; // Default value
        }
        *routing_header.hops.last().unwrap()
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

/*  - This function registers a node with the simulation controller.
   It adds the node's command sender to the command_senders map, allowing the controller to send commands to that node.
*/
    pub fn register_node(&mut self, node_id: NodeId, command_sender: Sender<DroneCommand>) {
        self.command_senders.insert(node_id, command_sender);
    }
}


// Example of how to use SimulationController from network_initializer:

// In network_initializer.rs:

// use crate::simulation_controller::SimulationController;
// ... other imports

/* ... Inside the NetworkInit::new function ...

let (event_sender, event_receiver) = unbounded(); // Create event channel

let mut sim_controller = SimulationController::new(event_receiver); // Initialize controller

// Create and register nodes (example - adapt to your node types)
// ... loop through config/setup to create drones, clients, servers
//     and get their command senders
for _ in 0..10 { // Example: create 10 drones
    let (command_sender, command_receiver) = unbounded();
    let drone_id = get_next_drone_id(); // Logic to get drone IDs                                       TODO logic to get drone IDs

    sim_controller.register_node(drone_id, command_sender);

    // Spawn the drone thread
    thread::spawn(move || {
        let mut drone = Drone::new(/* ... drone parameters, including command_receiver ... */);
        drone.set_event_sender(event_sender.clone()); // Provide event sender to the drone
        drone.run();
    });


}


// ... Similarly, create and register clients and servers and provide them with the event_sender


// Decide how to run the simulation controller:

// 1. On a separate thread:
thread::spawn(move || {
    sim_controller.run();
});

// 2. On the current thread (if no other processing is needed):
// sim_controller.run();


// ... rest of  network initialization

*/
