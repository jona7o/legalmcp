//! SHA-256 content hashing for change detection (ADR-12).
//!
//! `hash_content` computes the canonical SHA-256 hex digest of a document's
//! markdown content. The same digest is stored in `documents.content_hash` and
//! `document_versions.content_hash`, and is used to detect document changes on
//! each crawl cycle.

use sha2::{Digest, Sha256};

/// Compute the SHA-256 hex digest of `content`.
///
/// The returned string is 64 lowercase hex characters, matching the
/// `documents.content_hash` column type (`TEXT`).
///
/// # Examples
///
/// ```
/// use ingest::hasher::hash_content;
///
/// let digest = hash_content("§ 1 Die Rechtsfähigkeit des Menschen beginnt mit der Geburt.");
/// assert_eq!(digest.len(), 64);
/// assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
/// ```
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute the SHA-256 hex digest of a raw API key.
///
/// Only the hash is stored in `api_keys.key_hash`; the raw key is shown to the
/// user once at creation time and never persisted.
pub fn hash_api_key(raw_key: &str) -> String {
    hash_content(raw_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            hash_content(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn output_is_64_hex_chars() {
        let digest = hash_content("some legal text");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn different_content_produces_different_hash() {
        let a = hash_content("version 1");
        let b = hash_content("version 2");
        assert_ne!(a, b);
    }

    #[test]
    fn same_content_produces_same_hash() {
        let content = "§ 242 Treu und Glauben";
        assert_eq!(hash_content(content), hash_content(content));
    }

    #[test]
    fn api_key_hash_matches_content_hash() {
        let key = "lmcp_test_abc123";
        assert_eq!(hash_api_key(key), hash_content(key));
    }
}
