use serde::de::DeserializeOwned;
use crate::clients::client_chen::{ClientChen, FragmentsHandler, PacketCreator, PacketsReceiver, Sending, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
impl FragmentsHandler for ClientChen{
    fn handle_fragment(&mut self, msg_packet: Packet, fragment: Fragment) {
        self.decreasing_using_times_when_receiving_packet(&msg_packet);
        self.storage
            .input_packet_disk
            .insert((msg_packet.session_id, fragment.fragment_index), msg_packet.clone());
        self.storage
            .fragment_assembling_buffer
            .insert((msg_packet.session_id, fragment.fragment_index), msg_packet.clone());

        // Send an ACK instantly
        let ack_packet = self.create_ack_packet_from_receiving_packet(msg_packet.clone());
        self.send(ack_packet);
    }

    fn get_total_n_fragments(&mut self, session_id: SessionId) -> Option<u64> {
        self.storage
            .fragment_assembling_buffer
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

    fn handle_fragments_in_buffer_with_checking_status<T: Serialize + Deserialize>(&mut self) {
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
                    let session_fragments = self
                        .storage
                        .fragment_assembling_buffer
                        .iter()
                        .filter(|(&(session, _), _)| session == session_id)
                        .collect::<Vec<_>>();

                    if let Some((_, first_packet)) = session_fragments.first() {
                        let initiator_id = self.get_packet_destination(first_packet);

                        if let Ok(message) = self.reassemble_fragments_in_buffer::<T>() {
                            self.process_message(initiator_id, message);
                        } else {
                            eprintln!("Failed to reassemble fragments for session: {:?}", session_id);
                        }
                    }
                }
            }
        }
    }

    fn process_message<T: Serialize>(&mut self, initiator_id: NodeId, message: T) {
        match message {
            Response::ServerType(server_type) => self.update_topology_entry(initiator_id, server_type),
            Response::ClientRegistered => self.register_client(initiator_id),
            Response::MessageFrom(client_id, message) => {
                self.storage
                    .message_chat
                    .entry(client_id)
                    .or_insert_with(Vec::new)
                    .push((Speaker::HimOrHer, message));
            }
            Response::ListClients(list_users) => {
                self.communication
                    .registered_communication_servers
                    .insert(initiator_id, list_users);
            }
            Response::ListFiles(_) | Response::File(_) | Response::Media(_) => {
                // Placeholder for file/media handling
            }
            Response::Err(error) => {
                eprintln!("Error received: {:?}", error);
            }
            _ => {
                eprintln!("Unhandled message type: {:?}", message);
            }
        }
    }

    fn register_client(&mut self, initiator_id: NodeId) {
        if let Some(entry) = self.network_info.topology.entry(initiator_id).or_default().specific_info {
            if let SpecificInfo::ServerInfo(ref mut server_info) = entry {
                match server_info.server_type {
                    ServerType::Communication => {
                        self.communication
                            .registered_communication_servers
                            .insert(initiator_id, vec![self.metadata.node_id]);
                    }
                    ServerType::Text | ServerType::Media => {
                        self.communication.registered_content_servers.insert(initiator_id);
                    }
                    _ => {}
                }
            }
        }
    }

    fn reassemble_fragments_in_buffer<T: Serialize + for<'a> Deserialize<'a>>(&mut self) -> Result<T, String> {
        let mut keys: Vec<(SessionId, FragmentIndex)> = self
            .storage
            .fragment_assembling_buffer
            .keys()
            .cloned()
            .collect();

        // Sort the keys by fragment index
        keys.sort_by_key(|key| key.1);

        let mut serialized_entire_msg = String::new();

        for key in keys {
            if let Some(associated_packet) = self.storage.fragment_assembling_buffer.get(&key) {
                match &associated_packet.pack_type {
                    PacketType::MsgFragment(fragment) => {
                        if let Ok(data_str) = std::str::from_utf8(&fragment.data) {
                            serialized_entire_msg.push_str(data_str);
                        } else {
                            return Err(format!("Invalid UTF-8 sequence in fragment for key: {:?}", key));
                        }
                    }
                    _ => {
                        return Err(format!("Unexpected packet type for key: {:?}", key));
                    }
                }
            } else {
                return Err(format!("Missing packet for key: {:?}", key));
            }
        }

        serde_json::from_str(&serialized_entire_msg).map_err(|e| format!("Failed to deserialize message: {}", e))
    }
}
