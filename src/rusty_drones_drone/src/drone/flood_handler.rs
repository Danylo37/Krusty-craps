use crate::drone::RustyDrone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{FloodRequest, FloodResponse, NodeType, Packet};

impl RustyDrone {
    pub(super) fn respond_flood_request(&mut self, session_id: u64, flood: &FloodRequest) {
        if self.already_received_flood(flood) {
            self.respond_old(session_id, flood);
        } else {
            self.respond_new(session_id, flood);
        }
    }

    fn respond_old(&self, session_id: u64, request: &FloodRequest) {
        let mut new_path = request.path_trace.clone();
        new_path.push((self.id, NodeType::Drone));

        let mut hops = new_path
            .iter()
            .map(|(node_id, _)| *node_id)
            .rev()
            .collect::<Vec<_>>();

        if hops.last() != Some(&request.initiator_id) {
            hops.push(request.initiator_id);
        }

        self.send_to_next(Packet::new_flood_response(
            SourceRoutingHeader { hop_index: 1, hops },
            session_id,
            FloodResponse {
                flood_id: request.flood_id,
                path_trace: new_path,
            },
        ));
    }

    fn respond_new(&self, session_id: u64, flood: &FloodRequest) {
        let prev_hop = flood
            .path_trace
            .last()
            .map(|x| x.0)
            .unwrap_or(flood.initiator_id);

        let mut new_flood = flood.clone();
        new_flood.path_trace.push((self.id, NodeType::Drone));

        self.flood_exept(
            prev_hop,
            &Packet::new_flood_request(
                SourceRoutingHeader {
                    hop_index: 0,
                    hops: vec![],
                },
                session_id,
                new_flood,
            ),
        );
    }
}
