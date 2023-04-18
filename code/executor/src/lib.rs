#![feature(unsize)]
#![allow(unused)]

use std::{io::Write, sync::Arc};

use anyhow::{anyhow, ensure, Context, Result};
use async_trait::async_trait;
use clap::Parser;
use futures::{stream::BoxStream, SinkExt, StreamExt};
use protocol::{ClientPacket, Packet, ServerPacket};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use tokio_openai::ChatRequest;
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};
use tracing::{debug, error, info, instrument};
use utils::{default, Stream};

use crate::process::{Process, WebSocketComm};

mod command;
mod process;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long, default_value = "127.0.0.1")]
    pub ip: String,

    #[clap(short, long, default_value = "8080")]
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum Event {
    Connected,
}

#[async_trait]
pub trait Comm {
    async fn send(&mut self, packet: ServerPacket) -> anyhow::Result<()>;
    async fn recv(&mut self) -> anyhow::Result<ClientPacket>;
}

struct SimpleComm {
    tx: tokio::sync::mpsc::UnboundedSender<ServerPacket>,
    rx: tokio::sync::mpsc::UnboundedReceiver<ClientPacket>,
}

#[async_trait]
impl Comm for SimpleComm {
    async fn send(&mut self, packet: ServerPacket) -> anyhow::Result<()> {
        self.tx.send(packet)?;
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<ClientPacket> {
        self.rx.recv().await.context("Failed to receive packet")
    }
}

impl SimpleComm {
    #[instrument(skip(tx, rx))]
    pub fn new(
        tx: tokio::sync::mpsc::UnboundedSender<ServerPacket>,
        rx: tokio::sync::mpsc::UnboundedReceiver<ClientPacket>,
    ) -> Self {
        debug!("New SimpleComm");
        Self { tx, rx }
    }
}

/// Launch using [`SimpleComm`] and return (tx, rx) for sending and receiving packets.
pub fn launch() -> (
    UnboundedSender<ClientPacket>,
    UnboundedReceiver<ServerPacket>,
) {
    let executor = Executor::new().unwrap();

    let (tx1, rx1) = tokio::sync::mpsc::unbounded_channel();
    let (tx2, rx2) = tokio::sync::mpsc::unbounded_channel();

    let comm = SimpleComm::new(tx1, rx2);

    tokio::spawn(async move {
        handle_client(executor, comm).await;
    });

    (tx2, rx1)
}

fn launch_comm(comm: impl Comm + Send + 'static) {
    let executor = Executor::new().unwrap();
    tokio::spawn(async move {
        handle_client(executor, comm).await;
    });
}

pub fn launch_websocket(args: Args) -> UnboundedReceiver<Event> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        info!("Starting executor");

        let executor = Executor::new().unwrap();

        let Args { ip, port } = args;

        let addr = format!("{ip}:{port}");

        let listener = TcpListener::bind(&addr).await.unwrap();

        tx.send(Event::Connected).unwrap();

        info!("Listening on: {addr}");

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let ws_stream = accept_async(socket).await.unwrap();
            info!(
                "New WebSocket connection: {}",
                ws_stream.get_ref().peer_addr().unwrap() /* TODO: is this unwrap bad? What if it
                                                          * panics O_O */
            );

            let ws = WebSocketComm::new(ws_stream);

            let executor = executor.clone();
            tokio::spawn(async move {
                handle_client(executor, ws).await;
            });
        }
    });

    rx
}

async fn run(input: impl AsRef<str> + Send) -> Result<Stream<Result<String>>> {
    let input = input.as_ref();
    ensure!(!input.is_empty(), "no input provided");

    let exec = Executor::new()?;

    let res = exec.run(input).await.unwrap();
    Ok(res)
}

type Ctx = Arc<Inner>;

struct Inner {
    ai: tokio_openai::Client,
    req: reqwest::Client,
}

#[derive(Clone)]
pub struct Executor {
    ctx: Ctx,
}

/// construct a new context
fn ctx() -> Result<Ctx> {
    let inner = Inner {
        ai: tokio_openai::Client::simple()?,
        req: reqwest::Client::new(),
    };

    Ok(Arc::new(inner))
}

impl Executor {
    fn new() -> Result<Self> {
        Ok(Self { ctx: ctx()? })
    }
}

impl Executor {
    /// run from an input prompt
    async fn run(&self, input: &str) -> Result<utils::Stream<Result<String>>> {
        let sys = "Take in a command and output Rust code that achieves that command. Only output \
                   code. Do not output any other text. Include comments when necessary.";

        let request = ChatRequest::new().sys_msg(sys).user_msg(input);

        let mut res = self.ctx.ai.stream_chat(request).await.unwrap();

        let res = res.boxed();

        Ok(res)
    }
}

fn normalize(mut program: String) -> String {
    // TODO: improve normalization. we only want be trimming the first and last lines
    // for instance, if there is a comment in the middle of the program that includes triple
    // backticks, we do not want to replace it
    program
        .replace("```rust", "")
        .replace("```", "")
        .trim()
        .to_string()
}

#[instrument(skip(executor, comm))]
async fn handle_client(executor: Executor, comm: impl Comm) {
    let process = Process::new(executor, comm);

    if let Err(e) = process.run().await {
        error!("Error: {}", e);
    }
}
