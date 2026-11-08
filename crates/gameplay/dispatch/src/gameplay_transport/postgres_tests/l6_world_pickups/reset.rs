//! Reuse the production reset drain, activation and fresh-admission writers.
use super::super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, debug};
use crate::content::QualifiedNativeEntryRoom;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::admission_authority_publication::*;
use crate::gameplay_transport::connection::{ControllerBinding, SessionContinuity};
use std::time::Duration;

pub(crate) async fn cycle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    room: &QualifiedNativeEntryRoom,
) -> TestResult<AdmittedSession> {
    let world = owner.world_id;
    let channel = owner.channel_id;
    assert!(
        room.source_world().is_none(),
        "this is the actual Bundle collision path"
    );
    assert!(room.movement_cells().house_tiles().is_known_empty());
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(world.as_bytes().as_slice()).bind(room.compiled().server_digest().as_slice())
        .bind(room.movement_cells().scope().map_revision.as_str()).bind(room.compiled().client_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).fetch_one(&h.pool).await?);
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$2,$4,$5,'[]'::jsonb,1)"
        )
        .bind(world.as_bytes().as_slice())
        .bind(room.compiled().server_digest().as_slice())
        .bind(room.movement_cells().scope().map_revision.as_str())
        .bind(room.compiled().client_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice())
        .fetch_one(&h.pool)
        .await?
    );
    let reset = h
        .root
        .retiring_world_reset(world)
        .await?
        .ok_or("published retiring reset")?;
    eprintln!("POSTGRES_WORLD_RESET_STARTED l6_small_ruby_native_pickup");
    let shutdown = oteryn_foundation::CancellationToken::new();
    tokio::time::timeout(
        Duration::from_secs(65),
        crate::gameplay_transport::listener::control_loss_lifecycle(
            owner,
            crate::gameplay_transport::listener::LostControl {
                session: admitted,
                wait: Duration::ZERO,
                reset: Some(reset),
            },
            &shutdown,
        ),
    )
    .await?;
    let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
    assert_eq!(
        store
            .current_session(admitted.game_session_id)
            .await?
            .session_state(),
        crate::foundation::GameSessionState::Terminal
    );
    assert_eq!(
        h.root
            .finish_world_reset(authority, &h.node, &reset, room, room)
            .await
            .map_err(debug)?,
        None
    );
    super::super::planned_world_reset::reset_fixture::replace(h, world, channel, &h.node).await.map_err(|error| format!("replace assignment: {error}"))?;
    for _ in 0..2 {
        assert_eq!(
            h.root
                .finish_world_reset(authority, &h.node, &reset, room, room)
                .await
                .map_err(|error| format!("finish after reassignment: {error:?}"))?,
            Some(1)
        );
    }
    assert_eq!(h.root.active_world_reset_epoch(world).await?, Some(1));
    assert!(h.root.retiring_world_reset(world).await?.is_none());
    super::super::planned_world_reset::reset_fixture::publish_ready(h, 2).await.map_err(|error| format!("publish readiness: {error}"))?;

    // A new verified grant must acquire a new session and lease. Never resurrect
    // the retired controller or edit its membership/lease rows in the fixture.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::super::fresh_fixture::fresh::Source::new(now)?;
    source.current.scope_ownership_generation = 2;
    let guards = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
        h.root.clone(),
    );
    source
        .rows
        .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    for row in &mut source.rows {
        let previous = guards
            .load(std::slice::from_ref(&row.key))
            .await?
            .into_iter()
            .next()
            .flatten()
            .ok_or("prior admission evidence")?;
        if let AdmissionAuthorityGuardStateV1::Character {
            lease_generation,
            holder,
            ..
        } = &previous.state
        {
            assert!(
                holder.is_none(),
                "the production reset drain releases its character lease"
            );
            source.current.character_lease_generation = *lease_generation;
        }
        row.state = previous.state;
        row.publication_revision = previous.publication_revision + 1;
        if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
            // The independent fixture owner observes security again after the
            // real drain. A stale pre-reset observation is not fresh authority.
            let revision = security.provenance.source_revision + 1;
            let decision = format!("l6-native-pickup-platform-observation-{revision}");
            security.provenance.source_revision = revision;
            security.provenance.accepted_source_revision = revision;
            security.provenance.decision_identity = decision.clone();
            security.provenance.accepted_decision_identity = decision;
            security.provenance.source_observed_at = now;
            security.provenance.publication_revision = row.publication_revision;
        }
        row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: previous.publication_revision,
        };
        row.source.source_revision = previous.source.source_revision + 1;
        row.source.source_observed_at = now;
        row.source.decision_identity = "l6-native-pickup-fresh-after-reset".into();
    }
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(|error| format!("fresh evidence preparation: {error:?}"))?;
    guards.publish(&publication).await.map_err(|error| format!("fresh evidence publication: {error:?}"))?;
    // Include the actual ready Runtime publication in the independently resolved
    // input to the verifier, using its existing immutable values and revisions.
    let runtime_key = AdmissionAuthorityGuardKeyV1::Runtime(
        crate::foundation::RuntimeScopeRefV1::channel(world, channel),
    );
    let ready = guards
        .load(std::slice::from_ref(&runtime_key))
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or("reset readiness")?;
    source.rows.push(ready);
    let session = crate::foundation::GameSessionId::decode(&id(88)).map_err(debug)?;
    let transport =
        crate::foundation::AuthenticatedTransportRefV1::decode(&id(89)).map_err(debug)?;
    let request = source.request_with_nonce(session, transport, None, now, Some([88; 32])).map_err(|error| format!("fresh request: {error}"))?;
    assert!(matches!(
        store.commit(&request).await.map_err(|error| format!("fresh admission commit: {error:?}"))?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    let current = store.current_session_at(session).await?.0;
    let fence =
        crate::gameplay_transport::item_fence_of(current).ok_or("fresh reset Item fence")?;
    assert_eq!(fence.scope_ownership_generation.get(), 2);
    let mut next = admitted;
    next.game_session_id = session;
    next.runtime_actor = None;
    next.controller = Some(ControllerBinding {
        transport,
        account_id: admitted.controller.ok_or("source controller")?.account_id,
    });
    next.item_fence = Some(fence);
    next.continuity = SessionContinuity::FRESH;
    eprintln!("POSTGRES_WORLD_RESET_COMPLETED l6_small_ruby_native_pickup epoch=1");
    Ok(next)
}
