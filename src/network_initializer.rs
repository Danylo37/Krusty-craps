use std::collections::HashMap;
use std::{fs, thread};
use crossbeam_channel::*;

use wg_2024::{
    packet::Packet,
    config::{Client, Config, Drone, Server},
    controller::Command,
    drone::{Drone as DroneTrait, DroneOptions},
    network::NodeId,
};

use crate::drones::KrustyCrapDrone;

const GROUPS: [&str; 10] = [
    "Group1",
    "Group2",
    "Group3",
    "Group4",
    "Group5",
    "Group6",
    "Group7",
    "Group8",
    "Group9",
    "Group10"
];

pub struct NetworkInit {
    node_channels: HashMap<NodeId, (Sender<Command>, Receiver<Command>)>,
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

        //Looping to get Drones
        for (i,drone_config) in config.drone.into_iter().enumerate() {

            //Generating Controller channels and Packet Receiver Channel)
            let (simulation_send, simulation_recv):(Sender<Command>,Receiver<Command>) = unbounded();
            let (_packet_send, packet_recv) = unbounded();
            //Inserting in HashMap
            self.node_channels.insert(drone_config.id, (simulation_send.clone(), simulation_recv.clone()));


            //Looping through Groups and creating/activating drones
            let whichGroup = GROUPS.get(i).copied().unwrap();
            thread::spawn(move || {
                let mut drone = match whichGroup{
                    "Group1" => KrustyCrapDrone::new(
                            DroneOptions{
                                id: drone_config.id,
                                sim_contr_send: simulation_send,
                                sim_contr_recv: simulation_recv,
                                packet_recv,
                                packet_send: Default::default(),
                                pdr: 0.0,
                            }
                        ),
                    _ => panic!(),
                };

                drone.run();
            });
        }

        //Looping through servers (we have to decide how to split since we have two)
        for client_config in config.client {
            let (simulation_send, simulation_recv):(Sender<Command>,Receiver<Command>) = unbounded();
            self.node_channels.insert(client_config.id, (simulation_send, simulation_recv));

            thread::spawn(move || {
                //todo
            });
        }

        //Looping through Clients
        for server_config in config.server {
            let (simulation_send, simulation_recv):(Sender<Command>,Receiver<Command>) = unbounded();
            self.node_channels.insert(server_config.id, (simulation_send, simulation_recv));

            thread::spawn(move || {
                //todo
            });
        }

    }

}