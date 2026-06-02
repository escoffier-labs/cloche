//! Self-contained HTML gallery export.
//!
//! Renders a batch of captures into a single shareable HTML file with images
//! embedded as base64 `data:` URIs, so the output has no external dependencies.

use std::path::Path;

/// A single capture as the gallery view needs it.
pub struct GalleryItem<'a> {
    pub title: String,
    pub app: Option<String>,
    pub target: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_at: String,
    pub output_dir: String,
    pub image_path: Option<&'a Path>,
}

/// Render the gallery page. Reads each item's image and embeds it inline.
pub fn render(title: &str, items: &[GalleryItem]) -> String {
    let mut cards = String::new();
    for item in items {
        cards.push_str(&render_card(item));
    }
    if items.is_empty() {
        cards.push_str("<p class=\"empty\">No captures found.</p>");
    }
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{title}</title>\n<style>{css}</style>\n</head>\n<body>\n\
<header><h1>{title}</h1><p class=\"count\">{count} capture{plural}</p></header>\n\
<main class=\"grid\">\n{cards}</main>\n</body>\n</html>\n",
        title = escape_html(title),
        css = CSS,
        count = items.len(),
        plural = if items.len() == 1 { "" } else { "s" },
        cards = cards,
    )
}

fn render_card(item: &GalleryItem) -> String {
    let image = match item.image_path {
        Some(path) => match std::fs::read(path) {
            Ok(bytes) => format!(
                "<img src=\"data:image/png;base64,{}\" alt=\"{}\" loading=\"lazy\">",
                base64_encode(&bytes),
                escape_html(&item.title),
            ),
            Err(_) => "<div class=\"missing\">image unavailable</div>".to_string(),
        },
        None => "<div class=\"missing\">no image</div>".to_string(),
    };

    let dimensions = match (item.width, item.height) {
        (Some(w), Some(h)) => format!("{w}x{h}"),
        _ => "unknown size".to_string(),
    };
    let app = item.app.as_deref().unwrap_or("unknown app");

    format!(
        "<figure class=\"card\">\n{image}\n<figcaption>\n\
<span class=\"card-title\">{title}</span>\n\
<span class=\"meta\">{app} &middot; {target} &middot; {dimensions}</span>\n\
<span class=\"meta\">{created_at}</span>\n\
<span class=\"path\">{output_dir}</span>\n\
</figcaption>\n</figure>\n",
        image = image,
        title = escape_html(&item.title),
        app = escape_html(app),
        target = escape_html(&item.target),
        dimensions = dimensions,
        created_at = escape_html(&item.created_at),
        output_dir = escape_html(&item.output_dir),
    )
}

const CSS: &str = "\
:root{color-scheme:dark}\
*{box-sizing:border-box}\
body{margin:0;background:#0e1116;color:#e6edf3;font:15px/1.5 system-ui,-apple-system,Segoe UI,sans-serif}\
header{padding:32px 32px 8px}\
h1{margin:0;font-size:22px}\
.count{margin:4px 0 0;color:#8b949e}\
.grid{display:grid;gap:20px;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));padding:24px 32px 48px}\
.card{margin:0;background:#161b22;border:1px solid #30363d;border-radius:12px;overflow:hidden;display:flex;flex-direction:column}\
.card img{width:100%;height:auto;display:block;background:#0e1116}\
.missing{padding:48px;text-align:center;color:#8b949e;background:#0e1116}\
figcaption{padding:12px 14px;display:flex;flex-direction:column;gap:2px}\
.card-title{font-weight:600;word-break:break-word}\
.meta{color:#8b949e;font-size:13px}\
.path{color:#6e7681;font-size:12px;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;word-break:break-all}\
.empty{padding:32px;color:#8b949e}";

/// Standard base64 (RFC 4648) with padding.
pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Escape text for safe inclusion in HTML element content and attributes.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"Man"), "TWFu");
    }

    #[test]
    fn escapes_html_metacharacters() {
        assert_eq!(
            escape_html("<a href=\"x\">'&'</a>"),
            "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;"
        );
    }

    #[test]
    fn render_includes_title_and_card_metadata() {
        let items = [GalleryItem {
            title: "Firefox <main>".to_string(),
            app: Some("firefox".to_string()),
            target: "active".to_string(),
            width: Some(1280),
            height: Some(720),
            created_at: "2026-06-01T00:00:00Z".to_string(),
            output_dir: "/tmp/appshot".to_string(),
            image_path: None,
        }];
        let html = render("My Shots", &items);
        assert!(html.contains("<title>My Shots</title>"));
        assert!(html.contains("Firefox &lt;main&gt;"));
        assert!(html.contains("1280x720"));
        assert!(html.contains("1 capture<"));
        assert!(html.contains("no image"));
    }

    #[test]
    fn render_reports_empty_gallery() {
        let html = render("Empty", &[]);
        assert!(html.contains("No captures found."));
        assert!(html.contains("0 captures"));
    }
}
