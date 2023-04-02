use anyhow::{ensure, Context};
use async_trait::async_trait;
use utils::str::StringExt;

use crate::{
    command::{Command, Zsh},
    Ctx,
};

#[async_trait]
impl Command for Zsh {
    async fn execute(&self, exec: Ctx, input: &str) -> anyhow::Result<String> {
        let output = tokio::process::Command::new("zsh")
            .arg("-c")
            .arg(input)
            .output()
            .await?;

        ensure!(output.status.success(), "zsh command failed");

        let mut output = String::from_utf8(output.stdout).context("could not parse to UTF-8")?;
        output.trim_end_in_place(); // remove trailing newline

        Ok(output)
    }
}
