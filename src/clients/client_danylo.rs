use crossbeam_channel::{Receiver, Sender};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

struct ClientDanylo {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    packet_send: Sender<Packet>,
    packet_recv: Receiver<Packet>,
}

impl ClientDanylo {
    fn new() {
        todo!()
    }

    fn run(&mut self) {
        todo!()
    }

    fn connect(&mut self) {
        todo!()
    }

    fn send_message(&mut self) {
        todo!()
    }

    fn receive_message(&mut self) {
        todo!()
    }

    fn discover_network(&mut self) {
        todo!()
    }

    fn serialize_message(&mut self) {
        todo!()
    }

    fn fragment_message(&mut self) {
        todo!()
    }

    fn reassemble_message(&mut self) {
        todo!()
    }

    fn handle_nack(&mut self) {
        todo!()
    }
}
