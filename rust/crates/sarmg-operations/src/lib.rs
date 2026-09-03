//! Deterministic transition rules. Storage and product execution are adapters.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
    DeadLetter,
    Resolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    pub operation_id: String,
    pub namespace: String,
    pub target_key: String,
    pub idempotency_digest: [u8; 32],
    pub request_fingerprint: [u8; 32],
    pub state: OperationState,
    pub attempt: u32,
    pub max_attempts: u32,
    pub not_before_micros: i64,
    pub lease_owner: Option<String>,
    pub lease_expiry_micros: Option<i64>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Transition {
    Claim {
        owner: String,
        lease_expiry_micros: i64,
    },
    Succeed,
    Fail {
        code: String,
        retryable: bool,
        retry_not_before_micros: i64,
    },
    MarkIndeterminate {
        code: String,
    },
    DeadLetter {
        code: String,
    },
    Resolve {
        code: String,
    },
}

pub fn transition(operation: &mut Operation, event: Transition) -> Result<(), Error> {
    match (&operation.state, event) {
        (
            OperationState::Pending,
            Transition::Claim {
                owner,
                lease_expiry_micros,
            },
        ) if lease_expiry_micros > operation.not_before_micros => {
            operation.state = OperationState::Running;
            operation.lease_owner = Some(owner);
            operation.lease_expiry_micros = Some(lease_expiry_micros);
        }
        (OperationState::Running, Transition::Succeed) => {
            operation.state = OperationState::Succeeded;
            clear_lease(operation);
        }
        (
            OperationState::Running,
            Transition::Fail {
                code,
                retryable: true,
                retry_not_before_micros,
            },
        ) => {
            operation.attempt = operation
                .attempt
                .checked_add(1)
                .ok_or(Error::AttemptOverflow)?;
            operation.error_code = Some(code);
            clear_lease(operation);
            if operation.attempt >= operation.max_attempts {
                operation.state = OperationState::DeadLetter;
            } else {
                operation.state = OperationState::Pending;
                operation.not_before_micros = retry_not_before_micros;
            }
        }
        (
            OperationState::Running,
            Transition::Fail {
                code,
                retryable: false,
                ..
            },
        ) => {
            operation.state = OperationState::Failed;
            operation.error_code = Some(code);
            clear_lease(operation);
        }
        (OperationState::Running, Transition::MarkIndeterminate { code }) => {
            operation.state = OperationState::Unknown;
            operation.error_code = Some(code);
            clear_lease(operation);
        }
        (OperationState::Unknown, Transition::DeadLetter { code }) => {
            operation.state = OperationState::DeadLetter;
            operation.error_code = Some(code);
        }
        (
            OperationState::Unknown | OperationState::Failed | OperationState::DeadLetter,
            Transition::Resolve { code },
        ) => {
            operation.state = OperationState::Resolved;
            operation.error_code = Some(code);
            clear_lease(operation);
        }
        (state, event) => {
            return Err(Error::InvalidTransition {
                state: *state,
                event: format!("{event:?}"),
            });
        }
    }
    Ok(())
}
fn clear_lease(operation: &mut Operation) {
    operation.lease_owner = None;
    operation.lease_expiry_micros = None;
}

pub fn validate_idempotency(
    existing: &Operation,
    digest: &[u8; 32],
    fingerprint: &[u8; 32],
) -> Result<(), Error> {
    if &existing.idempotency_digest != digest {
        return Err(Error::IdempotencyKeyMismatch);
    }
    if &existing.request_fingerprint != fingerprint {
        return Err(Error::IdempotencyConflict);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContext {
    pub operation_id: String,
    pub namespace: String,
    pub target_key: String,
    pub attempt: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationOutcome<T> {
    Succeeded(T),
    DefinitiveFailure { code: String, retryable: bool },
    Indeterminate { code: String },
}

#[async_trait]
pub trait OperationExecutor: Send + Sync {
    type Request: Send;
    type Result: Send;
    async fn execute(
        &self,
        context: OperationContext,
        request: Self::Request,
    ) -> OperationOutcome<Self::Result>;
}

pub const PLATFORM_OPERATIONS_DDL: &str = r#"CREATE TABLE _sarmg_operations (
 operation_id TEXT PRIMARY KEY NOT NULL, namespace TEXT NOT NULL, target_key TEXT NOT NULL,
 idempotency_digest BLOB NOT NULL CHECK(length(idempotency_digest)=32), request_fingerprint BLOB NOT NULL CHECK(length(request_fingerprint)=32),
 request_payload BLOB NOT NULL, state TEXT NOT NULL CHECK(state IN ('pending','running','succeeded','failed','unknown','dead_letter','resolved')),
 attempt INTEGER NOT NULL CHECK(attempt>=0), max_attempts INTEGER NOT NULL CHECK(max_attempts>0), not_before_micros INTEGER NOT NULL,
 lease_owner TEXT, lease_expiry_micros INTEGER, error_code TEXT,
 UNIQUE(namespace,idempotency_digest)
);
CREATE UNIQUE INDEX _sarmg_operations_active_target ON _sarmg_operations(namespace,target_key) WHERE state IN ('running','unknown');
CREATE TABLE _sarmg_operation_audit_outbox (event_id TEXT PRIMARY KEY NOT NULL, operation_id TEXT NOT NULL REFERENCES _sarmg_operations(operation_id), payload_json TEXT NOT NULL, created_at_micros INTEGER NOT NULL, delivered_at_micros INTEGER);"#;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    #[error("invalid operation transition from {state:?}: {event}")]
    InvalidTransition {
        state: OperationState,
        event: String,
    },
    #[error("operation attempt overflow")]
    AttemptOverflow,
    #[error("idempotency digest does not identify this operation")]
    IdempotencyKeyMismatch,
    #[error("same idempotency key was used with a different request")]
    IdempotencyConflict,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pending() -> Operation {
        Operation {
            operation_id: "o".into(),
            namespace: "n".into(),
            target_key: "t".into(),
            idempotency_digest: [1; 32],
            request_fingerprint: [2; 32],
            state: OperationState::Pending,
            attempt: 0,
            max_attempts: 2,
            not_before_micros: 0,
            lease_owner: None,
            lease_expiry_micros: None,
            error_code: None,
        }
    }
    #[test]
    fn unknown_is_never_retried() {
        let mut op = pending();
        transition(
            &mut op,
            Transition::Claim {
                owner: "w".into(),
                lease_expiry_micros: 1,
            },
        )
        .unwrap();
        transition(
            &mut op,
            Transition::MarkIndeterminate {
                code: "timeout".into(),
            },
        )
        .unwrap();
        assert_eq!(op.state, OperationState::Unknown);
        assert!(
            transition(
                &mut op,
                Transition::Claim {
                    owner: "w".into(),
                    lease_expiry_micros: 2
                }
            )
            .is_err()
        );
    }
    #[test]
    fn retry_budget_dead_letters() {
        let mut op = pending();
        for expected in [OperationState::Pending, OperationState::DeadLetter] {
            transition(
                &mut op,
                Transition::Claim {
                    owner: "w".into(),
                    lease_expiry_micros: 10,
                },
            )
            .unwrap();
            transition(
                &mut op,
                Transition::Fail {
                    code: "temporary".into(),
                    retryable: true,
                    retry_not_before_micros: 1,
                },
            )
            .unwrap();
            assert_eq!(op.state, expected);
        }
    }
}
