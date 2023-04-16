use std::sync::Arc;

use anyhow::bail;
use futures::StreamExt;
use parking_lot::RwLock;
use protocol::{Client, Packet};
use tracing::info;
use utils::default;

use crate::{
    process::{question::QAndA, reader::Reader, writer::Writer},
    Executor,
};

mod question;
mod reader;
mod writer;

#[derive(Default)]
struct Data {
    instruction: RwLock<Option<String>>,
    questions: RwLock<Vec<Packet<Client>>>,
}

impl Data {
    fn instruction_set(&self) -> bool {
        self.instruction.read().is_some()
    }
}

pub struct Process {
    executor: Executor,
    q_and_a: Option<QAndA>,
    read: Reader,
    write: Writer,
    data: Arc<Data>,
}

impl Process {
    pub fn new(executor: Executor, read: impl Into<Reader>, write: impl Into<Writer>) -> Self {
        Self {
            executor,
            read: read.into(),
            write: write.into(),
            data: default(),
            q_and_a: None,
        }
    }
}

impl Process {
    async fn process_packet(&mut self, packet: Packet<Client>) -> anyhow::Result<()> {
        match packet.data {
            Client::Instruction { instruction } => {
                info!("Instruction: {}", instruction);

                let mut q_and_a = QAndA::new(self.executor.clone(), instruction);
                let question = q_and_a.gen_question().await?;

                info!("Question: {}", question);

                self.write
                    .write(Packet::server(protocol::Question { question }))
                    .await?;

                self.q_and_a = Some(q_and_a);
            }
            Client::Answer { answer } => {
                let Some(q_and_a) = self.q_and_a.as_mut() else {
                    bail!("No question to answer");
                };

                q_and_a.answer(answer);
                let question = q_and_a.gen_question().await?; // TODO: other packets should be able to be processed
                                                              // while this is running

                info!("Question: {}", question);

                self.write
                    .write(Packet::server(protocol::Question { question }))
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            let packet = self.read.read().await?;
            self.process_packet(packet).await?;
        }
    }
}
