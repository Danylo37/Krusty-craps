///--------------------------
///todo!
/// 1) maybe do a flooding to update those things when the clients starts to run.
/// 2) protocol communication between the client and simulation controller
/// 4) function of updating the topology
/// 5) testing
/// Note: when you send the packet with routing the hop_index is increased in the receiving by a drone

use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::{CommandHandler, FragmentsHandler, PacketsReceiver, Sending};

pub(crate) struct ClientChen {
    // Client's metadata
    pub(crate) metadata: ClientMetadata,
    // Status information
    pub(crate) status: ClientStatus,
    // Communication-related data
    pub(crate) communication: CommunicationInfo,
    // Communication tools
    pub(crate) communication_tools: CommunicationTools,
    // Storage for packets and messages
    pub(crate) storage: ClientStorage,
    // Information about the current network topology
    pub(crate) network_info: NetworkInfo,
}

impl ClientChen {
    pub(crate) fn new(
        node_id: NodeId,
        node_type: NodeType,
        connected_nodes_ids: HashSet<NodeId>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
    ) -> Self {
        Self {
            // Client's metadata
            metadata: ClientMetadata {
                node_id,
                node_type,
            },

            // Status
            status: ClientStatus {
                flood_id: 0, // Initial value to be 0 for every new client
                session_id: (node_id as u64) << 56, // Put the id of the client in the first 8 bits
            },

            // Communication-related data
            communication: CommunicationInfo {
                connected_nodes_ids,
                server_registered: HashMap::new(),
                routing_table: HashMap::new(),
            },

            // Communication tools
            communication_tools: CommunicationTools {
                packet_send,
                packet_recv,
                controller_send,
                controller_recv,
            },

            // Storage
            storage: ClientStorage {
                fragment_assembling_buffer: HashMap::new(),
                output_buffer: HashMap::new(),
                input_packet_disk: HashMap::new(),
                output_packet_disk: HashMap::new(),
                packets_status: HashMap::new(),
                message_chat: HashMap::new(),
                file_storage: HashMap::new(),
            },

            // Network Info
            network_info: NetworkInfo{
              topology: HashMap::new(),
            },

        }
    }


    ///the client needs to obey to the controller commands
    ///then when there are packets arriving, need to handle them:
    /// -storing the fragments in the fragment buffer in order to reassemble them, send an ack for each of them
    /// -storing all the packets in the input packet disk (we don't unify the input packet disk with the output
    ///  packet disk because otherwise we need another protocol to manage them and I don't want to think much about it)
    ///
    ///in the free time, you need to handle the fragments in the buffer and send the packets in the buffer
    pub(crate) fn run(&mut self) {
        loop {
            select_biased! {
            recv(self.communication_tools.controller_recv) -> command_res => {
                if let Ok(command) = command_res {
                    self.handle_controller_command(command);
                }
            },
            recv(self.communication_tools.packet_recv) -> packet_res => {
                if let Ok(packet) = packet_res {
                    self.handle_received_packet(packet);
                }
            },
            default(std::time::Duration::from_millis(10)) => {
                self.handle_fragments_in_buffer_with_checking_status();
                self.send_packets_in_buffer_with_checking_status();
            },
        }
        }
    }
}

// Metadata about the client
pub(crate) struct ClientMetadata {
    pub(crate) node_id: NodeId,
    pub(crate) node_type: NodeType,
}

// Status of the client
pub(crate) struct ClientStatus {
    pub(crate) flood_id: FloodId,
    pub(crate) session_id: SessionId,
}

// Communication-related information
pub(crate) struct CommunicationInfo {
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
    pub(crate) server_registered: HashMap<ServerId, Vec<ClientId>>, // Servers registered by the client with respective registered clients
    pub(crate) routing_table: HashMap<NodeId, HashMap<Vec<(NodeId, NodeType)>, UsingTimes>>, // Routing information per protocol
}

// Tools for communication
pub(crate) struct CommunicationTools {
    pub(crate) packet_send: HashMap<NodeId, Sender<Packet>>,  // Sender for each connected node
    pub(crate) packet_recv: Receiver<Packet>,                // Unique receiver for this client
    pub(crate) controller_send: Sender<ClientEvent>,         // Sender for Simulation Controller
    pub(crate) controller_recv: Receiver<ClientCommand>,     // Receiver for Simulation Controller
}

// Storage-related data
pub struct ClientStorage {
    pub(crate) fragment_assembling_buffer: HashMap<(SessionId, FragmentIndex), Packet>, // Temporary storage for recombining fragments
    pub(crate) output_buffer: HashMap<(SessionId, FragmentIndex), Packet>,              // Buffer for outgoing messages
    pub(crate) input_packet_disk: HashMap<(SessionId, FragmentIndex), Packet>,          // Storage for received packets
    pub(crate) output_packet_disk: HashMap<(SessionId, FragmentIndex), Packet>,         // Storage for sent packets
    pub(crate) packets_status: HashMap<(SessionId, FragmentIndex), PacketStatus>,       // Map every packet with the status of sending
    pub(crate) message_chat: HashMap<ClientId, Vec<(Speaker, Message)>>,               // Chat messages with other clients
    pub(crate) file_storage: HashMap<ServerId, File>,                                  // Files received from media servers
}

pub(crate) struct NetworkInfo{
    pub(crate) topology: HashMap<NodeId, NodeInfo>,
}


pub struct NodeInfo{
    pub(crate) node_id: NodeId,
    pub(crate) node_type: NodeType,
    pub(crate) specific_info: SpecificInfo,
}
pub enum SpecificInfo{
    ClientInfo(ClientInformation),
    ServerInfo(ServerInformation),
    DroneInfo(DroneInformation),
}

pub struct ClientInformation{
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
}

pub struct ServerInformation{
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
    pub(crate) server_type: ServerType,
}

pub struct DroneInformation{
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
    pub(crate) drone_brand: DroneBrand,
}


pub enum DroneBrand{
    KrustyDrone,  // Our Drone
    RustyDrone,
    Rustable,
    BagelBomber,
    RustAndFurious,
    Fungi,
    RustBusters,
    RustEze,
    SkyLink,
    RollingDrones,
    BobryWLucie,

    //
    Undefined,
}


    ///PROTOCOL NOTES:

    ///REMINDER:
    ///we assume that the server when receives a flood_request from a client/server, it both:
    ///1) sends back a flood_response to the client
    ///2) forwards the flood_request to his neighbors, (the flood_request stops until encounters a client).


    ///FOR THE SERVER:
    /// notice the increasing using_times when sending packet is done in sending to connected nodes in edge nodes
    ///note that just the drones don't need to do that, the servers need to do that but only the part of the route
    ///that cuts the nodes before the server.
    ///and when the server receives an ack_packet directed to the original sender_client then the server
    ///decreases the using times of the first part of the route and increases the second part of the route
    ///contained in the source routing header of the ack_packet.



    ///usually send back an ack that contains the fragment_index and the session_id of the
    /// packet ack_packet of the ack will be the same session_id of the packet_arrived + 1
    ///so when I handle the ack I can recover the fragment packet doing
    /// packet_disk(ack_packet.session_id -1, Some(ack.fragment_index))



