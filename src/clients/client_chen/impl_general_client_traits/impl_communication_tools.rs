use crate::clients::client_chen::{ClientChen};
use crate::clients::client_chen::general_client_traits::*;
use crate::clients::client_chen::prelude::*;
impl CommunicationTools for ClientChen{
    fn get_discovered_servers_from_topology(&mut self) -> HashSet<ServerId> {
        self.network_info.topology.iter()
            .filter_map(|(&node_id, node_info)| {
                if let NodeType::Server = node_info.node_type {
                    Some(node_id)
                } else {
                    None
                }
            })
            .collect()
    }
    fn get_edge_nodes_from_topology(&mut self) -> HashSet<NodeId> {
        self.network_info.topology.iter()
            .filter_map(|(&node_id, node_info)| {
                match node_info.node_type {
                    NodeType::Server | NodeType::Client => Some(node_id),
                    _ => None,
                }
            })
            .collect()
    }

    ///just looping without worrying about repetitions
    fn get_communicable_clients_from_registered_servers(&mut self) -> HashSet<ClientId> {
        let mut communicable_clients = HashSet::new();
        for registered_clients in self.communication.registered_communication_servers.values() {
            for client in registered_clients {
                communicable_clients.insert(*client);
            }
        }
        communicable_clients
    }

    fn get_communicable_nodes(&mut self) -> HashSet<NodeId>{
        let mut communicable_nodes = HashSet::new();
        let servers = self.get_discovered_servers_from_topology();
        let communicable_clients = self.get_communicable_clients_from_registered_servers();
        communicable_nodes.extend(servers);
        communicable_nodes.extend(communicable_clients);
        communicable_nodes
    }
}