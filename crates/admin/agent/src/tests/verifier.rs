use super::*;
use crate::{
    channel::advertised,
    verifier::{self, Decision, Evidence},
};
use ErrorCode as E;

#[test]
fn verifier_rejections_and_conformance_vectors() -> Result<(), WireError> {
    let base = request()?;
    let p = policy()?;
    let mut cases: Vec<(&str, Request, Policy, Evidence, Option<E>)> = Vec::new();
    cases.push(("valid", base.clone(), p.clone(), Evidence::default(), None));
    let mut rotation_policy = p.clone();
    let next_key = ed25519_dalek::SigningKey::from_bytes(&[8; 32]);
    let mut next_signer = p.signers[0].clone();
    next_signer.key_id = "platform-admin-signer-2".into();
    next_signer.public_key = URL_SAFE_NO_PAD.encode(next_key.verifying_key().to_bytes());
    rotation_policy.signers.push(next_signer);
    let mut rotated = base.clone();
    rotated.signer_key_id = "platform-admin-signer-2".into();
    rotated.signature = sign(&rotated, &next_key)?;
    cases.push((
        "rotation-next-key",
        rotated,
        rotation_policy,
        Evidence::default(),
        None,
    ));
    let mut resent = base.clone();
    resent.nonce = URL_SAFE_NO_PAD.encode([2; 16]);
    resign(&mut resent)?;
    cases.push((
        "idempotency-recorded",
        resent,
        p.clone(),
        Evidence {
            prior: Some((digest(&base.params)?, response(&base))),
            ..Evidence::default()
        },
        None,
    ));
    let mut add =
        |name, mut r: Request, policy: Policy, evidence, code, signed| -> Result<(), WireError> {
            if signed {
                resign(&mut r)?;
            }
            cases.push((name, r, policy, evidence, Some(code)));
            Ok(())
        };
    let mut r = base.clone();
    r.operation = "unknown".into();
    add(
        "unknown-operation",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmUnknownOperation,
        true,
    )?;
    let mut r = base.clone();
    r.contract = "other".into();
    add(
        "unknown-contract",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmUnknownOperation,
        true,
    )?;
    let mut r = base.clone();
    r.operation_version = 2;
    add(
        "unknown-version",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmUnknownOperation,
        true,
    )?;
    let mut r = base.clone();
    r.actor.admin_id = "tampered".into();
    add(
        "tampered-field",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmBadSignature,
        false,
    )?;
    let mut r = base.clone();
    r.signature = sign(&r, &ed25519_dalek::SigningKey::from_bytes(&[8; 32]))?;
    add(
        "wrong-key",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmBadSignature,
        false,
    )?;
    let mut q = p.clone();
    q.signers[0].not_after = "2026-11-05T12:00:00Z".into();
    add(
        "expired-key",
        base.clone(),
        q,
        Evidence::default(),
        E::AdmSignerUntrusted,
        true,
    )?;
    let mut r = base.clone();
    r.signer_key_id = "removed-key".into();
    add(
        "rotated-out-key",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmSignerUntrusted,
        true,
    )?;
    let mut r = base.clone();
    r.expires_at = "2026-11-05T12:00:00Z".into();
    add(
        "expired",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmExpired,
        true,
    )?;
    let mut r = base.clone();
    r.issued_at = "2026-11-05T12:00:31Z".into();
    add(
        "future-issued",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmExpired,
        true,
    )?;
    let mut r = base.clone();
    r.expires_at = "2026-11-05T12:01:01Z".into();
    add(
        "excessive-lifetime",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmExpired,
        true,
    )?;
    add(
        "replay",
        base.clone(),
        p.clone(),
        Evidence {
            nonce_seen: true,
            ..Evidence::default()
        },
        E::AdmReplay,
        true,
    )?;
    let mut r = base.clone();
    r.params = json!({"unknown":true});
    add(
        "closed-params",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmInvalidParams,
        true,
    )?;
    let mut r = base.clone();
    r.nonce = "AA".into();
    add(
        "invalid-nonce",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmInvalidParams,
        true,
    )?;
    let mut r = base.clone();
    r.actor.permission = "game.db.admin".into();
    add(
        "permission",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmForbidden,
        true,
    )?;
    let mut r = base.clone();
    r.target.world_id = Some("disabled".into());
    add(
        "disabled-world",
        r,
        p.clone(),
        Evidence::default(),
        E::AdmForbidden,
        true,
    )?;
    let mut q = p.clone();
    q.operations.clear();
    add(
        "disabled-operation",
        base.clone(),
        q,
        Evidence::default(),
        E::AdmForbidden,
        true,
    )?;
    let mut q = p.clone();
    q.operations
        .get_mut("admin.policy.get")
        .ok_or("missing rule")?
        .min_risk_class = RiskClass::R3;
    let mut r = base.clone();
    r.risk_class = RiskClass::R2;
    add(
        "risk-downgrade",
        r,
        q.clone(),
        Evidence::default(),
        E::AdmRiskClassMismatch,
        true,
    )?;
    let mut r = base.clone();
    r.risk_class = RiskClass::R3;
    add(
        "missing-reason",
        r.clone(),
        q.clone(),
        Evidence::default(),
        E::AdmControlMissing,
        true,
    )?;
    r.reason = Some(Reason {
        code: "SUPPORT".into(),
        text: "ticket".into(),
        ticket_ref: None,
    });
    add(
        "missing-step-up",
        r.clone(),
        q.clone(),
        Evidence::default(),
        E::AdmControlMissing,
        true,
    )?;
    r.actor.step_up_at = Some("2026-11-05T11:54:59Z".into());
    add(
        "stale-step-up",
        r.clone(),
        q.clone(),
        Evidence::default(),
        E::AdmControlMissing,
        true,
    )?;
    r.actor.step_up_at = Some("2026-11-05T12:00:01Z".into());
    add(
        "future-step-up",
        r.clone(),
        q.clone(),
        Evidence::default(),
        E::AdmControlMissing,
        true,
    )?;
    r.actor.step_up_at = Some("2026-11-05T11:59:00Z".into());
    r.risk_class = RiskClass::R4;
    q.operations
        .get_mut("admin.policy.get")
        .ok_or("missing rule")?
        .min_risk_class = RiskClass::R4;
    r.approvals = vec![Approval {
        admin_id: r.actor.admin_id.clone(),
        approved_at: "2026-11-05T11:59:00Z".into(),
        approval_id: "approval-1".into(),
    }];
    add(
        "self-approval",
        r.clone(),
        q.clone(),
        Evidence::default(),
        E::AdmControlMissing,
        true,
    )?;
    r.approvals[0].admin_id = "a-3".into();
    r.approvals[0].approved_at = "2026-11-05T11:30:00Z".into();
    add(
        "stale-approval",
        r.clone(),
        q.clone(),
        Evidence::default(),
        E::AdmControlMissing,
        true,
    )?;
    r.approvals[0].approved_at = "2026-11-05T11:59:00Z".into();
    r.params = json!({"preview_digest":"different-effect-digest"});
    add(
        "preview-mismatch",
        r,
        q,
        Evidence {
            preview_digest: Some("expected-effect-digest".into()),
            ..Evidence::default()
        },
        E::AdmPreviewStale,
        true,
    )?;
    add(
        "idempotency-conflict",
        base.clone(),
        p.clone(),
        Evidence {
            prior: Some(("different-digest".into(), response(&base))),
            ..Evidence::default()
        },
        E::AdmIdempotencyConflict,
        true,
    )?;
    for code in [
        E::AdmTargetNotFound,
        E::AdmPreconditionFailed,
        E::AdmTargetUnavailable,
        E::AdmBusy,
        E::AdmAgentSuperseded,
        E::AdmLimitExceeded,
        E::AdmInternal,
    ] {
        add(
            match code {
                E::AdmTargetNotFound => "target-not-found",
                E::AdmPreconditionFailed => "precondition",
                E::AdmTargetUnavailable => "unavailable",
                E::AdmBusy => "busy",
                E::AdmAgentSuperseded => "superseded",
                E::AdmLimitExceeded => "limit",
                _ => "internal",
            },
            base.clone(),
            p.clone(),
            Evidence {
                precondition_error: Some(code),
                ..Evidence::default()
            },
            code,
            true,
        )?;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/admin-control/v1/vectors");
    for (name, r, policy, evidence, expected) in cases {
        let decision = verifier::verify(&r, &policy, &advertised(), &evidence, NOW);
        assert_eq!(decision.as_ref().err().copied(), expected, "{name}");
        let vector = json!({"request":r,"canonical_signed_utf8":String::from_utf8(signed_bytes(&r)?)?,"expected_error":expected,"clock":NOW,
            "policy":policy.view(),"trusted_signers":policy.signers.iter().map(|s| json!({"key_id":s.key_id,"public_key":s.public_key,"not_before":s.not_before,"not_after":s.not_after})).collect::<Vec<_>>(),
            "nonce_seen":evidence.nonce_seen,"prior_params_digest":evidence.prior.as_ref().map(|(d,_)|d),"prior_result":evidence.prior.as_ref().map(|(_,r)|r),"preview_digest":evidence.preview_digest,"precondition_error":evidence.precondition_error});
        let text = serde_json::to_string_pretty(&vector)? + "\n";
        let path = root.join(format!("{name}.json"));
        if std::env::var_os("UPDATE_ADMIN_GOLDENS").is_some() {
            std::fs::write(&path, &text)?;
        }
        assert_eq!(std::fs::read_to_string(path)?, text);
    }
    let evidence = Evidence {
        prior: Some((digest(&base.params)?, response(&base))),
        ..Evidence::default()
    };
    assert!(matches!(
        verifier::verify(&base, &p, &advertised(), &evidence, NOW),
        Ok(Decision::Recorded(_))
    ));
    Ok(())
}
#[test]
fn precedence_rotation_and_policy_validation() -> Result<(), WireError> {
    let mut r = request()?;
    let mut p = policy()?;
    assert_eq!(crate::policy::timestamp(&r.issued_at), Some(NOW));
    r.signature = "invalid".into();
    r.expires_at = "invalid".into();
    assert_eq!(
        verifier::verify(
            &r,
            &p,
            &advertised(),
            &Evidence {
                nonce_seen: true,
                ..Evidence::default()
            },
            NOW
        )
        .err(),
        Some(E::AdmBadSignature)
    );
    p.signers.push(p.signers[0].clone());
    p.signers[1].key_id = "next-key".into();
    r = request()?;
    r.signer_key_id = "next-key".into();
    resign(&mut r)?;
    assert!(verifier::verify(&r, &p, &advertised(), &Evidence::default(), NOW).is_ok());
    assert!(Policy::parse(&(policy_text() + "\nunknown = true")).is_err());
    let text = policy_text();
    assert!(Policy::parse(&text.replace("R0", "R1")).is_ok());
    assert!(Policy::parse(&text.replace("game.ops.read", "game.db.admin")).is_err());
    assert!(crate::channel::execute("world.list", &p).is_err());
    Ok(())
}

#[test]
fn every_stage_preserves_first_failure_and_r4_preview_can_match() -> Result<(), WireError> {
    let mut r = request()?;
    let mut p = policy()?;
    r.operation = "unknown".into();
    r.signature = "bad".into();
    assert_eq!(
        verifier::verify(&r, &p, &advertised(), &Evidence::default(), NOW).err(),
        Some(E::AdmUnknownOperation)
    );
    r = request()?;
    r.params = json!({"unknown":true});
    r.actor.permission = "forbidden".into();
    resign(&mut r)?;
    assert_eq!(
        verifier::verify(
            &r,
            &p,
            &advertised(),
            &Evidence {
                nonce_seen: true,
                ..Evidence::default()
            },
            NOW
        )
        .err(),
        Some(E::AdmReplay)
    );
    assert_eq!(
        verifier::verify(&r, &p, &advertised(), &Evidence::default(), NOW).err(),
        Some(E::AdmInvalidParams)
    );
    r.params = json!({});
    resign(&mut r)?;
    p.operations
        .get_mut("admin.policy.get")
        .ok_or("missing rule")?
        .min_risk_class = RiskClass::R4;
    assert_eq!(
        verifier::verify(&r, &p, &advertised(), &Evidence::default(), NOW).err(),
        Some(E::AdmForbidden)
    );
    r.actor.permission = "game.ops.read".into();
    resign(&mut r)?;
    assert_eq!(
        verifier::verify(&r, &p, &advertised(), &Evidence::default(), NOW).err(),
        Some(E::AdmRiskClassMismatch)
    );
    r.risk_class = RiskClass::R4;
    resign(&mut r)?;
    let mut state = Evidence {
        precondition_error: Some(E::AdmPreconditionFailed),
        prior: Some(("different".into(), response(&r))),
        ..Evidence::default()
    };
    assert_eq!(
        verifier::verify(&r, &p, &advertised(), &state, NOW).err(),
        Some(E::AdmControlMissing)
    );
    r.reason = Some(Reason {
        code: "TEST".into(),
        text: "approved test".into(),
        ticket_ref: None,
    });
    r.actor.step_up_at = Some("2026-11-05T11:59:00Z".into());
    r.approvals = vec![Approval {
        admin_id: "a-3".into(),
        approved_at: "2026-11-05T11:59:00Z".into(),
        approval_id: "approval-1".into(),
    }];
    r.params = json!({"preview_digest":"matching-preview"});
    resign(&mut r)?;
    state.preview_digest = Some("matching-preview".into());
    assert_eq!(
        verifier::verify(&r, &p, &advertised(), &state, NOW).err(),
        Some(E::AdmPreconditionFailed)
    );
    state.precondition_error = None;
    assert_eq!(
        verifier::verify(&r, &p, &advertised(), &state, NOW).err(),
        Some(E::AdmIdempotencyConflict)
    );
    state.prior = None;
    assert!(verifier::verify(&r, &p, &advertised(), &state, NOW).is_ok());
    let mut value = serde_json::to_value(&r)?;
    value["actor"]["unexpected"] = json!(true);
    assert!(serde_json::from_value::<Request>(value).is_err());
    let mut invalid_uuid = request()?;
    invalid_uuid.request_id = "019a5620-0000-7000-0000-000000000001".into();
    invalid_uuid.admin_operation_id = invalid_uuid.request_id.clone();
    resign(&mut invalid_uuid)?;
    assert_eq!(
        verifier::verify(
            &invalid_uuid,
            &policy()?,
            &advertised(),
            &Evidence::default(),
            NOW
        )
        .err(),
        Some(E::AdmInvalidParams)
    );
    let text = policy_text();
    let signer = text.split("[operations").next().ok_or("no signer")?;
    assert!(Policy::parse(&format!("{signer}{signer}{signer}")).is_err());
    assert!(Policy::parse(&text.replace("2027-01-01T00:00:00Z", "2025-01-01T00:00:00Z")).is_err());
    Ok(())
}
