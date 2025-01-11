//Outside libraries
use std::{
    collections::{HashMap, HashSet},
    env, fs, thread,
};
use crossbeam_channel::*;
use rand::prelude::*;
use tokio::sync::mpsc;
//Wg libraries
use wg_2024::{
    config::{Client, Config, Drone, Server},
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
    packet::{NodeType, Packet},
    drone::Drone as TraitDrone,
};

//Inner libraries
use crate::{
    clients::client::Client as TraitClient,
    general_use::{ClientEvent, ServerEvent, ClientType, ServerType, DroneId, UsingTimes},
    servers::{communication_server::CommunicationServer, text_server::TextServer, media_server::MediaServer, server::Server as ServerTrait},
    simulation_controller::SimulationController,
    ui::start_ui};
use crate::clients::client_danylo::ChatClientDanylo;
use crate::general_use::{ClientCommand, ClientId};
use crate::servers::content;

//Drones
use krusty_drone::KrustyCrapDrone;
use rusty_drones::RustyDrone;
use rolling_drone::RollingDrone;
use rustable_drone::RustableDrone;
use rustbusters_drone::RustBustersDrone;
use rusteze_drone::RustezeDrone;
use fungi_drone::FungiDrone;
use bagel_bomber::BagelBomber;
use skylink::SkyLinkDrone;
use RF_drone::RustAndFurious;
//use bobry_w_locie::drone::BoberDrone;


//UI
use crate::ui_traits::Monitoring;


//Drone Enum + iterator over it
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum DroneBrand {
    KrustyCrap,
    RustBusters,
    
    /*RustyDrone,
    Rustable,
    BagelBomber,
    RustAndFurious,
    Fungi,
    RustEze,
    SkyLink,
    RollingDrones,
    // BobryWLucie,*/
}

impl DroneBrand {
    // Returns an iterator over all variants of DroneBrand
    pub fn iter() -> impl Iterator<Item = DroneBrand> {
        [
            DroneBrand::KrustyCrap,
            DroneBrand::RustBusters,

            /*DroneBrand::Rustable,
            DroneBrand::BagelBomber,
            DroneBrand::RustAndFurious,
            DroneBrand::Fungi,
            DroneBrand::RustyDrone,
            DroneBrand::RustEze,
            DroneBrand::SkyLink,
            DroneBrand::RollingDrones,
            //DroneBrand::BobryWLucie,*/
        ]
            .into_iter()
    }
}

pub struct NetworkInitializer {
    drone_channels: HashMap<NodeId, Sender<Packet>>,
    client_channels: HashMap<NodeId, (Sender<Packet>, ClientType)>,
    server_channels: HashMap<NodeId, (Sender<Packet>, ServerType)>,
    drone_brand_usage: HashMap<DroneBrand, UsingTimes>,
    client_type_usage: HashMap<ClientType, UsingTimes>,
    sender_to_gui: mpsc::Sender<Vec<u8>>
}

impl NetworkInitializer {
    pub fn new(sender_to_gui: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            drone_channels: HashMap::new(),
            client_channels: HashMap::new(),
            server_channels: HashMap::new(),
            drone_brand_usage: DroneBrand::iter().map(|brand| (brand, 0)).collect(),
            client_type_usage: ClientType::iter().map(|client_type| (client_type, 0)).collect(),
            sender_to_gui,
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

            // Prepare parameters array for the macro

            let drone_params = (
                drone.id,
                drone_events_sender_clone,
                command_receiver,
                packet_receiver,
                packet_senders_collection,
                drone.pdr,
            );

            // Use helper function or macro (in this case function) to create and spawn drones based on their brand
            match self.choose_drone_brand_evenly() {
                DroneBrand::Our => self.create_and_spawn_drone::<KrustyCrap>(controller, drone_params),
                DroneBrand::RustBusters => self.create_and_spawn_drone::<RustBustersDrone>(controller, drone_params),
                
                /*DroneBrand::RollingDrones => self.create_and_spawn_drone::<RollingDrone>(controller, drone_params),
                DroneBrand::Rustable => self.create_and_spawn_drone::<RustableDrone>(controller, drone_params),
                DroneBrand::RustyDrone => self.create_and_spawn_drone::<RustyDrone>(controller, drone_params),
                DroneBrand::RustEze => self.create_and_spawn_drone::<RustezeDrone>(controller, drone_params),
                DroneBrand::Fungi => self.create_and_spawn_drone::<FungiDrone>(controller, drone_params),
                DroneBrand::BagelBomber => self.create_and_spawn_drone::<BagelBomber>(controller, drone_params),
                DroneBrand::SkyLink => self.create_and_spawn_drone::<SkyLinkDrone>(controller, drone_params),
                DroneBrand::RustAndFurious => self.create_and_spawn_drone::<RustAndFurious>(controller, drone_params),
                //DroneBrand::BobryWLucie => self.create_and_spawn_drone::<BoberDrone>(controller, drone_params),*/
            }
        }
    }
    fn create_and_spawn_drone<T>(
        &mut self,
        controller: &mut SimulationController,
        drone_params: (
            DroneId,
            Sender<DroneEvent>,
            Receiver<DroneCommand>,
            Receiver<Packet>,
            HashMap<NodeId, Sender<Packet>>,
            f32,
        ),
    ) where
        T: TraitDrone + Send + 'static, // Ensure T implements the Drone trait and is Sendable
    {
        let (drone_id, event_sender, cmd_receiver, pkt_receiver, pkt_senders, pdr) = drone_params;

        let drone_instance = controller.create_drone::<T>(
            drone_id,
            event_sender,
            cmd_receiver,
            pkt_receiver,
            pkt_senders,
            pdr,
        );

        thread::spawn(move || {
            match drone_instance {
                Ok(mut drone) => drone.run(),
                Err(e) => panic!("Failed to run drone {}: {}", drone_id, e),
            }
        });
    }

    fn choose_drone_brand_evenly(&mut self) -> DroneBrand {
        // Transform the DroneBrand enum into iterator and then collect into a vector
        let drone_brands = DroneBrand::iter().collect::<Vec<_>>();
        // We retain the Brands that are least used.
        if let Some(&min_usage) = self.drone_brand_usage.values().min() {
            let min_usage_drone_brands: Vec<_> = drone_brands
                .iter()
                .filter(|&&drone_brand| self.drone_brand_usage.get(&drone_brand) == Some(&min_usage))
                .cloned()
                .collect();
            // From those we choose randomly one Brand and we use it
            if let Some(&chosen_brand) = min_usage_drone_brands.choose(&mut rand::thread_rng()) {
                // Update usage count
                if let Some(usage) = self.drone_brand_usage.get_mut(&chosen_brand) {
                    *usage += 1;
                }
                return chosen_brand;
            }
        }
        //Shouldn't happen
        DroneBrand::Fungi
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

            // Create packet channel between the client and the other nodes
            let (packet_sender, packet_receiver) = unbounded();

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

            let client_params = (
                client.id,
                client_events_sender_clone,
                command_receiver,
                packet_receiver,
                packet_senders_collection,
                );

            let mut client_type;
            match self.choose_client_type_evenly() {
                ClientType::Web => {
                    //To change in Web
                    client_type = ClientType::Chat;
                    self.create_and_spawn_client::<ChatClientDanylo>(client_params);
                    self.client_channels.insert(client.id, (packet_sender , ClientType::Chat));
                },

                ClientType::Chat=> {
                    client_type = ClientType::Chat;
                    self.create_and_spawn_client::<ChatClientDanylo>(client_params);
                    self.client_channels.insert(client.id, (packet_sender , ClientType::Chat));
                }
            };

            controller.register_client(client.id, command_sender, client_type);

        }
    }

    fn choose_client_type_evenly(&mut self) -> ClientType {
        // Transform the DroneBrand enum into iterator and then collect into a vector
        let client_types = ClientType::iter().collect::<Vec<_>>();
        // We retain the Brands that are least used.
        if let Some(&min_usage) = self.client_type_usage.values().min() {
            let min_usage_client_types: Vec<_> = client_types
                .iter()
                .filter(|&&client_type| self.client_type_usage.get(&client_type) == Some(&min_usage))
                .cloned()
                .collect();
            // From those we choose randomly one Brand and we use it
            if let Some(&chosen_type) = min_usage_client_types.choose(&mut rand::thread_rng()) {
                // Update usage count
                if let Some(usage) = self.client_type_usage.get_mut(&chosen_type) {
                    *usage += 1;
                }
                return chosen_type;
            }
        }
        //Shouldn't happen
        ClientType::Web
    }

    fn create_and_spawn_client<T>(   //without gui monitoring
        &mut self,
        client_params: (
            ClientId,
            Sender<ClientEvent>,
            Receiver<ClientCommand>,
            Receiver<Packet>,
            HashMap<NodeId, Sender<Packet>>,
        ),
    ) where
        T: TraitClient + Send + 'static, // Ensure T implements the Client trait and is Sendable
    {
        let (client_id, event_sender, cmd_receiver, pkt_receiver, pkt_senders) = client_params;

        let mut client_instance = T::new(
            client_id,
            pkt_senders,
            pkt_receiver,
            event_sender,
            cmd_receiver,
        );

        thread::spawn(move || {
            client_instance.run();
        });
    }

    fn create_and_spawn_client_with_monitoring<T>(   //with gui monitoring
                                     &mut self,
                                     sender_to_gui: mpsc::Sender<Vec<u8>>,
                                     client_params: (
                                         ClientId,
                                         Sender<ClientEvent>,
                                         Receiver<ClientCommand>,
                                         Receiver<Packet>,
                                         HashMap<NodeId, Sender<Packet>>,
                                     ),
    ) where
        T: TraitClient + Monitoring +  Send + 'static, // Ensure T implements the Client trait and is Sendable
    {
        let (client_id, event_sender, cmd_receiver, pkt_receiver, pkt_senders) = client_params;

        let mut client_instance = T::new(
            client_id,
            pkt_senders,
            pkt_receiver,
            event_sender,
            cmd_receiver,
        );

        thread::spawn(move || {
            client_instance.run_with_monitoring(sender_to_gui);
        });
    }

    /// SERVERS GENERATION
    pub fn create_servers(
        &mut self,
        servers: Vec<Server>,
        controller: &mut SimulationController,
        server_events_sender: Sender<ServerEvent>,
        topology: HashMap<NodeId, Vec<NodeId>>,
    ) {
        let mut text_server_used = false;
        let mut vec_files = Vec::new();

        for server in servers {
            let (command_sender, command_receiver) = unbounded();

            // Creating sender to this server and receiver of this server
            let (packet_sender, packet_receiver) = unbounded();

            // Clone sender for server events
            let server_events_sender_clone = server_events_sender.clone();

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

            //Choosing type
            let server_type;
            //Fast fix on many servers
            let mut server_instance_comm: Option<CommunicationServer> = None;
            let mut server_instance_text: Option<TextServer>= None;
            let mut server_instance_media: Option<MediaServer>= None;

            if (random::<u8>()%2 == 0){
                server_type = ServerType::Communication;

                server_instance_comm = Some(CommunicationServer::new(
                    server.id,
                    server_events_sender_clone,
                    command_receiver,
                    packet_receiver,
                    packet_senders_collection,
                ));

            }else{
                if text_server_used {
                    let content = content::get_media(vec_files.clone());
                    server_type = ServerType::Media;

                    server_instance_media = Some(MediaServer::new(
                        server.id,
                        content,
                        server_events_sender_clone,
                        command_receiver,
                        packet_receiver,
                        packet_senders_collection,
                    ));
                }else{
                    vec_files = content::choose_random_texts();
                    server_type = ServerType::Text;

                    server_instance_text = Some(TextServer::new(
                        server.id,
                        vec_files.iter().map(|(_, file)| file.clone()).collect::<Vec<String>>(),
                        server_events_sender_clone,
                        command_receiver,
                        packet_receiver,
                        packet_senders_collection,
                   ));
                }
            };

            controller.register_server(server.id, command_sender, server_type);

            self.server_channels.insert(server.id, (packet_sender, server_type));

            // Create and run server
            thread::spawn(move ||
                match server_type {
                    ServerType::Communication => {
                        if let Some(mut server_instance) = server_instance_comm {
                            server_instance.run();
                        }
                    },
                    ServerType::Media => {
                        if let Some(mut server_instance) = server_instance_media {
                            server_instance.run();
                        }
                    },
                    ServerType::Text => {
                        if let Some(mut server_instance) = server_instance_text {
                            server_instance.run();
                        }
                    }
                    ServerType::Undefined => panic!("what?")
                }
            );
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
        if let Some((sender, _)) = self.client_channels.get(&node_id) {
            return Some(sender.clone());
        }
        if let Some((sender, _)) = self.server_channels.get(&node_id) {
            return Some(sender.clone());
        }
        None // Sender not found in any HashMap
    }

    fn get_type(&self, node_id: &NodeId) -> Option<NodeType> {
        if self.drone_channels.contains_key(node_id) {
            return Some(NodeType::Drone);
        }
        if self.drone_channels.contains_key(node_id) {
            return Some(NodeType::Client);
        }
        if self.drone_channels.contains_key(node_id) {
            return Some(NodeType::Server);
        }
        None // Node type not found
    }
}

