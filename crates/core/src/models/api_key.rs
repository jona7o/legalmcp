use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A hashed API key stored in `api_keys`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    /// SHA-256 hex of the raw key (never stored in plaintext).
    pub key_hash: String,
    /// First 8 characters of the raw key (display only, for identification).
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
}

impl ApiKey {
    /// Returns true if the key is active (enabled and not expired).
    pub fn is_active(&self) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(expires_at) = self.expires_at {
            return Utc::now() < expires_at;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn active_key_with_no_expiry() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            key_hash: "abc".to_string(),
            key_prefix: "abc12345".to_string(),
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            enabled: true,
        };
        assert!(key.is_active());
    }

    #[test]
    fn disabled_key_is_inactive() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            key_hash: "abc".to_string(),
            key_prefix: "abc12345".to_string(),
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            enabled: false,
        };
        assert!(!key.is_active());
    }

    #[test]
    fn expired_key_is_inactive() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            key_hash: "abc".to_string(),
            key_prefix: "abc12345".to_string(),
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: Some(Utc::now() - Duration::hours(1)),
            enabled: true,
        };
        assert!(!key.is_active());
    }
}
