use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Legal document source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: Uuid,
    pub name: String,
    pub jurisdiction: String,
    pub language: String,
    pub base_url: String,
    pub crawler_type: CrawlerType,
    pub cron_schedule: String,
    pub config_json: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Identifies which crawler implementation handles a source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrawlerType {
    Bundesrecht,
    Bayern,
    EurLex,
    Legifrance,
    Normattiva,
    Boe,
    Custom(String),
}

impl CrawlerType {
    /// Return the string representation stored in the `sources.crawler_type` column.
    pub fn as_str(&self) -> &str {
        match self {
            CrawlerType::Bundesrecht => "bundesrecht",
            CrawlerType::Bayern => "bayern",
            CrawlerType::EurLex => "eur_lex",
            CrawlerType::Legifrance => "legifrance",
            CrawlerType::Normattiva => "normattiva",
            CrawlerType::Boe => "boe",
            CrawlerType::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for CrawlerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for CrawlerType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "bundesrecht" => CrawlerType::Bundesrecht,
            "bayern" => CrawlerType::Bayern,
            "eur_lex" => CrawlerType::EurLex,
            "legifrance" => CrawlerType::Legifrance,
            "normattiva" => CrawlerType::Normattiva,
            "boe" => CrawlerType::Boe,
            other => CrawlerType::Custom(other.to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crawler_type_roundtrip_serde() {
        for ct in [
            CrawlerType::Bundesrecht,
            CrawlerType::Bayern,
            CrawlerType::EurLex,
            CrawlerType::Legifrance,
            CrawlerType::Normattiva,
            CrawlerType::Boe,
            CrawlerType::Custom("test_source".to_string()),
        ] {
            let json = serde_json::to_string(&ct).unwrap();
            let back: CrawlerType = serde_json::from_str(&json).unwrap();
            assert_eq!(ct, back);
        }
    }

    #[test]
    fn crawler_type_from_str() {
        use std::str::FromStr;
        assert_eq!(
            CrawlerType::from_str("bundesrecht").unwrap(),
            CrawlerType::Bundesrecht
        );
        assert_eq!(
            CrawlerType::from_str("eur_lex").unwrap(),
            CrawlerType::EurLex
        );
        let custom = CrawlerType::from_str("my_custom").unwrap();
        assert_eq!(custom, CrawlerType::Custom("my_custom".to_string()));
    }
}
