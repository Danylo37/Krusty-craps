use std::collections::{HashMap, HashSet};

use crossbeam_channel::*;
use std::{env, fs, thread};

use wg_2024::{
    config::{Client, Config, Drone, Server},
    controller::DroneEvent,
    network::NodeId,
    packet::{NodeType, Packet},
};

use krusty_drone::drone::drone::KrustyCrapDrone;
use wg_2024::drone::Drone as TraitDrone;

use crate::servers;
use crate::servers::server::Server as ServerTrait;

use crate::clients::client_chen::ClientChen;
use crate::general_use::{ClientEvent, ServerEvent};
use crate::simulation_controller::SimulationController;
use crate::ui::start_ui;

pub struct NetworkInitializer {
    drone_channels: HashMap<NodeId, Sender<Packet>>,
    client_channels: HashMap<NodeId, Sender<Packet>>,
    server_channels: HashMap<NodeId, Sender<Packet>>,
}

impl NetworkInitializer {
    pub fn new() -> Self {
        Self {
            drone_channels: HashMap::new(),
            client_channels: HashMap::new(),
            server_channels: HashMap::new(),
        }
    }

    pub fn initialize_from_file(&mut self, config_path: &str) {
        // Log the current directory for debugging purposes
        println!("Current directory: {:?}", env::current_dir().expect("Failed to get current directory"));

        // Construct the full path to the configuration file
        let config_path = env::current_dir()
            .expect("Failed to get current directory")
            .join(config_path);

        // Read and parse the configuration file
        let config_data = fs::read_to_string(config_path).expect("Unable to read config file");
        let config: Config = toml::from_str(&config_data).expect("Failed to parse TOML config");

        // Build the network topology
        let mut topology = HashMap::new();
        for drone in &config.drone {
            topology.insert(drone.id, drone.connected_node_ids.clone());
        }
        for client in &config.client {
            topology.insert(client.id, client.connected_drone_ids.clone());
        }
        for server in &config.server {
            topology.insert(server.id, server.connected_drone_ids.clone());
        }

        // Create event channels for drones, clients, and servers
        let (drone_event_sender, drone_event_receiver) = unbounded();
        let (client_event_sender, client_event_receiver) = unbounded();
        let (server_event_sender, server_event_receiver) = unbounded();

        // Initialize the simulation controller
        let mut controller = SimulationController::new(
            drone_event_sender.clone(),
            drone_event_receiver,
            client_event_sender.clone(),
            client_event_receiver,
            server_event_sender.clone(),
            server_event_receiver,
        );

        // Initialize drones, clients, and servers
        self.create_drones(config.drone, &mut controller, drone_event_sender, topology.clone());
        self.create_clients(config.client, &mut controller, client_event_sender, topology.clone());
        self.create_servers(config.server, &mut controller, server_event_sender, topology.clone());

        // Start the user interface
        println!("Starting User Interface");
        start_ui(controller);
    }

    ///DRONES GENERATION

    fn create_drones(
        &mut self,
        drones: Vec<Drone>,
        controller: &mut SimulationController,
        drone_events_sender: Sender<DroneEvent>,
        topology: HashMap<NodeId, Vec<NodeId>>,
    ) {
        for drone in drones {
            // Adding channel to controller
            let (command_sender, command_receiver) = unbounded();
            controller.register_drone(drone.id, command_sender);

            // Creating channels with the connected nodes
            let (packet_sender, packet_receiver) = unbounded();

            // Storing it for future usages
            self.drone_channels.insert(drone.id, packet_sender);

            // Clone sender for drone events
            let drone_events_sender_clone = drone_events_sender.clone();

            // Gather connected nodes and their senders
            let connected_nodes_ids = topology
                .get(&drone.id)
                .cloned()
                .unwrap_or_default();

            let connected_nodes_ids_set: HashSet<_> = connected_nodes_ids.into_iter().collect();

            let packet_senders_collection: HashMap<_, _> = connected_nodes_ids_set
                .iter()
                .filter_map(|&node| self.get_sender_for_node(node).map(|sender| (node, sender.clone())))
                .collect();

            // Create Drone
            let mut drone_instance = controller.create_drone::<KrustyCrapDrone>(
                drone.id,                       // drone_id: NodeId
                drone_events_sender_clone,      // event_sender: Sender<DroneEvent>
                command_receiver,               // command_receiver: Receiver<Command>
                packet_receiver,                // packet_receiver: Receiver<Packet>
                packet_senders_collection,      // packet_senders: HashMap<NodeId, Sender<Packet>>
                drone.pdr,                      // pdr: f64
            );

            // Spawn a thread for each drone
            thread::spawn(move || {
                match drone_instance {
                    Ok(mut drone) => drone.run(),
                    Err(e) => panic!("{}", e),
                }
            });
        }
    }



    ///CLIENTS GENERATION

    fn create_clients(
        &mut self,
        clients: Vec<Client>,
        controller: &mut SimulationController,
        client_events_sender: Sender<ClientEvent>,
        topology: HashMap<NodeId, Vec<NodeId>>,
    ) {
        for client in clients {
            // Create command channel between controller and clients
            let (command_sender, command_receiver) = unbounded();
            controller.register_client(client.id, command_sender);

            // Create packet channel between the client and the other nodes
            let (packet_sender, packet_receiver) = unbounded();
            self.client_channels.insert(client.id, packet_sender);

            // Clone sender for client events
            let client_events_sender_clone = client_events_sender.clone();

            // Gather connected nodes and their senders
            let connected_nodes_ids = topology
                .get(&client.id)
                .cloned()
                .unwrap_or_default();

            let connected_nodes_ids_set: HashSet<_> = connected_nodes_ids.into_iter().collect();

            let packet_senders_collection: HashMap<_, _> = connected_nodes_ids_set
                .iter()
                .filter_map(|&node| self.get_sender_for_node(node).map(|sender| (node, sender.clone())))
                .collect();

            // Initialize client
            let mut client_instance = ClientChen::new(
                client.id,                       // node_id: NodeId
                NodeType::Client,                // node_type: NodeType
                connected_nodes_ids_set.clone(), // connected_nodes_ids: HashSet<NodeId>
                packet_senders_collection,       // pack_send: HashMap<NodeId, Sender<Packet>>
                packet_receiver,                 // pack_recv: Receiver<Packet>
                client_events_sender_clone,      // controller_send: Sender<ClientEvent>
                command_receiver,                // controller_recv: Receiver<ClientCommand>
            );

            // Spawn a thread for each client
            thread::spawn(move || {
                client_instance.run();
            });
        }
    }
    /// SERVERS GENERATION
    pub fn create_servers(
        &mut self,
        servers: Vec<Server>,
        controller: &mut SimulationController,
        server_events_sender: Sender<ServerEvent>,
        topology: HashMap<NodeId, Vec<NodeId>>,
    ) {
        for server in servers {
            let (command_sender, command_receiver) = unbounded();
            controller.register_server(server.id, command_sender);

            // Creating sender to this server and receiver of this server
            let (packet_sender, packet_receiver) = unbounded();

            // Clone sender for server events
            let server_events_sender_clone = server_events_sender.clone();

            // Storing it for future usage
            self.server_channels.insert(server.id, packet_sender);

            // Gather connected nodes and their senders
            let connected_nodes_ids = topology
                .get(&server.id)
                .cloned()
                .unwrap_or_default();

            let connected_nodes_ids_set: HashSet<_> = connected_nodes_ids.clone().into_iter().collect();

            let packet_senders_collection: HashMap<_, _> = connected_nodes_ids_set
                .iter()
                .filter_map(|&node| self.get_sender_for_node(node).map(|sender| (node, sender.clone())))
                .collect();

            // Create and run server
            thread::spawn(move || {
                let mut server_instance = servers::communication_server::CommunicationServer::new(
                    server.id,
                    connected_nodes_ids,
                    server_events_sender_clone,
                    command_receiver,
                    packet_receiver,
                    packet_senders_collection,
                );

                server_instance.run();
            });
        }
    }



    ///CREATING NETWORK
    ///
    /// not needed function, you do it inside of the create function.
    fn connect_nodes(&self, controller: &mut SimulationController, topology: HashMap<NodeId, Vec<NodeId>>) {
        // Cloning to avoid problems in borrowing
        let cloned_topology = topology.clone();

        // Create the channels
        for (node_id, connected_nodes_ids) in cloned_topology.iter() {
            for &connected_node_id in connected_nodes_ids {

                // Retrieve the Sender channel based on node type
                let node_type = self.get_type(node_id);
                let sender = self.get_sender_for_node(connected_node_id).unwrap();

                // Add the senders to the connected nodes
                match node_type {
                    Some(NodeType::Drone) => controller.add_sender(*node_id, NodeType::Drone ,connected_node_id, sender),
                    Some(NodeType::Client) => controller.add_sender(*node_id, NodeType::Client ,connected_node_id, sender),
                    Some(NodeType::Server) => controller.add_sender(*node_id, NodeType::Server , connected_node_id, sender),
                    
                    None => panic!("Sender channel not found for node {}!", *node_id),
                };
            }
        }

    }

    ///no need to use the option when we are creating senders for every node in the functions of create_drones,...
    ///but it's rather needed for the get method of the vectors...
    fn get_sender_for_node(&self, node_id: NodeId) -> Option<Sender<Packet>> {
        if let Some(sender) = self.drone_channels.get(&node_id) {
            return Some(sender.clone());
        }
        if let Some(sender) = self.client_channels.get(&node_id) {
            return Some(sender.clone());
        }
        if let Some(sender) = self.server_channels.get(&node_id) {
            return Some(sender.clone());
        }
        None // Sender not found in any HashMap
    }

    fn get_type(&self, node_id: &NodeId) -> Option<NodeType> {
        if self.drone_channels.contains_key(node_id) {
            return Some(NodeType::Drone);
        }
        if self.client_channels.contains_key(node_id) {
            return Some(NodeType::Client);
        }
        if self.server_channels.contains_key(node_id) {
            return Some(NodeType::Server);
        }
        None // Node type not found
    }
}

