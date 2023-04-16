use anyhow::bail;
use derive_build::Build;
use futures::{stream::SplitStream, StreamExt};
use protocol::Packet;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

#[derive(Build)]
pub struct Reader {
    #[required]
    inner: SplitStream<WebSocketStream<TcpStream>>,
}

impl From<SplitStream<WebSocketStream<TcpStream>>> for Reader {
    fn from(inner: SplitStream<WebSocketStream<TcpStream>>) -> Self {
        Self { inner }
    }
}

impl Reader {
    pub fn inner(&self) -> &SplitStream<WebSocketStream<TcpStream>> {
        &self.inner
    }

    pub async fn read(&mut self) -> anyhow::Result<Packet<protocol::Client>> {
        let msg = self.inner.next().await.unwrap()?;

        let Message::Text(msg) = msg else {
            bail!("Expected text message, got: {:?}", msg)
        };

        let res = serde_json::from_str(&msg)?;

        Ok(res)
    }
}
