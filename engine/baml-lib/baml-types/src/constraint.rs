use crate::JinjaExpression;

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, Hash)]
pub struct Constraint {
    pub level: ConstraintLevel,
    pub expression: ConstraintExpression,
    pub label: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, Hash)]
pub enum ConstraintExpression {
    Jinja(JinjaExpression),
    // For now, we'll just store native expressions as a placeholder
    // The actual parsing logic will handle converting from AST variants
    Native(String), // We'll store the string representation for now
}

impl Constraint {
    pub fn new_check(label: &str, expr: &str) -> Self {
        Self {
            label: Some(label.to_string()),
            level: ConstraintLevel::Check,
            expression: ConstraintExpression::Jinja(JinjaExpression(expr.to_string())),
        }
    }

    pub fn new_assert(label: &str, expr: &str) -> Self {
        Self {
            label: Some(label.to_string()),
            level: ConstraintLevel::Assert,
            expression: ConstraintExpression::Jinja(JinjaExpression(expr.to_string())),
        }
    }

    pub fn as_check(self) -> Option<(String, ConstraintExpression)> {
        match self.level {
            ConstraintLevel::Check => Some((
                self.label
                    .expect("Checks are guaranteed by the pest grammar to have a label."),
                self.expression,
            )),
            ConstraintLevel::Assert => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, Eq, Hash, Ord, PartialOrd)]
pub enum ConstraintLevel {
    Check,
    Assert,
}

/// The user-visible schema for a failed check.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
pub struct ResponseCheck {
    pub name: String,
    pub expression: String,
    pub status: String,
}

impl ResponseCheck {
    /// Convert a Constraint and its status to a ResponseCheck.
    /// Returns `None` if the Constraint is not a check (i.e.,
    /// if it doesn't meet the invariants that level==Check and
    /// label==Some).
    pub fn from_check_result(
        (
            Constraint {
                level,
                expression,
                label,
            },
            succeeded,
        ): (Constraint, bool),
    ) -> Option<Self> {
        match (level, label) {
            (ConstraintLevel::Check, Some(label)) => {
                let status = if succeeded {
                    "succeeded".to_string()
                } else {
                    "failed".to_string()
                };
                let expr_string = match expression {
                    ConstraintExpression::Jinja(jinja) => jinja.0,
                    ConstraintExpression::Native(native) => native,
                };
                Some(ResponseCheck {
                    name: label,
                    expression: expr_string,
                    status,
                })
            }
            _ => None,
        }
    }
}
