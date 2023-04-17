#![feature(unsize)]
#![allow(unused)]

use std::{io::Write, sync::Arc};

use anyhow::{anyhow, ensure, Result};
use clap::Parser;
use futures::{stream::BoxStream, SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_openai::ChatRequest;
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};
use tracing::{debug, error, info};
use utils::{default, Stream};

use crate::process::Process;

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

pub fn launch(args: Args) -> UnboundedReceiver<Event> {
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

            let executor = executor.clone();
            tokio::spawn(async move {
                handle_client(executor, ws_stream).await;
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

async fn handle_client(executor: Executor, ws_stream: WebSocketStream<TcpStream>) {
    let (mut write, mut read) = ws_stream.split();

    let process = Process::new(executor, read, write);

    if let Err(e) = process.run().await {
        error!("Error: {}", e);
    }
}
