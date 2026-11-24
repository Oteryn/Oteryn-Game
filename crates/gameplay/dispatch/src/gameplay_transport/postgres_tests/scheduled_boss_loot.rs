//! W05d reaches the native kill drain and the durable personal reward owner.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::{self, scenario_thread};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_foundation::scheduled_events::{Schedule, current_rates, test_clock};
use std::time::Instant;

// The event covers the native owner's wall-clock death stamp. Advance the schedule clock
// past its end before draining, without changing the timestamp stored by lethal capture.
const EVENT_START: i64 = 946_684_800; // 2000-01-01T00:00:00Z
const AFTER_EVENT: i64 = 4_102_444_800; // 2100-01-01T00:00:00Z

#[test]
fn scheduled_boss_loot_rate_uses_frozen_death_time() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = Instant::now();
    scenario_thread::run_async(async |admin| {
        let schedule = Schedule::parse(
            r#"{"events":[{"name":"boss-rate","startdate":"01/01/2000","enddate":"12/31/2099",
                "ingame":{"bosslootrate":0}}]}"#,
        )
        .map_err(debug)?;
        let harness =
            Harness::create_with_admission_seed(admin, "scheduled_boss_loot", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_reward_boss_fixture(
                world, None,
            );
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        owner.movement_cells = deps.room.movement_cells().into();
        let session = deps.admitted.game_session_id;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(session)
        .await?
        .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        let attacker = CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(world.as_bytes().as_slice())
            .bind(deps.admitted.channel_id.as_bytes().as_slice())
            .execute(&harness.pool)
            .await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(world.as_bytes().as_slice())
            .bind(deps.admitted.channel_id.as_bytes().as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(frame_digest.as_slice())
            .execute(&harness.pool)
            .await?;
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| {
                policy.definition_key == "oteryn:creature.rat" && policy.reward_boss == Some(true)
            })
            .ok_or("pinned reward boss")?;
        let now = owner.owner_now();
        let batch = {
            let mut runtime = owner.runtime.lock().await;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    CharacterLease::new(attacker, fence.character_lease_generation)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
            let pin = runtime.content_pin().clone();
            let boss = runtime
                .realize_native_qualification_spawn(
                    pin,
                    &policy.definition_key,
                    MovementLocalPosition {
                        x: 3,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?
                .actor;
            OwnerCombatBatch {
                caster: deps.actor,
                attacker,
                current_lease_generation: fence.character_lease_generation,
                command: CommandRef::new(session, CommandId::new(91).map_err(debug)?),
                occurrence: SpellOccurrenceBinding {
                    id: "scheduled-boss-loot".into(),
                    revisions: ["r1"; 5].map(str::to_owned),
                },
                binding: b"scheduled-boss-loot".to_vec(),
                anchor: None,
                now_us: Some(now),
                now_ms: now.get() / 1_000,
                effects: vec![OwnerCombatEffect {
                    target: boss,
                    sub_ordinal: 0,
                    change: OwnerCombatChange::Damage {
                        target_atom: runtime.creature_spell_target_atom(boss).map_err(debug)?,
                        magnitude: policy.maximum_health,
                    },
                }],
                deferred: None,
            }
        };
        let mut first = None;
        for (replay, settlement_second) in [(false, AFTER_EVENT), (true, AFTER_EVENT + 1)] {
            let clock = test_clock::set(
                schedule.clone(),
                if replay {
                    settlement_second
                } else {
                    AFTER_EVENT - 1
                },
            );
            {
                let mut runtime = owner.runtime.lock().await;
                let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
                let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
                assert_eq!(receipt.applied, !replay);
                let attack = owner.states.attack.try_lock().map_err(debug)?;
                record_batch_kills(
                    &mut runtime,
                    KillSink {
                        prey: None,
                        activity: &[],
                        queue: attack.kills(),
                        rewards: native.reward_table(),
                        charms: None,
                    },
                    &receipt,
                    batch.now_ms,
                );
                assert!(attack.kills().has_session(session));
            }
            drop(clock);
            let _settlement_clock = test_clock::set(schedule.clone(), settlement_second);
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            let settlement: (Vec<u8>, i64, Vec<u8>, i64, i64) = sqlx::query_as(
                "SELECT uuid_send(death_id), minted_at, intent,
                        (SELECT count(*) FROM game_boss_reward_contributors WHERE eligible),
                        (SELECT count(*) FROM game_boss_reward_items)
                 FROM game_boss_reward_deaths",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert!((EVENT_START..AFTER_EVENT).contains(&(settlement.1 / 1_000)));
            assert_eq!(
                settlement.3, 1,
                "the personal reward owner settles the contributor"
            );
            assert_eq!(
                settlement.4, 0,
                "the zero rate active at lethal capture suppresses personal loot"
            );
            if replay {
                assert_eq!(first.as_ref(), Some(&settlement));
            } else {
                first = Some(settlement);
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })?;
    eprintln!(
        "SCHEDULED_BOSS_LOOT_POSTGRES_CASE_EXECUTED frozen_death_time elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn scheduled_boss_loot_rate_leaves_ordinary_loot_alone() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = Instant::now();
    scenario_thread::run("scheduled-boss-loot-ordinary", || {
        let schedule = Schedule::parse(
            r#"{"events":[{"name":"two-rates","startdate":"01/01/2000","enddate":"12/31/2099",
                "ingame":{"bosslootrate":0,"lootrate":200}}]}"#,
        )
        .map_err(debug)?;
        let _clock = test_clock::set(schedule, AFTER_EVENT - 1);
        assert_eq!(current_rates().loot, 200);
        // The ordinary fixture checks exact durable corpse quantities, including a guaranteed
        // drop on each of the two ordinary loot rolls.
        kill_reward_e2e_fixtures::scenario()
    })?;
    eprintln!(
        "SCHEDULED_BOSS_LOOT_POSTGRES_CASE_EXECUTED ordinary_loot elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}
