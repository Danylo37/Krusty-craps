use crate::clients::Client as TraitClient;
///--------------------------
///todo!
/// 1) maybe do a flooding to update those things when the clients starts to run.
/// 2) protocol communication between the client and simulation controller
/// 3) testing
/// 4) handle the chat messages
/// 5) web browser client traits
/// 6) implement the client trait to my client: so the connected_nodes_ids are directly derived from the packet_send
/// Note: when you send the packet with routing the hop_index is increased in the receiving by a drone

use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::{CommandHandler, FragmentsHandler, PacketsReceiver, Router, Sending};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ClientChen {
    // Client's metadata
    pub(crate) metadata: NodeMetadata,
    // Status information
    pub(crate) status: NodeStatus,
    // Communication-related data
    pub(crate) communication: CommunicationInfo,
    // Communication tools
    pub(crate) communication_tools: CommunicationTools,
    // Storage for packets and messages
    pub(crate) storage: NodeStorage,
    // Information about the current network topology
    pub(crate) network_info: NetworkInfo,
}

#[derive(Debug)]
pub enum ClientEvent {
    PacketSent(Packet),
    SenderRemoved(NodeId),
    SenderAdded(NodeId),
    DoingFlood(FloodId),
    FloodIsFinished(FloodId),
}
impl TraitClient for ClientChen {
    fn new(
        id: NodeId,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
    ) -> Self {
        Self {
            // Client's metadata
            metadata: NodeMetadata {
                node_id: id,
                node_type: NodeType::Client,
            },

            // Status
            status: NodeStatus {
                flood_id: 0, // Initial value to be 0 for every new client
                session_id: (id as u64) << 56, // Put the id of the client in the first 8 bits
            },

            // Communication-related data
            communication: CommunicationInfo {
                connected_nodes_ids: packet_send.keys().cloned().collect(),
                registered_communication_servers: HashMap::new(),
                registered_content_servers: HashSet::new(),
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
            storage: NodeStorage {
                irresolute_path_traces: HashSet::new(),
                fragment_assembling_buffer: HashMap::new(),
                output_buffer: HashMap::new(),
                input_packet_disk: HashMap::new(),   //if at the end of the implementation still doesn't need then delete
                output_packet_disk: HashMap::new(),  //if at the end of the implementation still doesn't need then delete
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

    fn run(&mut self) {
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
                    self.update_routing_checking_status();
                 },
            }
        }
    }

}

// Metadata about the client
#[derive(Clone)]
pub(crate) struct NodeMetadata {
    pub(crate) node_id: NodeId,
    pub(crate) node_type: NodeType,
}

// Status of the client
#[derive(Clone)]
pub(crate) struct NodeStatus {
    pub(crate) flood_id: FloodId,
    pub(crate) session_id: SessionId,
}

// Communication-related information
#[derive(Clone)]
pub(crate) struct CommunicationInfo {
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
    pub(crate) registered_communication_servers: HashMap<ServerId, Vec<ClientId>>, // Servers registered by the client with respective registered clients
    pub(crate) registered_content_servers: HashSet<ServerId>,
    pub(crate) routing_table: HashMap<NodeId, HashMap<Vec<(NodeId, NodeType)>, UsingTimes>>, // Routing information per protocol
}

// Tools for communication
#[derive(Clone)]
pub(crate) struct CommunicationTools {
    pub(crate) packet_send: HashMap<NodeId, Sender<Packet>>,  // Sender for each connected node
    pub(crate) packet_recv: Receiver<Packet>,                // Unique receiver for this client
    pub(crate) controller_send: Sender<ClientEvent>,         // Sender for Simulation Controller
    pub(crate) controller_recv: Receiver<ClientCommand>,     // Receiver for Simulation Controller
}

// Storage-related data
#[derive(Clone)]
pub struct NodeStorage {
    pub(crate) irresolute_path_traces: HashSet<Vec<(NodeId, NodeType)>>,   //Temporary storage for the path_traces that are received, but we didn't know how to process them
    pub(crate) fragment_assembling_buffer: HashMap<(SessionId, FragmentIndex), Packet>, // Temporary storage for recombining fragments
    pub(crate) output_buffer: HashMap<(SessionId, FragmentIndex), Packet>,              // Buffer for outgoing messages
    pub(crate) input_packet_disk: HashMap<(SessionId, FragmentIndex), Packet>,          // Storage for received packets
    pub(crate) output_packet_disk: HashMap<(SessionId, FragmentIndex), Packet>,         // Storage for sent packets
    pub(crate) packets_status: HashMap<(SessionId, FragmentIndex), PacketStatus>,       // Map every packet with the status of sending
    pub(crate) message_chat: HashMap<ClientId, Vec<(Speaker, Message)>>,               // Chat messages with other clients
    pub(crate) file_storage: HashMap<ServerId, File>,                                  // Files received from media servers
}

#[derive(Clone)]
pub(crate) struct NetworkInfo{
    pub(crate) topology: HashMap<NodeId, NodeInfo>,
}

#[derive(Clone)]
pub struct NodeInfo{
    pub(crate) node_id: NodeId,
    pub(crate) node_type: NodeType,
    pub(crate) specific_info: SpecificInfo,
}
#[derive(Clone)]
pub enum SpecificInfo{
    ClientInfo(ClientInformation),
    ServerInfo(ServerInformation),
    DroneInfo(DroneInformation),
}
#[derive(Clone)]
pub struct ClientInformation{
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
}
impl ClientInformation{
    fn new(connected_nodes_ids: HashSet<NodeId>) -> ClientInformation{
        ClientInformation{
            connected_nodes_ids
        }
    }
}

#[derive(Clone)]
pub struct ServerInformation{
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
    pub(crate) server_type: ServerType,
}

impl ServerInformation{
    fn new(connected_nodes_ids: HashSet<NodeId>, server_type: ServerType) -> ServerInformation{
        ServerInformation{
            connected_nodes_ids,
            server_type
        }
    }
}

#[derive(Clone)]
pub struct DroneInformation{
    pub(crate) connected_nodes_ids: HashSet<NodeId>,
}
impl DroneInformation{
    fn new(connected_nodes_ids: HashSet<NodeId>) -> DroneInformation{
        DroneInformation{
            connected_nodes_ids
        }
    }
}





