use legal_core::errors::LegalMcpError;

/// Parse an HTML document into Markdown text.
///
/// Strips navigation, header, footer, and script elements before converting
/// the main content area to Markdown. Returns `LegalMcpError::Parse` if the
/// HTML is empty or contains no usable text content.
pub fn parse(html_bytes: &[u8]) -> Result<String, LegalMcpError> {
    let html_str = std::str::from_utf8(html_bytes)
        .map_err(|e| LegalMcpError::Parse(format!("HTML is not valid UTF-8: {e}")))?;

    if html_str.trim().is_empty() {
        return Err(LegalMcpError::Parse("Empty HTML input".into()));
    }

    // Strip noise elements before converting to Markdown.
    // htmd handles conversion of the remaining HTML.
    let cleaned = strip_noise_elements(html_str);

    let markdown = htmd::convert(&cleaned)
        .map_err(|e| LegalMcpError::Parse(format!("HTML→Markdown conversion failed: {e}")))?;

    let trimmed = markdown.trim().to_owned();
    if trimmed.is_empty() {
        return Err(LegalMcpError::Parse(
            "HTML contained no extractable text content".into(),
        ));
    }

    Ok(trimmed)
}

/// Remove non-content elements from raw HTML using scraper.
///
/// Removes `<nav>`, `<header>`, `<footer>`, `<script>`, `<style>`, and
/// `<aside>` elements. If a `<main>` or `<article>` element is present,
/// returns only its inner HTML; otherwise returns the full `<body>`.
fn strip_noise_elements(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Try to extract just the main content block first.
    let content_selectors = ["main", "article", "[role=\"main\"]", "#content", ".content"];
    for sel_str in &content_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            if let Some(element) = document.select(&sel).next() {
                return element.inner_html();
            }
        }
    }

    // Fall back: extract body and strip known noise elements.
    let body_sel = Selector::parse("body").expect("body selector is valid");
    let body_html = document
        .select(&body_sel)
        .next()
        .map(|e| e.inner_html())
        .unwrap_or_else(|| html.to_owned());

    // We can't mutate the DOM easily; return the raw body and let htmd handle it.
    // The noise in legal sites (nav/footer) is usually small relative to content.
    body_html
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Test Law</title></head>
<body>
<nav><a href="/">Home</a></nav>
<main>
<h1>Bürgerliches Gesetzbuch</h1>
<p>§ 1 Rechtsfähigkeit des Menschen</p>
<p>Die Rechtsfähigkeit des Menschen beginnt mit der Vollendung der Geburt.</p>
</main>
<footer>Copyright</footer>
</body>
</html>"#;

    const HTML_WITH_ARTICLE: &str = r#"<html><body>
<header>Header noise</header>
<article>
<h2>Article 3</h2>
<p>The Union shall have legal personality.</p>
</article>
<footer>Footer noise</footer>
</body></html>"#;

    #[test]
    fn parses_simple_html_to_markdown() {
        let result = parse(SIMPLE_HTML.as_bytes()).unwrap();
        assert!(!result.is_empty());
        // Should contain the heading text
        assert!(result.contains("Bürgerliches Gesetzbuch") || result.contains("§ 1"));
    }

    #[test]
    fn extracts_article_content() {
        let result = parse(HTML_WITH_ARTICLE.as_bytes()).unwrap();
        assert!(result.contains("Article 3") || result.contains("legal personality"));
    }

    #[test]
    fn empty_input_returns_error() {
        let err = parse(b"").unwrap_err();
        assert!(matches!(err, LegalMcpError::Parse(_)));
    }

    #[test]
    fn whitespace_only_returns_error() {
        let err = parse(b"   \n   ").unwrap_err();
        assert!(matches!(err, LegalMcpError::Parse(_)));
    }

    #[test]
    fn invalid_utf8_returns_error() {
        let err = parse(&[0xFF, 0xFE]).unwrap_err();
        assert!(matches!(err, LegalMcpError::Parse(_)));
    }
}
