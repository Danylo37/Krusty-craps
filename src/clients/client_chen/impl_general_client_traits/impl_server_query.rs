use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::{ClientChen, Sending, ServerQuery};
use crate::clients::client_chen::general_client_traits::*;

impl ServerQuery for ClientChen{
    fn register_to_server(&mut self, server_id: NodeId) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::RegisterClient(self.metadata.node_id))  //we could avoid the self.metadata.node_id getting the initiator_id from packet
        }                                                                    //so we could omit the NodeId in the field in Query::RegisterClient(NodeId)
    }

    fn unregister_from_server(&mut self, server_id: ServerId) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::UnregisterClient(self.metadata.node_id))
        }
    }

    fn ask_server_type(&mut self, server_id: ServerId) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskType);
        }
    }

    fn ask_list_clients(&mut self, server_id: ServerId) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskType);
        }
        self.send_query(server_id, Query::AskListClients);
    }

    fn send_message_to_client(&mut self, server_id: ServerId, client_id: ClientId, message: Message) {
        if self.get_discovered_servers_from_topology().contains(&server_id){
            self.send_query(server_id, Query::SendMessageTo(client_id, message));
        }
    }

    fn ask_list_files(&mut self, server_id: ServerId) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskListFiles);
        }
    }

    fn ask_file(&mut self, server_id: ServerId, file_ref: u8) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskFile(file_ref));
        }
    }

    fn ask_media(&mut self, server_id: ServerId, media_ref: String) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskMedia(media_ref));
        }
    }
}