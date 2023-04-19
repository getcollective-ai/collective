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

async fn run(args: Args) -> anyhow::Result<()> {
    info!("Starting frontend-cli");

    let (tx, rx) = comms::setup_comms(&args).await?;

    // setup terminal
    let mut terminal = terminal::setup().await?;

    // create app and run it
    let app = App::new(tx, rx);
    let res = app.run(&mut terminal).await;

    // cleanup
    terminal::stop(terminal).await?;

    res
}

#[tokio::main]
async fn main() {
    // when this guard is dropped, the file we are writing to
    // will be flushed and closed.
    let _guard = bootstrap::setup_tracing();

    let args = Args::parse();

    ctrlc::set_handler(move || {
        CANCEL_TOKEN.cancel();
    })
    .expect("Error setting Ctrl-C handler");

    if let Err(err) = run(args).await {
        error!("{err:?}");
    }
}

#[derive(Debug)]
enum Event {
    Terminal(crossterm::event::Event),
    Packet(protocol::ServerPacket),
}
