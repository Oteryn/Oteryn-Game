//! Detached fixed-name DATA over real admitted Player and current actor owners.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{Dependencies, TestResult};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::source_names::{SourceNameRole, creature_named};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::sync::Arc;

#[test]
fn srx3_source_names_detached_sql_reproves_two_players_npc_registered_dead_creature_postgres()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-source-names", || {
        run(async |admin| {
            super::reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::Murcion,
                MovementLocalPosition {
                    x: 32974,
                    y: 32365,
                    floor: 15,
                },
            )?;
            // Four actual actors and an NPC replacement share the existing
            // finite carrier; no runtime resource ceiling is changed.
            let deps = Dependencies::with_content_and_capacity(
                scene.pin.clone(),
                scene.room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                8,
            )?;
            let h = Harness::create_with_admission_seed(admin, "srx3_source_names", false, false)
                .await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            h.seed_appearance_observer().await?;
            super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Murcion)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let native = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .ok_or("current native policies")?;
            let catalogue = crate::content::NpcDataCatalogue::load(
                std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
                std::ffi::OsStr::new("world"),
                crate::content::npc_catalogue_preproduction_limits(),
            )
            .map_err(debug)?;
            let model = catalogue.service_model_for_project().map_err(debug)?;
            let talk = Arc::new(NpcTalkBook::from_model(&model));
            assert_eq!(
                talk.display_name("oteryn:npc.captain_bluebear"),
                Some("Captain Bluebear")
            );
            owner.states.chat.install_npc_talk(talk.clone());
            let placements = NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: "oteryn:npc.captain_bluebear".into(),
                    placements: vec![NpcPlacement {
                        x: 32975,
                        y: 32365,
                        floor: -15,
                        direction: MovementFacing::South,
                    }],
                }],
            };
            let (npc, dead) = {
                use crate::foundation::runtime_actor_spell_types::{
                    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
                };
                let mut runtime = owner.runtime.lock().await;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let npc = runtime
                    .place_npcs(&placements, |key| talk.display_name(key).is_some(), true)
                    .map_err(debug)?
                    .actors[0];
                // Explicit mechanics precondition only: actual native Rat type,
                // actual captured cell and real canonical HP publication.
                let pin = runtime.content_pin().clone();
                let rat = runtime
                    .realize_native_qualification_spawn_for_source(
                        65017,
                        pin,
                        "oteryn:creature.rat",
                        MovementLocalPosition {
                            x: 32973,
                            y: 32365,
                            floor: 15,
                        },
                    )
                    .map_err(debug)?;
                let fence = admitted.item_fence.ok_or("actual primary fence")?;
                let now = owner.owner_now();
                let batch = OwnerCombatBatch {
                    caster: admitted.runtime_actor.ok_or("actual primary Player")?,
                    attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    current_lease_generation: fence.character_lease_generation,
                    command: crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(17).map_err(debug)?,
                    ),
                    occurrence: SpellOccurrenceBinding {
                        id: "srx3:source-name-hp0".into(),
                        revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
                    },
                    binding: b"srx3:source-name-hp0".to_vec(),
                    anchor: None,
                    now_us: Some(now),
                    now_ms: now.get() / 1000,
                    effects: vec![OwnerCombatEffect {
                        target: rat.actor,
                        sub_ordinal: 0,
                        change: OwnerCombatChange::Damage {
                            target_atom: runtime
                                .creature_spell_target_atom(rat.actor)
                                .map_err(debug)?,
                            magnitude: rat.maximum_health,
                        },
                    }],
                    deferred: None,
                };
                let stage = runtime.stage_spell_batch(&batch).map_err(debug)?;
                runtime.commit_spell_batch(stage).map_err(debug)?;
                assert_eq!(
                    runtime
                        .companion_snapshot_including_dead(rat.actor)
                        .map_err(debug)?
                        .health,
                    0
                );
                assert!(
                    runtime
                        .source_registered_creature_census()
                        .map_err(debug)?
                        .iter()
                        .any(|(actor, _)| *actor == rat.actor)
                );
                (npc, rat.actor)
            };
            let names = ["Fixture Hero", "Fixture Watcher", "Captain Bluebear", "Rat"];
            // An actually admitted second Player is not yet a Runtime slot.
            // Its later publication must invalidate the complete old cohort.
            let old_plan = {
                let runtime = owner.runtime.lock().await;
                runtime
                    .owner_fence()
                    .map_err(|e| format!("names initial owner fence: {e:?}"))?;
                let players = runtime
                    .positioned_melee_census()
                    .map_err(|e| format!("names initial Player census: {e:?}"))?
                    .into_iter()
                    .filter_map(|(actor, _, session)| session.map(|session| (actor, session)))
                    .collect::<Vec<_>>();
                for &(actor, session) in &players {
                    runtime
                        .bound_attacker_lease(actor, session)
                        .map_err(|e| format!("names initial actual bound Player lease: {e:?}"))?;
                    runtime
                        .read_actor_position(actor)
                        .map_err(|e| format!("names initial actual Player position: {e:?}"))?;
                    if crate::durability::admission_journal::party_target_binding::observe_current_visible_actor_for_source_read(&runtime,actor,session).is_none() {
                        return Err("names initial actual Player visible observation unavailable".into());
                    }
                }
                crate::durability::source_creature_names::prepare_source_player_names(
                    &runtime, &names, &players,
                )
                .map_err(|e| format!("names initial detached Player plan: {e:?}"))?;
                for npc in runtime.visible_entities().npcs {
                    let key = runtime
                        .npc_key(npc.actor)
                        .ok_or("names initial current NPC key")?;
                    let talk = owner
                        .states
                        .chat
                        .npc_talk()
                        .ok_or("names initial installed NPC talk book")?;
                    talk.display_name(key)
                        .ok_or("names initial current NPC display name")?;
                    runtime
                        .read_movement_actor_position(npc.actor)
                        .map_err(|e| format!("names initial current NPC position: {e:?}"))?;
                }
                for (actor, _) in runtime
                    .source_registered_creature_census()
                    .map_err(|e| format!("names initial registered Creature census: {e:?}"))?
                {
                    runtime
                        .companion_snapshot_including_dead(actor)
                        .map_err(|e| format!("names initial registered Creature state: {e:?}"))?;
                }
                eprintln!(
                    "SRX3_SOURCE_NAMES_CAPTURE_PREREQUISITES_READY players={} capacity={}",
                    players.len(),
                    runtime.actor_slot_capacity()
                );
                owner
                    .prepare_source_names(&runtime, &names)
                    .map_err(|_| "prepare initial names")?
            };
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("actual Node scope lane")?;
            let old_read = owner
                .read_source_names(&permit, &old_plan)
                .await
                .map_err(|_| "detached initial SQL names")?;
            drop(permit);
            let observer =
                super::appearance_change_postgres::observer_visibility_tests::initialize(
                    &deps, &owner,
                )
                .await?;
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    owner.seal_source_names(&runtime, old_read).is_err(),
                    "new current Player invalidates old cohort"
                );
            }
            let plan = {
                let runtime = owner.runtime.lock().await;
                owner
                    .prepare_source_names(&runtime, &names)
                    .map_err(|_| "prepare complete names")?
            };
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("actual shared scope lane")?;
            let read = owner
                .read_source_names(&permit, &plan)
                .await
                .map_err(|_| "detached complete SQL names")?;
            let selected = {
                let runtime = owner.runtime.lock().await;
                owner
                    .seal_source_names(&runtime, read)
                    .map_err(|_| "reprove complete source names")?
            };
            assert_eq!(selected.len(), 4);
            assert_eq!(
                creature_named(&selected, "Fixture Hero")
                    .map_err(|_| "Hero ambiguity")?
                    .ok_or("Hero")?
                    .role,
                SourceNameRole::Player
            );
            assert_eq!(
                creature_named(&selected, "Fixture Watcher")
                    .map_err(|_| "Watcher ambiguity")?
                    .ok_or("Watcher")?
                    .actor,
                observer
            );
            assert_eq!(
                creature_named(&selected, "Captain Bluebear")
                    .map_err(|_| "NPC ambiguity")?
                    .ok_or("NPC")?
                    .actor,
                npc
            );
            let hp0 = creature_named(&selected, "RAT")
                .map_err(|_| "Creature ambiguity")?
                .ok_or("registeredHP0 Creature")?;
            assert_eq!((hp0.actor, hp0.role), (dead, SourceNameRole::Creature));
            // A readonly replay returns the same identities without changing
            // canonical Character or actor state.
            let replay = owner
                .read_source_names(&permit, &plan)
                .await
                .map_err(|_| "readonly replay")?;
            {
                let runtime = owner.runtime.lock().await;
                assert_eq!(
                    owner
                        .seal_source_names(&runtime, replay)
                        .map_err(|_| "reprove replay")?,
                    selected
                );
            }
            let stale_npc = owner
                .read_source_names(&permit, &plan)
                .await
                .map_err(|_| "read before NPC replacement")?;
            {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .remove_dynamic_npc("oteryn:npc.captain_bluebear")
                    .map_err(debug)?;
                let replaced = runtime
                    .place_npcs(&placements, |key| talk.display_name(key).is_some(), true)
                    .map_err(debug)?
                    .actors[0];
                assert_ne!(replaced, npc);
                assert!(
                    owner.seal_source_names(&runtime, stale_npc).is_err(),
                    "removed/replaced NPC generation invalidates old names"
                );
            }
            let plan_after_npc = {
                let runtime = owner.runtime.lock().await;
                owner
                    .prepare_source_names(&runtime, &names)
                    .map_err(|_| "prepare after NPC replacement")?
            };
            let stale_dead = owner
                .read_source_names(&permit, &plan_after_npc)
                .await
                .map_err(|_| "read before dead actor retirement")?;
            {
                let mut runtime = owner.runtime.lock().await;
                runtime.remove_encounter_actor(dead).map_err(debug)?;
                assert!(
                    owner.seal_source_names(&runtime, stale_dead).is_err(),
                    "removed registeredHP0 Creature invalidates old names"
                );
            }
            let pose_plan = {
                let runtime = owner.runtime.lock().await;
                owner
                    .prepare_source_names(&runtime, &names)
                    .map_err(|_| "prepare before real movement")?
            };
            let stale_pose = owner
                .read_source_names(&permit, &pose_plan)
                .await
                .map_err(|_| "read before real movement")?;
            drop(permit);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(500))
                .ok_or("current Player move clock")?;
            let (moved, _) = owner
                .ordinary_paced_step(
                    admitted.runtime_actor.ok_or("actual Player")?,
                    admitted.game_session_id,
                    oteryn_protocol_oteryn::world_spatial::StepDirection::West,
                    None,
                    None,
                )
                .await;
            assert_eq!(
                moved.disposition,
                oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
            );
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    owner.seal_source_names(&runtime, stale_pose).is_err(),
                    "actual current Player translation invalidates old name DATA"
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_SOURCE_NAMES_DETACHED_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
