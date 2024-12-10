#![cfg(test)]
use crate::drone::RustyDrone;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

pub struct DroneOptions {
    pub controller_send: Sender<DroneEvent>,
    pub controller_recv: Receiver<DroneCommand>,
    pub packet_recv: Receiver<Packet>,
    pub packet_send: HashMap<NodeId, Sender<Packet>>,
    pub packet_drone_in: Sender<Packet>,
    pub command_send: Sender<DroneCommand>,
    pub event_recv: Receiver<DroneEvent>,
}

impl DroneOptions {
    pub fn new() -> Self {
        let (controller_send, event_recv) = unbounded::<DroneEvent>();
        let (command_send, controller_recv) = unbounded::<DroneCommand>();
        let (packet_drone_in, packet_recv) = unbounded::<Packet>();
        let packet_send = HashMap::<NodeId, Sender<Packet>>::new();
        Self {
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            packet_drone_in,
            command_send,
            event_recv,
        }
    }

    pub(crate) fn new_with_sc(
        controller_send: Sender<DroneEvent>,
        event_recv: Receiver<DroneEvent>,
    ) -> Self {
        let (command_send, controller_recv) = unbounded::<DroneCommand>();
        let (packet_drone_in, packet_recv) = unbounded::<Packet>();
        let packet_send = HashMap::<NodeId, Sender<Packet>>::new();
        Self {
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            packet_drone_in,
            command_send,
            event_recv,
        }
    }

    pub fn create_drone(&self, id: NodeId, pdr: f32) -> RustyDrone {
        RustyDrone::new(
            id,
            self.controller_send.clone(),
            self.controller_recv.clone(),
            self.packet_recv.clone(),
            self.packet_send.clone(),
            pdr,
        )
    }

    pub fn assert_expect_drone_event(&self, expected_event: DroneEvent) {
        assert_eq!(expected_event, self.event_recv.try_recv().unwrap());
    }

    pub fn assert_expect_drone_event_fail(&self) {
        assert!(self.event_recv.try_recv().is_err());
    }
}
