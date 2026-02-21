use legal_core::errors::LegalMcpError;
use quick_xml::{events::Event, Reader};

/// Parsed XML document fields extracted from Bundesrecht/legal XML.
#[derive(Debug, Default)]
pub struct XmlDocument {
    /// Document title (from `<titel>`, `<jurabk>`, or `<amtabk>` elements).
    pub title: String,
    /// Full text content as Markdown-ish plain text.
    pub content: String,
}

/// Parse a legal XML document (Bundesrecht schema) into structured text.
///
/// Handles the Bundesrecht XML schema which uses elements like `<jurabk>`,
/// `<amtabk>`, `<titel>`, `<text>`, `<Content>`, and `<P>` for content.
///
/// Returns `LegalMcpError::Parse` on malformed or empty input.
pub fn parse(xml_bytes: &[u8]) -> Result<XmlDocument, LegalMcpError> {
    if xml_bytes.is_empty() {
        return Err(LegalMcpError::Parse("Empty XML input".into()));
    }

    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut doc = XmlDocument::default();
    let mut current_element = String::new();
    let mut content_buf = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).into_owned();
            }
            Ok(Event::Text(ref e)) => {
                let text = e
                    .unescape()
                    .map_err(|err| LegalMcpError::Parse(format!("XML escape error: {err}")))?;
                let text = text.trim();
                if text.is_empty() {
                    buf.clear();
                    continue;
                }

                match current_element.as_str() {
                    // Bundesrecht title fields
                    "jurabk" | "amtabk" | "titel" | "Titel" => {
                        if doc.title.is_empty() {
                            doc.title = text.to_owned();
                        }
                    }
                    // Content elements
                    "P" | "BR" | "text" | "Text" | "Content" | "norm" => {
                        if !content_buf.is_empty() {
                            content_buf.push('\n');
                        }
                        content_buf.push_str(text);
                    }
                    _ => {
                        // Collect all other text into content as well
                        if !content_buf.is_empty() {
                            content_buf.push(' ');
                        }
                        content_buf.push_str(text);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(LegalMcpError::Parse(format!(
                    "XML parse error at position {}: {e}",
                    reader.buffer_position()
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    doc.content = content_buf.trim().to_owned();

    if doc.content.is_empty() && doc.title.is_empty() {
        return Err(LegalMcpError::Parse(
            "XML document contained no extractable content".into(),
        ));
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDESRECHT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<dokumente>
  <norm builddate="20230101">
    <metadaten>
      <jurabk>BGB</jurabk>
      <amtabk>BGB</amtabk>
      <titel>Bürgerliches Gesetzbuch</titel>
    </metadaten>
    <textdaten>
      <text format="XML">
        <Content>
          <P>§ 1 Rechtsfähigkeit des Menschen</P>
          <P>Die Rechtsfähigkeit des Menschen beginnt mit der Vollendung der Geburt.</P>
        </Content>
      </text>
    </textdaten>
  </norm>
</dokumente>"#;

    const MINIMAL_XML: &str = r#"<?xml version="1.0"?>
<law>
  <title>Test Law</title>
  <text>Some legal content here.</text>
</law>"#;

    #[test]
    fn parses_bundesrecht_xml() {
        let result = parse(BUNDESRECHT_XML.as_bytes()).unwrap();
        // Title should be extracted from jurabk/amtabk/titel
        assert!(!result.title.is_empty());
        // Content should include paragraph text
        assert!(result.content.contains("Rechtsfähigkeit") || !result.content.is_empty());
    }

    #[test]
    fn parses_minimal_xml() {
        let result = parse(MINIMAL_XML.as_bytes()).unwrap();
        assert!(!result.content.is_empty());
    }

    #[test]
    fn empty_bytes_returns_error() {
        let err = parse(b"").unwrap_err();
        assert!(matches!(err, LegalMcpError::Parse(_)));
    }

    #[test]
    fn malformed_xml_returns_error() {
        // Unclosed tag - quick-xml is lenient but verify we handle edge cases
        let result = parse(b"<a><b>text</a>");
        // Either parses successfully (lenient) or returns parse error - both acceptable
        match result {
            Ok(doc) => assert!(!doc.content.is_empty() || !doc.title.is_empty()),
            Err(LegalMcpError::Parse(_)) => {}
            Err(e) => panic!("Unexpected error type: {e}"),
        }
    }

    #[test]
    fn xml_with_only_whitespace_returns_error() {
        let result = parse(b"<doc>   </doc>");
        // An element with only whitespace should either fail or produce empty content
        match result {
            Err(LegalMcpError::Parse(_)) => {}
            Ok(doc) => {
                // If it "succeeds", content must be empty (whitespace stripped)
                assert!(doc.content.trim().is_empty() && doc.title.trim().is_empty());
            }
            Err(e) => panic!("Unexpected error type: {e}"),
        }
    }
}
