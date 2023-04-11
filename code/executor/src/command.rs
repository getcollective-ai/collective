//! Commands are executed as such
//!
//! ```text
//! {cmd header}
//! {args}
//! ```
//!
//! where {cmd data} is one line of RON
//! but {args} can be several lines

use async_trait::async_trait;
use derive_discriminant::Discriminant;

use crate::Ctx;

mod bash;
mod librs;
mod zsh;

/// The command we are executing
#[derive(Discriminant)]
enum Cmd {
    /// a zsh script to execute
    Zsh,
    /// a bash script to execute
    Bash,
    LibRs,
}

#[async_trait]
trait Command {
    async fn execute(&self, ctx: Ctx, input: &str) -> anyhow::Result<String>;
}
