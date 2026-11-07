//! Fresh-login recovery of a native occurrence after an uncertain wire command.
//! Reuses the terminal release and fresh publication path from full_world_e2e.
use super::super::fixture::{Harness, debug, id};
use super::super::{Dependencies, TestResult};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{GameSessionId, GameSessionState};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, LogoutReply, SessionContinuity,
};
use crate::gameplay_transport::last_position::AdmissionPosition;
use oteryn_protocol_oteryn::logout::LogoutResult;

pub(super) async fn fresh_login(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let old_lease = retire_session(h, owner, admitted).await?;
    admit_successor(h, owner, deps, admitted, old_lease, 110).await
}

/// Finish the old owner before a fixture boots a different Content generation.
pub(super) async fn retire_session(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<u64> {
    let previous = *admitted;
    let previous_actor = previous.runtime_actor.ok_or("old Forge actor")?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let old = store.current_session(previous.game_session_id).await?;
    let old_lease = old.current_character_lease().generation();
    assert_eq!(old.session_state(), GameSessionState::Active);
    assert_eq!(
        owner.logout_session(previous).await,
        LogoutReply::Result(LogoutResult::Accepted),
        "fresh-login recovery first durably terminates the old session"
    );
    assert_eq!(
        store
            .current_session(previous.game_session_id)
            .await?
            .session_state(),
        GameSessionState::Terminal
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(previous_actor, previous.game_session_id)
            .is_err(),
        "the terminal release retires the old actor; no pending stream is reopened"
    );
    Ok(old_lease)
}

/// Admit through the supplied current owner; the old session is already terminal.
pub(super) async fn admit_successor(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: &mut AdmittedSession,
    old_lease: u64,
    session_tag: u8,
) -> TestResult {
    let previous = *admitted;
    let previous_controller = previous.controller.ok_or("old Forge controller")?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let old = store.current_session(previous.game_session_id).await?;
    assert_eq!(old.session_state(), GameSessionState::Terminal);
    let session = publish_successor(h, old_lease, session_tag).await?;
    assert_ne!(session, previous.game_session_id);
    let current = store.current_session(session).await?;
    assert_eq!(current.session_state(), GameSessionState::Active);
    assert_eq!(
        current.current_character_lease().generation(),
        old_lease + 1
    );
    assert_eq!(current.commit().character_id(), old.commit().character_id());
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fresh Forge fence")?;
    let transport = current.current_transport().ok_or("fresh Forge transport")?;
    assert_ne!(transport, previous_controller.transport);
    assert_eq!(fence.connection_generation.get(), 1);
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        actor
    };
    let gameplay = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fresh Forge gameplay fence")?
        .ok_or("fresh Forge active session")?;
    let position = owner
        .read_admission_position(gameplay)
        .await
        .map_err(|()| "fresh Forge admission position")?;
    assert!(matches!(position, AdmissionPosition::Stored(_)));
    {
        let mut runtime = owner.runtime.lock().await;
        owner
            .initialize_admitted_position(&mut runtime, actor, session, &position)
            .await
            .map_err(debug)?;
    }
    assert!(owner.track_restored(session, actor, &position, gameplay));
    deps.initialize_player(owner, actor, session).await;
    let mut successor = AdmittedSession {
        game_session_id: session,
        world_id: previous.world_id,
        channel_id: previous.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport,
            account_id: previous_controller.account_id,
        }),
        continuity: SessionContinuity {
            selected_capabilities: previous.continuity.selected_capabilities,
            ..SessionContinuity::FRESH
        },
        item_fence: Some(fence),
    };
    owner.admit_loop_training(&mut successor).await;
    assert_eq!(successor.first_entry, FirstEntryOutcome::Positioned);
    assert_eq!(successor.continuity.next_command_id, 1);
    assert_eq!(successor.continuity.server_sequence, 0);
    *admitted = successor;
    Ok(())
}

/// Refresh current account/signing publications, then commit the existing fixture's
/// successor session. This neither edits admission rows nor clears command ingress.
async fn publish_successor(
    h: &Harness,
    previous_lease: u64,
    session_tag: u8,
) -> TestResult<GameSessionId> {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1 as Key, AdmissionAuthorityGuardStateV1 as State,
        AdmissionAuthorityPublicationV1, AdmissionPublicationPreconditionV1,
    };
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let mut source = super::super::fresh_fixture::fresh::Source::new(now)?;
    let keys = source
        .rows
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("current Forge admission publications")?;
    let mut refresh = super::super::fresh_fixture::fresh::Source::new(now)?;
    refresh.rows = source
        .rows
        .iter()
        .filter(|row| matches!(row.key, Key::Account { .. } | Key::SigningTrust { .. }))
        .cloned()
        .collect();
    for row in &mut refresh.rows {
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: row.publication_revision,
        };
        row.publication_revision += 1;
        row.source.source_revision += 1;
        row.source.decision_identity = format!("forge-source-{}", row.source.source_revision);
        row.source.source_observed_at = now;
        if let State::Account { security, .. } = &mut row.state {
            security.provenance.publication_revision = row.publication_revision;
            security.provenance.source_revision = row.source.source_revision;
            security.provenance.accepted_source_revision = row.source.source_revision;
            security.provenance.decision_identity = row.source.decision_identity.clone();
            security.provenance.accepted_decision_identity = row.source.decision_identity.clone();
            security.provenance.source_observed_at = now;
        }
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&refresh, now).map_err(debug)?;
    guards.publish(&publication).await.map_err(debug)?;
    source.rows = guards
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("refreshed Forge admission publications")?;
    source.current.character_lease_generation = previous_lease;
    let next = if session_tag == 110 {
        source
            .appearance_observer_request(&crate::gameplay_transport::canonical_uuid(&id(41)), now)?
    } else {
        assert_eq!(
            session_tag, 111,
            "only the restored-content successor is additional"
        );
        restored_request(&source, now)?
    };
    assert!(matches!(
        FreshAdmissionStore::from_root(h.root.clone())
            .commit(&next)
            .await?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    GameSessionId::decode(&id(session_tag)).map_err(debug)
}

/// Bounded adaptation of monk_save_fresh_fixture.rs:342–421: a second fresh
/// login needs a distinct nonce, attempt, session and transport. Keep the same
/// fixture signing key and real verifier/authorization/claim/commit flow; never
/// reconstruct authority from a stored receipt or edit current admission rows.
fn restored_request(
    source: &super::super::fresh_fixture::fresh::Source,
    now: i64,
) -> TestResult<crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1> {
    use crate::foundation::admission_authority_publication::FreshAdmissionClaimTransitionV1;
    use crate::foundation::fnd04_verifier::{
        FreshDurabilityCurrentAuthorityV1, FreshDurabilityTrustContext,
        verify_fresh_grant_durability_v1,
    };
    use crate::foundation::fresh_admission_durability::{
        FreshAdmissionCommitAuthorizationV1, FreshAdmissionCommitRequestV1,
        FreshAdmissionDurabilityFlowV1, FreshAdmissionDurabilityPortV1, FreshAdmissionOperationV1,
        FreshAdmissionSubmissionV1,
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ed25519_dalek::{Signer, SigningKey};
    let current = &source.current;
    let payload = serde_json::json!({
        "iss": "urn:oteryn:platform:game-admission", "aud": "urn:oteryn:game:admission",
        "iat": now, "nbf": now, "exp": now + 10,
        "jti": URL_SAFE_NO_PAD.encode([111; 32]),
        "profile": "oteryn-pre-admission-v1", "purpose": "fresh_entry",
        "attempt_ref": crate::gameplay_transport::canonical_uuid(&id(111)),
        "account_id": current.account_id,
        "character_id": crate::gameplay_transport::canonical_uuid(current.character_id.as_bytes()),
        "world_id": crate::gameplay_transport::canonical_uuid(current.world_id.as_bytes()),
        "channel_id": crate::gameplay_transport::canonical_uuid(current.channel_id.as_bytes()),
        "account_security_generation": "1", "route_revision": current.route_revision,
        "runtime_observation_revision": current.runtime_observation_revision,
        "scope_ownership_generation": current.scope_ownership_generation.to_string(),
        "protocol_major": 1, "transport_profile": 1,
        "ruleset_revision": current.ruleset_revision, "content_revision": current.content_revision,
        "map_revision": current.map_revision, "world_policy_revision": current.world_policy_revision,
        "offer_revision": current.offer_revision,
    });
    let header = r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#;
    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload.to_string())
    );
    let key = SigningKey::from_bytes(&[31; 32]);
    let token = format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(key.sign(signing_input.as_bytes()).to_bytes())
    );
    let facts = verify_fresh_grant_durability_v1(
        &token,
        now,
        &FreshDurabilityTrustContext::from_owning_source(source),
        &FreshDurabilityCurrentAuthorityV1::from_owning_source(source),
    )
    .map_err(debug)?;
    let authorization = FreshAdmissionCommitAuthorizationV1::new(
        &facts,
        GameSessionId::decode(&id(111)).map_err(debug)?,
        crate::foundation::AuthenticatedTransportRefV1::decode(&[111; 16]).map_err(debug)?,
        source,
        now,
    )
    .map_err(debug)?;
    let transition =
        FreshAdmissionClaimTransitionV1::prepare(source, &authorization, now).map_err(debug)?;
    let mut flow =
        FreshAdmissionDurabilityFlowV1::begin(authorization, transition).map_err(debug)?;
    struct Capture(Option<FreshAdmissionCommitRequestV1>);
    impl FreshAdmissionDurabilityPortV1 for Capture {
        fn submit(
            &mut self,
            request: &FreshAdmissionCommitRequestV1,
        ) -> FreshAdmissionSubmissionV1 {
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
    let mut capture = Capture(None);
    flow.submit(&mut capture).map_err(debug)?;
    capture
        .0
        .ok_or_else(|| "restored Forge fresh admission did not submit".into())
}
