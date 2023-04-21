use derive_build::Build;
use futures::{stream::SplitSink, SinkExt};
use protocol::ServerPacket;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

#[derive(Build)]
pub struct Writer {
    #[required]
    inner: SplitSink<WebSocketStream<TcpStream>, Message>,
}

impl From<SplitSink<WebSocketStream<TcpStream>, Message>> for Writer {
    fn from(inner: SplitSink<WebSocketStream<TcpStream>, Message>) -> Self {
        Self { inner }
    }
}

impl Writer {
    pub async fn write(&mut self, element: ServerPacket) -> anyhow::Result<()> {
        let s = serde_json::to_string(&element)?;

        let message = Message::Text(s);

        self.inner.send(message).await?;

        Ok(())
    }
}
