use std::collections::HashMap;
use std::{fs, thread};
use crossbeam_channel::*;

use wg_2024::{
    packet::{Packet, NodeType},
    config::{Config,Drone,Server,Client},
    controller::{DroneEvent, DroneCommand},
    network::NodeId,
};
use crate::drones::KrustyCrapDrone;
use crate::server;
use crate::clients;
use crate::general_use::{ClientCommand,ClientEvent, ServerCommand, ServerEvent};
use crate::simulation_controller::SimulationController;

pub struct NetworkInit {
    drone_sender_channels: HashMap<NodeId, Sender<Packet>>,
    clients_sender_channels: HashMap<NodeId, Sender<Packet>>,
    servers_sender_channels: HashMap<NodeId, Sender<Packet>>,
}

impl NetworkInit {
    pub fn new(input: &str) -> NetworkInit {
        NetworkInit {
            drone_sender_channels: HashMap::new(),
            clients_sender_channels: HashMap::new(),
            servers_sender_channels: HashMap::new(),
        }
    }
    fn parse(&mut self, input: &str){

        //Deserializing the TOML file
        let config_data =
            fs::read_to_string("examples/config/input.toml").expect("Unable to read config file");
        let config: Config = toml::from_str(&config_data).expect("Unable to parse TOML");

        //Splitting information - getting data about neighbours
        let mut neighbours: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for drone in &config.drone{
            neighbours.insert(drone.id, drone.connected_node_ids.clone());
        }
        for drone in &config.drone{
            neighbours.insert(drone.id, drone.connected_node_ids.clone());
        }
        for drone in &config.drone{
            neighbours.insert(drone.id, drone.connected_node_ids.clone());
        }

        //Creating the channels for sending Events to Controller (For Drones, Clients and Servers)
        let (to_control_event_drone, control_get_event_drone) = unbounded();
        let (to_control_event_client, control_get_event_client) = unbounded();
        let (to_control_event_server, control_get_event_server) = unbounded();

        
        //Creating controller
        let mut controller = SimulationController::new(
            to_control_event_drone.clone(),
            control_get_event_drone,
            to_control_event_client.clone(),
            control_get_event_client,
            to_control_event_server.clone(),
            control_get_event_server
        );


        //Looping to get Drones
        self.create_drones(config.drone, &mut controller, to_control_event_drone);

        //Looping through servers (we have to decide how to split since we have two)
        self.create_clients(config.client, &mut controller, to_control_event_client);

        //Looping through Clients
        self.create_servers(config.server, &mut controller, to_control_event_server);

        //Connecting the Nodes
        self.connect_nodes(&mut controller, neighbours);

    }

    pub fn create_drones(&mut self, config_drone : Vec<Drone>, controller: &mut SimulationController, to_contr_event: Sender<DroneEvent>) {
        for (i,drone) in config_drone.into_iter().enumerate() {

            //Adding channel to controller
            let (to_drone_command_sender,drone_get_command_recv) = unbounded();
            controller.register_drone(drone.id, to_drone_command_sender);

            //Creating receiver for Drone
            let (packet_sender, packet_receiver) = unbounded();

            //Storing it for future usages
            self.drone_sender_channels.insert(drone.id, packet_sender);

            //Copy of contrEvent
            let copy_contr_event = to_contr_event.clone();

            //Creating Drone
            thread::spawn(move || {
                let mut drone = SimulationController::create_drone(
                    drone.id,
                    copy_contr_event,
                    drone_get_command_recv,
                    packet_receiver,
                    HashMap::new(),
                    drone.pdr);
                drone.run();
            });
        }
    }

    fn create_clients(&mut self, config_client: Vec<Client>, controller: &mut SimulationController, to_contr_event: Sender<ClientEvent> ) {
        for client in config_client {

            let (to_client_command_sender, client_get_command_recv):(Sender<ClientCommand>,Receiver<ClientCommand>) = unbounded();
            controller.register_client(client.id,to_client_command_sender);
            let (packet_sender, packet_receiver) = unbounded();

            //
            self.drone_sender_channels.insert(client.id, packet_sender);

            //Copy of contrEvent
            let copy_contr_event = to_contr_event.clone();

            //FINISH HERE
            // copy_contr_event is the channel to send ClientEvent to the controller
            // client_get_command_recv is obv
            // packet_receiver same
            thread::spawn(move || {

                //client.run();
            });
        }
    }
    fn create_servers(&mut self, config_server: Vec<Server>, controller: &mut SimulationController, to_contr_event: Sender<ServerEvent> ) {
        for server in config_server {
            let (to_server_command_sender, server_get_command_recv):(Sender<ServerCommand>,Receiver<ServerCommand>) = unbounded();
            controller.register_server(server.id, to_server_command_sender);

            //Creating receiver for Server
            let (packet_sender, packet_receiver) = unbounded();

            //Storing it for future usages
            self.servers_sender_channels.insert(server.id, packet_sender);

            //Copy of contrEvent
            let copy_contr_event = to_contr_event.clone();

            thread::spawn(move || {

                let mut server = server::Server::new(
                    server.id,
                    Vec::new(),
                    copy_contr_event,
                    server_get_command_recv,
                    packet_receiver,
                    HashMap::new(),
                    Vec::new(),
                );

                server.run();
            });

        }
    }

    fn connect_nodes(&self, controller: &mut SimulationController, neighbours: HashMap<NodeId, Vec<NodeId>>) {
        for (node_id, connected_node_ids) in neighbours.iter() {
            for &connected_node_id in connected_node_ids {
                // Retrieve the Sender channel based on node type
                let sender_channel = match self.get_sender_for_node(*node_id) {
                    Some((NodeType::Drone, sender)) =>
                        // Use the 'controller' to establish the connection
                        controller.add_sender(*node_id, NodeType::Drone,connected_node_id, sender),
                    
                    Some((NodeType::Client, sender)) =>
                        controller.add_sender(*node_id, NodeType::Client,connected_node_id, sender),
                    
                    Some((NodeType::Server, sender)) =>
                        controller.add_sender(*node_id, NodeType::Server,connected_node_id, sender),
                    
                    None => panic!("Sender channel not found for node {}!", *node_id),
                };
            }
        }
    }

    fn get_sender_for_node(&self, node_id: NodeId) -> Option<(NodeType, Sender<Packet>)> {
        if let Some(sender) = self.drone_sender_channels.get(&node_id) {
            return Some((NodeType::Drone, sender.clone()));
        }
        if let Some(sender) = self.clients_sender_channels.get(&node_id) {
            return Some((NodeType::Client, sender.clone()));
        }
        if let Some(sender) = self.servers_sender_channels.get(&node_id) {
            return Some((NodeType::Server, sender.clone()));
        }
        None // Sender not found in any HashMap
    }
}

