//! Markdown to sanitised HTML: comrak renders GitHub-flavoured Markdown without raw
//! HTML, then ammonia keeps only a known set of tags and attributes.

use comrak::{Options, markdown_to_html};

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

    let html = markdown_to_html(markdown, &options);
    ammonia::Builder::default()
        .add_tags(["input", "section"])
        .add_tag_attributes("input", ["type", "checked", "disabled"])
        .add_generic_attributes(["class", "id", "aria-label", "data-sourcepos"])
        .clean(&html)
        .to_string()
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
}
