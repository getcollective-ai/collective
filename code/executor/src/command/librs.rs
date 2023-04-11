use anyhow::Context;
use async_trait::async_trait;

use crate::{
    command::{Command, LibRs},
    Ctx,
};

#[async_trait]
impl Command for LibRs {
    async fn execute(&self, ctx: Ctx, input: &str) -> anyhow::Result<String> {
        let url = format!("https://lib.rs/crates/{input}");

        let html = ctx.req.get(url).send().await?.text().await?;

        let dom = tl::parse(&html, tl::ParserOptions::default())?;
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("readme")
            .context("Failed to find find readme")?
            .get(parser)
            .context("Failed to parse #readme")?;

        let element = element.inner_html(parser);
        Ok(format!("{}", element))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx;

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let ctx = ctx()?;
        let cmd = LibRs;
        let output = cmd.execute(ctx, "bitflags").await.unwrap();
        println!("{}", output);

        Ok(())
    }
}
