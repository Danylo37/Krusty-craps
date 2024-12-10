use crate::drone::RustyDrone;
use wg_2024::controller::DroneCommand;

impl RustyDrone {
    pub(super) fn handle_commands(&mut self, command: &DroneCommand) -> bool {
        match command {
            DroneCommand::Crash => return true,
            DroneCommand::SetPacketDropRate(pdr) => self.pdr = *pdr,
            DroneCommand::RemoveSender(node_id) => {
                self.packet_send.remove(node_id);
            }
            DroneCommand::AddSender(node_id, sender) => {
                self.packet_send.insert(*node_id, sender.clone());
            }
        }
        false
    }
}
