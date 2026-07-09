use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub level: FindingLevel,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub errors: Vec<Finding>,
    pub warnings: Vec<Finding>,
}

impl ValidationReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.errors.push(Finding {
            level: FindingLevel::Error,
            path: path.into(),
            message: message.into(),
        });
    }

    pub fn warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(Finding {
            level: FindingLevel::Warning,
            path: path.into(),
            message: message.into(),
        });
    }

    pub fn extend(&mut self, other: ValidationReport) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}
