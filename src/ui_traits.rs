use tokio::sync::mpsc;
use std::future::Future;

pub trait Monitoring {
    fn run_with_monitoring(
        &mut self,
        sender_to_gui: mpsc::Sender<Vec<u8>>,
    ) -> impl Future<Output = ()> + Send;
}
