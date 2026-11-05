use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::VerifyingKey;
use oteryn_admin_contracts::{Operation, PolicyOperation, PolicyView, RiskClass, WireError};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub fn timestamp(value: &str) -> Option<i64> {
    OffsetDateTime::parse(value, &Rfc3339)
        .ok()
        .map(|t| t.unix_timestamp())
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signer {
    pub key_id: String,
    pub public_key: String,
    pub not_before: String,
    pub not_after: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationPolicy {
    pub min_risk_class: RiskClass,
    pub allowed_permissions: BTreeSet<String>,
    /// Empty means global operations only; world-scoped calls require explicit enablement.
    pub worlds: BTreeSet<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub signers: Vec<Signer>,
    pub operations: BTreeMap<String, OperationPolicy>,
}
impl Policy {
    pub fn parse(text: &str) -> Result<Self, WireError> {
        let policy: Self = toml::from_str(text)?;
        if policy.signers.is_empty() || policy.signers.len() > 2 {
            return Err("one or two signers required".into());
        }
        let mut ids = BTreeSet::new();
        for s in &policy.signers {
            if s.key_id.is_empty()
                || !ids.insert(&s.key_id)
                || s.key().is_none()
                || timestamp(&s.not_before)
                    .zip(timestamp(&s.not_after))
                    .is_none_or(|(a, b)| a >= b)
            {
                return Err("invalid signer roster".into());
            }
        }
        for (name, rule) in &policy.operations {
            let op = Operation::parse(name).ok_or("unknown policy operation")?;
            if rule.min_risk_class < op.minimum()
                || rule.allowed_permissions.is_empty()
                || rule
                    .allowed_permissions
                    .iter()
                    .any(|p| p != op.permission())
            {
                return Err(
                    "policy cannot lower catalog risk or grant unrelated permissions".into(),
                );
            }
        }
        Ok(policy)
    }
    pub fn view(&self) -> PolicyView {
        PolicyView {
            operations: self
                .operations
                .iter()
                .map(|(operation, r)| PolicyOperation {
                    operation: operation.clone(),
                    min_risk_class: r.min_risk_class,
                    allowed_permissions: r.allowed_permissions.iter().cloned().collect(),
                    worlds: r.worlds.iter().cloned().collect(),
                })
                .collect(),
        }
    }
}
impl Signer {
    pub fn key(&self) -> Option<VerifyingKey> {
        let bytes: [u8; 32] = URL_SAFE_NO_PAD
            .decode(&self.public_key)
            .ok()?
            .try_into()
            .ok()?;
        VerifyingKey::from_bytes(&bytes).ok()
    }
    pub fn trusted_at(&self, now: i64) -> bool {
        timestamp(&self.not_before)
            .zip(timestamp(&self.not_after))
            .is_some_and(|(a, b)| a <= now && now < b)
    }
}
