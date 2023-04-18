use std::error::Error;

use clap::Parser;
use once_cell::sync::Lazy;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::app::App;

mod app;
mod bootstrap;
mod comms;
mod terminal;
mod ui;
mod widget;

static CANCEL_TOKEN: Lazy<CancellationToken> = Lazy::new(CancellationToken::new);

#[derive(Parser, Clone)]
pub struct Args {
    #[clap(short, long, default_value = "127.0.0.1")]
    ip: String,
    #[clap(short, long, default_value = "8080")]
    port: u16,

    #[clap(long, default_value = "false")]
    remote: bool,
}

async fn run() -> anyhow::Result<()> {
    info!("Starting frontend-cli");

    let args = Args::parse();

    let (tx, rx) = comms::setup_comms(&args).await?;

    // setup terminal
    let mut terminal = terminal::setup().await?;

    // create app and run it
    let app = App::new(tx, rx);
    let res = app.run(&mut terminal).await;

    // cleanup
    terminal::stop(terminal).await?;

    res?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _guard = bootstrap::setup_tracing();

    if let Err(err) = run().await {
        error!("{err:?}");
    }

    Ok(())
}

#[derive(Debug)]
enum Event {
    Terminal(crossterm::event::Event),
    Packet(protocol::ServerPacket),
}
