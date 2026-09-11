use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstraintId(String);

impl ConstraintId {
    pub fn new(value: impl Into<String>) -> Result<Self, ConstraintIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ConstraintIdError);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstraintIdError;

impl fmt::Display for ConstraintIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("constraint id must not be empty")
    }
}

impl Error for ConstraintIdError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DegradationLevel {
    None,
    Cosmetic,
    IdentityAffecting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedConstraint {
    pub constraint: ConstraintId,
    pub reason: String,
    pub degradation: DegradationLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledPrompt {
    pub prompt: String,
    pub applied_constraints: Vec<ConstraintId>,
    pub dropped_constraints: Vec<DroppedConstraint>,
    pub degradation: DegradationLevel,
    pub compiler_version: String,
}

impl CompiledPrompt {
    pub fn new(
        prompt: impl Into<String>,
        applied_constraints: Vec<ConstraintId>,
        dropped_constraints: Vec<DroppedConstraint>,
        compiler_version: impl Into<String>,
    ) -> Result<Self, CompiledPromptError> {
        let compiler_version = compiler_version.into();
        if compiler_version.is_empty() {
            return Err(CompiledPromptError::MissingCompilerVersion);
        }
        if dropped_constraints
            .iter()
            .any(|constraint| constraint.reason.is_empty())
        {
            return Err(CompiledPromptError::MissingDropReason);
        }

        let degradation = dropped_constraints
            .iter()
            .map(|constraint| constraint.degradation)
            .max()
            .unwrap_or(DegradationLevel::None);

        Ok(Self {
            prompt: prompt.into(),
            applied_constraints,
            dropped_constraints,
            degradation,
            compiler_version,
        })
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        self.degradation != DegradationLevel::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledPromptError {
    MissingCompilerVersion,
    MissingDropReason,
}

impl fmt::Display for CompiledPromptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingCompilerVersion => "compiler version must not be empty",
            Self::MissingDropReason => "every dropped constraint must explain why it was dropped",
        })
    }
}

impl Error for CompiledPromptError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationPolicy {
    pub block_identity_affecting_degradation: bool,
}

impl GenerationPolicy {
    #[must_use]
    pub const fn allows(self, degradation: DegradationLevel) -> bool {
        !(self.block_identity_affecting_degradation
            && matches!(degradation, DegradationLevel::IdentityAffecting))
    }
}

pub trait PromptCompiler<Intent> {
    type Error;

    fn compile(&self, intent: &Intent) -> Result<CompiledPrompt, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constraint(value: &str) -> ConstraintId {
        ConstraintId::new(value).unwrap()
    }

    #[test]
    fn degradation_is_derived_from_dropped_constraints() {
        let compiled = CompiledPrompt::new(
            "portrait of Alice",
            vec![constraint("framing.medium_close_up")],
            vec![
                DroppedConstraint {
                    constraint: constraint("negative_prompt.no_text"),
                    reason: "provider has no negative-prompt channel".to_owned(),
                    degradation: DegradationLevel::Cosmetic,
                },
                DroppedConstraint {
                    constraint: constraint("identity.alice.reference_pack"),
                    reason: "provider does not accept identity references".to_owned(),
                    degradation: DegradationLevel::IdentityAffecting,
                },
            ],
            "image-prompt/1",
        )
        .unwrap();

        assert_eq!(compiled.degradation, DegradationLevel::IdentityAffecting);
        assert!(compiled.is_degraded());
    }

    #[test]
    fn policy_can_block_identity_affecting_degradation() {
        let policy = GenerationPolicy {
            block_identity_affecting_degradation: true,
        };

        assert!(policy.allows(DegradationLevel::Cosmetic));
        assert!(!policy.allows(DegradationLevel::IdentityAffecting));
    }

    #[test]
    fn every_dropped_constraint_requires_a_reason() {
        let result = CompiledPrompt::new(
            "prompt",
            Vec::new(),
            vec![DroppedConstraint {
                constraint: constraint("camera.slow_dolly"),
                reason: String::new(),
                degradation: DegradationLevel::Cosmetic,
            }],
            "video-prompt/1",
        );

        assert_eq!(result.unwrap_err(), CompiledPromptError::MissingDropReason);
    }
}
