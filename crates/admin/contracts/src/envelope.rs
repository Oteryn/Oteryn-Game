use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Command,
    Query,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub world_id: Option<String>,
    pub node_id: Option<String>,
    pub character_id: Option<String>,
    pub account_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub admin_id: String,
    pub roles: Vec<String>,
    pub permission: String,
    pub mfa_at: String,
    pub step_up_at: Option<String>,
    pub ip_hash: String,
    pub session_ref: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reason {
    pub code: String,
    pub text: String,
    pub ticket_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub admin_id: String,
    pub approved_at: String,
    pub approval_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub contract: String,
    pub request_id: String,
    pub admin_operation_id: String,
    pub operation: String,
    pub operation_version: u32,
    pub kind: Kind,
    pub target: Target,
    /// Kept opaque until verifier step 5; decoded by the operation's closed schema.
    pub params: Value,
    pub dry_run: bool,
    pub actor: Actor,
    pub reason: Option<Reason>,
    pub approvals: Vec<Approval>,
    pub risk_class: RiskClass,
    pub issued_at: String,
    pub expires_at: String,
    pub nonce: String,
    pub signer_key_id: String,
    pub signature: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Accepted,
    InProgress,
    Succeeded,
    Failed,
    Rejected,
    Expired,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AdmUnknownOperation,
    AdmSignerUntrusted,
    AdmBadSignature,
    AdmExpired,
    AdmReplay,
    AdmInvalidParams,
    AdmForbidden,
    AdmRiskClassMismatch,
    AdmControlMissing,
    AdmPreviewStale,
    AdmIdempotencyConflict,
    AdmTargetNotFound,
    AdmPreconditionFailed,
    AdmTargetUnavailable,
    AdmBusy,
    AdmAgentSuperseded,
    AdmLimitExceeded,
    AdmInternal,
}
impl ErrorCode {
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::AdmTargetUnavailable | Self::AdmBusy | Self::AdmInternal
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorDetail {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub step: u32,
    pub of: u32,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub aggregate: String,
    pub id: String,
    pub before_revision: Option<String>,
    pub after_revision: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub effects: Vec<Effect>,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub request_id: String,
    pub admin_operation_id: String,
    pub status: Status,
    pub error: Option<ErrorDetail>,
    pub result: Option<crate::OperationResult>,
    pub dry_run_preview: Option<Preview>,
    pub game_ledger_id: Option<String>,
    pub audit_event_ids: Vec<String>,
    pub effective_revision: Option<String>,
    pub progress: Option<Progress>,
    pub completed_at: Option<String>,
    pub agent_id: String,
    pub agent_build: String,
    pub signature: String,
}
