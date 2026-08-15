//! Named shortcuts that fill [`crate::settings::PipelineSettings`].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Fast,
    Balanced,
    Quality,
}

#[cfg(test)]
mod tests {
    use super::Preset;

    #[test]
    fn serde_lowercase_roundtrip() {
        let json = serde_json::to_string(&Preset::Balanced).unwrap();
        assert_eq!(json, "\"balanced\"");
        let parsed: Preset = serde_json::from_str("\"fast\"").unwrap();
        assert_eq!(parsed, Preset::Fast);
    }
}
