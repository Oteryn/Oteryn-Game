use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub contract_version: u8,
    pub command: String,
    pub variant: String,
    pub issuer_authority: String,
    pub issuer_decision_id: String,
    pub source_revision: String,
    pub operation_id: String,
    pub account_id: String,
    pub termination_id: String,
    pub lifecycle_generation: String,
    pub expected_recovery_generation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    pub scheduled_for: String,
    pub issued_at_source: String,
    pub expires_at_source: String,
    pub audience: String,
}

pub fn decode(raw: &[u8], command: &str, operation: &str) -> Result<Intent, &'static str> {
    if raw.len() > 4096 || raw.iter().any(|b| *b <= 0x20 || *b == b'\\') {
        return Err(INTENT_INVALID);
    }
    let head: Head = serde_json::from_slice(raw).map_err(|_| INTENT_INVALID)?;
    match &head.command {
        Some(serde_json::Value::String(stored)) if stored == command => {}
        Some(serde_json::Value::String(_)) => return Err(OPERATION_CONFLICT),
        _ => return Err(INTENT_INVALID),
    }
    let variant = match command {
        "CloseAccount" => "PLATFORM_ACCOUNT_CLOSE",
        "ResolveAccountClosure" => "PLATFORM_ACCOUNT_CLOSE_RESOLUTION",
        _ => return Err(UNSUPPORTED),
    };
    if head.contract_version != Some(serde_json::Value::from(2))
        || head.variant != Some(serde_json::Value::from(variant))
    {
        return Err(UNSUPPORTED);
    }
    let value: Intent = serde_json::from_slice(raw).map_err(|_| INTENT_INVALID)?;
    if serde_json::to_vec(&value).map_err(|_| INTENT_INVALID)? != raw
        || value.operation_id != operation
        || value.issuer_authority != ISSUER_AUTHORITY
        || value.audience != AUDIENCE
        || [
            &value.operation_id,
            &value.account_id,
            &value.termination_id,
            &value.issuer_decision_id,
        ]
        .iter()
        .any(|id| !uuid_v7(id))
        || [
            &value.source_revision,
            &value.lifecycle_generation,
            &value.expected_recovery_generation,
            &value.scheduled_for,
            &value.issued_at_source,
            &value.expires_at_source,
        ]
        .iter()
        .any(|v| counter(v).is_none())
    {
        return Err(INTENT_INVALID);
    }
    let issued = counter(&value.issued_at_source).ok_or(INTENT_INVALID)?;
    let expires = counter(&value.expires_at_source).ok_or(INTENT_INVALID)?;
    if expires <= issued
        || expires - issued > MAX_TTL_SECONDS
        || i64::try_from(expires).is_err()
        || i64::try_from(counter(&value.scheduled_for).ok_or(INTENT_INVALID)?).is_err()
    {
        return Err(INTENT_INVALID);
    }
    match (
        &value.close_operation_id,
        value.decision.as_deref(),
        command,
    ) {
        (None, None, "CloseAccount") => {}
        (Some(close), Some("CANCEL" | "FINALIZE"), "ResolveAccountClosure") if uuid_v7(close) => {}
        _ => return Err(INTENT_INVALID),
    }
    Ok(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ClosureState {
    #[serde(rename = "CLOSED_DETACHED")]
    ClosedDetached,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "FINALIZED")]
    Finalized,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultData {
    pub account_id: String,
    pub termination_id: String,
    pub lifecycle_generation: String,
    pub closure_state: ClosureState,
    pub detached_characters: String,
    pub manifest_sha256: String,
    pub projection_epoch: String,
    pub projection_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_sha256: Option<String>,
}

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl ResultData {
    pub fn seal(&mut self) -> Result<(), DurabilityError> {
        self.proof_sha256 = None;
        self.proof_sha256 = Some(digest(
            &serde_json::to_vec(self).map_err(|_| DurabilityError::Unavailable)?,
        ));
        Ok(())
    }
}

#[derive(Serialize)]
struct Receipt<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    command: &'a str,
    operation_id: &'a str,
    outcome: &'static str,
    result_code: &'a str,
    result_data: Option<&'a ResultData>,
    recovery_generation: String,
    decided_at: String,
}

pub fn encode_receipt(
    source: &str,
    command: &str,
    operation: &str,
    code: &str,
    data: Option<&ResultData>,
    recovery: u64,
    now: u64,
) -> Result<Vec<u8>, DurabilityError> {
    let raw = serde_json::to_vec(&Receipt {
        contract_version: 2,
        operation: RECEIPT_OPERATION,
        source_authority: source,
        command,
        operation_id: operation,
        outcome: if code == OK { "COMMITTED" } else { "REJECTED" },
        result_code: code,
        result_data: data,
        recovery_generation: recovery.to_string(),
        decided_at: now.to_string(),
    })
    .map_err(|_| DurabilityError::Unavailable)?;
    if raw.len() > 2048 {
        return Err(DurabilityError::Unavailable);
    }
    Ok(raw)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn account_deletion_wire_rejects_malformed_identity_and_noncanonical_bodies() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/account-deletion/closure.json"
        )))
        .unwrap();
        let raw = fixture["vectors"][0]["wire"].as_str().unwrap();
        let operation = "0192a000-0000-7000-8000-000000000030";
        assert!(decode(raw.as_bytes(), "CloseAccount", operation).is_ok());
        for body in [
            format!(" {raw}"),
            raw.replace("\"command\":\"CloseAccount\",", ""),
            raw.replace("\"command\":\"CloseAccount\"", "\"command\":null"),
            raw.replace("\"source_revision\":\"30\"", "\"source_revision\":\"030\""),
            raw.replace("1790000300", "1790000301"),
            raw.replace(
                "\"lifecycle_generation\":\"8\"",
                "\"lifecycle_generation\":\"18446744073709551616\"",
            ),
            raw.replace("\"account_id\":\"0192a000", "\"account_id\":\"0192A000"),
            raw.replace(
                "\"contract_version\":2",
                "\"extra\":true,\"contract_version\":2",
            ),
            raw.replace(
                "\"source_revision\":\"30\"",
                "\"source_revision\":\"30\",\"source_revision\":\"30\"",
            ),
        ] {
            assert_eq!(
                decode(body.as_bytes(), "CloseAccount", operation),
                Err(INTENT_INVALID),
                "{body}"
            );
        }
        assert_eq!(
            decode(
                raw.replace(
                    "\"command\":\"CloseAccount\"",
                    "\"command\":\"CreateCharacter\""
                )
                .as_bytes(),
                "CloseAccount",
                operation
            ),
            Err(OPERATION_CONFLICT)
        );
        assert_eq!(
            decode(
                raw.replace("\"contract_version\":2", "\"contract_version\":1")
                    .as_bytes(),
                "CloseAccount",
                operation
            ),
            Err(UNSUPPORTED)
        );
    }
}
