//! Ajax condition persistence uses the mature authenticated fresh-admission owner.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionId;
async fn fresh_request(
    h: &Harness,
) -> TestResult<(
    GameSessionId,
    crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
)> {
    let (character, account, successor) = (id(41), id(40), id(110));
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
    let template = super::super::super::fresh_fixture::fresh::Source::new(now)?;
    let current = crate::foundation::fnd04_verifier::FreshCurrentEvidence {
        account_id: crate::gameplay_transport::canonical_uuid(&account),
        character_id: crate::foundation::CharacterId::decode(&character).map_err(debug)?,
        ..template.current
    };
    let mut source =
        super::super::super::fresh_fixture::fresh::Source::with_current(now, current.clone())?;
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
        .ok_or("current source publications")?;
    let mut refresh =
        super::super::super::fresh_fixture::fresh::Source::with_current(now, current)?;
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
        row.source.decision_identity = format!("full-world-source-{}", row.source.source_revision);
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
        .ok_or("refreshed current publications")?;
    source.current.character_lease_generation = 2;
    let next = if character == id(41) {
        source.appearance_observer_request(
            &crate::gameplay_transport::canonical_uuid(&character),
            now,
        )?
    } else {
        source.request_with_nonce(
            GameSessionId::decode(&successor).map_err(debug)?,
            crate::foundation::AuthenticatedTransportRefV1::decode(&id(112)).map_err(debug)?,
            Some((
                &source.current.account_id,
                &crate::gameplay_transport::canonical_uuid(&character),
            )),
            now,
            Some([91; 32]),
        )?
    };
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&next).await?,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    // A replayed commit of the same request admits no third lease.
    let _ = store.commit(&next).await;
    let next_session = GameSessionId::decode(&successor).map_err(debug)?;
    assert_eq!(
        store
            .current_session(next_session)
            .await?
            .current_character_lease()
            .generation(),
        3
    );
    Ok((next_session, next))
}

async fn actor_state(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
) -> TestResult<crate::spell::cast::PlayerSpellState> {
    let runtime = owner.runtime.lock().await;
    Ok(owner
        .states
        .spell_states
        .lock()
        .await
        .get(&runtime, actor, session)
        .ok_or("actual source Fire actor")?
        .clone())
}
fn source_fire(
    state: &crate::spell::cast::PlayerSpellState,
) -> Option<&oteryn_gameplay_foundation::foundation::condition::ConditionInstance<String>> {
    state
        .owned_conditions()
        .instances()
        .iter()
        .find(|instance| instance.definition().key() == "npc.ajax.source_fire")
}
fn align_owner(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    at: u64,
) -> TestResult {
    if owner.owner_now().get() < at {
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_micros(at))
            .ok_or("actual source owner clock")?;
    }
    Ok(())
}
// Run directly after fresh Fire source commit and before the separate replay
// case's existing two-second reconnect fixture advance.
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    health_before: u32,
) -> TestResult {
    use crate::gameplay_transport::connection::{FirstEntryOutcome, LogoutReply};
    let actor = admitted.runtime_actor.ok_or("actual Fire actor")?;
    let first = actor_state(owner, actor, admitted.game_session_id).await?;
    let fire = source_fire(&first).ok_or("actual installed Fire")?;
    let born = fire.started_at();
    assert_eq!(
        first.health, health_before,
        "source delayed Fire changes no immediate HP"
    );
    assert_eq!(fire.remaining_total(), 100);
    owner
        .tick_spell_periodic_at(
            actor,
            admitted.game_session_id,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(born + 1_999_999),
        )
        .await;
    assert_eq!(
        actor_state(owner, actor, admitted.game_session_id)
            .await?
            .health,
        health_before,
        "actual periodic owner cannot deal Fire before two seconds"
    );
    // The existing periodic owner evaluates whole milliseconds. Sample at the
    // first representable instant after each exact microsecond deadline, as in
    // field_runes' mature periodic fixture; the source interval is unchanged.
    let sampled_born = born.div_ceil(1_000) * 1_000;
    let (mut raw_total, mut applied_total, mut regenerated_total) = (0_u32, 0_u32, 0_u32);
    let mut expected_health = health_before;
    for tick in 1_u32..=5 {
        let at = sampled_born + u64::from(tick) * 2_000_000;
        let ledger = fire_tick_ledger(owner, actor, admitted.game_session_id, at).await?;
        raw_total += ledger.raw;
        applied_total += ledger.applied;
        regenerated_total += ledger.regenerated;
        expected_health = ledger.health_after;
        assert!(
            owner
                .tick_spell_periodic_at(
                    actor,
                    admitted.game_session_id,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(at)
                )
                .await
                .is_some(),
            "actual native periodic owner publishes source Fire tick {tick}"
        );
        let state = actor_state(owner, actor, admitted.game_session_id).await?;
        assert_eq!(
            state.health, expected_health,
            "actual HP exactly composes mature regeneration and source damage"
        );
        assert_eq!(
            source_fire(&state).ok_or("unspent Fire")?.remaining_total(),
            100 - tick * 10
        );
    }
    align_owner(owner, born + 10_001_000)?;
    let old_fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "actual old fence")?
        .ok_or("old lease")?;
    let position = owner
        .read_admission_position(old_fence)
        .await
        .map_err(|_| "old position")?;
    assert!(owner.track_restored(admitted.game_session_id, actor, &position, old_fence));
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(actor, admitted.game_session_id)
            .is_err()
    );
    let saved = h
        .root
        .read_character_conditions(owner.character, old_fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("real logout saved conditions")?;
    let frozen=oteryn_gameplay_foundation::foundation::condition::ConditionStore::<String>::from_persistent_snapshot(&saved.conditions,0)
        .map_err(debug)?;
    let remaining = frozen
        .instances()
        .iter()
        .find(|instance| instance.definition().key() == "npc.ajax.source_fire")
        .ok_or("logout retains unspent Fire")?;
    assert_eq!(remaining.remaining_total(), 50);
    let next_due = saved
        .conditions
        .get("conditions")
        .and_then(serde_json::Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.pointer("/definition/key")
                    .and_then(serde_json::Value::as_str)
                    == Some("npc.ajax.source_fire")
            })
        })
        .and_then(|row| row.get("next_tick_at"))
        .and_then(serde_json::Value::as_u64)
        .ok_or("saved precise Fire deadline")?;
    // Advance the existing monotonic fixture clock by one offline minute (no
    // real minute sleep). Fire restoration rebases the stored remaining ticks;
    // unlike Mute, this owner never subtracts a wall-clock logout interval.
    align_owner(owner, born + 70_001_000)?;
    let (session, request) = fresh_request(h).await?;
    let new_actor = {
        let mut runtime = owner.runtime.lock().await;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        runtime.commit_fresh_session(reserved).map_err(debug)?
    };
    let (outcome, fence, _) = owner
        .initialize_first_entry(&request, session, request.binding().transport, new_actor)
        .await;
    assert_eq!(outcome, FirstEntryOutcome::Positioned);
    let fence = fence.ok_or("actual fresh item fence")?;
    assert!(fence.character_lease_generation > old_fence.character_lease_generation);
    assert_ne!(session, admitted.game_session_id);
    assert_ne!(new_actor, actor);
    let restored = actor_state(owner, new_actor, session).await?;
    assert_eq!(
        restored.health, expected_health,
        "actual login restores saved native HP"
    );
    let restored_fire = source_fire(&restored).ok_or("actual login restores remaining Fire")?;
    assert_eq!(
        restored_fire.remaining_total(),
        50,
        "offline time consumes no condition ticks"
    );
    let next = restored_fire
        .started_at()
        .checked_add(next_due)
        .ok_or("rebased source tick")?
        .div_ceil(1_000)
        * 1_000;
    for index in 0_u32..5 {
        let at = next + u64::from(index) * 2_000_000;
        let ledger = fire_tick_ledger(owner, new_actor, session, at).await?;
        raw_total += ledger.raw;
        applied_total += ledger.applied;
        regenerated_total += ledger.regenerated;
        expected_health = ledger.health_after;
        assert!(
            owner
                .tick_spell_periodic_at(
                    new_actor,
                    session,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
                        next + u64::from(index) * 2_000_000
                    )
                )
                .await
                .is_some()
        );
        assert_eq!(
            actor_state(owner, new_actor, session).await?.health,
            expected_health
        );
    }
    let final_state = actor_state(owner, new_actor, session).await?;
    assert_eq!(
        final_state.health,
        health_before + regenerated_total - applied_total,
        "all ten actual turns conserve source damage and independent regeneration"
    );
    assert_eq!(
        raw_total, 100,
        "exactly ten raw Fire10 occurrences are consumed"
    );
    assert_eq!(final_state.health, expected_health);
    assert!(source_fire(&final_state).is_none());
    align_owner(owner, next + 8_001_000)?;
    assert!(owner.save_conditions_at_actor_end(new_actor, session).await);
    let exhausted = h
        .root
        .read_character_conditions(owner.character, old_fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("actual final condition save")?;
    let exhausted_store=oteryn_gameplay_foundation::foundation::condition::ConditionStore::<String>::from_persistent_snapshot(&exhausted.conditions,0)
        .map_err(debug)?;
    assert!(
        !exhausted_store
            .instances()
            .iter()
            .any(|instance| instance.definition().key() == "npc.ajax.source_fire"),
        "normal actor-end save cannot resurrect the exhausted Fire episode"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_npc_source_actor_effects WHERE source_key='oteryn:npc-cosmetic/ajax/barbarian-fire'").fetch_one(&h.pool).await?,1);
    Ok(())
}

/// Independent mature regeneration and typed-hit phases form the expected HP
/// ledger; the real periodic owner must publish their composition exactly once.
struct FireTickLedger {
    health_after: u32,
    regenerated: u32,
    applied: u32,
    raw: u32,
}
async fn fire_tick_ledger(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    at: u64,
) -> TestResult<FireTickLedger> {
    use oteryn_gameplay_foundation::foundation::condition::{TickFacts, TickKind};
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    assert!(!states.has_pending_spell_commit(actor, session));
    let room = owner
        .rooms
        .gate_periodic_damage()
        .ok_or("actual periodic room")?;
    let position = runtime.read_actor_position(actor).map_err(debug)?;
    let _target = crate::durability::native_world_items::SpellGroundTarget::from_native_owner(
        room,
        &runtime,
        position.position(),
    )
    .map_err(debug)?;
    let pz = crate::gameplay_transport::actor_spell::qualified_protection_zone(
        &mut runtime,
        owner.movement_cells,
        actor,
    )
    .ok_or("actual periodic zone")?;
    assert!(!pz, "this controlled source lifecycle stands outside PZ");
    let state = states
        .get(&runtime, actor, session)
        .ok_or("actual source player")?;
    let mut due = state.clone();
    let ticks = due.source_take_due(
        at,
        TickFacts {
            in_protection_zone: pz,
            standing_on_field: None,
        },
    );
    assert_eq!(
        ticks.len(),
        1,
        "one source occurrence due at each native turn"
    );
    let tick = &ticks[0];
    assert_eq!(tick.provenance.definition_key, "npc.ajax.source_fire");
    assert!(matches!(
        tick.kind,
        TickKind::Damage {
            element: oteryn_gameplay_domain::domain::condition::DotElement::Fire,
            amount: 10,
            refused: false
        }
    ));
    let regenerated = due
        .health
        .checked_sub(state.health)
        .ok_or("regeneration cannot inflict damage")?;
    let (after, hit) = crate::spell::actor_conditions::stage_typed_hit_in_world(
        &runtime,
        actor,
        &due,
        10,
        crate::player_lethal::PlayerHitKind::from_dot(
            oteryn_gameplay_domain::domain::condition::DotElement::Fire,
        ),
        at,
    )
    .map_err(debug)?;
    // Pinned Creature::mitigateDamage truncates the compound result even when
    // ConditionDamage disabled armor/defense checks; absent facts preserve10.
    let expected_damage = state.player_mitigation_percent(at).map_or(10, |percent| {
        (10_f64 - f64::from(10_f32 * percent) / 100.0)
            .max(0.0)
            .trunc() as u32
    });
    assert_eq!(
        hit.applied, expected_damage,
        "current native mitigation follows the pinned condition path"
    );
    assert!(hit.applied > 0 && hit.applied <= 10);
    assert_eq!(after.health, due.health - hit.applied);
    Ok(FireTickLedger {
        health_after: after.health,
        regenerated,
        applied: hit.applied,
        raw: 10,
    })
}
