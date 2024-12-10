use crate::drone::RustyDrone;
use wg_2024::controller::DroneEvent::*;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

impl RustyDrone {
    #[inline(always)]
    pub fn send_to_next(&self, packet: Packet) {
        // Intentional unwrap
        let next_hop = packet.routing_header.current_hop().unwrap();
        let channel = self.packet_send.get(&next_hop).unwrap();

        let _ = channel.send(packet.clone());
        let _ = self.controller_send.send(PacketSent(packet));
    }

    #[inline(always)]
    pub(super) fn flood_exept(&self, previous_hop: NodeId, packet: &Packet) {
        for (node_id, channel) in self.packet_send.iter() {
            if *node_id != previous_hop {
                let _ = channel.send(packet.clone());
            }
        }
    }

    #[inline(always)]
    pub(super) fn use_shortcut(&self, packet: Packet) {
        let _ = self.controller_send.send(ControllerShortcut(packet));
    }

    #[inline(always)]
    pub(super) fn notify_dropped(&self, packet: Packet) {
        let _ = self.controller_send.send(PacketDropped(packet));
    }
}
