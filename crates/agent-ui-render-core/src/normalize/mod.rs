use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    diagnostic::Finding,
    domain::Report,
    wire::compact::{self, normalize_compact_report},
};

#[derive(Debug, Error)]
pub enum NormalizeError {
    #[error("top-level value must be an object")]
    TopLevelNotObject,
    #[error("unsupported compact input version: {0}")]
    UnsupportedVersion(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizationResult {
    pub input: Report,
    pub warnings: Vec<Finding>,
}

pub fn normalize_report(value: &Value) -> Result<NormalizationResult, NormalizeError> {
    let Some(object) = value.as_object() else {
        return Err(NormalizeError::TopLevelNotObject);
    };
    match object.get("version").and_then(Value::as_u64) {
        Some(compact::VERSION) => {
            let (input, warnings) = normalize_compact_report(value);
            Ok(NormalizationResult { input, warnings })
        }
        other => Err(NormalizeError::UnsupportedVersion(
            other.map_or("null".to_owned(), |version| version.to_string()),
        )),
    }
}
