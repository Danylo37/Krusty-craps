use std::collections::HashMap;
use std::{fs, thread};
use crossbeam_channel::*;

use wg_2024::{
    packet::Packet,
    config::{Client, Config, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    drone::{Drone as DroneTrait},
    network::NodeId,
};

use crate::drones::KrustyCrapDrone;
use crate::server::*;
use crate::clients::*;
use crate::general_use::{ClientCommand, ServerCommand};

pub struct NetworkInit {
    drone_sender_channels: HashMap<NodeId, Sender<ServerCommand>>,
    clients_sender_channels: HashMap<NodeId, Sender<ClientCommand>>,
    servers_sender_channels: HashMap<NodeId, Sender<ServerCommand>>,
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

               //Creating controller
        let (sim_sender_drone, sim_recv_drone) = unbounded();
        let (sim_sender_client, sim_recv_client) = unbounded();
        let (sim_sender_server, sim_recv_server) = unbounded();

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
        self.connect_nodes();

    }

    pub fn create_drones(&mut self, config_drone : Vec<Drone>, controller: &mut SimulationController, to_contr_event: Sender<DroneEvent>) {
        for (i,drone) in config_drone.into_iter().enumerate() {

            //Adding channel to controller)
            let (to_drone_command_sender,drone_get_command_recv) = unbounded();
            controller.register_drone(drone.id,to_drone_command_sender.clone());

            //Creating receiver for Drone
            let (packet_sender, packet_receiver) = unbounded();

            //Storing it for future usages
            self.drone_sender_channels.insert(drone.id, packet_sender.clone());

            //Creating Drone
            thread::spawn(move || {
                let mut drone = SimulationController::create_drone(
                    drone.id,
                    to_contr_event,
                    drone_get_command_recv,
                    packet_receiver,
                    HashMap::new(),
                    pdr);
                drone.run();
            });
        }
    }

    fn create_clients(&mut self, config_client: Vec<Client>, controller: &mut SimulationController ) {
        for client in config_client {
            let (simulation_send, simulation_recv):(Sender<ClientCommand>,Receiver<ClientCommand>) = unbounded();
            controller.register_client(client.id,simulation_send);
            let (packet_sender, packet_receiver) = unbounded();

            self.drone_sender_channels.insert(client.id, packet_sender.clone());

            thread::spawn(move || {
                controller.register_drone(drone.id,simulation_send);
            });
        }
    }
    fn create_servers(&mut self, config_server: Vec<Server>, controller: &mut SimulationController ) {
        for server in config_server {
            let (simulation_send, simulation_recv):(Sender<ServerCommand>,Receiver<ServerCommand>) = unbounded();
            controller.register_drone(server.id,simulation_send);

            thread::spawn(move || {
                //todo
            });
        }
    }

    fn connect_nodes(&self) {

    }

}
