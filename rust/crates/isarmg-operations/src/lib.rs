use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Pending,
    Running,
    Succeeded,
    Failed,
    CancelRequested,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub operation_id: Uuid,
    pub kind: String,
    pub state: OperationState,
    pub idempotency_key: Option<String>,
}

impl Operation {
    pub fn new(kind: impl Into<String>, idempotency_key: Option<String>) -> Self {
        Self {
            operation_id: Uuid::new_v4(),
            kind: kind.into(),
            state: OperationState::Pending,
            idempotency_key,
        }
    }
}
