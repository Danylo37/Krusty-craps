use crate::clients::client_chen::{ClientChen, FragmentsHandler, PacketCreator, PacketsReceiver, Sending, ServerInformation, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
impl FragmentsHandler for ClientChen{
    fn handle_fragment(&mut self, msg_packet: Packet, fragment: Fragment) {
        self.decreasing_using_times_when_receiving_packet(&msg_packet);
        self.storage.input_packet_disk.insert((msg_packet.session_id, fragment.fragment_index), msg_packet.clone());
        self.storage.fragment_assembling_buffer.insert((msg_packet.session_id, fragment.fragment_index), msg_packet.clone());
        //send an ack instantly
        let ack_packet = self.create_ack_packet_from_receiving_packet(msg_packet.clone());
        self.send(ack_packet);
    }

    fn get_total_n_fragments(&mut self, session_id: SessionId) -> Option<u64> {
        self.storage.fragment_assembling_buffer
            .iter()
            .find_map(|(&(session, _), packet)| {
                if session == session_id {
                    if let PacketType::MsgFragment(fragment) = &packet.pack_type {
                        return Some(fragment.total_n_fragments);
                    }
                }
                None
            })
    }

    fn handle_fragments_in_buffer_with_checking_status<T: Serialize>(&mut self) {
        // Aggregate session IDs and their corresponding fragment IDs
        let sessions: HashMap<SessionId, Vec<FragmentIndex>> = self
            .storage
            .fragment_assembling_buffer
            .keys()
            .fold(HashMap::new(), |mut acc, &(session_id, fragment_id)| {
                acc.entry(session_id).or_insert_with(Vec::new).push(fragment_id);
                acc
            });

        // Iterate over each session and process fragments
        for (session_id, fragments) in sessions {
            if let Some(total_n_fragments) = self.get_total_n_fragments(session_id) {
                if fragments.len() as u64 == total_n_fragments {
                    // Get the first fragment for the session (avoid redundant lookup)
                    if let Some((first_key, first_packet)) = self.storage.fragment_assembling_buffer
                        .iter()
                        .find(|(&(session, _), _)| session == session_id)
                    {
                        // Get the packet destination
                        let initiator_id = Self::get_packet_destination(first_packet);

                        // Reassemble the fragments into the final message
                        let message: T = self.reassemble_fragments_in_buffer();

                        // Process the message based on its type
                        self.process_message(initiator_id, message);
                    }
                }
            }
        }
    }

    fn process_message<T: Serialize>(&mut self, initiator_id: NodeId, message: T) {
        match message {
            Response::ServerType(server_type) => {
                if let Some(entry) = self.network_info.topology.entry(initiator_id).or_default().specific_info {
                    if let SpecificInfo::ServerInfo(mut server_info) = entry {
                        server_info.server_type = server_type;
                    }
                }
            }
            Response::ClientRegistered => {
                if let Some(entry) = self.network_info.topology.entry(initiator_id).or_default().specific_info {
                    if let SpecificInfo::ServerInfo(server_info) = entry {
                        self.register_client(server_info, initiator_id);
                    }
                }
            }
            Response::MessageFrom(client_id, message) => {
                self.storage.message_chat
                    .entry(client_id)
                    .or_insert_with(Vec::new)
                    .push((Speaker::HimOrHer, message));
            }
            Response::ListClients(list_users) => {
                self.communication.registered_communication_servers.insert(initiator_id, list_users);
            }
            Response::ListFiles(_) | Response::File(_) | Response::Media(_) => {
                // Placeholder for file/media handling
            }
            Response::Err(error) => {
                eprintln!("Error received: {:?}", error);
            }
            _ => {
                // Handle other cases if necessary
            }
        }
    }

    fn register_client(&mut self, server_info: &mut ServerInformation, initiator_id: NodeId) {
        if let ServerType::Communication = server_info.server_type {
            self.communication.registered_communication_servers.insert(initiator_id, vec![self.metadata.node_id]);
        } else if let ServerType::Text | ServerType::Media = server_info.server_type {
            self.communication.registered_content_servers.insert(initiator_id);
        }
    }
    fn reassemble_fragments_in_buffer<T: Serialize + for<'de> Deserialize<'de>>(&mut self) -> T {
        let mut keys: Vec<(SessionId, FragmentIndex)> = self
            .storage
            .fragment_assembling_buffer
            .keys()
            .cloned() // Get owned copies of keys
            .collect();

        // Sort the keys by fragment index (key.1)
        keys.sort_by_key(|key| key.1);

        // Initialize the serialized message
        let mut serialized_entire_msg = String::new();

        // Iterate through sorted keys to reassemble the message
        for key in keys {
            if let Some(associated_packet) = self.storage.fragment_assembling_buffer.get(&key) {
                match &associated_packet.pack_type {
                    PacketType::MsgFragment(fragment) => {
                        // Safely push data into the serialized message
                        if let Ok(data_str) = std::str::from_utf8(&fragment.data) {
                            serialized_entire_msg.push_str(data_str);
                        } else {
                            panic!("Invalid UTF-8 sequence in fragment for key: {:?}", key);
                        }
                    }
                    _ => {
                        panic!("Unexpected packet type for key: {:?}", key);
                    }
                }
            } else {
                panic!("Missing packet for key: {:?}", key);
            }
        }
        // Deserialize the fully assembled string
        match serde_json::from_str(&serialized_entire_msg) {
            Ok(deserialized_msg) => deserialized_msg,
            Err(e) => panic!("Failed to deserialize message: {}", e),
        }
    }
}
