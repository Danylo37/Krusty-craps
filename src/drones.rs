use crossbeam_channel::{select, Receiver, Sender};
use std::collections::HashMap;
use wg_2024::controller::Command;
use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::network::NodeId;
use wg_2024::packet::{Ack, Fragment, Nack, Packet, PacketType};

struct KrustyCrapDrone {
    id: NodeId,
    sim_contr_send: Sender<Command>,
    sim_contr_recv: Receiver<Command>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
}

impl Drone for KrustyCrapDrone {
    fn new(options: DroneOptions) -> Self {
        Self {
            id: options.id,
            sim_contr_send: options.sim_contr_send,
            sim_contr_recv: options.sim_contr_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr,
            packet_send: options.packet_send,
        }
    }

    fn run(&mut self) {
        loop {
            select! {
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match packet.pack_type {
                            PacketType::Nack(nack) => self.handle_nack(nack),
                            PacketType::Ack(ack) => self.handle_ack(ack),
                            PacketType::MsgFragment(fragment) => self.handle_fragment(fragment),
                            PacketType::FloodRequest(_) => unimplemented!(),
                            PacketType::FloodResponse(_) => unimplemented!(),
                        }
                    }
                },
                recv(self.sim_contr_recv) -> command_res => {
                    if let Ok(_command) = command_res {
                            todo!();
                    }
                }
            }
        }
    }
}

impl KrustyCrapDrone {

    fn handle_nack(&mut self, _nack: Nack) {
        todo!()
    }

    fn handle_ack(&mut self, _ack: Ack) {
        todo!()
    }

    fn handle_fragment(&mut self, _fragment: Fragment) {
        todo!()
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
}
