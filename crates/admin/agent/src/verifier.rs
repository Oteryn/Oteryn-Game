//! Pure verification over immutable policy and store snapshots; the driver claims
//! nonces atomically between freshness and parameter validation.
use crate::policy::{Policy, timestamp};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_admin_contracts::{ErrorCode as E, *};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct Evidence {
    pub nonce_seen: bool,
    pub prior: Option<(String, Response)>,
    /// Digest recomputed from current operation preconditions/effects, never supplied by the caller.
    pub preview_digest: Option<String>,
    pub precondition_error: Option<E>,
}
#[derive(Debug, Clone)]
pub enum Decision {
    Execute { params_digest: String },
    Recorded(Box<Response>),
}

/// Steps 1–3. No operation parameters are interpreted before authentication.
pub fn authenticate(
    r: &Request,
    p: &Policy,
    advertised: &BTreeSet<String>,
    now: i64,
) -> Result<Operation, E> {
    let op = Operation::parse(&r.operation)
        .filter(|_| {
            r.contract == CONTRACT && r.operation_version == 1 && advertised.contains(&r.operation)
        })
        .ok_or(E::AdmUnknownOperation)?;
    let key = p
        .signers
        .iter()
        .find(|s| s.key_id == r.signer_key_id && s.trusted_at(now))
        .and_then(|s| s.key())
        .ok_or(E::AdmSignerUntrusted)?;
    verify_signature(r, &r.signature, &key).map_err(|_| E::AdmBadSignature)?;
    let (issued, expires) = timestamp(&r.issued_at)
        .zip(timestamp(&r.expires_at))
        .ok_or(E::AdmExpired)?;
    let max_lifetime = if r.kind == Kind::Query {
        60
    } else if r.risk_class == RiskClass::R4 {
        1800
    } else {
        300
    };
    if issued > now.saturating_add(30)
        || now >= expires
        || expires <= issued
        || expires.saturating_sub(issued) > max_lifetime
    {
        return Err(E::AdmExpired);
    }
    Ok(op)
}
/// Steps 4–8, with first-failure precedence independent of executor behavior.
pub fn authorize(
    r: &Request,
    p: &Policy,
    op: Operation,
    state: &Evidence,
    now: i64,
) -> Result<String, E> {
    if state.nonce_seen {
        return Err(E::AdmReplay);
    }
    if URL_SAFE_NO_PAD
        .decode(&r.nonce)
        .map_or(true, |b| b.len() != 16)
    {
        return Err(E::AdmInvalidParams);
    }
    op.validate_params(&r.params)
        .map_err(|_| E::AdmInvalidParams)?;
    op.validate_target(&r.target)
        .map_err(|_| E::AdmInvalidParams)?;
    let uuid7 = |s: &str| {
        uuid::Uuid::parse_str(s).is_ok_and(|u| {
            u.get_version_num() == 7
                && u.get_variant() == uuid::Variant::RFC4122
                && u.to_string() == s
        })
    };
    if !uuid7(&r.request_id)
        || !uuid7(&r.admin_operation_id)
        || r.kind != Kind::Query
        || r.admin_operation_id != r.request_id
    {
        return Err(E::AdmInvalidParams);
    }
    let rule = p.operations.get(&r.operation).ok_or(E::AdmForbidden)?;
    if !rule.allowed_permissions.contains(&r.actor.permission)
        || r.target
            .world_id
            .as_ref()
            .is_some_and(|w| !rule.worlds.contains(w))
    {
        return Err(E::AdmForbidden);
    }
    let class = rule.min_risk_class.max(op.minimum());
    if r.risk_class < class {
        return Err(E::AdmRiskClassMismatch);
    }
    let issued = timestamp(&r.issued_at).ok_or(E::AdmExpired)?;
    if class >= RiskClass::R2
        && r.reason
            .as_ref()
            .is_none_or(|v| v.code.trim().is_empty() || v.text.trim().is_empty())
    {
        return Err(E::AdmControlMissing);
    }
    if class >= RiskClass::R3
        && r.actor
            .step_up_at
            .as_deref()
            .and_then(timestamp)
            .is_none_or(|step| step > issued || issued.saturating_sub(step) > 300)
    {
        return Err(E::AdmControlMissing);
    }
    if class == RiskClass::R4 {
        if !r.approvals.iter().any(|a| {
            a.admin_id != r.actor.admin_id
                && !a.admin_id.is_empty()
                && !a.approval_id.is_empty()
                && timestamp(&a.approved_at)
                    .is_some_and(|t| t <= issued && now.saturating_sub(t) < 1800)
        }) {
            return Err(E::AdmControlMissing);
        }
        if !r.dry_run
            && state.preview_digest.as_deref().is_none_or(|expected| {
                r.params
                    .get("preview_digest")
                    .and_then(serde_json::Value::as_str)
                    != Some(expected)
            })
        {
            return Err(E::AdmPreviewStale);
        }
    }
    if let Some(error) = state.precondition_error {
        return Err(error);
    }
    digest(&r.params).map_err(|_| E::AdmInvalidParams)
}
/// Step 9. Called only after operation controls and preconditions succeeded.
pub fn idempotency(params_digest: &str, prior: Option<&(String, Response)>) -> Result<Decision, E> {
    if let Some((prior_digest, response)) = prior {
        if prior_digest != params_digest {
            return Err(E::AdmIdempotencyConflict);
        }
        return Ok(Decision::Recorded(Box::new(response.clone())));
    }
    Ok(Decision::Execute {
        params_digest: params_digest.into(),
    })
}
pub fn verify(
    r: &Request,
    p: &Policy,
    advertised: &BTreeSet<String>,
    nonce_store: &Evidence,
    clock: i64,
) -> Result<Decision, E> {
    let op = authenticate(r, p, advertised, clock)?;
    let params_digest = authorize(r, p, op, nonce_store, clock)?;
    idempotency(&params_digest, nonce_store.prior.as_ref())
}
