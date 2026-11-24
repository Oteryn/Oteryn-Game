//! Crystal 201189f Herald lever uses the boot publication and native Instance handoff.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::content::NativeEntryStart;
use crate::content::bosses::activity_template::captured_boss_levers;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{ChannelContentPin, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use crate::map::{overlay::TilePos, view};
use oteryn_protocol_oteryn::{
    boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
    world_object::{UseDisposition, WorldObjectTarget},
    world_spatial::StepDisposition,
};

const BOSS: &str = "oteryn:boss.herald_of_fire";
const CREATURE: &str = "oteryn:creature.herald_of_fire";
const LEVER: TilePos = TilePos {
    x: 32435,
    y: 32653,
    floor: -15,
};

#[test]
fn herald_source_lever_boots_and_enters_fresh_instance() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("herald-source-lever", || {
        scenario(false, false, CallbackCase::Original)
    })
}

#[test]
fn herald_lever_refuses_wrong_pad_and_replays_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("herald-source-lever-replay", || {
        scenario(true, false, CallbackCase::Original)?;
        scenario(false, true, CallbackCase::Original)
    })
}

#[derive(Clone, Copy, Debug)]
enum CallbackCase {
    Original,
    DurableField,
    UncreditedDeath,
    PoisonDeath,
    FarPad,
    LostAcknowledgement,
    FireFields,
    EnvironmentalLastHit,
    FrozenDeadline,
    FieldStep,
    FieldProjection,
    RedFloors,
    StaleParty,
}

#[test]
fn herald_environmental_last_hit_settles_player_damage() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_environmental_last_hit_settles_player_damage",
        || scenario(false, false, CallbackCase::EnvironmentalLastHit),
    )
}

#[test]
fn herald_deadline_millisecond_is_shared_by_victory_and_quest() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_deadline_millisecond_is_shared_by_victory_and_quest",
        || scenario(false, false, CallbackCase::FrozenDeadline),
    )
}

#[test]
fn herald_fields_apply_environmental_step_without_field_capability() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_fields_apply_environmental_step_without_field_capability",
        || scenario(false, false, CallbackCase::FieldStep),
    )
}

#[test]
fn herald_defers_a_due_poison_tick_until_ground_projects() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_defers_a_due_poison_tick_until_ground_projects",
        || scenario(false, false, CallbackCase::FieldProjection),
    )
}

#[test]
fn herald_three_red_floors_reset_and_clear_only_the_instance() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_three_red_floors_reset_and_clear_only_the_instance",
        || scenario(false, false, CallbackCase::RedFloors),
    )
}

#[test]
fn herald_fifth_pad_requires_use_reach() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_fifth_pad_requires_use_reach",
        || scenario(false, false, CallbackCase::FarPad),
    )
}

#[test]
fn herald_use_reconciles_lost_commit_acknowledgement() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_use_reconciles_lost_commit_acknowledgement",
        || scenario(false, false, CallbackCase::LostAcknowledgement),
    )
}

#[test]
fn herald_ai_telegraphs_and_refreshes_25_durable_fire_fields() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_ai_telegraphs_and_refreshes_25_durable_fire_fields",
        || scenario(false, false, CallbackCase::FireFields),
    )
}

#[test]
fn herald_player_poison_settles_the_lethal_receipt() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_player_poison_settles_the_lethal_receipt",
        || scenario(false, false, CallbackCase::PoisonDeath),
    )
}

#[test]
fn herald_shared_party_settles_nondamaging_members() -> TestResult {
    super::srvenc_source_deaths_fixtures::scenario("herald_shared_party", "herald_of_fire")
}

#[test]
fn herald_ignores_idle_peers_after_sharing_is_disabled() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "herald_ignores_idle_peers_after_sharing_is_disabled",
        || scenario(false, false, CallbackCase::StaleParty),
    )
}

#[test]
fn herald_is_immune_on_durable_rune_field() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("herald-durable-field", || {
        scenario(false, false, CallbackCase::DurableField)
    })
}

#[test]
fn herald_uncredited_death_opens_exit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("herald-uncredited-death", || {
        scenario(false, false, CallbackCase::UncreditedDeath)
    })
}

fn scenario(wrong_pad: bool, replay: bool, case: CallbackCase) -> TestResult {
    let started = std::time::Instant::now();
    eprintln!(
        "POSTGRES_CASE_EXECUTED herald_lever wrong_pad={wrong_pad} replay={replay} case={case:?}"
    );
    let result = run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "herald_lever", false, false).await?;
        let result: TestResult = async {
            let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            let fresh = super::house_transfer_fixtures::prepare_owned_session_request(&h).await?;
            let expected = fixture::fence(1)?;
            let RuntimeScopeRefV1::Channel { world_id, channel_id } = expected.runtime_scope else {
                return Err("source Channel".into());
            };
            let (controller, activation) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    world_id, "content-1",
                )?;
            let active = controller.active().ok_or("active content")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let source_room = activation.qualified_room().clone();
            let activation_pin = activation.into_channel_pin();
            let candidates = captured_boss_levers().map_err(debug)?;
            let candidate = candidates.iter().find(|row| row.creature_key.as_deref() == Some(CREATURE))
                .ok_or("captured Herald lever")?;
            assert_eq!(candidate.boss_position, Some([32496, 32656, 15]));
            let bundle = super::srvenc_source_boss_fixtures::source_bundle(
                world_id, channel_id, native, candidate)?;
            assert!(bundle.enterable(TilePos { x: 32418, y: 32686, floor: -13 }),
                "the native map must keep the source exit's debris passable");
            assert!(!bundle.enterable(TilePos { x: 32493, y: 32663, floor: -15 }),
                "the donor's fifth landing requires the sealed forced entry");
            let cells = bundle.movement_cells(source_room.movement_cells()).map_err(debug)?;
            let start = if wrong_pad {
                NativeEntryStart { x: 32434, y: 32653, floor: 15 }
            } else if matches!(case, CallbackCase::FarPad) {
                NativeEntryStart { x: 32435, y: 32658, floor: 15 }
            } else {
                NativeEntryStart { x: 32435, y: 32654, floor: 15 }
            };
            let room = source_room.bundle_view(
                cells.clone(), bundle.frame_binding_digest(world_id), bundle.map_revision_digest(), start,
            ).ok_or("source bundle combat room")?;
            let (mut templates, diagnostics) = controller.qualify_initial_activity_template(
                &source_room, &room, "content-1", &candidates);
            if matches!(case, CallbackCase::FrozenDeadline) {
                templates.iter_mut().find(|room| room.boss_key() == BOSS)
                    .ok_or("qualified Herald fixture")?.set_fixture_fight_seconds(2);
            }
            let template = templates.iter().find(|room| room.boss_key() == BOSS)
                .ok_or_else(|| format!("Herald boot qualification: {:?}", diagnostics.iter()
                    .find(|diagnostic| diagnostic.path == candidate.path)))?;
            assert_eq!(template.minimum_level(), 12);
            let cells_of = |x, y| crate::content::LogicalCell { x, y, z: 15 };
            assert_eq!(template.lever_positions(),
                (32654..=32658).map(|y| cells_of(32435, y)).collect::<Vec<_>>());
            assert_eq!(template.entry_positions(),
                (32659..=32663).map(|y| cells_of(32493, y)).collect::<Vec<_>>());
            assert_eq!(template.boss_position(), crate::content::LogicalCell { x: 32496, y: 32656, z: 15 });
            let pin = ChannelContentPin::from_activation(
                world_id, active.activation_sequence(), active.identity().server_artifact_digest(),
                active.identity().client_artifact_digest(), bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(), (start.x, start.y, start.floor),
            );
            let mut deps = Dependencies::with_content(
                pin.clone(), source_room, crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            deps.combat_room = Some(room);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = Some(active);
            if matches!(case, CallbackCase::PoisonDeath | CallbackCase::EnvironmentalLastHit | CallbackCase::FrozenDeadline | CallbackCase::StaleParty) {
                let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
                owner.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
            }
            owner.movement_cells = (&cells).into();
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES, &[CAPABILITY_BOSS_ENCOUNTER_V1],
            ).ok_or("selected boss capability")?;
            super::boss_activity_fixtures::issue_content_and_reset_with_runtime_pin(
                &h, &activation_pin, &pin, bundle.map_revision()).await?;
            super::srx3_activity_source_fixture::prepare_player_with_operation(
                &h, &owner, &mut admitted, 12, "knight", fresh.operation(),
            ).await.map_err(|error| format!("prepare level-12 Herald participant: {error}"))?;
            let fence = admitted.item_fence.ok_or("live item fence")?;
            let items: &[(&str, i64)] = match case {
                CallbackCase::DurableField => &[("oteryn:item.tibia.i2118", 1)],
                _ => &[],
            };
            super::item_look_fixtures::seed_main_backpack(&h, items).await?;
            owner.register_current_runtime().await.map_err(|_| "register Channel runtime")?;
            owner.activity_runtimes.remember(admitted).map_err(|_| "register admitted session")?;
            let peer = if matches!(case, CallbackCase::StaleParty) {
                Some(Box::pin(prepare_herald_peer(&h, &owner, admitted)).await?)
            } else {
                None
            };
            owner.activity_runtimes.register_template(template).map_err(|_| "register template")?;
            ActivityHandoffOwner::new(h.root.clone())
                .publish_qualified_boss_rooms(&h.node, &templates).await?;
            let published: (i64, i16, i64) = sqlx::query_as(
                "SELECT minimum_level, maximum_players, cooldown_seconds FROM game_boss_room_publications WHERE boss_key=$1",
            ).bind(BOSS).fetch_one(&h.pool).await?;
            assert_eq!(published, (12, 5, 72000));
            let instance = super::activity_runtime_fixtures::allocate_ready_instance(&h, 88).await?;
            let map = owner.bundle_map().ok_or("Herald origin map")?;
            let tile = map.overlay.base().tile(LEVER.x, LEVER.y, LEVER.floor).ok_or("physical lever tile")?;
            let ordinal = (0..u8::try_from(tile.ids().len())?).find(|&i| tile.appearance(i) == Some(8911))
                .ok_or("physical Herald lever")?;
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend(view::placement_key(LEVER, ordinal).ok_or("lever placement")?.to_be_bytes());
            let target = WorldObjectTarget { placement, expected_revision: 0 };
            let command = UseCommand { game_session_id: admitted.game_session_id, command_id: 1, item_fence: Some(fence) };
            if wrong_pad {
                assert_eq!(lever_use(&owner, deps.actor, command, target).await?.disposition,
                    UseDisposition::Rejected);
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
                    .fetch_one(&h.pool).await?;
                assert_eq!(count, 0);
                assert!(owner.activity_runtimes.get(instance, 1).is_none());
                return Ok(());
            }
            let command = UseCommand { command_id: 2, ..command };
            if matches!(case, CallbackCase::LostAcknowledgement) {
                crate::gameplay_transport::native_activity::srx3_faults::stop_after_herald_commit_once();
            }
            let outcome = lever_use(&owner, deps.actor, command, target.clone()).await?;
            if matches!(case, CallbackCase::FarPad) {
                assert_eq!(outcome.disposition, UseDisposition::TooFar);
                let groups: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
                    .fetch_one(&h.pool).await?;
                assert_eq!(groups, 0);
                return Ok(());
            }
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            let switch = owner.activity_runtimes.pending_switch(admitted).map_err(|_| "scope switch")?
                .ok_or("committed Instance handoff")?;
            assert_eq!(switch.destination.item_fence.ok_or("Instance fence")?.runtime_scope, instance);
            if matches!(case, CallbackCase::LostAcknowledgement) {
                assert_eq!(lever_use(&owner, deps.actor, command, target.clone()).await?.disposition,
                    UseDisposition::Committed);
                let groups: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups WHERE activity_key=$1 AND state=2")
                    .bind(BOSS).fetch_one(&h.pool).await?;
                assert_eq!(groups, 1);
                return Ok(());
            }
            if !replay {
                owner.activity_runtimes.finish_switch(admitted, switch.destination)
                    .map_err(|_| "finish source scope switch")?;
                let peer = if let Some(peer) = peer {
                    let switch = owner.activity_runtimes.pending_switch(peer).map_err(|_| "peer scope switch")?
                        .ok_or("committed peer handoff")?;
                    owner.activity_runtimes.finish_switch(peer, switch.destination)
                        .map_err(|_| "finish peer scope switch")?;
                    Some(switch.destination)
                } else {
                    None
                };
                let mut destination = owner.resolve_runtime_view(switch.destination).await
                    .map_err(|_| "resolve Herald Instance")?.ok_or("current Herald Instance")?;
                match case {
                    CallbackCase::Original => assert_herald_callbacks(&mut destination, switch.destination).await?,
                    CallbackCase::DurableField => {
                        assert_durable_field_immunity(&destination, switch.destination).await?;
                        return Ok(());
                    }
                    CallbackCase::UncreditedDeath | CallbackCase::PoisonDeath
                    | CallbackCase::EnvironmentalLastHit | CallbackCase::FrozenDeadline | CallbackCase::StaleParty => {
                        Box::pin(assert_periodic_exit(&h, &destination, switch.destination, case, peer)).await?;
                    }
                    CallbackCase::LostAcknowledgement => return Err("lost acknowledgement case delivered its switch".into()),
                    CallbackCase::FireFields | CallbackCase::FieldStep | CallbackCase::RedFloors
                    | CallbackCase::FieldProjection => {
                        Box::pin(assert_fire_field_attack(&destination, switch.destination, case)).await?;
                        return Ok(());
                    }
                    CallbackCase::FarPad => return Err("far pad allocated an Instance".into()),
                }
                let deadlines: (i64, Option<i64>) = sqlx::query_as(
                    "SELECT deadline, exit_deadline FROM game_boss_room_activities WHERE boss_key=$1",
                ).bind(BOSS).fetch_one(&h.pool).await?;
                assert_eq!(deadlines.1, Some(deadlines.0));
            }
            if replay {
                assert_eq!(lever_use(&owner, deps.actor, command, target).await?.disposition, UseDisposition::Committed);
            }
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups WHERE activity_key=$1 AND state=2")
                .bind(BOSS).fetch_one(&h.pool).await?;
            assert_eq!(count, 1);
            Ok(())
        }.await;
        result.and(h.cleanup().await)
    });
    if result.is_ok() {
        eprintln!(
            "POSTGRES_CASE_COMPLETED herald_lever wrong_pad={wrong_pad} replay={replay} case={case:?} elapsed={:?}",
            started.elapsed()
        );
    } else {
        eprintln!(
            "POSTGRES_CASE_FAILED herald_lever wrong_pad={wrong_pad} replay={replay} case={case:?} elapsed={:?}",
            started.elapsed()
        );
    }
    result
}

async fn assert_herald_callbacks(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::foundation::{CharacterId, CommandId, CommandRef};
    use crate::map::view::MapFacts;
    let fence = admitted.item_fence.ok_or("Herald Instance fence")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("Herald native Item and loot content")?;
    let rewards = native.reward_table().row(CREATURE).map_err(debug)?;
    assert_eq!(
        rewards
            .corpse_item
            .as_ref()
            .ok_or("Herald corpse")?
            .production_key,
        "oteryn:item.tibia.i53108"
    );
    let map = owner.bundle_map().ok_or("bounded Herald Instance map")?;
    let boss_tile = TilePos {
        x: 32496,
        y: 32656,
        floor: -15,
    };
    let portal_tile = TilePos {
        x: 32496,
        y: 32661,
        floor: -15,
    };
    assert!(map.facts.movement_items(portal_tile).is_empty());
    let now = owner.owner_now();
    {
        let attack = owner.states.attack.lock().await;
        let mut runtime = owner.runtime.lock().await;
        let player = admitted.runtime_actor.ok_or("Instance participant")?;
        let entry = runtime
            .read_actor_position(player)
            .map_err(debug)?
            .position();
        assert_eq!((entry.x, entry.y, entry.floor), (32493, 32659, 15));
        let creatures = runtime.visible_entities().creatures;
        assert_eq!(creatures.len(), 1);
        let boss = creatures[0].actor;
        assert_eq!(
            runtime
                .companion_snapshot(boss)
                .map_err(debug)?
                .state
                .policy
                .definition_key,
            CREATURE
        );
        assert!(map.facts.create_movement_item(boss_tile, 2118, 0, || true));
        let field = map
            .facts
            .movement_items(boss_tile)
            .into_iter()
            .find_map(|entry| {
                if let view::EntrySource::Added(id) = entry.source {
                    (entry.facts.appearance_id == 2118).then_some(id)
                } else {
                    None
                }
            })
            .ok_or("current source fire field")?;
        let mut batch = OwnerCombatBatch {
            caster: player,
            attacker: CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "m9:herald-field-immunity".to_owned(),
                revisions: ["r1"; 5].map(str::to_owned),
            },
            binding: b"m9:herald-field-immunity".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: boss,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(boss).map_err(debug)?,
                    magnitude: 900,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let result = runtime.commit_spell_batch(staged).map_err(debug)?;
        let health = result
            .effects
            .first()
            .and_then(|r| r.health)
            .ok_or("immune hit receipt")?;
        assert_eq!((health.health_before, health.health_after), (900, 900));
        assert!(map.facts.movement_items(portal_tile).is_empty());
        assert!(map.facts.remove_movement_item(boss_tile, field.get(), 2118));
        batch.command =
            CommandRef::new(admitted.game_session_id, CommandId::new(92).map_err(debug)?);
        batch.occurrence.id = "m9:herald-lethal".into();
        batch.binding = b"m9:herald-lethal".to_vec();
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert_eq!(
            receipt
                .effects
                .first()
                .and_then(|r| r.health)
                .ok_or("lethal hit receipt")?
                .health_after,
            0
        );
        let states = owner.states.spell_states.lock().await;
        owner.record_spell_kills_locked(&attack, &states, &mut runtime, std::iter::once(&receipt));
        assert!(attack.kills().has_session(admitted.game_session_id));
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        map.facts
            .movement_items(portal_tile)
            .iter()
            .filter(|entry| entry.facts.appearance_id == 25051)
            .count(),
        1
    );
    let actor = admitted.runtime_actor.ok_or("Instance participant")?;
    let target = {
        let runtime = owner.runtime.lock().await;
        owner
            .wire_observation(
                &runtime,
                crate::foundation::MovementLocalPosition {
                    x: 32496,
                    y: 32661,
                    floor: 15,
                },
            )
            .ok_or("source portal wire position")?
    };
    let route = owner
        .item_walk_route(
            actor,
            admitted.game_session_id,
            crate::gameplay_transport::paced_movement::AutoWalkTarget::Cell(target.actor_position),
        )
        .await
        .map_err(|_| "route to the source portal")?
        .ok_or("reachable source portal")?;
    assert!(!route.steps.is_empty());
    for direction in route.steps {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("test movement cadence")?;
        let (step, _) = owner
            .paced_step(actor, admitted.game_session_id, direction)
            .await;
        assert_eq!(step.disposition, StepDisposition::Moved);
        owner
            .committed_paced_movement(admitted, step.moved_to)
            .await;
    }
    let switch = owner
        .activity_runtimes
        .pending_switch(admitted)
        .map_err(|_| "Herald portal return handoff")?
        .ok_or("committed Herald portal return")?;
    let returned = owner
        .resolve_runtime_view(switch.destination)
        .await
        .map_err(|_| "resolve Herald return Channel")?
        .ok_or("Herald return Channel")?;
    let runtime = returned.runtime.lock().await;
    let player = switch.destination.runtime_actor.ok_or("returned player")?;
    let after = runtime
        .read_actor_position(player)
        .map_err(debug)?
        .position();
    assert_eq!((after.x, after.y, after.floor), (32418, 32686, 13));
    Ok(())
}

async fn assert_durable_field_immunity(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    use crate::foundation::{CommandId, CommandRef};

    let fence = admitted.item_fence.ok_or("Instance fence")?;
    let (boss, ground, digest) = {
        let runtime = owner.runtime.lock().await;
        let boss = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("Herald")?
            .actor;
        let at = runtime.read_actor_position(boss).map_err(debug)?.position();
        let pin = runtime.content_pin();
        let revision = |digest: [u8; 32]| {
            format!(
                "sha256:{}",
                digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            )
        };
        (
            boss,
            LoopGround {
                spatial_position: [
                    at.x.to_be_bytes().as_slice(),
                    at.y.to_be_bytes().as_slice(),
                    at.floor.to_be_bytes().as_slice(),
                ]
                .concat(),
                map_revision: revision(pin.map_revision_digest()),
                content_revision: revision(pin.server_artifact_digest()),
                placement_context: pin.frame_binding_digest().to_vec(),
            },
            pin.server_artifact_digest(),
        )
    };

    // Seed the donor's fire-field Item, then use the real scoped Ground writer.
    // The callback depends on live Item identity at the cell, independent of origin.
    let field = fixture::id(71);
    {
        let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
        let receipt = owner
            .root
            .commit_item_loop_move(
                &permit,
                owner.character,
                owner.holder,
                &fence,
                CommandRef::new(admitted.game_session_id, CommandId::new(93).map_err(debug)?),
                &HeraldFieldFixture(digest),
                LoopMoveRequest {
                    item: field,
                    count: None,
                    pickup_cell: None,
                    destination: LoopDestination::Ground(ground.clone()),
                },
            )
            .await?
            .map_err(debug)?;
        assert_eq!(receipt.operation, 4);
    }
    assert!(
        owner
            .root
            .read_item_loop_ground_for_scope(fence.runtime_scope)
            .await?
            .iter()
            .any(|row| row.item.item_instance_id == field
                && row.item.definition.production_key == "oteryn:item.tibia.i2118"
                && row.item.ground.spatial_position == ground.spatial_position)
    );
    assert_eq!(
        commit_fixture_hit(owner, admitted, boss, 94, 250).await?,
        (900, 900)
    );
    {
        let permit = owner.spell_lane_permit().await.ok_or("field pickup lane")?;
        let receipt = owner
            .root
            .commit_item_loop_move(
                &permit,
                owner.character,
                owner.holder,
                &fence,
                CommandRef::new(admitted.game_session_id, CommandId::new(95).map_err(debug)?),
                &HeraldFieldFixture(digest),
                LoopMoveRequest {
                    item: field,
                    count: None,
                    pickup_cell: Some(ground.spatial_position),
                    destination: LoopDestination::MainBackpack,
                },
            )
            .await?
            .map_err(debug)?;
        assert_eq!(receipt.operation, 5);
    }
    assert_eq!(
        commit_fixture_hit(owner, admitted, boss, 96, 250).await?,
        (900, 650)
    );
    Ok(())
}

async fn commit_fixture_hit(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    boss: crate::foundation::ExactActorRef,
    sequence: u64,
    magnitude: i64,
) -> TestResult<(i64, i64)> {
    use crate::foundation::{CharacterId, CommandId, CommandRef};
    let fence = admitted.item_fence.ok_or("Instance fence")?;
    let player = admitted.runtime_actor.ok_or("Instance participant")?;
    let _permit = owner
        .spell_lane_permit()
        .await
        .ok_or("Herald combat lane")?;
    let now = owner.owner_now();
    let mut runtime = owner.runtime.lock().await;
    let batch = OwnerCombatBatch {
        caster: player,
        attacker: CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
        current_lease_generation: fence.character_lease_generation,
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(sequence).map_err(debug)?,
        ),
        occurrence: SpellOccurrenceBinding {
            id: format!("m9:herald-hit:{sequence}"),
            revisions: ["r1"; 5].map(str::to_owned),
        },
        binding: format!("m9:herald-hit:{sequence}").into_bytes(),
        anchor: None,
        now_us: Some(now),
        now_ms: now.get() / 1000,
        effects: vec![OwnerCombatEffect {
            target: boss,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: runtime.creature_spell_target_atom(boss).map_err(debug)?,
                magnitude,
            },
        }],
        deferred: None,
    };
    let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
    let hit = runtime.commit_spell_batch(staged).map_err(debug)?;
    let health = hit
        .effects
        .first()
        .and_then(|effect| effect.health)
        .ok_or("damage receipt")?;
    Ok((health.health_before, health.health_after))
}

struct HeraldFieldFixture([u8; 32]);

impl crate::durability::item_transfer::ItemWeightLookup for HeraldFieldFixture {
    fn generation_digest(&self) -> [u8; 32] {
        self.0
    }
    fn weight_of(
        &self,
        _: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        Ok(1)
    }
}

impl crate::durability::item_loop_move::ItemLoopContent for HeraldFieldFixture {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0
    }

    fn loop_facts(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<crate::durability::item_transfer::ItemDefinitionFacts> {
        use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
        if definition.family != "Item" || definition.revision_ref != "definition-r1" {
            return None;
        }
        let container = match definition.production_key.as_str() {
            "oteryn:item.tibia.i2854" => true,
            "oteryn:item.tibia.i2118" => false,
            _ => return None,
        };
        Some(ItemDefinitionFacts {
            definition: definition.clone(),
            weight: Some(1),
            stack: ItemStackClass::NonStackable,
            container_capacity: container.then_some(20),
            container_slot_equip_pattern: container,
        })
    }
}

async fn prepare_herald_peer(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    leader: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult<crate::gameplay_transport::connection::AdmittedSession> {
    use crate::gameplay_transport::connection::{AdmittedSession, ControllerBinding, FirstEntryOutcome, SessionContinuity};
    use crate::world_party_runtime_postgres_cases::{FreshPlayerOwners, committed_fresh_player_from_source_owned};
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&h.pool).await?;
    let (current, operation) = committed_fresh_player_from_source_owned(
        &FreshPlayerOwners { root: h.root.clone(), node: h.node.clone(), recovery: &h.recovery },
        60, 63, 171, now, Some("platform-test"),
    ).await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("actual peer fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(fence.game_session_id).map_err(debug)?;
        runtime.commit_fresh_session(reservation).map_err(debug)?
    };
    let mut continuity = SessionContinuity::FRESH;
    continuity.selected_capabilities = leader.continuity.selected_capabilities;
    let mut peer = AdmittedSession {
        game_session_id: fence.game_session_id, world_id: leader.world_id, channel_id: leader.channel_id,
        runtime_actor: Some(actor), first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("actual peer transport")?,
            account_id: fixture::id(63),
        }),
        continuity, item_fence: Some(fence),
    };
    super::srx3_activity_source_fixture::prepare_player_with_operation(
        h, owner, &mut peer, 12, "knight", &operation,
    ).await?;
    player_on(owner, actor, 32435, 32655).await?;
    owner.activity_runtimes.remember(peer).map_err(|_| "remember Herald peer")?;
    let leader = leader.item_fence.ok_or("actual leader fence")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq,shared_xp_enabled) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,3,true)")
        .bind(fixture::id(0xb9).as_slice()).bind(fixture::id(42).as_slice()).bind(leader.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($3,'hex')::uuid,1,1,encode($4,'hex')::uuid),(encode($2,'hex')::uuid,encode($3,'hex')::uuid,2,1,encode($4,'hex')::uuid)")
        .bind(leader.character_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fixture::id(0xb9).as_slice()).bind(fixture::id(43).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(peer)
}

async fn assert_periodic_exit(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    case: CallbackCase,
    peer: Option<crate::gameplay_transport::connection::AdmittedSession>,
) -> TestResult {
    use crate::foundation::condition::{ConditionDefinition, ConditionLineage, TickFacts};
    use crate::map::view::MapFacts;
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
    use oteryn_simulation_determinism::DecisionOccurrenceId;

    let map = owner.bundle_map().ok_or("Herald Instance map")?;
    let portal = TilePos {
        x: 32496,
        y: 32661,
        floor: -15,
    };
    let count_portals = || {
        map.facts
            .movement_items(portal)
            .iter()
            .filter(|item| item.facts.appearance_id == 25051)
            .count()
    };
    assert_eq!(count_portals(), 0);
    const MISSION: &str =
        "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/mission";
    let character = admitted
        .item_fence
        .ok_or("Herald recipient fence")?
        .character_id;
    let player_owned = matches!(case, CallbackCase::PoisonDeath | CallbackCase::FrozenDeadline | CallbackCase::StaleParty);
    let credited = !matches!(case, CallbackCase::UncreditedDeath);
    let xp_before: i64 = if credited {
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h,
            owner,
            &admitted,
            &[(MISSION.into(), 1)],
            0xa7,
        )
        .await?;
        owner.admit_quest_session(&admitted).await;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        // The prerequisite seeder writes its own XP receipt to advance CharacterRevision.
        sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?
    } else {
        0
    };
    if let Some(peer) = peer {
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h, owner, &peer, &[(MISSION.into(), 1)], 0xb8,
        ).await?;
        owner.admit_quest_session(&peer).await;
        owner.refresh_quest_session(peer.game_session_id, false).await;
        assert!(owner.runtime.lock().await.cached_world_party(
            admitted.runtime_actor.ok_or("Herald leader")?, admitted.game_session_id,
        ).map_err(debug)?.is_some_and(|party| party.shared_xp_enabled));
        sqlx::query("UPDATE game_parties SET revision=revision+1, shared_xp_enabled=false WHERE party_id=encode($1,'hex')::uuid")
            .bind(fixture::id(0xb9).as_slice()).execute(&h.pool).await?;
    }
    let boss = owner
        .runtime
        .lock()
        .await
        .visible_entities()
        .creatures
        .first()
        .ok_or("Herald")?
        .actor;
    if credited {
        assert_eq!(
            commit_fixture_hit(owner, admitted, boss, 94, 125).await?,
            (900, 775)
        );
    }
    let frozen_at = if matches!(case, CallbackCase::FrozenDeadline) {
        let deadline: i64 = sqlx::query_scalar(
            "SELECT deadline FROM game_boss_room_activities WHERE boss_key=$1",
        ).bind(BOSS).fetch_one(&h.pool).await?;
        let database_now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
        ).fetch_one(&h.pool).await?;
        if deadline >= database_now {
            tokio::time::sleep(std::time::Duration::from_millis(u64::try_from(deadline - database_now)? + 2)).await;
        }
        owner.states.attack.lock().await.kills().seed_herald_death_clock(boss, deadline);
        Some(deadline)
    } else {
        None
    };
    let element = if player_owned {
        DotElement::Poison
    } else {
        DotElement::Energy
    };
    let source = if player_owned {
        Some(admitted.runtime_actor.ok_or("poison caster")?)
    } else {
        None
    };
    let now = owner.owner_now();
    {
        let mut runtime = owner.runtime.lock().await;
        let mut spell_states = owner.states.spell_states.lock().await;
        let lineage = if let Some(actor) = source {
            let fence = admitted.item_fence.ok_or("poison caster fence")?;
            assert!(
                spell_states
                    .initialize(
                        &runtime,
                        actor,
                        admitted.game_session_id,
                        crate::gameplay_transport::actor_spell::tests::FACTS,
                        (0, 0),
                        now,
                    )
                    .is_some()
            );
            let digest = runtime.content_pin().server_artifact_digest();
            Some(ConditionLineage {
                source_character: *fence.character_id.as_bytes(),
                source_session: *admitted.game_session_id.as_bytes(),
                source_lease_generation: fence.character_lease_generation,
                source_actor_placement: actor.placement_identity(),
                source_scope_generation: actor.scope_generation().get(),
                source_content_digest: digest,
                source_catalog_digest: digest,
                source_creation_command: 95,
            })
        } else {
            None
        };
        let field = CreatureFieldEntry {
            definition: ConditionDefinition::new(
                "fixture.herald.environment",
                1,
                ConditionValues::DamageOverTime {
                    element,
                    total_min: 2000,
                    total_max: 2000,
                    per_tick: 2000,
                    interval_ms: 1000,
                    delayed: false,
                },
            )
            .ok_or("uncredited field condition")?,
            source,
            lineage,
            occurrence: DecisionOccurrenceId::from_bytes([13; 16]),
        };
        runtime
            .commit_creature_field_entry(boss, now.get(), Some(&field))
            .map_err(|error| format!("field entry: {error:?}"))?;
        if peer.is_none() {
        let hit = spell_states
            .tick_source_creature_conditions(
                &mut runtime,
                boss,
                TickFacts {
                    in_protection_zone: false,
                    standing_on_field: Some(element),
                },
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
            )
            .map_err(|error| format!("poison turn: {error:?}"))?;
        assert_eq!(hit.health_after, 0);
        assert!(
            hit.impacts
                .iter()
                .any(|impact| impact.source == source && impact.damage > 0)
        );
        }
    }
    if peer.is_some() {
        owner.drain_monster_melee_at(now).await;
    } else {
        owner.drain_monster_melee().await;
    }
    if credited {
        assert!(
            owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(admitted.game_session_id),
            "a lethal tick after player damage must enqueue the sealed death"
        );
        if let Some(deadline) = frozen_at {
            assert_eq!(
                owner.states.attack.lock().await.kills().herald_death_times(boss),
                Some((deadline, u64::try_from(deadline)?)),
                "the room and quest must use the same exact deadline millisecond",
            );
        }
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
        let rewards: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_xp_receipts WHERE character_id=encode($2,'hex')::uuid AND experience_awarded>0),
                    (SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'),
                    (SELECT value FROM game_character_quest_tracks WHERE track_key=$1 AND character_id=encode($2,'hex')::uuid)",
        ).bind(MISSION).bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(rewards, (xp_before + 1, 1, 3));
        if let Some(peer) = peer {
            let character = peer.item_fence.ok_or("Herald peer fence")?.character_id;
            let value: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1 AND character_id=encode($2,'hex')::uuid"
            ).bind(MISSION).bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(value, 1, "the idle peer's old roster must not grant Burning Heart credit");
        }
    } else {
        let corpses: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'",
        ).fetch_one(&h.pool).await?;
        assert_eq!(corpses, 0);
    }
    assert_eq!(count_portals(), 1);
    owner.drain_monster_melee().await;
    assert_eq!(count_portals(), 1);
    Ok(())
}

async fn assert_fire_field_attack(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    case: CallbackCase,
) -> TestResult {
    use oteryn_simulation_determinism::SemanticTimeMicros;
    let map = owner.bundle_map().ok_or("Herald attack map")?;
    let scope = admitted.item_fence.ok_or("Herald fence")?.runtime_scope;
    let grid = (32492..=32500)
        .step_by(2)
        .flat_map(|x| {
            (32652..=32660)
                .step_by(2)
                .map(move |y| TilePos { x, y, floor: -15 })
        })
        .collect::<std::collections::BTreeSet<_>>();
    let telegraphs = || {
        map.facts
            .movement_effects_since(0)
            .into_iter()
            .filter(|(_, _, effect)| *effect == 53557)
            .map(|(_, tile, _)| tile)
            .collect::<Vec<_>>()
    };
    let start = owner.owner_now().get();
    let at = |offset| {
        let now = start + offset;
        owner.states.test_owner_time.store(now + 1, std::sync::atomic::Ordering::Relaxed);
        SemanticTimeMicros::from_micros(now)
    };
    owner.drain_monster_melee_at(at(0)).await;
    owner.drain_monster_melee_at(at(7_999_999)).await;
    assert!(telegraphs().is_empty());
    owner.drain_monster_melee_at(at(8_000_000)).await;
    let cues = telegraphs();
    assert_eq!(cues.len(), 25);
    assert_eq!(
        cues.into_iter().collect::<std::collections::BTreeSet<_>>(),
        grid
    );
    owner.drain_monster_melee_at(at(9_499_999)).await;
    assert!(
        owner
            .root
            .read_item_loop_ground_for_scope(scope)
            .await?
            .is_empty()
    );
    if matches!(case, CallbackCase::FieldProjection) {
        return assert_refresh_blocks_due_poison(owner, at(9_500_000)).await;
    }
    owner.drain_monster_melee_at(at(9_500_000)).await;
    let cells = owner.root.read_item_loop_ground_for_scope(scope).await?;
    assert_eq!(cells.len(), 25);
    assert!(
        cells
            .iter()
            .all(|row| row.item.definition.production_key == "oteryn:item.tibia.i2118")
    );
    if matches!(case, CallbackCase::FieldStep) {
        return assert_field_step(owner, admitted).await;
    }
    if matches!(case, CallbackCase::RedFloors) {
        return assert_red_floors(owner, admitted).await;
    }
    owner.drain_monster_melee_at(at(16_000_000)).await;
    assert_eq!(telegraphs().len(), 50);
    owner.drain_monster_melee_at(at(17_500_000)).await;
    let refreshed = owner.root.read_item_loop_ground_for_scope(scope).await?;
    assert_eq!(refreshed.len(), 25);
    assert!(refreshed.iter().all(|row| {
        cells
            .iter()
            .all(|old| old.item.item_instance_id != row.item.item_instance_id)
    }));
    Ok(())
}

async fn lever_use(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    target: WorldObjectTarget,
) -> TestResult<crate::gameplay_transport::connection::UseOutcome> {
    if let Some(outcome) = owner
        .use_event_world_object(actor, command, target.clone())
        .await
        .map_err(|_| "unresolved Herald USE")?
    {
        return Ok(outcome);
    }
    Ok(owner.use_world_object(actor, command, target).await)
}

async fn assert_refresh_blocks_due_poison(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    due: oteryn_simulation_determinism::SemanticTimeMicros,
) -> TestResult {
    use crate::foundation::condition::ConditionDefinition;
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
    use oteryn_simulation_determinism::DecisionOccurrenceId;

    let boss = owner.runtime.lock().await.visible_entities().creatures[0].actor;
    let poison = CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "fixture.herald.pending_refresh_poison",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Poison,
                total_min: 125,
                total_max: 125,
                per_tick: 125,
                interval_ms: 1000,
                delayed: false,
            },
        ).ok_or("poison condition")?,
        source: None,
        lineage: None,
        occurrence: DecisionOccurrenceId::from_bytes([14; 16]),
    };
    crate::gameplay_transport::loop_move::fail_next_herald_projection(move |runtime| {
        assert!(runtime.commit_creature_field_entry(boss, due.get(), Some(&poison)).is_ok());
    });
    assert_eq!(owner.states.spell_states.lock().await.herald_field_schedule.next_due(),
        Some(due.get()), "the telegraphed refresh is due");
    owner.drain_monster_melee_at(due).await;
    let scope = owner.bundle_map().ok_or("Herald map")?.overlay.runtime_scope();
    assert_eq!(owner.root.read_item_loop_ground_for_scope(scope).await?.len(), 25);
    assert_eq!(
        owner.states.spell_states.lock().await.herald_field_schedule.next_due(),
        Some(due.get()),
        "the committed occurrence stays pending until Ground is projected",
    );
    assert_eq!(owner.runtime.lock().await.live_creature_health(boss), Some(900),
        "committed fire must protect Herald while its projection is unavailable");
    owner.drain_monster_melee_at(due).await;
    assert_eq!(owner.runtime.lock().await.live_creature_health(boss), Some(900));
    Ok(())
}

async fn player_on(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    x: i32,
    y: i32,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let mut movement = runtime.borrow_movement_position();
    let mut current = movement.read(actor).map_err(debug)?;
    while (current.position().x, current.position().y) != (x, y) {
        let at = current.position();
        let next = if at.x != x {
            crate::foundation::MovementLocalPosition { x: at.x + (x - at.x).signum(), ..at }
        } else {
            crate::foundation::MovementLocalPosition { y: at.y + (y - at.y).signum(), ..at }
        };
        current = movement.commit_cardinal(current, next).map_err(debug)?;
    }
    Ok(())
}

async fn assert_field_step(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    let actor = admitted.runtime_actor.ok_or("Herald participant")?;
    let session = admitted.game_session_id;
    assert!(owner.required_capabilities.effective().binary_search(&30).is_err());
    let hp = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        states.get(&runtime, actor, session).ok_or("initialized participant")?.vitals().health
    };
    player_on(owner, actor, 32493, 32660).await?;
    let (outcome, _) = owner.paced_step(actor, session, StepDirection::East).await;
    assert_eq!(outcome.disposition, StepDisposition::Moved);
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let cast = states.get(&runtime, actor, session).ok_or("initialized participant")?;
    assert!(cast.vitals().health < hp || !cast.owned_conditions().instances().is_empty(),
        "an exact Herald fire field must apply environmental contact with capability 30 optional");
    Ok(())
}

async fn assert_red_floors(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    let actor = admitted.runtime_actor.ok_or("Herald participant")?;
    let map = owner.bundle_map().ok_or("Herald map")?;
    let scope = map.overlay.runtime_scope();
    let plates = [(32496, 32649), (32503, 32656), (32496, 32663)];
    let appearances = || plates.map(|(x, y)| {
        let tile = TilePos { x, y, floor: -15 };
        view::compose_tile(&map.overlay, &map.facts, tile).ok().flatten().and_then(|tile| {
            tile.entries.into_iter().find_map(|entry| {
                let id = entry.facts.appearance_id;
                matches!(id, 53095..=53096).then_some(id)
            })
        })
    });
    assert_eq!(appearances(), [Some(53096); 3]);
    for (i, (x, y)) in plates.into_iter().enumerate() {
        let (from_y, direction) = if i == 0 {
            (y + 1, StepDirection::North)
        } else {
            (y - 1, StepDirection::South)
        };
        player_on(owner, actor, i32::from(x), i32::from(from_y)).await?;
        let (outcome, duration) = owner.paced_step(actor, admitted.game_session_id, direction).await;
        assert_eq!(outcome.disposition, StepDisposition::Moved);
        let expected = if i < 2 {
            std::array::from_fn(|plate| Some(if plate <= i { 53095 } else { 53096 }))
        } else {
            [Some(53096); 3]
        };
        assert_eq!(appearances(), expected, "Crystal red-floor StepIn state");
        assert_eq!(owner.root.read_item_loop_ground_for_scope(scope).await?.len(), if i == 2 { 0 } else { 25 });
        if let Some(duration) = duration {
            owner.states.test_owner_time.fetch_add(
                u64::try_from(duration.as_micros())? + 1,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
    }
    assert!(owner.runtime.lock().await.activity_herald_is_alive());
    Ok(())
}
