use async_tungstenite::tungstenite::Message;
use futures::*;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        println!("Usage: ws-redirector [SERVER_ADDR] [CLIENT_ADDR...]");
        return;
    }

    let server_addr = args[1].as_str();
    let mut clients = Vec::with_capacity(args.len() - 2);

    for client_addr in &args[2..] {
        let client_addr = client_addr.to_owned();
        let (sx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
        clients.push(sx);
        tokio::spawn(async move {
            loop {
                if let Ok((client, _)) = async_tungstenite::tokio::connect_async(&client_addr).await
                {
                    let (mut write, _read) = client.split();
                    println!("Connected to client: {}", client_addr);
                    while let Some(data) = rx.recv().await {
                        if let Err(e) = write.send(data).await {
                            eprintln!("Error writing to client: {}", e);
                            break;
                        }
                    }
                }
                println!("Disconnected from client: {}", client_addr);
                while rx.try_recv().is_ok() {}
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }

    let listener = TcpListener::bind(server_addr)
        .await
        .expect("can't bind address on server");

    while let Ok((client, _)) = listener.accept().await {
        if let Ok(addr) = client.peer_addr() {
            if let Ok(client) = async_tungstenite::tokio::accept_async(client).await {
                println!("New WebSocket connection: {}", addr);
                let (_write, mut read) = client.split();

                while let Some(Ok(data)) = read.next().await {
                    for client in &clients {
                        if let Err(e) = client.send(data.clone()) {
                            eprintln!("Error sending to client: {}", e);
                        }
                    }
                }
            }
        }
    }
}
