#![allow(unused)]

use anyhow::ensure;
use log::{error, info};
use tokio::process::Command;
use tokio_openai::{ChatRequest, Msg};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        error!("{e}");
    }
}

async fn run() -> anyhow::Result<()> {
    env_logger::init();
    build().await?;

    Ok(())
}

/// TODO: include dockerfile in the binary
async fn build() -> anyhow::Result<()> {
    info!("🔨 building image");
    let mut cmd = Command::new("docker");

    cmd.arg("build").arg("-t").arg("auton").arg(".");

    let res = cmd.output().await?;

    ensure!(res.status.success(), "is success");
    Ok(())
}

/// run the image `collective`
/// TODO: launch executor in the container
fn start() -> anyhow::Result<()> {
    todo!()
}
