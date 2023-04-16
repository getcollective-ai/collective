use derive_build::Build;
use futures::{stream::SplitSink, SinkExt};
use protocol::{ClientPacket, Packet, ServerPacket};
use tokio::net::TcpStream;
// use futures::stream::{SplitSink, SplitStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::process::reader::Reader;

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
    pub fn inner(&self) -> &SplitSink<WebSocketStream<TcpStream>, Message> {
        &self.inner
    }

    pub async fn write(
        &mut self,
        element: ServerPacket,
    ) -> anyhow::Result<()> {
        let s = serde_json::to_string(&element)?;

        let message = Message::Text(s);

        self.inner.send(message).await?;

        Ok(())
    }
}
