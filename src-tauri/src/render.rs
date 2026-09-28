//! Markdown to sanitised HTML: comrak renders GitHub-flavoured Markdown without raw
//! HTML, syntect marks up fenced code by language, then ammonia keeps only a known set
//! of tags and attributes.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::sync::LazyLock;

use comrak::adapters::SyntaxHighlighterAdapter;
use comrak::options::Plugins;
use comrak::{Options, html, markdown_to_html_with_plugins};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// bat's syntaxes, a superset of Sublime's defaults with TypeScript, TOML, Dockerfile...
static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

/// Wraps each scope of the code in a `<span class="hl-…">`, coloured by the front end's
/// stylesheet. Unlike comrak's own syntect adapter, it keeps the `data-sourcepos` of the `<pre>`.
struct Highlighter;

impl SyntaxHighlighterAdapter for Highlighter {
    fn write_highlighted(
        &self,
        output: &mut dyn Write,
        lang: Option<&str>,
        code: &str,
    ) -> fmt::Result {
        let syntax = lang
            .filter(|lang| !lang.is_empty())
            .and_then(|lang| SYNTAXES.find_syntax_by_token(lang));
        let Some(syntax) = syntax else {
            return html::escape(output, code);
        };
        let mut generator = ClassedHTMLGenerator::new_with_class_style(
            syntax,
            &SYNTAXES,
            ClassStyle::SpacedPrefixed { prefix: "hl-" },
        );
        for line in LinesWithEndings::from(code) {
            if generator
                .parse_html_for_line_which_includes_newline(line)
                .is_err()
            {
                return html::escape(output, code);
            }
        }
        output.write_str(&generator.finalize())
    }

    fn write_pre_tag(
        &self,
        output: &mut dyn Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        html::write_opening_tag(output, "pre", attributes)
    }

    fn write_code_tag(
        &self,
        output: &mut dyn Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        html::write_opening_tag(output, "code", attributes)
    }
}

pub fn to_html(markdown: &str) -> String {
    let mut options = Options::default();
    let ext = &mut options.extension;
    ext.strikethrough = true;
    ext.table = true;
    ext.autolink = true;
    ext.tasklist = true;
    ext.footnotes = true;
    ext.alerts = true;
    ext.description_lists = true;
    ext.front_matter_delimiter = Some("---".into());
    ext.header_id_prefix = Some("h-".into());
    ext.header_id_prefix_in_href = true;
    options.render.r#unsafe = false;
    // `data-sourcepos` lets the front end scroll to the line a search matched.
    options.render.sourcepos = true;

    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&Highlighter);

    let html = markdown_to_html_with_plugins(markdown, &options, &plugins);
    ammonia::Builder::default()
        .add_tags(["input", "section"])
        .add_tag_attributes("input", ["type", "checked", "disabled"])
        .add_generic_attributes(["class", "id", "aria-label", "data-sourcepos"])
        .clean(&html)
        .to_string()
}

/// Compiles the grammars notes use most, which syntect otherwise does on first use:
/// about 200 ms for TypeScript alone.
pub fn warm_up() {
    to_html("```ts\nx\n```\n```sh\nx\n```\n```rust\nx\n```\n```json\nx\n```");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_gfm() {
        let html =
            to_html("---\ntitle: x\n---\n# Hi\n\n- [x] done\n\n| a |\n|---|\n| 1 |\n\n~~no~~");
        assert!(!html.contains("title: x"));
        assert!(
            html.contains(r#"<h1 id="h-hi" data-sourcepos="4:1-4:4">"#),
            "{html}"
        );
        assert!(html.contains(r#"type="checkbox""#));
        assert!(html.contains("<table "));
        assert!(html.contains(">no</del>"));
    }

    #[test]
    fn drops_scripts_and_handlers() {
        let html = to_html(
            "<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n[x](javascript:alert(1))",
        );
        assert!(!html.contains("<script"));
        assert!(!html.contains("onerror"));
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn highlights_fenced_code_by_language() {
        let html = to_html("```ts\nconst a = \"<b>\";\n```\n\n```\nlet b = 1;\n```");
        assert!(
            html.contains(r#"<pre data-sourcepos="1:1-3:3"><code class="language-ts">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="hl-storage hl-type"#),
            "{html}"
        );
        assert!(html.contains("&lt;b&gt;"), "{html}");
        assert!(
            html.contains(r#"<pre data-sourcepos="5:1-7:3"><code>let b = 1;"#),
            "{html}"
        );
    }

    #[test]
    fn leaves_unknown_languages_plain() {
        let html = to_html("```nope\n<i>x</i>\n```");
        assert!(
            html.contains(r#"<code class="language-nope">&lt;i&gt;x&lt;/i&gt;"#),
            "{html}"
        );
    }
}
