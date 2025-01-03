use crate::clients::client_chen::{ClientChen, ClientEvents};
use crate::general_use::Message;

impl ClientEvents for ClientChen{
    fn message_sent_to_client(&mut self, message: Message) {
        todo!()
    }

    fn message_received_from_client(&mut self, message: Message) {
        todo!()
    }

    fn message_received_from_server(&mut self, message: Message) {
        todo!()
    }
}