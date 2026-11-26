//! Radiant respawn, combat, and durable kill settlement on the composed owner.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{
    CharacterId, CharacterLease, GameSessionId, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::kill_reward::{
    DrainOutcome, KillSink, TakeOutcome, record_creature_kill,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_items::loot_plan::{LootPlanDeathKey, plan_creature_loot};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;

#[test]
fn summer_population_radiant_spawn_kill_claim_each() -> TestResult {
    scenario_thread::run_async(async |admin| {
        use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
        let _normal_rates = test_clock::set(Schedule::default(), 0);
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let declarations = [
            include_str!("../../../../../../content/world/spawns/spawns-00000-01999.json"),
            include_str!("../../../../../../content/world/spawns/spawns-02000-03999.json"),
        ]
        .into_iter()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let quest_catalogue = std::sync::Arc::new(
            oteryn_gameplay_quests::quest::task_board::extend(lowered.catalogue())
                .map_err(debug)?,
        );
        for name in [
            "acolyte",
            "inquisitor",
            "paragon",
            "templar",
            "warden",
            "zealot",
        ] {
            let key = format!("oteryn:creature.radiant_{name}");
            let respawn_ms = declarations
                .iter()
                .filter_map(|shard| shard["records"].as_array())
                .flatten()
                .filter_map(|source| source["declaration"]["points"].as_array())
                .flatten()
                .find(|point| point["creature"] == key)
                .and_then(|point| point["respawn_ms"].as_u64())
                .ok_or_else(|| format!("{key} has no pinned respawn period"))?;
            let h = Harness::create_with_admission_seed(
                admin.clone(),
                "summer_radiant_full_run",
                false,
                false,
            )
            .await?;
            eprintln!("w16summer-marker: summer_population_radiant_spawn_kill_claim_each db ready");
            let started = std::time::Instant::now();
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let room = pin.qualified_room().clone();
            // Transplant one source point to the open Thalom combat tile. The
            // bundle compiler's summer_population test covers actual map cells.
            let player_start = (2, 11, -7);
            let spawn_cell = (1, 11, -7);
            let mut deps = Dependencies::with_content_and_capacity(
                ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    active.identity().server_artifact_digest(),
                    active.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    room.map_revision_digest(),
                    player_start,
                ),
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                3, // One player, one creature, and activation headroom.
            )?;
            deps.combat_room = Some(deps.room.clone());
            let actor = deps.actor;
            let session = deps.admitted.game_session_id;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.quest_catalogue = Some(std::sync::Arc::clone(&quest_catalogue));
            owner.movement_cells = deps.room.movement_cells().into();
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
                .bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), NULL, $4, $5, $6)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(pin.activation_sequence().to_string())
                .bind(active.identity().server_artifact_digest().as_slice())
                .bind(active.identity().client_artifact_digest().as_slice())
                .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            let reward = native.reward_table().row(&key).map_err(debug)?;
            let loot_ref = reward.loot_table_ref.as_ref().ok_or("pinned loot table")?;
            // Allocate a deterministic occurrence with a positive roll, preserving the
            // activated drop probabilities. The respawn reuses the first free slot.
            let generation = (2..=20_000)
                .find(|generation| {
                    plan_creature_loot(
                        LootPlanDeathKey::new(
                            id(42),
                            id(43),
                            1,
                            actor.actor_local_id() + 1,
                            *generation,
                        ),
                        loot_ref,
                        &reward.loot_table,
                    )
                    .is_ok_and(|plan| !plan.entries.is_empty())
                })
                .ok_or("positive pinned loot occurrence")?;
            let (point, initial) = {
                let mut runtime = deps.runtime.lock().await;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let hp = u64::try_from(
                    runtime
                        .companion_policy(&key)
                        .map_err(debug)?
                        .maximum_health,
                )?;
                let unused_session = GameSessionId::decode(&id(245)).map_err(debug)?;
                for _ in 2..generation {
                    let unused = runtime
                        .reserve_fresh_session(unused_session)
                        .map_err(debug)?;
                    runtime
                        .rollback_definitely_uncommitted(unused)
                        .map_err(debug)?;
                }
                runtime
                    .realize_activation_spawns(vec![(
                        vec![spawn_cell],
                        respawn_ms,
                        crate::map::boot::BUNDLE_OCCUPANCY_RETRY_MS,
                        key.clone(),
                        hp,
                    )])
                    .map_err(debug)?;
                let (point, _, live) = runtime.spawn_point_creatures()[0];
                let initial = live.ok_or("initial activation spawn")?;
                // Start with a vacant point so the first credited death is
                // the one belonging to the actor spawned by the owner cycle.
                assert!(runtime.kill_creature_for_test(initial));
                (point, initial)
            };
            owner.drain_monster_melee().await;
            for _ in 0..2 {
                let due = deps
                    .runtime
                    .lock()
                    .await
                    .pending_respawn(point)
                    .map(|timer| timer.due_micros);
                let Some(due) = due else { break };
                let advance = due.saturating_sub(owner.owner_now().get());
                owner.clock_origin -= Duration::from_micros(advance);
                owner.drain_monster_melee().await;
            }
            let target = {
                let mut runtime = deps.runtime.lock().await;
                let live = runtime.spawn_point_creatures()[0]
                    .2
                    .ok_or_else(|| format!("{key} did not respawn through the monster cycle"))?;
                assert_ne!(live, initial, "{key} must get a new actor generation");
                assert_eq!(live.actor_local_generation(), generation);
                assert_eq!(
                    runtime.creature_spawn_cell(live),
                    Some(MovementLocalPosition {
                        x: spawn_cell.0,
                        y: spawn_cell.1,
                        floor: spawn_cell.2,
                    })
                );
                runtime
                    .bind_attacker_lease(
                        actor,
                        session,
                        CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                            .map_err(debug)?,
                    )
                    .map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                let mut states = owner.states.spell_states.try_lock().map_err(debug)?;
                states
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            level: 1_000_000,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .ok_or("player combat state")?;
                let row = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|row| row.actor == live)
                    .ok_or("visible Radiant")?;
                EntityRef {
                    identity: live.placement_identity(),
                    generation: row.generation,
                }
            };
            let mut admitted = deps.admitted;
            owner.admit_progression_session(&mut admitted).await;
            assert!(
                owner.progression_binding(session).is_some(),
                "{key} progression admission"
            );
            owner.refresh_quest_session(session, true).await;
            {
                let runtime = deps.runtime.lock().await;
                let mut attacks = owner.states.attack.try_lock().map_err(debug)?;
                assert_eq!(
                    attacks.set_target(
                        &runtime,
                        Some(&deps.room),
                        actor,
                        session,
                        owner.owner_now(),
                        Some(target),
                        1
                    ),
                    AttackIntentDisposition::Ok
                );
            }
            for _ in 0..64 {
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
                owner.clock_origin -= Duration::from_secs(3);
            }
            let (facts, target, observed_at) = {
                let attack = owner.states.attack.lock().await;
                let TakeOutcome::Taken(taken) = attack.kills().take(session, 2) else {
                    return Err(format!("{key} did not die to the attack owner").into());
                };
                let entry = taken.entry().ok_or("captured kill")?;
                let replay = (entry.facts, entry.target, entry.activity_observed_at);
                taken.requeue();
                replay
            };
            let death = facts.death;
            let plan = plan_creature_loot(
                LootPlanDeathKey::new(
                    *death.world_id().as_bytes(),
                    *death.channel_id().ok_or("Channel death")?.as_bytes(),
                    death.scope_ownership_generation().get(),
                    death.actor_local_id(),
                    death.actor_local_generation(),
                ),
                loot_ref,
                &reward.loot_table,
            )?;
            assert!(!plan.entries.is_empty(), "{key} exercises loot minting");
            let expected = plan
                .entries
                .iter()
                .map(|entry| {
                    (
                        i64::from(entry.draw_ordinal),
                        entry.purpose_key.clone(),
                        entry.item.family.clone(),
                        entry.item.production_key.clone(),
                        entry.item.revision_ref.clone(),
                        i64::from(entry.quantity),
                    )
                })
                .collect::<Vec<_>>();
            let race = reward.race.as_ref().ok_or("pinned Radiant race")?;
            for replay in [false, true] {
                if replay {
                    let mut runtime = deps.runtime.lock().await;
                    let attack = owner.states.attack.try_lock().map_err(debug)?;
                    record_creature_kill(
                        &mut runtime,
                        KillSink {
                            prey: None,
                            activity: &[],
                            queue: attack.kills(),
                            rewards: native.reward_table(),
                            charms: None,
                        },
                        target,
                        facts.principal,
                        facts.death_at_ms,
                        observed_at,
                    );
                    assert!(
                        attack.kills().has_session(session),
                        "replay reaches the reward owner"
                    );
                }
                for _ in 0..8 {
                    if owner.drain_kill_rewards(session).await == DrainOutcome::Drained {
                        break;
                    }
                    owner.root.maintain_ready_once().await.map_err(debug)?;
                }
                assert!(
                    !owner
                        .states
                        .attack
                        .lock()
                        .await
                        .kills()
                        .has_session(session),
                    "{key} settlement"
                );
                let corpse = {
                    let corpses = deps.runtime.lock().await.visible_corpses();
                    assert_eq!(corpses.len(), 1, "{key} corpse count");
                    assert_eq!(
                        corpses[0].item.production_key,
                        reward
                            .corpse_item
                            .as_ref()
                            .ok_or("pinned corpse")?
                            .production_key
                    );
                    corpses[0].item.item_instance_id
                };
                let actual: Vec<(i64, String, String, String, String, i64)> = sqlx::query_as(
                    "SELECT r.draw_ordinal,r.loot_purpose_key,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity
                     FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id)
                     JOIN game_item_mint_receipts r USING(item_instance_id)
                     WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY r.draw_ordinal LIMIT $2")
                    .bind(corpse.as_slice()).bind(i64::try_from(expected.len())? + 1)
                    .fetch_all(&h.pool).await?;
                assert_eq!(actual, expected, "{key} exact pinned corpse loot");
                let (xp_count, xp_awarded, kills): (i64, i64, i64) = sqlx::query_as(
                    "SELECT count(*), COALESCE(sum(experience_awarded),0)::bigint,
                            (SELECT kill_count FROM game_character_bestiary_progress WHERE character_id=encode($1,'hex')::uuid AND race_key=$2)
                     FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).bind(race.key()).fetch_one(&h.pool).await?;
                assert_eq!((xp_count, kills), (1, 1), "{key} XP and race credit");
                // The durable level-one character receives the existing 100% low-level bonus.
                assert_eq!(xp_awarded, 2 * reward.xp_amount.get(), "{key} native XP");
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            eprintln!(
                "w16summer: {key} settled and replayed in {:.2?}",
                started.elapsed()
            );
        }
        Ok(())
    })
}
