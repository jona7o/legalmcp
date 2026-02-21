pub mod html;
pub mod pdf;
pub mod xml;

/// Document format indicator for parser dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    Html,
    Xml,
    Pdf,
}
