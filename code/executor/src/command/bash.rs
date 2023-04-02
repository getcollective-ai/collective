use anyhow::{ensure, Context};
use async_trait::async_trait;
use utils::str::StringExt;

use crate::{
    command::{Bash, Command},
    Ctx,
};

#[async_trait]
impl Command for Bash {
    async fn execute(&self, _exec: Ctx, input: &str) -> anyhow::Result<String> {
        let output = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(input)
            .output()
            .await?;

        ensure!(output.status.success(), "bash command failed");

        let mut output = String::from_utf8(output.stdout).context("could not parse to UTF-8")?;
        output.trim_end_in_place(); // remove trailing newline

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::{command::Command, ctx, Ctx};

    #[tokio::test]
    async fn test_oneline() -> anyhow::Result<()> {
        let exec = ctx()?;
        let cmd = super::Bash;

        let output = cmd.execute(exec, "echo hello there").await?;

        assert_eq!(output, "hello there");

        Ok(())
    }

    #[tokio::test]
    async fn test_multiline() -> anyhow::Result<()> {
        let exec = ctx()?;
        let cmd = super::Bash;

        let input = r#"echo hello
        echo there"#;

        let output = cmd.execute(exec, input).await?;

        assert_eq!(output, "hello\nthere");

        Ok(())
    }
}
