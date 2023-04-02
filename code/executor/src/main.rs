#![feature(unsize)]
#![allow(unused)]

use std::{io::Write, sync::Arc};

use anyhow::ensure;
use futures::StreamExt;
use openai::ChatRequest;
use utils::default;

mod command;

#[tokio::main]
async fn main() {
    if let Err(e) = main2().await {
        eprintln!("{e}");
    }
}

async fn main2() -> anyhow::Result<()> {
    let exec = Executor::new()?;
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    ensure!(!input.is_empty(), "no input provided");

    exec.run(&input).await?;
    Ok(())
}

type Ctx = Arc<Inner>;

struct Inner {
    ai: openai::Client,
}

struct Executor {
    ctx: Ctx,
}

/// construct a new context
fn ctx() -> anyhow::Result<Ctx> {
    let inner = Inner {
        ai: openai::Client::simple()?,
    };

    Ok(Arc::new(inner))
}

impl Executor {
    fn new() -> anyhow::Result<Self> {
        Ok(Self { ctx: ctx()? })
    }
}

impl Executor {
    /// run from an input prompt
    async fn run(&self, input: &str) -> anyhow::Result<()> {
        let sys = "Take in a command and output Rust code that achieves that command. Only output \
                   code. Do not output any other text. Include comments when necessary.";

        let request = ChatRequest::new().sys_msg(sys).user_msg(input);

        let mut res = self.ctx.ai.stream_chat(request).await?;

        let mut res = res.boxed();

        while let Some(msg) = res.next().await {
            let msg = msg?;
            print!("{msg}");

            // flush
            std::io::stdout().flush()?;
        }

        println!();

        Ok(())
    }
}
