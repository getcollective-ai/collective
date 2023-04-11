#![feature(unsize)]
#![allow(unused)]

use std::{io::Write, sync::Arc};

use anyhow::{anyhow, ensure, Result};
use futures::{stream::BoxStream, StreamExt};
use openai::ChatRequest;
use utils::{default, Stream};

mod command;

#[tokio::main]
async fn main() {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    if let Err(e) = run(input).await {
        eprintln!("{e}");
    }
}

async fn run(input: impl AsRef<str> + Send) -> Result<Stream<Result<String>>> {
    let input = input.as_ref();
    ensure!(!input.is_empty(), "no input provided");

    let exec = Executor::new()?;

    let res = exec.run(input).await?;
    Ok(res)
}

type Ctx = Arc<Inner>;

struct Inner {
    ai: openai::Client,
    req: reqwest::Client,
}

struct Executor {
    ctx: Ctx,
}

/// construct a new context
fn ctx() -> Result<Ctx> {
    let inner = Inner {
        ai: openai::Client::simple()?,
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

        let mut res = self.ctx.ai.stream_chat(request).await?;

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

// #[cfg(test)]
// mod tests {
//     use anyhow::{bail, ensure};
//     use futures::TryStreamExt;
//     use tokio::{fs::File, io::AsyncWriteExt};
//
//     use crate::{normalize, run};
//
//     /// compiles program and runs it
//     async fn rust_run(program: impl AsRef<str> + Send) -> anyhow::Result<String> {
//         let program = program.as_ref();
//         let dir = tempfile::tempdir_in(std::env::temp_dir())?;
//
//         let dir = dir.path();
//         let file_path = dir.join("main.rs");
//
//         let mut file = File::create(&file_path).await?;
//         file.write_all(program.as_bytes()).await?;
//
//         let output_path = dir.join("main");
//
//         let rustc = tokio::process::Command::new("rustc")
//             .arg(file_path)
//             .arg("-o")
//             .arg(&output_path)
//             .output()
//             .await?;
//
//         if !rustc.status.success() {
//             let err = String::from_utf8(rustc.stderr)?;
//             bail!(err)
//         }
//
//         ensure!(output_path.is_file());
//
//         // run command
//         let output = tokio::process::Command::new(output_path).output().await?;
//
//         ensure!(output.status.success());
//
//         let output = String::from_utf8(output.stdout)?;
//
//         Ok(output)
//     }
//
//     #[tokio::test]
//     async fn test_simple_run() -> anyhow::Result<()> {
//         let program = run("add two numbers 2 and 2").await?;
//
//         let program: Vec<_> = program.try_collect().await?;
//         let program = program.join("");
//
//         let program = normalize(program);
//
//         let res = rust_run(&program).await?;
//         let res = res.trim();
//
//         assert!(res.contains('4'));
//
//         Ok(())
//     }
// }
