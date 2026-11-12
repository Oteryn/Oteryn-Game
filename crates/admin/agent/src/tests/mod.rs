mod channel;
mod postgres;
mod verifier;
use crate::policy::Policy;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_admin_contracts::*;
use serde_json::json;
pub const NOW: i64 = 1793880000;
pub fn key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[7; 32])
}
pub fn policy_text() -> String {
    format!(
        r#"
[[signers]]
key_id = "platform-admin-signer-1"
public_key = "{}"
not_before = "2026-01-01T00:00:00Z"
not_after = "2027-01-01T00:00:00Z"
[operations."admin.policy.get"]
min_risk_class = "R0"
allowed_permissions = ["game.ops.read"]
worlds = ["w-antica"]
"#,
        URL_SAFE_NO_PAD.encode(key().verifying_key().to_bytes())
    )
}
pub fn policy() -> Result<Policy, WireError> {
    Policy::parse(&policy_text())
}
pub fn request() -> Result<Request, WireError> {
    let mut request: Request = serde_json::from_value(json!({
        "contract":CONTRACT,"request_id":"019a5620-0000-7000-8000-000000000001",
        "admin_operation_id":"019a5620-0000-7000-8000-000000000001",
        "operation":"admin.policy.get","operation_version":1,"kind":"query",
        "target":{"world_id":null,"node_id":null,"character_id":null,"account_ref":null},
        "params":{},"dry_run":false,"actor":{"admin_id":"a-17","roles":["game_viewer"],"permission":"game.ops.read",
        "mfa_at":"2026-11-05T11:59:00Z","step_up_at":null,"ip_hash":"test-hash","session_ref":"test-session"},
        "reason":null,"approvals":[],"risk_class":"R0","issued_at":"2026-11-05T12:00:00Z","expires_at":"2026-11-05T12:01:00Z",
        "nonce":URL_SAFE_NO_PAD.encode([1;16]),"signer_key_id":"platform-admin-signer-1","signature":""
    }))?;
    resign(&mut request)?;
    Ok(request)
}
pub fn resign(r: &mut Request) -> Result<(), WireError> {
    r.signature = sign(r, &key())?;
    Ok(())
}
pub fn response(r: &Request) -> Response {
    Response {
        request_id: r.request_id.clone(),
        admin_operation_id: r.admin_operation_id.clone(),
        status: Status::Succeeded,
        error: None,
        result: Some(OperationResult::AdminPolicyGet(PolicyView {
            operations: Vec::new(),
        })),
        dry_run_preview: None,
        game_ledger_id: Some("019a5620-0000-7000-8000-000000000002".into()),
        audit_event_ids: Vec::new(),
        effective_revision: None,
        progress: None,
        completed_at: Some("2026-11-05T12:00:00Z".into()),
        agent_id: "test-agent".into(),
        agent_build: "test-build".into(),
        signature: String::new(),
    }
}
