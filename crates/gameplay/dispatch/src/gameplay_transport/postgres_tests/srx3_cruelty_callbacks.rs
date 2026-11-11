//! Actual pinned Cruelty entry and native defense callback; no seeded catalogue.
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_cruelty_actual_source_entry_and_native_defense_chain_postgres() -> TestResult {
    run_source_entry(false)
}

#[test]
fn srx3_cruelty_canceled_before_names_drains_original_source_use_postgres() -> TestResult {
    run_source_entry_case(
        false,
        Some(super::srx3_soul_war_source_use::RecoveryPoint::BeforeNames),
    )
}

#[test]
fn srx3_cruelty_canceled_after_write_reconciles_source_effect_once_postgres() -> TestResult {
    run_source_entry_case(
        false,
        Some(super::srx3_soul_war_source_use::RecoveryPoint::AfterWrite),
    )
}

#[test]
fn srx3_cruelty_completed_handoff_retry_preserves_source_effect_once_postgres() -> TestResult {
    run_source_entry_case(
        false,
        Some(super::srx3_soul_war_source_use::RecoveryPoint::Handoff),
    )
}

pub(super) fn run_source_entry(attack_cancellation: bool) -> TestResult {
    run_source_entry_case(attack_cancellation, None)
}

fn run_source_entry_case(
    attack_cancellation: bool,
    recovery: Option<super::srx3_soul_war_source_use::RecoveryPoint>,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-cruelty-native", move || {
        run(async move |admin| {
            reset();
            // Crystal201189f lib/quests/soul_war.lua:422-448: literal first
            // waiting tile and its real source map; no seeded catalogue recipe.
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsCruelty,
                MovementLocalPosition {
                    x: 33854,
                    y: 31854,
                    floor: 6,
                },
            )?;
            let deps: Dependencies = scene.dependencies()?;
            let seed = if attack_cancellation {
                "srx3_soul_war_attack_cancellation"
            } else {
                "srx3_cruelty_native"
            };
            let h = Harness::create_with_admission_seed(admin, seed, false, false).await?;
            // Imported prior quest access is a historical fixture fact, following
            // srx3_taints. New admission/callback writes keep all normal guards.
            let mut historical = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *historical)
                .await?;
            sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,true,0,NULL,false,false)")
                .bind(id(41).as_slice()).execute(&mut *historical).await?;
            historical.commit().await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            // Historical carried units only; every tested use goes through the
            // actual fenced Item/Quest owner after fresh admission.
            super::item_look::seed_main_backpack(
                &h,
                &[
                    ("oteryn:item.tibia.i33891", 1),
                    ("oteryn:item.tibia.i33891", 1),
                ],
            )
            .await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.quest_catalogue = Some(std::sync::Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            install_scene(
                &mut owner,
                &scene,
                PinnedActivitySourceRoom::GoshnarsCruelty,
            )?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[
                    4,
                    6,
                    7,
                    10,
                    11,
                    15,
                    18,
                    23,
                    92,
                    wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                ],
            )
            .ok_or("actual native boss capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 216).await?;

            let mut frames = Vec::new();
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                "oteryn:creature.goshnar_s_cruelty",
                true,
                &mut frames,
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "actual Cruelty source entry: {reply:?}"
            );
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("actual Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            assert!(matches!(instance, RuntimeScopeRefV1::Instance { .. }));
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual source owner")?;
            let view = owner.runtime_view(&registered);
            let boss = {
                let runtime = view.runtime.lock().await;
                runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|c| {
                        runtime.matches_live_creature_identity(
                            c.actor,
                            b"oteryn:creature.goshnar_s_cruelty",
                        )
                    })
                    .ok_or("actual source Cruelty actor")?
                    .actor
            };
            if attack_cancellation {
                super::srx3_soul_war_attacks::actual_source_suffix(
                    &view,
                    admitted
                        .runtime_actor
                        .ok_or("actual source attack Player")?,
                    admitted.game_session_id,
                    boss,
                )
                .await?;
                drop(view);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let initial_defense = {
                let runtime = view.runtime.lock().await;
                runtime
                    .effective_creature_defense(&runtime.companion_snapshot(boss).map_err(debug)?)
                    .map_err(debug)?
            };
            drop(view);
            // Advance actual current native turns, never a wall-time shortcut to
            // the source accumulated 15,000 ms onThink counter.
            for _ in 0..16 {
                owner.runtime_view(&registered).drain_monster_melee().await;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("owner clock")?;
            }
            let view = owner.runtime_view(&registered);
            let runtime = view.runtime.lock().await;
            let current = runtime.companion_snapshot(boss).map_err(debug)?;
            assert_eq!(
                runtime
                    .effective_creature_defense(&current)
                    .map_err(debug)?,
                initial_defense + 2,
                "actual native source 15s helper raises defense by two once"
            );
            assert_eq!(
                runtime.soul_war_cruelty_drain(&current).map_err(debug)?,
                Some(3)
            );
            let before_maw_health = current.health;
            drop(runtime);
            let player = admitted.runtime_actor.ok_or("actual source Maw player")?;
            {
                let mut runtime = view.runtime.lock().await;
                let before = runtime.read_actor_position(player).map_err(debug)?;
                assert_eq!(
                    before.position(),
                    MovementLocalPosition {
                        x: 33856,
                        y: 31872,
                        floor: 7
                    }
                );
                let destination = MovementLocalPosition {
                    x: 33856,
                    y: 31873,
                    floor: 7,
                };
                assert!(
                    !runtime
                        .position_occupied_by_other(player, destination)
                        .map_err(debug)?
                );
                let cells = view.effective_movement_cells();
                let selection = crate::movement::MovementEngineeringSelection {
                    owner_context: before.context(),
                    content_scope: cells.scope(),
                };
                crate::movement::step_cardinal(
                    &mut runtime.borrow_movement_position(),
                    player,
                    before,
                    &selection,
                    cells.index(),
                    crate::movement::CardinalStep::South,
                )
                .map_err(debug)?;
                assert_eq!(
                    runtime
                        .read_actor_position(player)
                        .map_err(debug)?
                        .position(),
                    destination
                );
            }
            let native = view
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or("actual source policies")?;
            let index = native.item_index().ok_or("actual Item index")?;
            let source = index
                .definition_ref("Item", "oteryn:item.tibia.i33891", "definition-r1")
                .ok_or("actual Mortal Essence")?;
            let maw = index
                .definition_ref("Item", "oteryn:item.tibia.i33890", "definition-r1")
                .ok_or("actual Greedy Maw source binding")?;
            let map = view.bundle_map().ok_or("actual private source map")?;
            let tile = crate::map::view::compose_tile(
                &map.overlay,
                &map.facts,
                crate::map::overlay::TilePos {
                    x: 33856,
                    y: 31874,
                    floor: -7,
                },
            )
            .map_err(debug)?
            .ok_or("literal Greedy Maw tile")?;
            let mut targets = tile.entries.iter().filter(|entry| {
                entry.facts.definition
                    == oteryn_protocol_oteryn::world_map::MapDefinition::Item(maw)
            });
            let target = targets.next().ok_or("actual current Greedy Maw")?;
            assert!(targets.next().is_none());
            let crate::map::view::EntrySource::Base { placement_key, .. } = target.source else {
                return Err("Greedy Maw must be its qualified source placement".into());
            };
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&placement_key.to_be_bytes());
            use oteryn_protocol_oteryn::world_object::{
                MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
            };
            let intent = UseIntent {
                target: UseTarget::ItemByDefinition(source),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                    placement,
                    expected_revision: map.facts.object_revision(placement_key),
                })),
                use_with_position: None,
            };
            let wrong = UseIntent {
                use_with_map_item: None,
                ..intent.clone()
            };
            let reply = super::quest_world_callbacks::wire(
                &view,
                &mut admitted,
                1,
                2,
                oteryn_protocol_oteryn::world_object::encode_item_use_intent(&wrong)
                    .map_err(debug)?,
            )
            .await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::NotPossible,
                "wrong Maw target must not spend its source unit"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/soul-war/source-items/greedy-maw'")
                .fetch_one(&h.pool).await?, 0);
            let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
                .map_err(debug)?;
            let reply = if let Some(point) = recovery {
                super::srx3_soul_war_source_use::recover_source_use(
                    &view,
                    &mut admitted,
                    2,
                    &intent,
                    point,
                    UseDisposition::Committed,
                )
                .await?
            } else {
                super::quest_world_callbacks::wire(&view, &mut admitted, 2, 2, payload.clone())
                    .await?
            };
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed,
                "real USE must consume Mortal Essence and publish the Cruelty effect"
            );
            let repeated = view
                .dispatch_soul_war_source_use(
                    player,
                    crate::gameplay_transport::connection::UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 2,
                        item_fence: admitted.item_fence,
                    },
                    &intent,
                    None,
                    None,
                )
                .await
                .map_err(|_| "retained Maw replay")?
                .ok_or("replayed source use")?;
            assert_eq!(repeated.disposition, UseDisposition::Committed);
            let cooldown =
                super::quest_world_callbacks::wire(&view, &mut admitted, 3, 2, payload).await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&cooldown)
                    .map_err(debug)?,
                UseDisposition::NotPossible,
                "a new command inside the real30s cooldown retains its unit"
            );
            let runtime = view.runtime.lock().await;
            let after = runtime.companion_snapshot(boss).map_err(debug)?;
            assert_eq!(
                runtime.effective_creature_defense(&after).map_err(debug)?,
                initial_defense
            );
            assert_eq!(
                runtime.soul_war_cruelty_drain(&after).map_err(debug)?,
                Some(2),
                "receipt replay and cooldown refusal must not repeat the source decrease"
            );
            assert_eq!(after.health, before_maw_health);
            assert!(!runtime.actor_spell_reserved(player));
            assert!(!runtime.actor_spell_reserved(boss));
            drop(runtime);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/soul-war/source-items/greedy-maw'")
                .fetch_one(&h.pool).await?, 1);
            let used_at: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(id(41).as_slice()).bind(oteryn_gameplay_quests::quest::soul_war::source_items::MAW)
                .fetch_one(&h.pool).await?;
            assert!(
                used_at > 0,
                "the actual transaction supplies the source timestamp"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT sum(i.quantity)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i33891' AND i.lifecycle=1")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?, 1,
                "one known action burns one actual unit; its replay and refusal burn none");
            drop(view);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    if attack_cancellation {
        eprintln!(
            "SRX3_SOUL_WAR_ATTACKS_POSTGRES_CASE_EXECUTED duration={:?}",
            started.elapsed()
        );
    } else {
        eprintln!(
            "SRX3_CRUELTY_NATIVE_POSTGRES_CASE_EXECUTED duration={:?}",
            started.elapsed()
        );
    }
    Ok(())
}
