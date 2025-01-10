use crate::clients::client_chen::prelude::*;

pub trait Monitoring{
    fn run_with_monitoring(&mut self, sender_to_gui:Sender<String>);
}

