//! Real Fresh and activity owners for the native corpse component fixture.
//! Source observations are isolated producer inputs, as in the existing party
//! fixture; no activity receipt, participant or current claim is inserted here.
use super::{TestResult, fixture, id, readiness};
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterOutcome, ActivityEnterRequest, ActivityHandoffOwner,
    ActivityParticipant,
};
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::character_last_position::{
    LastPositionCell, LastPositionRecord, LastVitals,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::fresh_admission_composition::{
    FreshAdmissionComposition, FreshAdmissionSubject,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    SourceObservation,
};
use crate::foundation::admission_authority_publication::FreshAdmissionClaimTransitionV1;
use crate::foundation::fnd04_verifier::{
    FreshDurabilityCurrentAuthorityV1, FreshDurabilityTrustContext,
    verify_fresh_grant_durability_v1,
};
use crate::foundation::fresh_admission_durability::{
    FreshAdmissionCommitAuthorizationV1, FreshAdmissionCommitRequestV1,
    FreshAdmissionDurabilityFlowV1, FreshAdmissionDurabilityPortV1, FreshAdmissionDurableOutcomeV1,
    FreshAdmissionOperationV1, FreshAdmissionSubmissionV1,
};
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, GameSessionId, RuntimeScopeRefV1,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};

const SOURCE: &str = "native-corpse-platform-fixture";
const KEY: &str = "native-corpse-fixture";
const ISSUER: &str = "urn:oteryn:platform:game-admission";
const PROFILE: &str = "oteryn-pre-admission-v1";

fn uuid_text(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

async fn observe_source(
    h: &fixture::Harness,
    subject: &FreshAdmissionSubject,
    signing: &SigningKey,
    now: i64,
) -> TestResult {
    let descriptor = DescriptorRegistration {
        revision: 1,
        facts: vec![1],
        installed_at: now,
    };
    let provenance = FreshStoreProvenance {
        namespace: "native-corpse-fixture".into(),
        authorization: "independent-platform-fixture".into(),
        source_authority: SOURCE.into(),
        initialized_at: now,
    };
    assert!(
        h.root
            .record_native_source_descriptor_issuance(
                SOURCE,
                descriptor.clone(),
                Some(provenance.clone()),
            )
            .await?
    );
    h.root
        .initialize_native_admission_source(&h.node, provenance, descriptor)
        .await?;
    let account = serde_json::json!({
        "version": 1, "operation": "ReadAccountSecurityV1", "result": "observed",
        "source_authority": SOURCE, "source_revision": "1", "decision_identity": "1",
        "source_observed_at": now.to_string(), "clock_uncertainty_seconds": "0",
        "account_id": subject.account_id, "purpose": "platform_security",
        "scope": "fresh_admission", "allowed": true, "minimum_valid_generation": "1",
    });
    let trust = serde_json::json!({
        "version": 1, "operation": "ReadFreshSigningTrustV1", "result": "observed",
        "source_authority": SOURCE, "source_revision": "1", "decision_identity": "1",
        "source_observed_at": now.to_string(), "clock_uncertainty_seconds": "0",
        "issuer": ISSUER, "profile": PROFILE, "key_purpose": "fresh_admission", "key_id": KEY,
        "trusted": true, "public_key": URL_SAFE_NO_PAD.encode(signing.verifying_key().to_bytes()),
    });
    for (operation, subject, decision, body) in [
        (
            NativeSourceOperation::ReadAccountSecurityV1,
            NativeSourceSubject::account_security(subject.account_id.clone())?,
            "1",
            account,
        ),
        (
            NativeSourceOperation::ReadFreshSigningTrustV1,
            NativeSourceSubject::signing_trust(ISSUER, PROFILE, "fresh_admission", KEY)?,
            "1",
            trust,
        ),
    ] {
        h.root
            .accept_native_source_observation(
                &h.node,
                SourceObservation {
                    source_authority: SOURCE.into(),
                    operation,
                    subject,
                    source_revision: 1,
                    decision_identity: decision.into(),
                    observed_at: now,
                    semantic_facts: serde_json::to_vec(&body)?,
                },
            )
            .await?;
    }
    Ok(())
}

fn fresh_request(
    composition: &FreshAdmissionComposition,
    subject: &FreshAdmissionSubject,
    signing: &SigningKey,
    now: i64,
) -> TestResult<FreshAdmissionCommitRequestV1> {
    let header = serde_json::json!({"alg": "Ed25519", "kid": KEY, "typ": "oteryn-admission+jwt"});
    let payload = serde_json::json!({
        "iss": ISSUER, "aud": "urn:oteryn:game:admission", "iat": now,
        "nbf": now, "exp": now + 10, "jti": URL_SAFE_NO_PAD.encode([7; 32]),
        "profile": PROFILE, "purpose": "fresh_entry", "attempt_ref": uuid_text(&id(86)),
        "account_id": subject.account_id,
        "character_id": uuid_text(subject.character_id.as_bytes()),
        "world_id": uuid_text(subject.world_id.as_bytes()),
        "channel_id": uuid_text(subject.channel_id.as_bytes()),
        "account_security_generation": "1", "route_revision": "route-1",
        "runtime_observation_revision": "runtime-1", "scope_ownership_generation": "1",
        "protocol_major": 1, "transport_profile": 1, "ruleset_revision": "ruleset-1",
        "content_revision": "content-1", "map_revision": "map-1",
        "world_policy_revision": "world-policy-1", "offer_revision": "offer-1",
    });
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(payload.to_string()),
    );
    let token = format!(
        "{input}.{}",
        URL_SAFE_NO_PAD.encode(signing.sign(input.as_bytes()).to_bytes()),
    );
    let facts = verify_fresh_grant_durability_v1(
        &token,
        now,
        &FreshDurabilityTrustContext::from_owning_source(composition),
        &FreshDurabilityCurrentAuthorityV1::from_owning_source(composition),
    )
    .map_err(fixture::debug)?;
    let authorization = FreshAdmissionCommitAuthorizationV1::new(
        &facts,
        GameSessionId::decode(&id(fixture::SESSION)).map_err(fixture::debug)?,
        AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(fixture::debug)?,
        composition,
        now,
    )
    .map_err(fixture::debug)?;
    let transition = FreshAdmissionClaimTransitionV1::prepare(composition, &authorization, now)
        .map_err(fixture::debug)?;
    let mut flow =
        FreshAdmissionDurabilityFlowV1::begin(authorization, transition).map_err(fixture::debug)?;
    let mut capture = Capture(None);
    flow.submit(&mut capture).map_err(fixture::debug)?;
    capture.0.ok_or_else(|| "Fresh owner did not submit".into())
}

struct Capture(Option<FreshAdmissionCommitRequestV1>);
impl FreshAdmissionDurabilityPortV1 for Capture {
    fn submit(&mut self, request: &FreshAdmissionCommitRequestV1) -> FreshAdmissionSubmissionV1 {
        if self.0.is_some() {
            return FreshAdmissionSubmissionV1::Unavailable;
        }
        self.0 = Some(request.clone());
        FreshAdmissionSubmissionV1::Accepted
    }

    fn reconcile(&mut self, _: &FreshAdmissionOperationV1) -> FreshAdmissionSubmissionV1 {
        FreshAdmissionSubmissionV1::Unavailable
    }
}

pub(super) async fn enter(h: &fixture::Harness, scope: RuntimeScopeRefV1) -> TestResult {
    let source_scope = fixture::fence(1)?.runtime_scope;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = source_scope
    else {
        return Err("fixture origin is not a Channel".into());
    };
    let subject = FreshAdmissionSubject {
        account_id: uuid_text(&id(40)),
        character_id: CharacterId::decode(&id(fixture::CHARACTER)).map_err(fixture::debug)?,
        world_id,
        channel_id,
        signing_key_id: KEY.into(),
    };
    let signing = SigningKey::from_bytes(&[31; 32]);
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, source_scope, 1, 1).await?)
            .await
            .map_err(fixture::debug)?,
        GuardPublicationDisposition::Applied,
    );
    // The request clock must include the readiness observation even when its
    // publication crosses a database-clock second boundary.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    observe_source(h, &subject, &signing, now).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    assert_eq!(
        h.root
            .publish_fresh_admission_sources(&authority, &h.node, &subject, now)
            .await?,
        GuardPublicationDisposition::Applied,
    );
    let composition = h
        .root
        .compose_fresh_admission(&authority, &h.node, &subject)
        .await?;
    let request = fresh_request(&composition, &subject, &signing, now)?;
    let capabilities = std::sync::Arc::new(move |current, generation, policy: &str| {
        current == source_scope && generation == 1 && policy == "world-policy-1"
    });
    assert!(matches!(
        h.root
            .commit_composed_fresh_admission(
                &authority,
                &h.node,
                &composition,
                &request,
                capabilities
            )
            .await?,
        FreshAdmissionDurableOutcomeV1::Committed(_),
    ));
    let snapshot = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(GameSessionId::decode(&id(fixture::SESSION)).map_err(fixture::debug)?)
        .await?
        .0;
    let source = CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(*subject.character_id.as_bytes())
            .map_err(fixture::debug)?,
        game_session_id: snapshot.current_game_session_id(),
        connection_generation: snapshot.current_connection_generation(),
        character_lease_generation: snapshot.current_character_lease().generation(),
        runtime_scope: snapshot.current_runtime_scope(),
        scope_ownership_generation: snapshot.current_scope_generation(),
    };
    assert_eq!(source.runtime_scope, source_scope);
    let pin = ActivityContentPin {
        coordinate_frame_digest: [0x11; 32],
        map_revision_digest: [0x22; 32],
        server_artifact_digest: [0x33; 32],
    };
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest.as_slice()).bind([0x44_u8; 32].as_slice())
        .bind(pin.coordinate_frame_digest.as_slice()).execute(&h.pool).await?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let allocation = owner
        .ready_allocations(&authority, &h.node, world_id, None)
        .await?
        .into_iter()
        .find(|allocation| allocation.scope == scope)
        .ok_or("current Instance allocation missing")?;
    let position = LastPositionRecord {
        world_id,
        cell: LastPositionCell {
            x: 100,
            y: 101,
            floor: 7,
        },
        coordinate_frame_digest: pin.coordinate_frame_digest,
        map_revision_digest: pin.map_revision_digest,
        server_artifact_digest: pin.server_artifact_digest,
        vitals: LastVitals::new(83, 17),
        rest: Some([2, 300, 4, 5]),
    };
    let request = ActivityEnterRequest {
        handoff_id: id(87),
        command_id: 1,
        initiating_character_id: subject.character_id,
        activity_key: "oteryn:activity.owner_fixture".into(),
        template_revision: "fixture-template-1".into(),
        content_revision: "content-1".into(),
        destination_scope: scope,
        destination_generation: allocation.generation,
        destination_assignment_source_revision: allocation.source_revision,
        destination_content: pin,
        time_limit_ms: 60_000,
        participants: vec![ActivityParticipant {
            source,
            destination_game_session_id: GameSessionId::decode(&id(90)).map_err(fixture::debug)?,
            destination_transport: Some(
                AuthenticatedTransportRefV1::decode(&[90; 16]).map_err(fixture::debug)?,
            ),
            selected_capabilities: vec![117],
            source_position: position,
            source_position_write_sequence: 1,
            destination_cell: LastPositionCell {
                x: 200,
                y: 201,
                floor: 7,
            },
            exit_position: LastPositionRecord {
                cell: LastPositionCell {
                    x: 102,
                    y: 103,
                    floor: 7,
                },
                ..position
            },
        }],
    };
    let entered = match owner.enter(&authority, &h.node, request).await? {
        ActivityEnterOutcome::Entered(receipt) => receipt,
        other => return Err(format!("native corpse activity entry: {other:?}").into()),
    };
    assert_eq!(entered.handoff_id, id(87));
    assert_eq!(entered.destination_scope, scope);
    assert_eq!(entered.participants.len(), 1);
    let proof: (bool, bool, bool, bool) = sqlx::query_as("SELECT g.state=2,e.holder_node_id=encode($2,'hex')::uuid AND e.holder_registration_revision=$3::text::numeric,NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id),game_activity_session_origin_valid(p.destination_game_session_id,p.character_id) FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) JOIN game_activity_handoff_participants p USING(handoff_id) WHERE g.handoff_id=encode($1,'hex')::uuid")
        .bind(entered.handoff_id.as_slice()).bind(h.node.fact().node_id().as_bytes().as_slice())
        .bind(h.node.fact().registration_revision().to_string()).fetch_one(&h.pool).await?;
    assert_eq!(proof, (true, true, true, true));
    Ok(())
}
