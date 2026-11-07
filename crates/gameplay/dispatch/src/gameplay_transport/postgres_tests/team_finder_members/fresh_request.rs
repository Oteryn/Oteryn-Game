//! Test-only successor grant data for the existing Fresh fixture authority.
//! Reuses the verifier, claim transition, flow and capture port; adds no admission path.
use super::super::fresh_fixture::fresh::Source;
use crate::bestiary_postgres_harness::{TestResult, debug, id};
use crate::foundation::{
    AuthenticatedTransportRefV1, GameSessionId, PRE_ADMISSION_PROFILE,
    admission_authority_publication::FreshAdmissionClaimTransitionV1,
    fnd04_verifier::{
        FreshDurabilityCurrentAuthorityV1, FreshDurabilityTrustContext,
        verify_fresh_grant_durability_v1,
    },
    fresh_admission_durability::{
        FreshAdmissionCommitAuthorizationV1, FreshAdmissionCommitRequestV1,
        FreshAdmissionDurabilityFlowV1,
    },
};
use crate::gameplay_transport::{PreparedRequest, canonical_uuid};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};

pub(super) fn prepare(source: &Source, now: i64) -> TestResult<FreshAdmissionCommitRequestV1> {
    // Same explicitly public fixture key, nonce, attempt and successor identities
    // as monk_save_fresh_fixture and its appearance_observer_request extension.
    // The World policy comes from the loaded Runtime publication in relog.rs;
    // the observer helper's fixed policy-1 token cannot represent this fixture.
    let current = &source.current;
    let payload = serde_json::json!({
        "iss": "urn:oteryn:platform:game-admission",
        "aud": "urn:oteryn:game:admission",
        "iat": now, "nbf": now, "exp": now.checked_add(10).ok_or("fixture grant deadline")?,
        "jti": URL_SAFE_NO_PAD.encode([8; 32]),
        "profile": PRE_ADMISSION_PROFILE,
        "purpose": "fresh_entry",
        "attempt_ref": "00000000-0000-7000-8000-000000000002",
        "account_id": current.account_id,
        "character_id": canonical_uuid(current.character_id.as_bytes()),
        "world_id": canonical_uuid(current.world_id.as_bytes()),
        "channel_id": canonical_uuid(current.channel_id.as_bytes()),
        "account_security_generation": "1",
        "route_revision": current.route_revision,
        "runtime_observation_revision": current.runtime_observation_revision,
        "scope_ownership_generation": current.scope_ownership_generation.to_string(),
        "protocol_major": 1, "transport_profile": 1,
        "ruleset_revision": current.ruleset_revision,
        "content_revision": current.content_revision,
        "map_revision": current.map_revision,
        "world_policy_revision": current.world_policy_revision,
        "offer_revision": current.offer_revision,
    });
    let header = br#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#;
    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload)?),
    );
    let signature = SigningKey::from_bytes(&[31; 32]).sign(signing_input.as_bytes());
    let token = format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    );
    let facts = verify_fresh_grant_durability_v1(
        &token,
        now,
        &FreshDurabilityTrustContext::from_owning_source(source),
        &FreshDurabilityCurrentAuthorityV1::from_owning_source(source),
    )
    .map_err(|error| format!("relog verify current grant: {error:?}"))?;
    let authorization = FreshAdmissionCommitAuthorizationV1::new(
        &facts,
        GameSessionId::decode(&id(110)).map_err(debug)?,
        AuthenticatedTransportRefV1::decode(&[8; 16]).map_err(debug)?,
        source,
        now,
    )
    .map_err(|error| format!("relog authorize current grant: {error:?}"))?;
    let transition = FreshAdmissionClaimTransitionV1::prepare(source, &authorization, now)
        .map_err(|error| format!("relog prepare current claims: {error:?}"))?;
    let mut flow =
        FreshAdmissionDurabilityFlowV1::begin(authorization, transition).map_err(debug)?;
    let mut prepared = PreparedRequest(None);
    flow.submit(&mut prepared).map_err(debug)?;
    prepared
        .0
        .ok_or_else(|| "relog fixture did not prepare a Fresh request".into())
}
