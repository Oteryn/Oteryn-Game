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
        scenario(false, false)
    })
}

#[test]
fn herald_lever_refuses_wrong_pad_and_replays_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("herald-source-lever-replay", || {
        scenario(true, false)?;
        scenario(false, true)
    })
}

fn scenario(wrong_pad: bool, replay: bool) -> TestResult {
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_EXECUTED herald_lever wrong_pad={wrong_pad} replay={replay}");
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
                NativeEntryStart { x: 32433, y: 32653, floor: 15 }
            } else {
                NativeEntryStart { x: 32435, y: 32654, floor: 15 }
            };
            let room = source_room.bundle_view(
                cells.clone(), bundle.frame_binding_digest(world_id), bundle.map_revision_digest(), start,
            ).ok_or("source bundle combat room")?;
            let (templates, diagnostics) = controller.qualify_initial_activity_template(
                &source_room, &room, "content-1", &candidates);
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
            super::item_look_fixtures::seed_main_backpack(&h, &[]).await?;
            owner.register_current_runtime().await.map_err(|_| "register Channel runtime")?;
            owner.activity_runtimes.remember(admitted).map_err(|_| "register admitted session")?;
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
                assert_eq!(owner.use_world_object(deps.actor, command, target).await.disposition,
                    UseDisposition::Rejected);
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_handoff_groups")
                    .fetch_one(&h.pool).await?;
                assert_eq!(count, 0);
                assert!(owner.activity_runtimes.get(instance, 1).is_none());
                return Ok(());
            }
            let command = UseCommand { command_id: 2, ..command };
            let outcome = owner.use_world_object(deps.actor, command, target.clone()).await;
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            let switch = owner.activity_runtimes.pending_switch(admitted).map_err(|_| "scope switch")?
                .ok_or("committed Instance handoff")?;
            assert_eq!(switch.destination.item_fence.ok_or("Instance fence")?.runtime_scope, instance);
            if !replay {
                owner.activity_runtimes.finish_switch(admitted, switch.destination)
                    .map_err(|_| "finish source scope switch")?;
                let mut destination = owner.resolve_runtime_view(switch.destination).await
                    .map_err(|_| "resolve Herald Instance")?.ok_or("current Herald Instance")?;
                assert_herald_callbacks(&mut destination, switch.destination).await?;
                let deadlines: (i64, Option<i64>) = sqlx::query_as(
                    "SELECT deadline, exit_deadline FROM game_boss_room_activities WHERE boss_key=$1",
                ).bind(BOSS).fetch_one(&h.pool).await?;
                assert_eq!(deadlines.1, Some(deadlines.0));
            }
            if replay {
                assert_eq!(owner.use_world_object(deps.actor, command, target).await.disposition, UseDisposition::Committed);
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
            "POSTGRES_CASE_COMPLETED herald_lever wrong_pad={wrong_pad} replay={replay} elapsed={:?}",
            started.elapsed()
        );
    } else {
        eprintln!(
            "POSTGRES_CASE_FAILED herald_lever wrong_pad={wrong_pad} replay={replay} elapsed={:?}",
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
