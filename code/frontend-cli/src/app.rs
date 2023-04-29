use std::{pin::pin, time::Duration};

use anyhow::Context;
use crossterm::event::{poll, KeyCode};
use futures::{future, future::Either};
use protocol::{client, server::Server};
use tracing::{debug, error};
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

        // channel that handles Events
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::spawn({
            let tx = tx.clone();
            move || loop {
                if CANCEL_TOKEN.is_cancelled() {
                    return;
                }

                let poll_result = match poll(Duration::from_millis(10)) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                };

                if poll_result {
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
        });

        let mut waiting_for_question = false;

        // receive a Packet<Server> and emit an Event::Packet(packet<server>)
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

                // grab the packet, wrap it into an Event.
                // send to the loop below
                if let Err(e) = tx.send(Event::Packet(packet)) {
                    debug!("Cannot send packet to terminal -> shutting down: {e:?}");
                    CANCEL_TOKEN.cancel();
                    return;
                }
            }
        });

        // handle all events, including events received from above
        // and send a Packet<Client> to the executor `fn process_packet`?
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
                    KeyCode::Enter => {
                        if ui.current_line().trim().is_empty() {
                            continue;
                        }
                        let packet = match self.instruction {
                            // instruction will only be None
                            // on the very first prompt of the user on the terminal
                            // all the subsequent prompts will be Some
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
                // answers sent from GPT to the frontend
                // are handled here
                Event::Packet(packet) => match packet.data {
                    Server::Question {
                        question,
                        is_first_word,
                        is_last_word,
                    } => {
                        if is_first_word || is_last_word {
                            ui.new_line();
                        }
                        // is first word, meaning this is the
                        // beggining of a new question
                        if is_first_word {
                            ui.current_line().push_str(&format!("> {question}"));
                        }
                        // is not first word, meaning the next words
                        // are the contiunation of the previous question
                        if !is_first_word {
                            ui.current_line().push_str(question.as_str());
                        }
                        if is_last_word {
                            waiting_for_question = false;
                        }
                    }
                },
                Event::Terminal(_) => {}
            }
        }
    }
}
