//I say i did a good job damn lillo

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

use wg_2024::{
    packet::{Fragment, PacketType, Packet},
    network::{NodeId, SourceRoutingHeader},
};

/// From controller to Server
#[derive(Debug, Clone)]
pub enum ServerCommand {
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>)
}

///Server-Controller
pub enum ServerEvent {
}

/// From controller to Client
pub enum ClientCommand {
    //Controller functions
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>),

    //Client behaviour
    AskTypeTo(NodeId),
    StartFlooding,
}

pub enum ClientEvent {

}

//Queries (Client -> Server)
#[derive(Deserialize, Serialize, Debug)]
pub enum Query{
    //Common-shared
    AskType,

    //To Communication Server
    AddClient(String, NodeId),
    AskListClients,
    SendMessageTo(String, String),

    //To Content Server
    //(Text)
    AskListFiles,
    AskFile(u8),
    //(Media)
    AskMedia(String), // String is the reference found in the files
}

//Server -> Client
#[derive(Deserialize, Serialize, Debug)]
pub enum Response {
    //Common-shared
    ServerType(ServerType),

    //From Communication Server
    MessageFrom(String, String),
    ListUsers(Vec<String>),

    //From Content Server
    //(Text)
    ListFiles(Vec<String>),
    File(String),
    //(Media)
    Media(String),

    //General Error
    Err(String)
}

///Material
#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub enum ServerType {
    Communication,
    Text,
    Media,
}

/// ###### Represents a message that is fragmented into smaller pieces for transmission.
pub struct Message {
    fragments: Vec<Fragment>,
    last_fragment_index: usize,
    session_id: u64,
    route: Vec<NodeId>,
}

impl Message {
    /// ###### Creates a new `Message` with the given session ID and route.
    ///
    /// ###### Arguments
    /// * `session_id` - A unique identifier for the session.
    /// * `route` - The sequence of nodes the message will traverse.
    pub fn new(session_id: u64, route: Vec<NodeId>) -> Message {
        Self {
            fragments: Vec::new(),
            last_fragment_index: 0,
            session_id,
            route,
        }
    }

    /// ###### Serializes the provided data and splits it into smaller fragments for sending.
    ///
    /// ###### Arguments
    /// * `data` - The data to be serialized. Must implement the `Serialize` trait.
    ///
    /// ###### Returns
    /// * `true` if the data was successfully serialized and fragmented.
    /// * `false` if serialization fails.
    ///
    /// If serialization fails, the function logs an error (if applicable) and does not modify the state.
    pub fn create_message_of<T: Serialize>(&mut self, data: T) -> bool {
        let serialized_message = match serde_json::to_string(&data) {
            Ok(string) => string,
            Err(_) => return false,
        };

        self.fragments = self.fragment(&serialized_message);
        true
    }

    /// ###### Splits a serialized message into fragments of a fixed size.
    ///
    /// ###### Arguments
    /// * `serialized_msg` - The serialized message as a string slice.
    ///
    /// ###### Returns
    /// A vector of `Fragment` objects representing the split message.
    pub fn fragment(&mut self, serialized_msg: &str) -> Vec<Fragment> {
        let serialized_msg_in_bytes = serialized_msg.as_bytes();
        let length_response = serialized_msg_in_bytes.len();

        let n_fragments = (length_response + 127) / 128;
        (0..n_fragments)
            .map(|i| {
                let start = i * 128;
                let end = ((i + 1) * 128).min(length_response);
                let fragment_data = &serialized_msg[start..end];
                Fragment::from_string(i as u64, n_fragments as u64, fragment_data.to_string())
            })
            .collect()
    }

    /// ###### Updates the route for the message.
    ///
    /// ###### Arguments
    /// * `route` - The new route for the message.
    pub fn update_route(&mut self, route: Vec<NodeId>) {
        self.route = route;
    }

    /// ###### Retrieves the packet for the specified fragment index.
    ///
    /// ###### Arguments
    /// * `fragment_index` - The index of the fragment to retrieve.
    ///
    /// ###### Returns
    /// An `Option<Packet>` containing the packet if the fragment exists, or `None`.
    pub fn get_fragment_packet(&self, fragment_index: usize) -> Option<Packet> {
        if let Some(fragment) = self.fragments.get(fragment_index).cloned() {
            let hops = self.route.clone();
            let routing_header = SourceRoutingHeader {
                hop_index: 0,
                hops,
            };

            let packet = Packet {
                routing_header,
                session_id: self.session_id,
                pack_type: PacketType::MsgFragment(fragment),
            };

            Some(packet)
        } else {
            None
        }
    }

    /// ###### Retrieves the index of the last fragment.
    ///
    /// ###### Returns
    /// The index of the last fragment.
    pub fn get_last_fragment_index(&self) -> usize {
        self.last_fragment_index
    }

    /// ###### Retrieves the route.
    ///
    /// ###### Returns
    /// The route.
    pub fn get_route(&self) -> &Vec<NodeId> {
        &self.route
    }

    /// ###### Increments the index of the last processed or sent fragment.
    pub fn increment_last_index(&mut self) {
        self.last_fragment_index += 1;
    }
}
