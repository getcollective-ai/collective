use std::{borrow::Cow, error::Error, io, pin::pin, time::Duration};

use anyhow::{bail, Context};
use clap::Parser;
use crossterm::{
    cursor,
    event::{poll, DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{
    future,
    future::Either,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use once_cell::sync::Lazy;
use protocol::Server;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
    Frame, Terminal,
};

static CANCEL_TOKEN: Lazy<CancellationToken> = Lazy::new(CancellationToken::new);

#[derive(Default)]
struct Label<'a> {
    text: Cow<'a, str>,
}

impl<'a> Widget for Label<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.left(), area.top(), self.text, Style::default());
    }
}

impl<'a> Label<'a> {
    fn text(mut self, text: impl Into<Cow<'a, str>>) -> Label<'a> {
        self.text = text.into();
        self
    }
}

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value = "127.0.0.1")]
    ip: String,
    #[clap(short, long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let Args { ip, port } = Args::parse();

    let address = format!("ws://{ip}:{port}");

    info!("Connecting to {address} via websocket...");
    let (websocket, _) = connect_async(&address).await.unwrap();

    // setup terminal
    info!("Setting up terminal");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    execute!(
        terminal.backend_mut(),
        cursor::Show,
        cursor::SetCursorStyle::BlinkingBar
    )?;

    // create app and run it
    let app = App::new(websocket);

    let res = app.run(&mut terminal).await;

    execute!(
        terminal.backend_mut(),
        cursor::SetCursorStyle::SteadyBlock,
        crossterm::cursor::Hide
    )?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    terminal.show_cursor()?;

    // restore terminal
    disable_raw_mode()?;

    info!("Exiting");

    CANCEL_TOKEN.cancel();

    if let Err(err) = res {
        eprintln!("{err:?}");
    }

    Ok(())
}

struct App {
    write: SplitSink<Ws, Message>,
    read: SplitStream<Ws>,
    instruction: Option<String>,
}

struct Writer {
    inner: SplitSink<Ws, Message>,
}

impl Writer {
    async fn write_packet(&mut self, packet: protocol::ClientPacket) -> anyhow::Result<()> {
        let msg = serde_json::to_string(&packet)?;
        self.inner.send(Message::Text(msg)).await?;

        Ok(())
    }
}

struct Reader {
    inner: SplitStream<Ws>,
}

impl Reader {
    async fn read_packet(&mut self) -> anyhow::Result<protocol::ServerPacket> {
        let msg = self.inner.next().await.unwrap()?;
        let Message::Text(msg) = msg else {
            bail!("Expected text message, got: {:?}", msg)
        };

        let res: protocol::ServerPacket = serde_json::from_str(&msg)?;

        Ok(res)
    }
}

#[derive(Debug)]
enum Event {
    Terminal(crossterm::event::Event),
    Packet(protocol::ServerPacket),
}

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

impl App {
    fn new(websocket: Ws) -> Self {
        let (write, read) = websocket.split();
        Self {
            write,
            read,
            instruction: None,
        }
    }

    async fn run<B: Backend + Send>(mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()> {
        let mut ui = Ui::new();

        let mut writer = Writer { inner: self.write };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::spawn({
            let tx = tx.clone();
            move || {
                loop {
                    if CANCEL_TOKEN.is_cancelled() {
                        return;
                    }
                    if poll(Duration::from_millis(10)).unwrap() {
                        // It's guaranteed that `read` won't block, because `poll` returned
                        // `Ok(true)`.

                        let event = crossterm::event::read().unwrap();
                        tx.send(Event::Terminal(event)).unwrap();
                    }
                }
            }
        });

        let mut waiting_for_question = false;

        tokio::spawn(async move {
            let mut reader = Reader { inner: self.read };
            loop {
                let cancel = pin!(CANCEL_TOKEN.cancelled());
                let packet = pin!(reader.read_packet());
                let packet = match future::select(cancel, packet).await {
                    Either::Left((..)) => {
                        return;
                    }
                    Either::Right((packet, ..)) => packet.unwrap(),
                };

                tx.send(Event::Packet(packet)).unwrap();
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
                    KeyCode::Enter => {
                        if ui.current_line().trim().is_empty() {
                            continue;
                        }
                        let packet = match self.instruction {
                            None => {
                                self.instruction = Some(ui.current_line().clone());
                                protocol::Packet::client(protocol::Instruction {
                                    instruction: ui.current_line().clone(),
                                })
                            }
                            Some(..) => protocol::Packet::client(protocol::Answer {
                                answer: ui.current_line().clone(),
                            }),
                        };

                        waiting_for_question = true;

                        ui.new_line();
                        writer.write_packet(packet).await?;
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

struct Ui {
    input: Vec<String>,
}

impl Ui {
    fn new() -> Self {
        Self {
            input: vec![String::new()],
        }
    }

    fn current_line(&mut self) -> &mut String {
        self.input.last_mut().unwrap()
    }

    fn new_line(&mut self) {
        self.input.push(String::new());
    }

    fn run<B: Backend>(&self, f: &mut Frame<B>) {
        let size = f.size();

        let mut render_loc = size;

        for i in 0..self.input.len() {
            let label = Label::default().text(&self.input[i]);
            f.render_widget(label, render_loc);
            render_loc.y += 1;
        }
        f.set_cursor(
            render_loc.x + u16::try_from(self.input.last().unwrap().len()).unwrap(),
            render_loc.y - 1,
        );
    }
}
