use futures::{SinkExt, StreamExt};
use protocol::{client::Client, server::Server, Packet};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info};

use crate::{Args, CANCEL_TOKEN};

pub async fn setup_comms(
    args: &Args,
) -> anyhow::Result<(
    mpsc::UnboundedSender<Packet<Client>>,
    mpsc::UnboundedReceiver<Packet<Server>>,
)> {
    let Args { remote, ip, port } = args;
    let res = match remote {
        false => {
            info!("Launching local executor...");
            executor::launch()
        }

        true => {
            let address = format!("ws://{ip}:{port}");

            info!("Connecting to {address} via websocket...");

            let (websocket, _) = connect_async(&address).await?;

            let (write, read) = websocket.split();

            let (tx1, mut rx1) = mpsc::unbounded_channel();
            let (tx2, rx2) = mpsc::unbounded_channel();

            tokio::spawn(async move {
                let mut write = write;
                while let Some(packet) = rx1.recv().await {
                    let packet = match serde_json::to_string(&packet) {
                        Ok(packet) => packet,
                        Err(err) => {
                            debug!("Failed to serialize packet: {}", err);
                            continue;
                        }
                    };
                    if let Err(e) = write.send(Message::Text(packet)).await {
                        debug!("Failed to send packet: {}. Shutting down", e);
                        CANCEL_TOKEN.cancel();
                    }
                }
            });

            tokio::spawn(async move {
                let mut read = read;
                while let Some(packet) = read.next().await {
                    let packet = match packet {
                        Ok(packet) => packet,
                        Err(e) => {
                            debug!("Failed to receive packet: {}. Shutting down", e);
                            CANCEL_TOKEN.cancel();
                            break;
                        }
                    };

                    let Ok(packet) = serde_json::from_str(&packet.to_string()) else {
                        debug!("Failed to deserialize packet");
                        continue;
                    };

                    if let Err(e) = tx2.send(packet) {
                        debug!("Failed to send packet: {}. Shutting down", e);
                        CANCEL_TOKEN.cancel();
                    }
                }
            });

            (tx1, rx2)
        }
    };

    Ok(res)
}
