#![allow(unused)]

use anyhow::ensure;
use openai::{ChatRequest, Msg};
use tokio::process::Command;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
    }
}

async fn run() -> anyhow::Result<()> {
    let openai = openai::Client::simple()?;

    let system = Msg::system(
        "Provide a list of questions (one per line) that need answering to effectively execute \
         the given prompt.",
    );

    let user = Msg::user("Create video caption library");

    let request = ChatRequest::from([system, user]).stop_at("\n");

    let res = openai.chat(request).await?;

    println!("{res}");

    Ok(())
}

async fn build() -> anyhow::Result<()> {
    let mut cmd = Command::new("docker");

    cmd.arg("build").arg("-t").arg("auton").arg(".");

    let res = cmd.output().await?;

    ensure!(res.status.success(), "is success");
    Ok(())
}

// /// run the image `collective`
// async fn start() -> anyhow::Result<()> {
//     let mut cmd = Command::new("docker");
//
//     cmd.arg("run")
//         .arg("-it")
//         .arg("--rm")
//         .arg("auton")
//         .stdin(std::process::Stdio::inherit())
//         .stdout(std::process::Stdio::inherit());
//
//     let mut pty = pty_process::Pty::new().unwrap();
//     pty.resize(pty_process::Size::new(24, 80)).unwrap();
//
//     let mut cmd = pty_process::Command::new("docker");
//
//     cmd.arg("run").arg("-it").arg("--rm").arg("auton");
//
//     nix
//     let child = cmd.spawn(&pty.pts().unwrap()).unwrap();
//
//     let s = String::new();
//
//     let (reader, tx) = pty.split();
//     let reader = BufReader::new(reader);
//
//     let mut lines = reader.lines();
//
//     while let Some(line) = lines.next_line().await? {
//         println!("{}", line);
//     }
//
//     Ok(())
// }
