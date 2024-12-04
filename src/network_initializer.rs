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
use crate::general_use::ServerCommand;

pub struct NetworkInit {
    node_channels: HashMap<NodeId, (Sender<ServerCommand>, Receiver<ServerCommand>)>, //Maybe a struct doesn't make that much sense
}

impl NetworkInit {
    pub fn new(input: &str) -> NetworkInit {
        NetworkInit {
            node_channels: HashMap::new(),
        }
    }
    fn parse(&mut self, input: &str){

        //Deserializing the TOML file
        let config_data =
            fs::read_to_string("examples/config/input.toml").expect("Unable to read config file");
        let config: Config = toml::from_str(&config_data).expect("Unable to parse TOML");

               //Creating controller
        let (_, simulation_recv): (_, Receiver<DroneEvent>) = unbounded();
        let mut controller = SimulationController::new(simulation_recv);

        //Looping to get Drones
        self.create_drones(config.drone, &mut controller);

        //Looping through servers (we have to decide how to split since we have two)
        self.create_clients(config.client, &mut controller);

        //Looping through Clients
        self.create_servers(config.server, &mut controller);

        //Connecting the Nodes
        self.connect_nodes();

    }

    pub fn create_drones(&mut self, config_drone : Vec<Drone>, controller: &mut SimulationController, groups: [&str; 12]) {
        for (i,drone) in config_drone.into_iter().enumerate() {

            //Adding channel to controller)
            let (simulation_send,_): (Sender<DroneCommand>,_) = unbounded();
            controller.register_node(drone.id,simulation_send);

            ///DA TOGLIERE (vedo dopo lillo)?? Inserting in HashMap
            ///self.node_channels.insert(drone.id, (simulation_send.clone(), simulation_recv.clone()));

            //Creating receiver for Drone
            let (_, _packet_receiver) = unbounded();

            //Creating Drone
            thread::spawn(move || {
                let mut drone = SimulationController::create_drone(
                    drone_id, event_sender,
                    command_receiver,
                    packet_receiver,
                    HashMap::new(),
                    pdr);
                drone.run();
            });
        }
    }

    fn create_clients(&mut self, config_client: Vec<Client>, controller: &mut SimulationController ) {
        for client in config_client {
            /*let (simulation_send, simulation_recv):(Sender<ServerCommand>,Receiver<ServerCommand>) = unbounded();
            controller.register_node(client.id,simulation_send);
            let (_, _packet_receiver) = unbounded();
            thread::spawn(move || {

            });*/
        }
    }
    fn create_servers(&mut self, config_server: Vec<Server>, controller: &mut SimulationController ) {
        for server in config_server {
            /*let (simulation_send, simulation_recv):(Sender<DroneCommand>,Receiver<DroneCommand>) = unbounded();
            self.node_channels.insert(server.id, (simulation_send, simulation_recv));

            thread::spawn(move || {
                //todo
            });*/
        }
    }

    fn connect_nodes(&self) {

    }

}
