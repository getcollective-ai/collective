use anyhow::Context;
use derive_build::Build;
use once_cell::sync::Lazy;
use regex::Regex;
use tl::{Node, Parser, ParserOptions};

#[derive(Build)]
struct HtmlToMd<'a> {
    #[required]
    html: &'a str,

    id: Option<String>,
}

impl HtmlToMd<'_> {
    fn run(self) -> anyhow::Result<String> {
        let dom = tl::parse(self.html, ParserOptions::default()).context("Failed to parse html")?;
        let parser = dom.parser();

        let mut s = String::new();

        match self.id {
            None => {
                for node in dom.children() {
                    let node = node.get(parser).context("Failed to parse node")?;
                    node_to_md(&mut s, node, parser)?;
                }
            }
            Some(id) => {
                let parent = dom
                    .get_element_by_id(id.as_str())
                    .context("Failed to find find id")?
                    .get(parser)
                    .context("Failed to parse #{id}")?;
                node_to_md(&mut s, parent, parser)?;
            }
        }

        static MULTI_NEWLINE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\n{2,}").expect("Failed to compile regex"));

        let s = MULTI_NEWLINE.replace_all(&s, "\n\n");

        let s = s.trim();

        Ok(s.to_string())
    }
}

fn node_to_md(s: &mut String, node: &Node, parser: &Parser) -> anyhow::Result<()> {
    match node {
        Node::Tag(tag) => {
            tag_to_md(s, tag, parser)?;
        }
        Node::Raw(raw) => {
            raw_to_md(s, raw);
        }
        Node::Comment(_) => {}
    }
    Ok(())
}

pub fn raw_to_md(s: &mut String, raw: &tl::Bytes) {
    let raw = raw.as_utf8_str();
    let raw = raw.as_ref();
    let raw = raw.replace("&amp;", "&");
    s.push_str(&raw);
}

pub fn tag_to_md(s: &mut String, tag: &tl::HTMLTag, parser: &Parser) -> anyhow::Result<()> {
    let name = tag.name().as_utf8_str();
    let name = name.as_ref();

    match name {
        "script" | "style" | "link" | "img" | "meta" => return Ok(()),
        _ => {}
    }

    let prefix = match name {
        "h1" => "# ",
        "h2" => "## ",
        "h3" => "### ",
        "h4" => "#### ",
        "h5" => "##### ",
        "li" => "- ",
        "ol" => "- ",
        // "tt" if is_rust => "`",
        "pre" => "```\n",
        _ => "",
    };

    let suffix = match name {
        // "tt" if is_rust => "`",
        "pre" => "```",
        _ => "",
    };

    s.push_str(prefix);

    for node in tag.children().top().iter() {
        let node = node.get(parser).context("Failed to parse node")?;
        node_to_md(s, node, parser)?;
    }

    s.push_str(suffix);

    Ok(())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::HtmlToMd;

    #[test]
    fn test_html_to_md() -> anyhow::Result<()> {
        let html = "<h1>hello</h1>\n<h2>world</h2>\n<h3>!</h3>";
        let md = HtmlToMd::new(html).run()?;
        assert_eq!(md, "# hello\n## world\n### !");

        Ok(())
    }

    #[tokio::test]
    async fn test_html_to_md_librs() -> anyhow::Result<()> {
        let req = reqwest::Client::new();

        let html = req
            .get("https://lib.rs/crates/bitflags")
            .send()
            .await?
            .text()
            .await?;

        let md = HtmlToMd::new(html).id("readme").run()?;

        println!("{}", md);
        Ok(())
    }
}
