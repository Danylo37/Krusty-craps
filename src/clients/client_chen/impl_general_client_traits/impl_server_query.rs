use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::{ClientChen, Sending, ServerQuery};

impl ServerQuery for ClientChen{
    fn register_to_server(&mut self, server_id: NodeId) {
        if self.get_discovered_servers().contains(&server_id) {
            //send a query of registration to the server
            self.send_query(server_id, Query::RegisterClient(self.metadata.node_id))  //we could avoid the self.metadata.node_id getting the initiator_id from packet
        }                                                                    //so we could omit the NodeId in the field in Query::RegisterClient(NodeId)
    }

    fn unregister_from_server(&mut self, server_id: ServerId) {
        todo!()
    }

    fn ask_server_type(&mut self, server_id: ServerId) {
        todo!()
    }

    fn ask_list_clients(&mut self, server_id: ServerId) {
        todo!()
    }

    fn send_message_to_client(&mut self, server_id: ServerId, message: Message) {
        todo!()
    }

    fn ask_list_files(&mut self, server_id: ServerId) {
        todo!()
    }

    fn ask_file(&mut self, server_id: ServerId, file_ref: u8) {
        todo!()
    }

    fn ask_media(&mut self, server_id: ServerId, media_ref: String) {
        todo!()
    }

    fn get_discovered_servers(&mut self) -> HashSet<ServerId> {
        let discovered_servers: HashSet<ServerId> = self
            .communication
            .edge_nodes
            .keys()
            .filter(|node_id| matches!(self.communication.edge_nodes.get(node_id), Some(NodeType::Server)))
            .cloned() // Clone just the filtered keys
            .collect();
        discovered_servers
    }
}