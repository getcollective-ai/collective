#![feature(unsize)]
#![allow(unused)]

use std::sync::Arc;

use utils::default;

mod command;

/// TODO: add executor functionality (running zsh cmd, cd, etc, and modifying files)
fn main() {
    println!("Hello, world!");
}

type Ctx = Arc<Inner>;

#[derive(Default)]
struct Inner {}

struct Executor {
    ctx: Ctx,
}

impl Executor {
    fn new() -> Self {
        Self { ctx: default() }
    }

    fn run(&self, input: &str) -> anyhow::Result<String> {
        // let cmd = command::Cmd::try_from(input)?;
        // let output = cmd.execute(self.ctx, input)?;
        // Ok(output)
        todo!()
    }
}
