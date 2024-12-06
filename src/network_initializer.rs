use std::collections::HashMap;
use std::{fs, thread};
use crossbeam_channel::*;

use wg_2024::{
    packet::Packet,
    config::{Config,Drone,Server,Client},
    controller::{DroneEvent, DroneCommand},
    network::NodeId,
};
use crate::drones::KrustyCrapDrone;
use crate::server;
use crate::clients;
use crate::general_use::{ClientCommand, ServerCommand};
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
        let (sim_sender_drone, sim_recv_drone) = unbounded();
        let (sim_sender_client, sim_recv_client) = unbounded();
        let (sim_sender_server, sim_recv_server) = unbounded();

        
        //Creating controller
        let mut controller = SimulationController::new(
            sim_sender_drone.clone(),
            sim_recv_drone,
            sim_sender_client.clone(),
            sim_recv_client,
            sim_sender_server.clone(),
            sim_recv_server
        );


        //Looping to get Drones
        self.create_drones(config.drone, &mut controller, sim_sender_drone);

        //Looping through servers (we have to decide how to split since we have two)
        self.create_clients(config.client, &mut controller, sim_sender_client);

        //Looping through Clients
        self.create_servers(config.server, &mut controller, sim_sender_server);

        //Connecting the Nodes
        self.connect_nodes(neighbours);

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

    fn create_clients(&mut self, config_client: Vec<Client>, controller: &mut SimulationController, to_contr_event: Sender<DroneEvent> ) {
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
    fn create_servers(&mut self, config_server: Vec<Server>, controller: &mut SimulationController, to_contr_event: Sender<DroneEvent> ) {
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
        
    }

}

