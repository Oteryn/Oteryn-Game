//! Real auto-attack dispatch and the registered PostgreSQL durability path.
use super::party_xp::{self, Case};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, actor_spell::TrainingSave};
use std::time::Duration;

#[path = "../../../../../../apps/game-server/tests/support/stamina_postgres_cases.rs"]
mod persistence;

#[test]
fn hunting_stamina_postgres_kill_experience_idle_checkpoint_and_reload() -> TestResult {
    party_xp::run(Case::Stamina)
}

async fn save_with_busy_readback(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    player: (ExactActorRef, GameSessionId),
    item: crate::durability::item_transfer::CurrentCharacterItemFence,
    gain: bool,
    expected: (u16, u16),
) -> TestResult {
    use crate::durability::character_build::BuildOccurrence;
    use crate::gameplay_transport::{AdmissionRetryBudget, hunting_stamina::STAMINA_READBACK_PAUSE};
    use std::sync::Arc;
    use tokio::sync::Notify;

    let (actor, session) = player;
    let receipts_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid",
    ).bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    // The existing root budget includes deterministic inspection while the save is paused;
    // every production read below still shares this one unchanged deadline.
    let budget = AdmissionRetryBudget::for_foreground_root(&session, owner.root)?;
    let deadline = budget.deadline;
    let pause = Arc::new((Notify::new(), Notify::new()));
    let saving = STAMINA_READBACK_PAUSE.scope(Arc::clone(&pause), async {
        if gain {
            owner.save_resting_stamina_with_budget(actor, session, &budget).await
        } else {
            owner.save_hunting_stamina_with_budget(actor, session, &budget).await
        }
    });
    tokio::pin!(saving);
    tokio::select! {
        () = pause.0.notified() => {}
        result = &mut saving => return Err(format!("stamina save ended before readback: {result:?}").into()),
    }
    assert!(owner.runtime.try_lock().is_ok());
    assert!(owner.states.spell_states.try_lock().is_ok());
    let receipt: (Vec<u8>, i32, i32) = sqlx::query_as(
        "SELECT uuid_send(build_occurrence_id),stamina_minutes_before,stamina_minutes_after FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid AND stamina_minutes_before<>stamina_minutes_after ORDER BY committed_character_revision DESC LIMIT 1",
    ).bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let pending = (
        BuildOccurrence::from_bytes(receipt.0.as_slice().try_into()?).map_err(debug)?,
        u16::try_from(receipt.1)?,
        u16::try_from(receipt.2)?,
    );
    assert_eq!((pending.1, pending.2), expected);
    let mut stale = item;
    stale.character_lease_generation = stale.character_lease_generation.checked_add(1)
        .ok_or("stale lease generation")?;
    assert_eq!(
        owner.stamina_readback_once(actor, session, stale, pending, gain).await.map_err(debug)?,
        TrainingSave::FencedOut,
        "readback must independently reject a stale durable fence",
    );
    let mut different = *pending.0.as_bytes();
    different[15] ^= 1;
    for changed in [
        (BuildOccurrence::from_bytes(different).map_err(debug)?, pending.1, pending.2),
        (pending.0, pending.1.checked_add(1).ok_or("changed before")?, pending.2),
        (pending.0, pending.1, pending.2.checked_add(1).ok_or("changed after")?),
    ] {
        assert_eq!(
            owner.stamina_readback_once(actor, session, item, changed, gain).await.map_err(debug)?,
            TrainingSave::Unknown,
            "readback must not acknowledge a different occurrence or either changed value",
        );
    }
    let held = owner.root.try_issue_semantic_pass()?;
    pause.1.notify_one();
    let polled = std::future::poll_fn(|cx| {
        std::task::Poll::Ready(std::future::Future::poll(saving.as_mut(), cx))
    }).await;
    assert!(polled.is_pending(), "post-write readback must wait for the busy holder");
    assert!(owner.runtime.try_lock().is_ok(), "retry waits outside the runtime lock");
    assert!(owner.states.spell_states.try_lock().is_ok(), "retry waits outside spell state");
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    assert_eq!(saving.await, TrainingSave::Saved);
    assert_eq!(budget.deadline, deadline, "readback must not renew the shared budget");
    let receipts_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid",
    ).bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(receipts_after, receipts_before + 1, "busy readback must never replay the write");
    assert_eq!(
        h.root.read_hunting_stamina(owner.character, item.character_id).await.map_err(debug)?,
        expected.1,
        "busy readback must not apply an extra debit or gain",
    );
    Ok(())
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session(session)
            .await?;
    let item = crate::gameplay_transport::item_fence_of(current).ok_or("item")?;
    // Pre-hunting legacy stamina establishes the baseline at its first hunting receipt.
    sqlx::query("UPDATE game_character_build_state SET stamina_minutes=840 WHERE character_id=encode($1,'hex')::uuid")
        .bind(item.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let minutes = h
        .root
        .read_hunting_stamina(owner.character, item.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(minutes, 840);
    let equipment =
        crate::gameplay_transport::equipment_move::read_current_equipment(owner, session)
            .await
            .ok_or("equipment")?;
    assert!(
        equipment.items.is_empty(),
        "the stamina fixture fights with bare fists"
    );
    let creature;
    {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .load_stamina(&runtime, actor, session, minutes);
        assert!(owner.states.attack.lock().await.set_modes(
            &runtime,
            actor,
            session,
            oteryn_protocol_oteryn::attack::FightModes {
                fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
            }
        ));
        let visible = runtime.visible_entities();
        let row = visible.creatures.first().ok_or("rat")?;
        creature = row.actor;
        assert_eq!(
            runtime
                .companion_snapshot(creature)
                .map_err(debug)?
                .state
                .policy
                .definition_key,
            "oteryn:creature.rat"
        );
        let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: creature.placement_identity(),
            generation: row.generation,
        };
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms().gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                Some(target),
                30
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
    }
    // Keep the content-owned rat alive between genuine swings; health is fixture setup,
    // while damage and hit qualification use production dispatch.
    // The content digest seeds swing damage and mitigation. Count three minutes between
    // landed hits, rather than assuming a fixed number of passes contains those hits.
    // This budget bounds setup/edge misses; neither the stop condition nor the elapsed
    // checks consult stamina. This exercises repeated damage without a kill.
    const MINUTE_MICROS: u64 = 60_000_000;
    let mut first_hit = None;
    let mut last_hit_at = None;
    let mut qualified_three_minutes = false;
    for pass in 0..128 {
        owner
            .runtime
            .lock()
            .await
            .set_test_creature_health(creature, 20)
            .map_err(debug)?;
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        if owner
            .runtime
            .lock()
            .await
            .live_creature_health(creature)
            .ok_or("living rat")?
            < 20
        {
            let now = owner.owner_now().get();
            let (first_pass, first_at) = *first_hit.get_or_insert((pass, now));
            if let Some(last) = last_hit_at {
                assert!(
                    now - last <= MINUTE_MICROS,
                    "real damage must keep hunting active"
                );
            }
            last_hit_at = Some(now);
            // Sixty 3-second clock advances prove at least three minutes between hits.
            if pass - first_pass >= 60 {
                assert!(
                    now - first_at < 4 * MINUTE_MICROS,
                    "the fixture must not hunt a fourth minute"
                );
                qualified_three_minutes = true;
                break;
            }
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
    }
    assert!(
        qualified_three_minutes,
        "three minutes of real monster damage within the pass budget"
    );
    let live = owner
        .states
        .spell_states
        .lock()
        .await
        .live_stamina(actor, session)
        .ok_or("live")?;
    assert_eq!(live, 840, "monster damage without kill XP never spends stamina");
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        840,
        "live until the checkpoint"
    );
    {
        let runtime = owner.runtime.lock().await;
        owner.states.attack.lock().await.set_target(
            &runtime,
            owner.rooms().gate_auto_attack(),
            actor,
            session,
            owner.owner_now(),
            None,
            31,
        );
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(3600))
        .ok_or("idle")?;
    owner.drain_auto_attacks().await;
    assert_eq!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .live_stamina(actor, session),
        Some(live),
        "idle owner passes spend nothing"
    );
    assert_eq!(owner.save_hunting_stamina(actor, session).await, TrainingSave::NotApplicable);
    assert_eq!(h.root.read_hunting_stamina(owner.character, item.character_id).await.map_err(debug)?, live,
        "idle checkpoint preserves durable stamina");
    // The XP writer consumes stamina for the kill before applying the penalty band.
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .set_test_creature_health(creature, 1)
            .map_err(debug)?;
        let row = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|row| row.actor == creature)
            .ok_or("living rat")?;
        let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: creature.placement_identity(),
            generation: row.generation,
        };
        owner.states.attack.lock().await.set_target(
            &runtime,
            owner.rooms().gate_auto_attack(),
            actor,
            session,
            owner.owner_now(),
            Some(target),
            32,
        );
    }
    for _ in 0..32 {
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        if owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(session)
        {
            break;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
    }
    let replay = {
        let attack = owner.states.attack.lock().await;
        let taken = match attack.kills().take(session, 2) {
            crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) => taken,
            _ => return Err("queued stamina kill".into()),
        };
        taken.entry().ok_or("stamina kill entry")?.clone()
    };
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    let xp:(i32,i64,i64) = sqlx::query_as("SELECT stamina_minutes,xp_awarded,raw_share FROM game_party_kill_xp_members WHERE character_id=encode($1,'hex')::uuid")
        .bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let live = 838;
    assert_eq!(xp.0, i32::from(live), "the first kill after a long gap spends two before XP modifiers");
    assert_eq!(owner.states.spell_states.lock().await.live_stamina(actor, session), Some(live));
    // Replaying the durable kill must not consume a second time.
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let replayed = owner.settle_party_experience(&permit, &replay).await.map_err(debug)?;
    drop(replayed);
    drop(permit);
    assert_eq!(owner.states.spell_states.lock().await.live_stamina(actor, session), Some(live));
    assert!(xp.2 > 0);
    assert_eq!(
        xp.1,
        xp.2 * 2 / 2,
        "level 1 doubles the share; the live stamina penalty halves it before checkpoint"
    );
    let corpses: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_mint_receipts \
         WHERE corpse_top_damage_character_id=encode($1,'hex')::uuid",
    )
    .bind(item.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(corpses >= 1, "the low-stamina kill still materialises its corpse");
    let corpse_loot: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_corpse_container_entries e \
         JOIN game_item_mint_receipts r ON r.item_instance_id=e.parent_item_instance_id \
         WHERE r.corpse_top_damage_character_id=encode($1,'hex')::uuid",
    )
    .bind(item.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        corpse_loot, 0,
        "the highest-damage owner in the final fourteen stamina hours receives no monster loot"
    );
    save_with_busy_readback(owner, h, (actor, session), item, false, (840, live)).await?;
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        live
    );
    assert_eq!(
        owner.save_hunting_stamina(actor, session).await,
        TrainingSave::NotApplicable
    );
    let receipt: (Vec<u8>,i32,i32) = sqlx::query_as("SELECT uuid_send(build_occurrence_id),stamina_minutes_before,stamina_minutes_after FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid AND stamina_minutes_before<>stamina_minutes_after ORDER BY committed_character_revision DESC LIMIT 1")
        .bind(item.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((receipt.1, receipt.2), (840, 838));
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&receipt.0);
    let occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(occurrence)
        .map_err(debug)?;
    let root = h
        .root
        .read_current_character(owner.character, item.character_id)
        .await
        .map_err(debug)?;
    let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
        character_id: item.character_id,
        game_session_id: session,
        connection_generation: item.connection_generation,
        character_lease_generation: item.character_lease_generation,
        runtime_scope: item.runtime_scope,
        scope_ownership_generation: item.scope_ownership_generation,
        expected_character_revision: root.revision,
    };
    let _slot = owner.revision_sequencer.acquire(item.character_id).await;
    assert!(matches!(
        h.root
            .commit_hunting_stamina(owner.character, owner.holder, fence, occurrence, (840, 838))
            .await
            .map_err(debug)?,
        crate::durability::character_build::BuildCommitOutcome::AlreadyCommitted(_)
    ));
    assert!(
        h.root
            .commit_hunting_stamina(owner.character, owner.holder, fence, occurrence, (840, 837))
            .await
            .is_err(),
        "conflicting occurrence cannot spend twice"
    );
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        live
    );
    drop(_slot);

    assert!(owner
        .states
        .spell_states
        .lock()
        .await
        .gain_resting_stamina(actor, session));
    let restored = live + 1;
    save_with_busy_readback(owner, h, (actor, session), item, true, (live, restored)).await?;
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        restored,
        "resting gain survives a canonical reload"
    );
    let gain_bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(build_occurrence_id) FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid AND cause='resting_stamina' ORDER BY committed_character_revision DESC LIMIT 1",
    )
    .bind(item.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let mut gain_id = [0; 16];
    gain_id.copy_from_slice(&gain_bytes);
    let gain_occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(gain_id)
        .map_err(debug)?;
    assert!(matches!(
        h.root
            .commit_resting_stamina(
                owner.character,
                owner.holder,
                fence,
                gain_occurrence,
                (live, restored),
            )
            .await
            .map_err(debug)?,
        crate::durability::character_build::BuildCommitOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        h.root
            .read_hunting_stamina(owner.character, item.character_id)
            .await
            .map_err(debug)?,
        restored,
        "replaying the retained gain does not apply it twice"
    );
    let root_after_gain = h
        .root
        .read_current_character(owner.character, item.character_id)
        .await
        .map_err(debug)?;
    let debit_fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
        expected_character_revision: root_after_gain.revision,
        ..fence
    };
    let debit_occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(
        crate::bestiary_postgres_harness::id(201),
    )
    .map_err(debug)?;
    assert!(matches!(
        h.root
            .commit_hunting_stamina(
                owner.character,
                owner.holder,
                debit_fence,
                debit_occurrence,
                (restored, restored - 1),
            )
            .await
            .map_err(debug)?,
        crate::durability::character_build::BuildCommitOutcome::Committed(_)
    ));
    let cap_occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(
        crate::bestiary_postgres_harness::id(202),
    )
    .map_err(debug)?;
    assert!(h
        .root
        .commit_resting_stamina(
            owner.character,
            owner.holder,
            debit_fence,
            cap_occurrence,
            (2520, 2521),
        )
        .await
        .is_err());

    let mut stale = fence;
    stale.character_lease_generation += 1;
    let occurrence = crate::durability::character_build::BuildOccurrence::from_bytes(
        crate::bestiary_postgres_harness::id(200),
    )
    .map_err(debug)?;
    assert!(matches!(
        h.root
            .commit_hunting_stamina(
                owner.character,
                owner.holder,
                stale,
                occurrence,
                (live, live - 1)
            )
            .await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)
    ));
    for sql in [
        "UPDATE game_character_build_state SET stamina_minutes=0 WHERE character_id=encode($1,'hex')::uuid",
        "SELECT game_hunting_set_stamina(encode($1,'hex')::uuid,encode($1,'hex')::uuid,0)",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        assert!(
            sqlx::query(sql)
                .bind(item.character_id.as_bytes().as_slice())
                .execute(&mut *tx)
                .await
                .is_err(),
            "runtime needs a same-transaction build receipt"
        );
        tx.rollback().await?;
    }
    Ok(())
}
