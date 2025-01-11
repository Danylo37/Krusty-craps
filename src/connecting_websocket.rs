use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    let ws_addr = "127.0.0.1:8000";
    let (tx, _) = broadcast::channel::<Vec<u8>>(1000); // Broadcast channel for binary messages

    let listener = TcpListener::bind(ws_addr).await.expect("Failed to bind WebSocket address");

    println!("WebSocket server running on {}", ws_addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.expect("Failed to accept WebSocket connection");
            println!("New WebSocket connection established");

            let (mut write, mut read) = ws_stream.split();

            // Task for broadcasting received binary messages
            tokio::spawn(async move {
                while let Ok(msg) = rx.recv().await {
                    if let Err(e) = write.send(Message::Binary(msg)).await {
                        eprintln!("Error sending binary message: {:?}", e);
                        break;
                    }
                }
            });

            // Receive binary messages from this client
            while let Some(Ok(Message::Binary(data))) = read.next().await {
                println!("Received binary message of length: {}", data.len()); // Print length of binary data
                let _ = tx.send(data); // Broadcast binary data to all clients
            }

            println!("WebSocket connection closed.");
        });
    }
}