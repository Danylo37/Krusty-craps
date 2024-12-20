use std::collections::HashMap;

use std::{env, fs, thread};
use crossbeam_channel::*;

use wg_2024::{
    packet::{Packet, NodeType},
    config::{Config,Drone,Server,Client},
    controller::{DroneEvent},
    network::NodeId,
};

use wg_2024::drone::Drone as TraitDrone;
use krusty_drone::drone::drone::KrustyCrapDrone;

use crate::servers;
use crate::servers::server::Server as ServerTrait;

use crate::clients;

use crate::general_use::{ClientCommand,ClientEvent, ServerCommand, ServerEvent, ServerType};
use crate::simulation_controller::SimulationController;
use crate::ui::start_ui;

pub struct NetworkInit {
    drone_sender_channels: HashMap<NodeId, Sender<Packet>>,
    clients_sender_channels: HashMap<NodeId, Sender<Packet>>,
    servers_sender_channels: HashMap<NodeId, Sender<Packet>>,
}

impl NetworkInit {
    pub fn new() -> NetworkInit {
        NetworkInit {
            drone_sender_channels: HashMap::new(),
            clients_sender_channels: HashMap::new(),
            servers_sender_channels: HashMap::new(),
        }
    }
    pub fn parse(&mut self, input: &str){

        println!("{:?}", env::current_dir().expect("Failed to get current directory"));

        // Construct the full path by joining the current directory with the input path
        let current_dir = env::current_dir().expect("Failed to get current directory");
        let input_path = current_dir.join(input);  // This combines the current directory with the `input` file name

        //Deserializing the TOML file
        let config_data =
            fs::read_to_string(input_path).expect("Unable to read config file");
        let config: Config = toml::from_str(&config_data).expect("Unable to parse TOML");

        //Splitting information - getting data about the topology

        /*
            NOTE: topology is a hashmap that maps every node (server, client, drone) with its neighbors.
        */

        let mut topology: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for drone in &config.drone{
            topology.insert(drone.id, drone.connected_node_ids.clone());
        }
        for client in &config.client{
            topology.insert(client.id, client.connected_drone_ids.clone());
        }
        for server in &config.server{
            topology.insert(server.id, server.connected_drone_ids.clone());
        }


        //Creating the channels for sending Events to Controller (For Drones, Clients and Servers)
        let (sender_of_drone_events, receiver_of_drone_events) = unbounded();
        let (sender_of_client_events, receiver_of_client_events) = unbounded();
        let (sender_of_server_events, receiver_of_server_events) = unbounded();

        
        //Creating controller
        let mut controller = SimulationController::new(
            /// NOTE: we clone the senders in order to be reused also by the drones, clients and server
            /// using the same logic, the receiver of the events are just for the controller
            /// so we don't want to clone them to make them available for others.

            //drone events
            sender_of_drone_events.clone(),
            receiver_of_drone_events,

            //client events
            sender_of_client_events.clone(),
            receiver_of_client_events,

            //server events
            sender_of_server_events.clone(),
            receiver_of_server_events,
        );

        //create the drones, clients, and servers through the config and connect them according to the network topology
        self.create_drones(config.drone, &mut controller, sender_of_drone_events, topology.clone());
        self.create_clients(config.client, &mut controller, sender_of_client_events, topology.clone());
        self.create_servers(config.server, &mut controller, sender_of_server_events, topology.clone());


        /// we do the connection of the nodes inside the create_drones function
        /// so basically the connect_nodes is useless
        /// the functions of add_sender, remove_sender etc. of the controller will be used in future,
        /// but not during the creation of the nodes.

        //self.connect_nodes(&mut controller, topology);

        println!("Starting UI");
        start_ui(controller);
    }


    ///DRONES GENERATION

    pub fn create_drones(&mut self,
                         drones : Vec<Drone>,
                         controller: &mut SimulationController,
                         drone_events_sender: Sender<DroneEvent>,
                         topology: HashMap<NodeId, Vec<NodeId>>) {

        for drone in drones {
            //Adding channel to controller
            let (command_sender,command_receiver) = unbounded();
            controller.register_drone(drone.id, command_sender);

            //Creating channels with the connected nodes
            let (packet_sender, packet_receiver) = unbounded();

            //Storing it for future usages
            self.drone_sender_channels.insert(drone.id, packet_sender);

            //Clone of the sender of the drone events.
            let drone_events_sender_clone = drone_events_sender.clone();
            let connected_nodes_ids = topology.get(&drone.id).unwrap().clone();

            //connecting the drone with its connected nodes
            let mut packet_senders_collection = HashMap::new();
            for node in connected_nodes_ids {
                packet_senders_collection.insert(node, self.get_sender_for_node(node).unwrap());
            }

            //Creating Drone
            let mut drone = controller.create_drone::<KrustyCrapDrone>(
                drone.id,
                drone_events_sender_clone,
                command_receiver,
                packet_receiver,
                packet_senders_collection,
                drone.pdr);

            thread::spawn(move || {
                match drone {
                    Ok(mut drone) => drone.run(),
                    Err(e) => panic!("{}",e),
                }
            });
        }
    }


    ///CLIENTS GENERATION

    fn create_clients(&mut self,
                      clients: Vec<Client>,
                      controller: &mut SimulationController,
                      client_events_sender: Sender<ClientEvent>,
                      topology: HashMap<NodeId, Vec<NodeId>>) {

        //function implementation
        for client in clients {
            //create command channel between controller and clients
            let (command_sender,command_receiver):(Sender<ClientCommand>,Receiver<ClientCommand>) = unbounded();
            controller.register_client(client.id,command_sender);

            //create packet channel between the client and the other nodes
            let (packet_sender, packet_receiver) = unbounded();
            self.clients_sender_channels.insert(client.id, packet_sender);

            //clone the things to be cloned for problems of moving.
            let client_events_sender_clone = client_events_sender.clone();
            //connection
            let mut connected_nodes_ids = topology.get(&client.id).unwrap().clone();
            let connected_nodes_ids_clone = connected_nodes_ids.clone();
            //packet senders
            let mut packet_senders_collection = HashMap::new();
            for node in connected_nodes_ids {
                packet_senders_collection.insert(node, self.get_sender_for_node(node).unwrap().clone());
            }

            let mut client = clients::client_chen::ClientChen::new(
                client.id,                          //node_id: NodeId
                NodeType::Client,                   //node_type: NodeType
                connected_nodes_ids_clone,          //connected_nodes_ids: Vec<NodeId>
                packet_senders_collection,          //pack_send: HashMap<NodeId, Sender<Packet>>
                packet_receiver,                    //pack_recv: Receiver<Packet>
                client_events_sender_clone,         //controller_send: Sender<ClientEvent>
                command_receiver,                   //controller_recv: Receiver<ClientCommand>
            );

            thread::spawn(move || {
                client.run();
            });

        }
    }

    /// SERVERS GENERATION
    fn create_servers(&mut self,
                      servers: Vec<Server>,
                      controller: &mut SimulationController,
                      server_events_sender: Sender<ServerEvent>,
                      topology: HashMap<NodeId, Vec<NodeId>>) {

        for server in servers {
            let (command_sender,command_receiver):(Sender<ServerCommand>,Receiver<ServerCommand>) = unbounded();
            controller.register_server(server.id, command_sender);

            //Creating sender to this server and receiver of this server
            let (packet_sender, packet_receiver) = unbounded();

            //sender of the events to the controller
            let server_events_sender_clone = server_events_sender.clone();

            //Storing it for future usages
            self.servers_sender_channels.insert(server.id, packet_sender);


            let mut connected_nodes_ids = topology.get(&server.id).unwrap().clone();
            let connected_nodes_ids_clone = connected_nodes_ids.clone();

            let mut packet_senders_collection = HashMap::new();
            for node in connected_nodes_ids {
                packet_senders_collection.insert(node, self.get_sender_for_node(node).unwrap().clone());
            }

            thread::spawn(move || {

                let mut server = servers::communication_server::CommunicationServer::new(
                    server.id,
                    connected_nodes_ids_clone,
                    server_events_sender_clone,
                    command_receiver,
                    packet_receiver,
                    packet_senders_collection,
                );

                server.run();
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
        if let Some(sender) = self.drone_sender_channels.get(&node_id) {
            return Some(sender.clone());
        }
        if let Some(sender) = self.clients_sender_channels.get(&node_id) {
            return Some(sender.clone());
        }
        if let Some(sender) = self.servers_sender_channels.get(&node_id) {
            return Some(sender.clone());
        }
        None // Sender not found in any HashMap
    }

    fn get_type(&self, node_id: &NodeId) -> Option<NodeType> {
        if let Some(sender) = self.drone_sender_channels.get(node_id) {
            return Some(NodeType::Drone);
        }
        if let Some(sender) = self.clients_sender_channels.get(node_id) {
            return Some(NodeType::Client);
        }
        if let Some(sender) = self.servers_sender_channels.get(node_id) {
            return Some(NodeType::Server);
        }
        None //Not found
    }
}

