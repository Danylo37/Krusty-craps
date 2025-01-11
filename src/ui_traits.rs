use tokio::sync::mpsc;

pub trait Monitoring{
    async fn run_with_monitoring(&mut self, sender_to_gui: mpsc::Sender<Vec<u8>>);
}
