use crate::clients::client_chen::{ClientChen, FragmentsHandler, PacketCreator, PacketsReceiver, Sending, SpecificInfo};
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
        let sessions: HashMap<SessionId, Vec<FragmentIndex>> = self
            .storage
            .fragment_assembling_buffer
            .keys()
            .fold(HashMap::new(), |mut acc, &(session_id, fragment_id)| {
                acc.entry(session_id).or_insert_with(Vec::new).push(fragment_id);
                acc
            });

        for (session_id, fragments) in sessions {
            if let Some(total_n_fragments) = self.get_total_n_fragments(session_id) {
                if fragments.len() as u64 == total_n_fragments {
                    // Get the first fragment for the session
                    let first_key = self.storage.fragment_assembling_buffer
                        .keys()
                        .find(|&&(session, _)| session == session_id)
                        .unwrap();

                    // Use the corresponding packet
                    let first_packet = self.storage.fragment_assembling_buffer.get(first_key).unwrap();

                    let initiator_id = Self::get_packet_destination(first_packet.clone());
                    let message: T = self.reassemble_fragments_in_buffer();

                    match message {
                        Response::ServerType(ServerType) => {
                            let entry = self.network_info.topology.entry(initiator_id).or_default();
                            if let SpecificInfo::ServerInfo(server_info) = &mut entry.specific_info{
                                server_info.server_type = ServerType;
                            }
                        }
                        Response::ClientRegistered => {
                            self.communication.server_registered.insert(initiator_id, vec![self.metadata.node_id]);
                        }
                        Response::MessageFrom(client_id, message) => {
                            self.storage.message_chat
                                .entry(client_id)
                                .or_insert_with(Vec::new)
                                .push((Speaker::HimOrHer, message));
                        }
                        Response::ListClients(list_users) => {
                            self.communication.server_registered.insert(initiator_id, list_users);
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
            }
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
