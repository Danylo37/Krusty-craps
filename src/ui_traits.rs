use tokio::sync::mpsc;
use std::future::Future;
use tokio::sync::Mutex;
use std::sync::Arc;
use crossbeam_channel::Receiver;
use crate::clients::client_chen::ClientChen;

pub async fn crossbeam_to_tokio_bridge<T: Send + 'static>(
    mut crossbeam_rx: Receiver<T>,
    tokio_tx: mpsc::Sender<T>,
) {
    loop {
        match crossbeam_rx.recv() {
            Ok(msg) => {
                if tokio_tx.send(msg).await.is_err() {
                    eprintln!("Tokio receiver dropped");
                    break;
                }
            }
            Err(RecvError) => {
                eprintln!("Crossbeam channel disconnected");
                break;
            }
        }
    }
}
pub trait Monitoring {
    fn run_with_monitoring(
        &mut self, // Use `&mut self` to allow mutation
        sender_to_gui: mpsc::Sender<Vec<u8>>,
    ) -> impl Future<Output = ()> + Send;
}