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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub errors: Vec<Finding>,
    pub warnings: Vec<Finding>,
    #[serde(skip, default = "unlimited_findings")]
    max_findings: usize,
    #[serde(skip)]
    errors_truncated: bool,
    #[serde(skip)]
    warnings_truncated: bool,
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            max_findings: unlimited_findings(),
            errors_truncated: false,
            warnings_truncated: false,
        }
    }
}

impl ValidationReport {
    #[must_use]
    pub fn with_max_findings(max_findings: usize) -> Self {
        Self {
            max_findings: max_findings.max(1),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    #[must_use]
    pub fn remaining_error_capacity(&self) -> usize {
        if self.errors_truncated {
            return 0;
        }
        self.max_findings
            .saturating_sub(self.finding_count())
            .saturating_add(1)
    }

    pub fn error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        if self.reserve_finding(FindingLevel::Error) {
            self.errors.push(Finding {
                level: FindingLevel::Error,
                path: path.into(),
                message: message.into(),
            });
        }
    }

    pub fn warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        if self.reserve_finding(FindingLevel::Warning) {
            self.warnings.push(Finding {
                level: FindingLevel::Warning,
                path: path.into(),
                message: message.into(),
            });
        }
    }

    pub fn extend(&mut self, other: ValidationReport) {
        for finding in other.errors {
            self.error(finding.path, finding.message);
        }
        for finding in other.warnings {
            self.warning(finding.path, finding.message);
        }
    }

    fn finding_count(&self) -> usize {
        self.errors.len().saturating_add(self.warnings.len())
    }

    fn reserve_finding(&mut self, level: FindingLevel) -> bool {
        if self.finding_count() < self.max_findings {
            return true;
        }

        let (already_truncated, target, label) = match level {
            FindingLevel::Error => (&mut self.errors_truncated, &mut self.errors, "errors"),
            FindingLevel::Warning => (&mut self.warnings_truncated, &mut self.warnings, "warnings"),
        };
        if !*already_truncated {
            *already_truncated = true;
            target.push(Finding {
                level,
                path: "$".to_owned(),
                message: format!(
                    "diagnostic limit {} reached; additional {label} were omitted",
                    self.max_findings
                ),
            });
        }
        false
    }
}

const fn unlimited_findings() -> usize {
    usize::MAX
}
