use std::sync::Arc;

use anyhow::bail;
use async_trait::async_trait;
use futures::StreamExt;
use parking_lot::RwLock;
use protocol::{client::Client, server, ClientPacket, Packet, ServerPacket};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tracing::info;
use utils::default;

use crate::{
    process::{question::QAndA, reader::Reader, writer::Writer},
    Comm, Executor,
};

mod question;
mod reader;
mod writer;

pub struct WebSocketComm {
    reader: Reader,
    writer: Writer,
}

impl WebSocketComm {
    pub fn new(socket: WebSocketStream<TcpStream>) -> Self {
        let (writer, reader) = socket.split();
        Self {
            reader: reader.into(),
            writer: writer.into(),
        }
    }
}

#[async_trait]
impl Comm for WebSocketComm {
    async fn send(&mut self, packet: ServerPacket) -> anyhow::Result<()> {
        self.writer.write(packet).await
    }

    async fn recv(&mut self) -> anyhow::Result<ClientPacket> {
        self.reader.read().await
    }
}

#[derive(Default)]
struct Data {
    instruction: RwLock<Option<String>>,
    questions: RwLock<Vec<ClientPacket>>,
}

impl Data {
    fn instruction_set(&self) -> bool {
        self.instruction.read().is_some()
    }
}

pub struct Process<C> {
    executor: Executor,
    q_and_a: Option<QAndA>,
    comm: C,
    data: Arc<Data>,
}

impl<C: Comm> Process<C> {
    pub fn new(executor: Executor, comm: C) -> Self {
        Self {
            executor,
            comm,
            data: default(),
            q_and_a: None,
        }
    }
}

impl<C: Comm> Process<C> {
    async fn process_packet(&mut self, packet: Packet<Client>) -> anyhow::Result<()> {
        match packet.data {
            Client::Instruction { instruction } => {
                info!("Instruction: {}", instruction);

                let mut q_and_a = QAndA::new(self.executor.clone(), instruction);
                let question = q_and_a.gen_question().await?;

                info!("Question: {}", question);

                self.comm
                    .send(Packet::server(server::Question { question }))
                    .await?;
                self.q_and_a = Some(q_and_a);
            }
            Client::Answer { answer } => {
                let Some(q_and_a) = self.q_and_a.as_mut() else {
                    bail!("No question to answer");
                };

                info!("Answer: {}", answer);

                q_and_a.answer(answer);
                let question = q_and_a.gen_question().await?; // TODO: other packets should be able to be processed
                                                              // while this is running

                info!("Question: {}", question);

                self.comm
                    .send(Packet::server(server::Question { question }))
                    .await?;
            }
            Client::Execute => {
                let Some(q_and_a) = self.q_and_a.as_mut() else {
                    bail!("No questions to execute on");
                };

                let res = q_and_a.plan().await?;

                self.comm
                    .send(Packet::server(server::Question { question: res }))
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            let packet = self.comm.recv().await?;
            self.process_packet(packet).await?;
        }
    }
}
