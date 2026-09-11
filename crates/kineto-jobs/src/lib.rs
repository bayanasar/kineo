use std::{error::Error, fmt, num::NonZeroU16};

use kineto_project::{ArtifactId, InputHash};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(String);

impl JobId {
    pub fn new(value: impl Into<String>) -> Result<Self, JobValueError> {
        non_empty(value.into()).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn new(value: impl Into<String>) -> Result<Self, JobValueError> {
        non_empty(value.into()).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderJobId(String);

impl ProviderJobId {
    pub fn new(value: impl Into<String>) -> Result<Self, JobValueError> {
        non_empty(value.into()).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn non_empty(value: String) -> Result<String, JobValueError> {
    if value.is_empty() {
        Err(JobValueError)
    } else {
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobValueError;

impl fmt::Display for JobValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("job identifier value must not be empty")
    }
}

impl Error for JobValueError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    WaitingForUser,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentState {
    Prepared,
    Invoked,
    Reconciled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobIntent {
    pub job_id: JobId,
    pub operation: String,
    pub idempotency_key: IdempotencyKey,
    pub input_hash: InputHash,
    pub artifact_id: Option<ArtifactId>,
    pub provider_job_id: Option<ProviderJobId>,
    pub state: IntentState,
}

impl JobIntent {
    pub fn new(
        job_id: JobId,
        operation: impl Into<String>,
        idempotency_key: IdempotencyKey,
        input_hash: InputHash,
        artifact_id: Option<ArtifactId>,
    ) -> Result<Self, JobIntentError> {
        let operation = operation.into();
        if operation.is_empty() {
            return Err(JobIntentError::MissingOperation);
        }
        Ok(Self {
            job_id,
            operation,
            idempotency_key,
            input_hash,
            artifact_id,
            provider_job_id: None,
            state: IntentState::Prepared,
        })
    }

    pub fn mark_invoked(
        &mut self,
        provider_job_id: Option<ProviderJobId>,
    ) -> Result<(), JobIntentError> {
        if self.state != IntentState::Prepared {
            return Err(JobIntentError::InvalidTransition);
        }
        self.provider_job_id = provider_job_id;
        self.state = IntentState::Invoked;
        Ok(())
    }

    pub fn mark_reconciled(&mut self) -> Result<(), JobIntentError> {
        if self.state != IntentState::Invoked {
            return Err(JobIntentError::InvalidTransition);
        }
        self.state = IntentState::Reconciled;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobIntentError {
    MissingOperation,
    InvalidTransition,
}

impl fmt::Display for JobIntentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingOperation => "job operation must not be empty",
            Self::InvalidTransition => "invalid write-ahead intent transition",
        })
    }
}

impl Error for JobIntentError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyCode([u8; 3]);

impl CurrencyCode {
    pub fn new(code: [u8; 3]) -> Result<Self, CurrencyCodeError> {
        if code.iter().all(u8::is_ascii_uppercase) {
            Ok(Self(code))
        } else {
            Err(CurrencyCodeError)
        }
    }

    #[must_use]
    pub const fn bytes(self) -> [u8; 3] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyCodeError;

impl fmt::Display for CurrencyCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("currency code must be three uppercase ASCII letters")
    }
}

impl Error for CurrencyCodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostEstimate {
    pub call_count: u32,
    pub amount_micros: u64,
    pub currency: CurrencyCode,
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Retryable,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffPolicy {
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub max_attempts: u16,
}

impl BackoffPolicy {
    pub fn validate(self) -> Result<Self, BackoffPolicyError> {
        if self.max_attempts == 0 || self.base_delay_ms > self.max_delay_ms {
            Err(BackoffPolicyError)
        } else {
            Ok(self)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffPolicyError;

impl fmt::Display for BackoffPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("backoff policy requires attempts > 0 and base delay <= max delay")
    }
}

impl Error for BackoffPolicyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderExecutionPolicy {
    pub max_concurrency: NonZeroU16,
    pub backoff: BackoffPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaidFallback {
    Forbidden,
    Allowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackPolicy {
    pub paid_fallback: PaidFallback,
    pub max_paid_amount_micros: Option<u64>,
    pub require_user_approval: bool,
}

impl FallbackPolicy {
    #[must_use]
    pub const fn allows_paid_amount(self, amount_micros: u64) -> bool {
        if matches!(self.paid_fallback, PaidFallback::Forbidden) {
            return false;
        }
        match self.max_paid_amount_micros {
            Some(limit) => amount_micros <= limit,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_handle_is_recorded_on_the_write_ahead_intent() {
        let mut intent = JobIntent::new(
            JobId::new("job_01J").unwrap(),
            "video.generate",
            IdempotencyKey::new("video.generate:sha256:abc").unwrap(),
            InputHash::new("sha256:abc").unwrap(),
            None,
        )
        .unwrap();

        let remote = ProviderJobId::new("remote-42").unwrap();
        intent.mark_invoked(Some(remote.clone())).unwrap();

        assert_eq!(intent.provider_job_id, Some(remote));
        assert_eq!(intent.state, IntentState::Invoked);
    }

    #[test]
    fn intent_cannot_skip_directly_from_prepared_to_reconciled() {
        let mut intent = JobIntent::new(
            JobId::new("job_02J").unwrap(),
            "image.generate",
            IdempotencyKey::new("image.generate:sha256:def").unwrap(),
            InputHash::new("sha256:def").unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(
            intent.mark_reconciled().unwrap_err(),
            JobIntentError::InvalidTransition
        );
    }

    #[test]
    fn money_is_integer_and_paid_fallback_respects_the_ceiling() {
        let usd = CurrencyCode::new(*b"USD").unwrap();
        let estimate = CostEstimate {
            call_count: 42,
            amount_micros: 12_500_000,
            currency: usd,
            estimated_duration_ms: Some(18 * 60 * 1000),
        };
        let policy = FallbackPolicy {
            paid_fallback: PaidFallback::Allowed,
            max_paid_amount_micros: Some(10_000_000),
            require_user_approval: true,
        };

        assert_eq!(estimate.currency.bytes(), *b"USD");
        assert!(!policy.allows_paid_amount(estimate.amount_micros));
    }

    #[test]
    fn invalid_backoff_configuration_is_rejected() {
        let result = BackoffPolicy {
            base_delay_ms: 5_000,
            max_delay_ms: 1_000,
            max_attempts: 3,
        }
        .validate();

        assert!(result.is_err());
    }
}
