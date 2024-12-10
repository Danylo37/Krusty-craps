use crate::drone::{utils, RustyDrone};
use wg_2024::packet::NackType::{DestinationIsDrone, Dropped, ErrorInRouting, UnexpectedRecipient};
use wg_2024::packet::{Nack, NackType, Packet, PacketType};

impl RustyDrone {
    pub(super) fn respond_normal(&self, packet: &Packet, crashing: bool) {
        let droppable = matches!(packet.pack_type, PacketType::MsgFragment(_));
        let routing = &packet.routing_header;

        // If unexpected packets
        if routing.current_hop() != Some(self.id) {
            self.nack_packet(packet, UnexpectedRecipient(self.id), droppable, true);
            return;
        }

        if crashing && droppable {
            self.nack_packet(packet, ErrorInRouting(self.id), droppable, false);
            return;
        }

        // In base of next existing or not
        match routing.next_hop() {
            None => {
                if droppable {
                    self.nack_packet(packet, DestinationIsDrone, droppable, false);
                }
                return;
            }
            Some(next) => {
                if !self.packet_send.contains_key(&next) {
                    self.nack_packet(packet, ErrorInRouting(next), droppable, true);
                    return;
                }
            }
        }

        if droppable && self.should_drop() {
            self.notify_dropped(packet.clone());
            self.nack_packet(packet, Dropped, droppable, false);
            return;
        }

        self.forward_packet(packet);
    }

    fn forward_packet(&self, packet: &Packet) {
        let mut routing_header = packet.routing_header.clone();
        routing_header.increase_hop_index();

        self.send_to_next(Packet {
            routing_header,
            session_id: packet.session_id,
            pack_type: packet.pack_type.clone(),
        });
    }

    fn nack_packet(
        &self,
        packet: &Packet,
        nack_type: NackType,
        droppable: bool,
        shortcuttable: bool,
    ) {
        if !droppable {
            if shortcuttable {
                let mut routing_header = packet.routing_header.clone();
                routing_header.increase_hop_index();

                self.use_shortcut(Packet {
                    routing_header,
                    session_id: packet.session_id,
                    pack_type: packet.pack_type.clone(),
                });
            }
            return;
        }

        self.send_to_next(Packet::new_nack(
            self.get_routing_back(&packet.routing_header),
            packet.session_id,
            Nack {
                nack_type,
                fragment_index: utils::get_fragment_index(&packet.pack_type),
            },
        ));
    }
}
