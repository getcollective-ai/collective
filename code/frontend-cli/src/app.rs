use std::{pin::pin, time::Duration};

use anyhow::Context;
use crossterm::event::{poll, KeyCode};
use futures::{future, future::Either};
use protocol::{client, server::Server};
use tracing::debug;
use tui::{backend::Backend, Terminal};

use crate::{ui::Ui, Event, CANCEL_TOKEN};

pub struct App {
    tx: tokio::sync::mpsc::UnboundedSender<protocol::ClientPacket>,
    rx: tokio::sync::mpsc::UnboundedReceiver<protocol::ServerPacket>,
    instruction: Option<String>,
}

impl App {
    pub fn new(
        tx: tokio::sync::mpsc::UnboundedSender<protocol::ClientPacket>,
        rx: tokio::sync::mpsc::UnboundedReceiver<protocol::ServerPacket>,
    ) -> Self {
        Self {
            tx,
            rx,
            instruction: None,
        }
    }

    // TODO: remove clippy::too_many_lines
    #[allow(clippy::too_many_lines)]
    pub async fn run<B: Backend + Send>(
        mut self,
        terminal: &mut Terminal<B>,
    ) -> anyhow::Result<()> {
        let mut ui = Ui::new();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::spawn({
            let tx = tx.clone();
            move || {
                loop {
                    if CANCEL_TOKEN.is_cancelled() {
                        return;
                    }
                    if poll(Duration::from_millis(10)).unwrap() {
                        // TODO: handle `unwrap` error in `poll`
                        // It's guaranteed that `read` won't block, because `poll` returned
                        // `Ok(true)`.

                        let Ok(event) = crossterm::event::read() else {
                            debug!("Cannot read event from terminal");
                            continue;
                        };

                        if let Err(e) = tx.send(Event::Terminal(event)) {
                            debug!("Cannot send event to terminal -> shutting down: {e:?}");
                            CANCEL_TOKEN.cancel();
                            return;
                        }
                    }
                }
            }
        });

        let mut waiting_for_question = false;

        tokio::spawn(async move {
            let mut reader = self.rx;
            loop {
                let cancel = pin!(CANCEL_TOKEN.cancelled());
                let packet = pin!(reader.recv());
                let packet = match future::select(cancel, packet).await {
                    Either::Left((..)) => {
                        return;
                    }
                    Either::Right((packet, ..)) => {
                        let Some(packet) = packet else {
                            debug!("Cannot grab packets -> shutting down");
                            CANCEL_TOKEN.cancel();
                            return;
                        };
                        packet
                    }
                };

                if let Err(e) = tx.send(Event::Packet(packet)) {
                    debug!("Cannot send packet to terminal -> shutting down: {e:?}");
                    CANCEL_TOKEN.cancel();
                    return;
                }
            }
        });

        loop {
            terminal.draw(|frame| ui.run(frame))?;

            let event = rx.recv().await.context("Failed to receive event")?;

            use crossterm::event::Event::Key as CrossKey;

            match event {
                Event::Terminal(CrossKey(key)) if key.code == KeyCode::Esc => {
                    return Ok(());
                }
                Event::Terminal(CrossKey(key)) if !waiting_for_question => match key.code {
                    KeyCode::Backspace => {
                        ui.current_line().pop();
                    }
                    KeyCode::Tab => {
                        ui.reset();
                        self.tx.send(protocol::Packet::client(client::Execute))?;
                    }
                    KeyCode::Enter => {
                        if ui.current_line().trim().is_empty() {
                            continue;
                        }
                        let packet = match self.instruction {
                            None => {
                                self.instruction = Some(ui.current_line().clone());
                                protocol::Packet::client(client::Instruction {
                                    instruction: ui.current_line().clone(),
                                })
                            }
                            Some(..) => protocol::Packet::client(client::Answer {
                                answer: ui.current_line().clone(),
                            }),
                        };

                        waiting_for_question = true;

                        ui.new_line();
                        self.tx.send(packet)?;
                    }
                    KeyCode::Char(c) => {
                        ui.current_line().push(c);
                    }
                    _ => {}
                },
                Event::Packet(packet) => match packet.data {
                    Server::Question { question } => {
                        ui.new_line();
                        ui.current_line().push_str(&format!("> {question}"));
                        ui.new_line();

                        waiting_for_question = false;
                    }
                },
                Event::Terminal(_) => {}
            }
        }
    }
}
