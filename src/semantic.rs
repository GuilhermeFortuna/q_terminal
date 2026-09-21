//! Exhaustive presentation roles shared by Rust-backed view models.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SemanticRole {
    Neutral,
    Positive,
    Negative,
    Warning,
    Critical,
    Stale,
    Disabled,
    Accent,
    Live,
}

impl SemanticRole {
    pub const ALL: [Self; 9] = [
        Self::Neutral,
        Self::Positive,
        Self::Negative,
        Self::Warning,
        Self::Critical,
        Self::Stale,
        Self::Disabled,
        Self::Accent,
        Self::Live,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Stale => "stale",
            Self::Disabled => "disabled",
            Self::Accent => "accent",
            Self::Live => "live",
        }
    }

    pub fn signed_decimal(value: &str) -> Self {
        let trimmed = value.trim();
        if trimmed.starts_with('-') && trimmed.chars().any(|ch| matches!(ch, '1'..='9')) {
            Self::Negative
        } else if trimmed.chars().any(|ch| matches!(ch, '1'..='9')) {
            Self::Positive
        } else {
            Self::Neutral
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticRole;
    use std::collections::HashSet;

    #[test]
    fn every_role_has_one_unique_wire_name() {
        let names: HashSet<_> = SemanticRole::ALL.iter().map(|role| role.as_str()).collect();
        assert_eq!(names.len(), SemanticRole::ALL.len());
    }

    #[test]
    fn signed_decimal_does_not_use_floating_point() {
        assert_eq!(SemanticRole::signed_decimal("0.00"), SemanticRole::Neutral);
        assert_eq!(
            SemanticRole::signed_decimal("12.50"),
            SemanticRole::Positive
        );
        assert_eq!(
            SemanticRole::signed_decimal("-0.25"),
            SemanticRole::Negative
        );
    }
}
