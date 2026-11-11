//! Source entry supplied by the Feroxa encounter owner; the moon alone never creates it.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};

const ENTRY: &str = "oteryn:quest-callback/grimvale-full-moon-entry";
const TRIGGER: Position = Position {
    x: 33430,
    y: 31537,
    floor: -11,
};
const DESTINATION: Position = Position {
    x: 33419,
    y: 31539,
    floor: -10,
};

#[test]
fn worldchg_grimvale_native_entry_requires_current_moon_without_quest_writes() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-grimvale", scenario)
}

fn scenario() -> TestResult {
    scenario_with_bindings(false)
}

pub(super) fn scenario_with_bindings(prove_bindings: bool) -> TestResult {
    // Fixture state is the real encounter's already-open source portal. This does not
    // assert that a full moon fires Feroxa or fabricate the donor's five-percent draw.
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": ENTRY, "quest": "oteryn:quest.grimvale_quest", "trigger": "step_in",
        "selector": {"position":TRIGGER,"unique":null,"action":null,"appearance":22761},
        "lever":null,"reversible":false,"passages":[],"teleport":DESTINATION,
        "storage":[],"offerings":[],"item_on_target":null,"return_on_failure":true,
        "failure_message":null,
        "sources":["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/lib/quests/grimvale.lua:34-45"]
    }))?;
    let world = super::quest_world_callbacks::world(&callback, TRIGGER)?;
    run(async |admin| {
        let start = std::time::Instant::now();
        let h =
            Harness::create_with_admission_seed(admin, "worldchg_grimvale", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let origin = super::quest_world_callbacks::local(Position {
            x: TRIGGER.x - 1,
            ..TRIGGER
        });
        deps.combat_room = Some(
            activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    activated.frame_binding().digest(),
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: origin.x,
                        y: origin.y,
                        floor: origin.floor,
                    },
                )
                .ok_or("qualified Grimvale bundle room")?,
        );
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        assert!(
            owner
                .states
                .quest_callbacks
                .fixture
                .set(vec![callback.clone()])
                .is_ok()
        );
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;

        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let native = generation.native_gameplay().ok_or("native")?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i2854".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        let world_id = admitted.world_id;
        for (epoch, day) in [(0_i64, 12_i32), (1, 13), (2, 13)] {
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,$4,12,0,0,'UTC'),'2031-06')")
                .bind(world_id.as_bytes().as_slice()).bind(epoch).bind([1_u8;32].as_slice()).bind(day).execute(&h.pool).await?;
        }
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(world_id.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        let inactive = h
            .root
            .world_changes(world_id, 0)
            .await?
            .ok_or("inactive day")?;
        assert!(!inactive.grimvale_full_moon);
        owner.reset_epoch = 0;
        let source_key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: TRIGGER.x,
                y: TRIGGER.y,
                floor: TRIGGER.floor,
            },
            1,
        )
        .ok_or("entry key")?;
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .object_revision(source_key),
            0
        );
        let before = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let east = encode_step_intent(StepDirection::East);
        super::quest_world_callbacks::wire(
            &owner,
            &mut admitted,
            1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            east.clone(),
        )
        .await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            origin,
            "an already-open encounter portal cannot bypass the current moon predicate"
        );
        assert_eq!(receipt_count(&h).await?, 0);
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .object_revision(source_key),
            0,
            "refused entry cannot mutate its native map object"
        );

        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=1 WHERE world_id=encode($1,'hex')::uuid")
            .bind(world_id.as_bytes().as_slice()).execute(&h.pool).await?;
        let active = h
            .root
            .world_changes(world_id, 1)
            .await?
            .ok_or("active day")?;
        assert!(active.grimvale_full_moon);
        if prove_bindings {
            super::engworldchg::assert_binding_lifecycle(&h, world_id, 1).await?;
        }
        owner.reset_epoch = 1;
        super::quest_world_callbacks::wire(
            &owner,
            &mut admitted,
            2,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            east.clone(),
        )
        .await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(DESTINATION)
        );
        assert_eq!(
            receipt_count(&h).await?,
            0,
            "stateless world observation is not personal quest progress"
        );
        let after = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(after.tracks(), before.tracks());
        // No second portal, reward or receipt is created by a repeated native command.
        super::quest_world_callbacks::wire(
            &owner,
            &mut admitted,
            2,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            east,
        )
        .await?;
        assert_eq!(receipt_count(&h).await?, 0);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(DESTINATION)
        );
        let query_fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "query fence")?
            .ok_or("query fence")?;
        let context = {
            let runtime = owner.runtime.lock().await;
            let snapshot = runtime.read_actor_position(deps.actor).map_err(debug)?;
            crate::durability::quest_world_callback::QuestWorldContext {
                party: Vec::new(),
                map_revision: crate::map::overlay::map_revision(
                    owner.bundle_map().ok_or("map")?.overlay.base(),
                ),
                content_revision: format!(
                    "sha256:{}",
                    runtime
                        .content_pin()
                        .server_artifact_digest()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: 1,
                actor_placement: deps.actor.placement_identity(),
                movement_revision: snapshot.revision(),
            }
        };
        let permit = owner.spell_lane_permit().await.ok_or("query permit")?;
        for _ in 0..2 {
            assert!(
                h.root
                    .observe_world_callback_gate(
                        &authority,
                        &h.node,
                        &permit,
                        query_fence,
                        context.clone(),
                        ENTRY.into()
                    )
                    .await?
            );
        }
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid")
            .bind(world_id.as_bytes().as_slice()).execute(&h.pool).await?;
        h.root.world_changes(world_id, 2).await?.ok_or("next day")?;
        assert!(
            !h.root
                .observe_world_callback_gate(
                    &authority,
                    &h.node,
                    &permit,
                    query_fence,
                    context,
                    ENTRY.into()
                )
                .await?,
            "a retained moon snapshot cannot authorize the next save"
        );
        assert_eq!(receipt_count(&h).await?, 0);
        drop(permit);
        let registered = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == ENTRY)
            .ok_or("production moon entry registration")?;
        assert_eq!(registered.selector, callback.selector);
        assert_eq!(registered.teleport, callback.teleport);
        assert!(registered.storage.is_empty());
        if prove_bindings {
            super::engworldchg::assert_native_quest_transition(
                &h,
                &owner,
                admitted.game_session_id,
                "grimvale",
                "oteryn:quest.grimvale_mini_world_change",
            )
            .await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert!(start.elapsed().as_secs_f64() > 1.0);
        eprintln!(
            "WORLDCHG-Q PG EXECUTED: Grimvale native stateless entry, current moon refusal/allow, replay, quest state unchanged; elapsed={:?}",
            start.elapsed()
        );
        Ok(())
    })
}

async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(ENTRY)
    .fetch_one(&h.pool)
    .await?)
}

fn producer_world(
    callback: &Callback,
    trigger: Position,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    // Physical monster reads require an admitted source ID, not engineering terrain:grass.
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i104".into();
    let mut cells = std::collections::BTreeMap::new();
    for x in trigger.x - 2..=trigger.x + 8 {
        for y in trigger.y - 1..=trigger.y + 1 {
            cells.insert((trigger.floor, x, y), Vec::new());
        }
    }
    for position in callback
        .passages
        .iter()
        .map(|p| p.position)
        .chain(callback.offerings.iter().map(|p| p.position))
        .chain(callback.teleport)
    {
        cells
            .entry((position.floor, position.x, position.y))
            .or_default();
    }
    let appearance = 1949;
    data.manifest.palette.push(bundle::PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{appearance}"),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: None,
        }),
    });
    data.manifest.palette.push(bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i22761".into(),
        family: bundle::Family::Terrain,
        id: 2,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: None,
        }),
    });
    // Retained middle.otbm contains three appearance-only13841 decorations.
    // They have no portable Item definition. A qualified source World must serve
    // their fixed Terrain identity; missing definitions still refuse the phase.
    data.manifest.palette.push(bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i13841".into(),
        family: bundle::Family::Terrain,
        id: 3,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Border,
            walkable: None,
            ground_speed: None,
            blocks_projectile: Some(false),
        }),
    });
    for (floor, x, y) in [(-11, 33380, 31537), (-11, 33457, 31531), (-10, 33420, 31539)] {
        cells.entry((floor, x, y)).or_default();
    }
    cells
        .entry((trigger.floor, trigger.x, trigger.y))
        .or_default()
        .push(sector::Item {
            palette: 1,
            depth: 0,
            attrs: sector::Attrs {
                teleport: Some((33457, 31531, (-11_i8).to_ne_bytes()[0])),
                unique: callback.selector.unique,
                action: callback.selector.action,
                ..Default::default()
            },
        });
    let floors: std::collections::BTreeSet<_> = cells.keys().map(|(f, _, _)| *f).collect();
    data.manifest.world = bundle::Extent {
        min_x: cells.keys().map(|(_, x, _)| *x).min().ok_or("min")?,
        min_y: cells.keys().map(|(_, _, y)| *y).min().ok_or("min")?,
        max_x: cells.keys().map(|(_, x, _)| *x).max().ok_or("max")? + 1,
        max_y: cells.keys().map(|(_, _, y)| *y).max().ok_or("max")? + 1,
        floors: floors.into_iter().collect(),
    };
    let mut sectors: std::collections::BTreeMap<(i8, u16, u16), Vec<sector::Tile>> =
        std::collections::BTreeMap::new();
    for ((floor, x, y), mut items) in cells {
        items.insert(
            0,
            sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            },
        );
        sectors
            .entry((floor, x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let mut sectors = sectors
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|t| (t.y, t.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: trigger.x - 1,
                y: trigger.y,
                floor: trigger.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

#[test]
fn worldchg_grimvale_final_native_wave_projects_actual_source_portal() -> TestResult {
    run_producer(false, false, false, false, false, false, false)
}

#[test]
fn qn5_feroxa_final_opens_portal_when_boss_spawn_is_blocked() -> TestResult {
    run_producer(true, false, false, false, false, false, false)
}

#[test]
fn qn5_feroxa_timed_native_maps_ignore_pending_delivery_and_reject_stale_reset() -> TestResult {
    run_producer(false, true, false, false, false, false, false)
}

#[test]
fn qn5_feroxa_spectators_return_to_saved_positions_once_on_both_source_timers() -> TestResult {
    run_producer(false, true, true, false, false, false, false)
}

#[test]
fn qn5_feroxa_map_phase_retires_actual_ground_once_with_world_event_audit() -> TestResult {
    run_producer(false, true, false, true, false, false, false)
}

// Crystal201189f8 data-global/lib/quests/grimvale.lua:25-38 retains the same
// actual 22761 ingress and 33419,31539,10 landing as the pinned9f5a72c donor.
// The personal entry stage is the explicit chosen Oteryn recipe, not source completion.
#[test]
fn qn5_grimvale_authored_entry_commits_through_real_movement_once() -> TestResult {
    run_producer(false, false, false, false, true, false, false)
}

#[test]
fn qn5_feroxa_crowded_corpses_clear_in_bounded_recoverable_batches() -> TestResult {
    run_producer(false, true, false, true, false, true, false)
}

#[test]
fn qn5_feroxa_new_owner_clears_previous_owner_corpse_custody() -> TestResult {
    run_producer(false, true, false, true, false, false, true)
}

fn run_producer(block_spawn: bool, prove_phases: bool, prove_spectators: bool, prove_clear: bool, prove_stage: bool, prove_batches: bool, prove_handoff: bool) -> TestResult {
    run_producer_proofs(ProducerProofs { block_spawn, prove_phases, prove_spectators, prove_clear, prove_stage, prove_batches, prove_handoff, large_tree: false })
}

#[test]
fn qn5_feroxa_oversized_tree_keeps_root_live_until_bounded_leaves_commit() -> TestResult {
    run_producer_proofs(ProducerProofs { block_spawn: false, prove_phases: true, prove_spectators: false, prove_clear: true, prove_stage: false, prove_batches: false, prove_handoff: false, large_tree: true })
}

fn run_producer_proofs(proofs: ProducerProofs) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-grimvale-producer", move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "worldchg_grimvale_producer",
                false,
                false,
            )
            .await?;
            let result = producer_scenario(&h, proofs).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })
}

use crate::durability::raid_runner::CommittedRaidAction;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{RaidCatalogue, RaidPosition, SelectedRaid};
async fn producer_seed(
    harness: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    key: &ProductionKey,
    selected: &SelectedRaid,
    firing: [u8; 16],
    digest: [u8; 32],
) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'raid-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,13,12,0,0,'UTC'),'2031-06')")
        .bind(owner.world_id.as_bytes().as_slice()).bind(digest.as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .execute(&harness.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,60000,0,NULL,false)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,0,$4,$5,1)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).bind(firing.as_slice())
        .bind(i64::from(selected.definition.waves.iter().map(|wave| wave.delay_ms).max().ok_or("wave")?)
            + i64::from(selected.definition.despawn_after_ms.unwrap_or(0))).bind(digest.as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    Ok(())
}

struct ProducerProofs {
    block_spawn: bool,
    prove_phases: bool,
    prove_spectators: bool,
    prove_clear: bool,
    prove_stage: bool,
    prove_batches: bool,
    prove_handoff: bool,
    large_tree: bool,
}

async fn producer_scenario(harness: &Harness, proofs: ProducerProofs) -> TestResult {
    let ProducerProofs { block_spawn, prove_phases, prove_spectators, prove_clear, prove_stage, prove_batches, prove_handoff, large_tree } = proofs;
    use crate::foundation::{NodeId, ScopeOwnershipGeneration};
    use crate::map::boot::BundleCollisionIndexExt;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let key = "raid:edron.feroxa";
    let expected = 1;
    let start = std::time::Instant::now();
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("QN5_M52_FORMS_POSTGRES_READY version={version}");
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let digest = active.identity().server_artifact_digest();
    let firing = id(91);
    let key = ProductionKey::new(key).map_err(debug)?;
    let phased_boss = key.as_str() == "raid:venore.dracola";
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let selected = catalogue
        .get(&key)
        .ok_or("catalogue raid")?
        .select(digest, firing)
        .map_err(debug)?;
    let mut area = *selected.areas.values().next().ok_or("source area")?;
    assert_eq!(selected.areas.len(), if phased_boss { 2 } else { 1 });
    for source_area in selected.areas.values() {
        assert_eq!(source_area.from.floor, area.from.floor);
        assert_eq!(source_area.to.floor, area.to.floor);
        area.from.x = area.from.x.min(source_area.from.x);
        area.from.y = area.from.y.min(source_area.from.y);
        area.to.x = area.to.x.max(source_area.to.x);
        area.to.y = area.to.y.max(source_area.to.y);
    }
    assert_eq!(area.from.floor, area.to.floor);
    assert_eq!(selected.definition.announcements.len(), 3);
    let from = (u16::try_from(area.from.x)?, u16::try_from(area.from.y)?);
    let to = (u16::try_from(area.to.x)?, u16::try_from(area.to.y)?);
    let native_floor = i8::try_from(area.from.floor.checked_neg().ok_or("native floor")?)?;
    let _ = (from, to, native_floor);
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == ENTRY)
        .ok_or("Grimvale native callback")?;
    let world = producer_world(&callback, TRIGGER)?;
    deps.room = activated.qualified_room().clone();
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return Err("bundle collision index".into());
    };
    let bundle_digest = index.map().overlay.base().digest();
    let selected = catalogue
        .get(&key)
        .ok_or("catalogue raid")?
        .select(bundle_digest, firing)
        .map_err(debug)?;
    assert_eq!(selected.definition.waves.len(), 1);
    assert_eq!(selected.definition.waves[0].delay_ms, 75 * 60_000);
    let first_area = selected
        .areas
        .get(&selected.definition.waves[0].entries[0].area_anchor)
        .ok_or("first source spawn area")?;
    assert_eq!(
        first_area.from,
        RaidPosition {
            x: 33380,
            y: 31537,
            floor: 11
        }
    );
    let room = deps
        .room
        .bundle_view(
            cells.clone(),
            world.frame_binding_digest(deps.admitted.world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: i32::from(TRIGGER.x) - 1,
                y: i32::from(TRIGGER.y),
                floor: -i16::from(TRIGGER.floor),
            },
        )
        .ok_or("bundle combat view")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        digest,
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (
            i32::from(TRIGGER.x) - 1,
            i32::from(TRIGGER.y),
            -i16::from(TRIGGER.floor),
        ),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        expected + 5, // four players plus one transient source create-before-remove body
        pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut extra_players = Vec::new();
    for (n, at) in [
        (81, Position { x: TRIGGER.x - 2, ..TRIGGER }),
        (82, if block_spawn {
            // An actual current player occupies the source's only spawn tile.
            Position { x: 33380, y: 31537, floor: -11 }
        } else { Position { x: 33457, y: 31531, floor: -11 } }),
        (83, DESTINATION),
    ] {
        let session = crate::foundation::GameSessionId::decode(&id(n)).map_err(debug)?;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime.initialize_pinned_test_position(actor, super::quest_world_callbacks::local(at)).map_err(debug)?;
        extra_players.push((actor, at));
    }
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.combat_room = Some(room);
    let (consumer, clock, activation, _) = crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
    if prove_stage {
        harness.root.latch_premium_activation(&activation.activation_id,
            activation.switch_over_us, activation.switch_over_us + 60_000_000).await.map_err(debug)?;
    }
    let premium = crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
        consumer, Some(clock), Some(activation), Some(harness.root.clone()));
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.reset_epoch = 1;
    let mut admitted = deps.admitted;
    if prove_stage {
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?.catalogue().clone()));
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), admitted.game_session_id);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner.current_quest_fence(admitted.game_session_id).await
            .map_err(|()| "stage fence")?.ok_or("stage fence")?;
        admitted.item_fence = Some(fence.item_fence());
        // Use the actual runtime frame, exactly as ordinary content activation does.
        // The raid-only fixture previously never needed a personal quest write.
        let frame = owner.runtime.lock().await.content_pin().frame_binding_digest();
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
            .execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
            .bind(digest.as_slice()).bind(active.identity().client_artifact_digest().as_slice())
            .bind(frame.as_slice()).execute(&harness.pool).await?;
        owner.runtime.lock().await.bind_attacker_lease(deps.actor, admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation).map_err(debug)?).map_err(debug)?;
        deps.initialize(&owner).await;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
    }
    producer_seed(harness, &owner, &key, &selected, firing, bundle_digest).await?;
    assert!(
        harness
            .root
            .world_changes(owner.world_id, 1)
            .await?
            .ok_or("moon")?
            .grimvale_full_moon
    );
    assert!(crate::map::floor::is_teleport(
        &cells,
        crate::content::LogicalCell {
            x: i32::from(TRIGGER.x),
            y: i32::from(TRIGGER.y),
            z: 11
        }
    ));
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let mut clear_corpses = Vec::new();
    if prove_phases {
        owner.apply_feroxa_map_clock(firing, 0).await.map_err(|_| "new firing before first announcement")?;
        let pending = harness.root.advance_raid_run(&harness.node, firing, owner.channel_id,
            generation, selected.definition.clone(), 0).await?.ok_or("pending announcement")?;
        assert!(matches!(pending.action, CommittedRaidAction::Announcement { .. }));
        let at = crate::map::overlay::TilePos { x: 33421, y: 31524, floor: -11 };
        if prove_clear {
            for (x, y, floor, expected) in [(33415_i32,31522_i32,11_i16,true),
                (33445,31554,11,true),(33414,31522,11,false),(33446,31522,11,false),
                (33415,31521,11,false),(33415,31555,11,false),(33415,31522,10,false)] {
                let mut encoded = Vec::new();
                encoded.extend_from_slice(&x.to_be_bytes()); encoded.extend_from_slice(&y.to_be_bytes());
                encoded.extend_from_slice(&floor.to_be_bytes());
                let inside: bool = sqlx::query_scalar("SELECT game_feroxa_clear_position($1)")
                    .bind(encoded).fetch_one(&harness.pool).await?;
                assert_eq!(inside, expected, "exact source inner rectangle/floor");
            }
            seed_feroxa_clear_item(harness, &owner, id(112), 33428, 31536).await?;
            seed_feroxa_clear_item(harness, &owner, id(114), 33457, 31531).await?;
            for n in 0..if large_tree { 32 } else if prove_batches { 30 } else { 1 } {
                clear_corpses.push(mint_feroxa_clear_corpse(harness, &owner, 119 + n,
                    if large_tree { 17 } else if prove_batches { 16 } else { 1 }, !prove_batches && !prove_handoff && !large_tree).await?);
            }
        }
        if large_tree {
            import_feroxa_large_tree(harness, &clear_corpses).await?;
        }
        if prove_handoff {
            // Install a coherent committed control-plane successor as fixture setup.
            // The tested caller must independently prove it in its ordinary SQL pass.
            use crate::durability::runtime_scope_assignment::{AssignmentCommand,AssignmentRequest,AssignmentOutcome,ControlActor,OperationKey,RuntimeScopeAssignmentWriter};
            let scope = owner.runtime.lock().await.binding().runtime_scope();
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
                .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
            let predecessor = harness.root.read_runtime_scope_predecessor(scope).await.map_err(debug)?.ok_or("predecessor")?;
            let writer = RuntimeScopeAssignmentWriter::open(harness.root.clone(),"sequencer-writer").await.map_err(debug)?;
            let outcome = writer.submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([218;32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace { scope, predecessor, target: harness.node.fact() },
            }).await.map_err(debug)?;
            let AssignmentOutcome::Committed(receipt) = outcome else { return Err(debug(outcome)); };
            assert_eq!(receipt.assignment.ownership_generation,2);
            use crate::foundation::admission_authority_publication::{AdmissionAuthorityGuardKeyV1,AdmissionAuthorityGuardStateV1,AdmissionAuthorityPublicationV1,AdmissionPublicationPreconditionV1};
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&harness.pool).await?;
            let mut source = super::fresh_fixture::fresh::Source::new(now)?;
            source.rows.retain(|r| matches!(r.key,AdmissionAuthorityGuardKeyV1::Runtime(_)));
            let guards = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(harness.root.clone());
            let prior = guards.load(&[AdmissionAuthorityGuardKeyV1::Runtime(scope)]).await.map_err(debug)?
                .into_iter().next().flatten().ok_or("current runtime publication")?;
            for row in &mut source.rows {
                if let AdmissionAuthorityGuardStateV1::Runtime { ownership_generation, .. } = &mut row.state {
                    *ownership_generation = 2;
                }
                row.publication_revision = prior.publication_revision + 1;
                row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet { expected_publication_revision: prior.publication_revision };
                row.source.authority = prior.source.authority.clone();
                row.source.source_revision = prior.source.source_revision + 1;
                row.source.decision_identity = "qn5-successor-readiness".into();
            }
            let publication = AdmissionAuthorityPublicationV1::prepare(&source,now).map_err(debug)?;
            harness.root.publish_runtime_readiness(&harness.node,&publication).await.map_err(debug)?;
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_world_raid_targets SET owner_generation=2").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_world_raid_channel_runs SET owner_generation=2").execute(&mut *tx).await?;
            tx.commit().await?;
            let pin = owner.runtime.lock().await.content_pin().clone();
            *owner.runtime.lock().await = ChannelRuntimeV1::from_committed_assignment(
                owner.world_id, owner.channel_id, NodeId::decode(&id(1)).map_err(debug)?,
                1, 2, 2, "runtime-scope-assignment:2", 8, pin).map_err(debug)?;
            owner.apply_feroxa_map_clock(firing, 1_799_999).await.map_err(|_| "successor authority before timer")?;
            let corpse = clear_corpses.first().ok_or("handoff corpse")?.0;
            // Change only historical placement generation; current authority stays2.
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=3 WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(corpse.as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            assert!(owner.apply_feroxa_map_clock(firing, 1_800_000).await.is_err(),
                "a successor must not consume future-generation custody");
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=5")
                .fetch_one(&harness.pool).await?;
            assert_eq!(receipts,0,"foreign-generation refusal must commit no partial clear");
            for item in std::iter::once(&corpse).chain(clear_corpses[0].1.iter()) {
                let live: i16 = sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(item.as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(live,1);
            }
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=1 WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(corpse.as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            owner.apply_feroxa_map_clock(firing, 1_800_000).await.map_err(|_| "successor corpse clear")?;
            for (corpse, loot) in &clear_corpses {
                for item in std::iter::once(corpse).chain(loot.iter()) {
                    let row: (i16,i64) = sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                        .bind(item.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(row,(2,0));
                }
            }
            let generations: Vec<String> = sqlx::query_scalar("SELECT ownership_generation::text FROM game_spell_item_receipts WHERE cause_kind=5")
                .fetch_all(&harness.pool).await?;
            assert_eq!(generations,vec!["2"]);
            assert_feroxa_recovery(harness, false).await?;
            eprintln!("QN5_M52_HANDOFF_CLEAR_PG_EXECUTED elapsed={:?}",start.elapsed());
            return Ok(());
        }
        owner.reset_epoch = 2;
        assert!(owner.apply_feroxa_map_clock(firing, 1_800_000).await.is_err());
        owner.reset_epoch = 1;
        owner.apply_feroxa_map_clock(firing, 1_799_999).await.map_err(|_| "before map timer")?;
        assert!(owner.bundle_map().ok_or("map")?.facts.phase_tile(at).map_err(debug)?.is_none());
        for (now, expected_ground, expected_no_logout) in [(1_800_000, 13989, true), (3_600_000, 8087, false)] {
            if prove_clear && now == 1_800_000 {
                let before: (i16, i64) = sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(112).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(before, (1, 17));
            }

            if prove_batches && now == 1_800_000 {
                assert!(owner.apply_feroxa_map_clock(firing, now).await.is_err(),
                    "an incomplete physical batch must defer native map publication");
                let batches: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=5 AND feroxa_firing_id=encode($1,'hex')::uuid AND feroxa_phase=1")
                    .bind(firing.as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(batches, 1, "first bounded batch must actually commit, not roll back forever");
                assert_feroxa_recovery(harness, false).await?;
                let complete: bool = sqlx::query_scalar("SELECT complete FROM game_world_feroxa_ground_clears WHERE firing_id=encode($1,'hex')::uuid AND phase=1")
                    .bind(firing.as_slice()).fetch_one(&harness.pool).await?;
                assert!(!complete);
                assert!(owner.bundle_map().ok_or("map")?.facts.phase_tile(at).map_err(debug)?.is_none());
            }
            if large_tree && now == 1_800_000 {
                let root = clear_corpses[0].1[16];
                let mut complete = false;
                for _ in 0..4 {
                    if owner.apply_feroxa_map_clock(firing, now).await.is_ok() {
                        complete = true;
                        break;
                    }
                    let batches: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=5 AND feroxa_firing_id=encode($1,'hex')::uuid")
                        .bind(firing.as_slice()).fetch_one(&harness.pool).await?;
                    assert!(batches > 0, "oversized trees must commit bounded progress");
                    assert_feroxa_recovery(harness, false).await?;
                    let live: i16 = sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                        .bind(root.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(live, 1, "an unfinished tree retains its actual Ground root");
                    assert!(owner.bundle_map().ok_or("map")?.facts.phase_tile(at).map_err(debug)?.is_none());
                }
                assert!(complete, "bounded leaf batches must finish the tree");
                let counts: Vec<i32> = sqlx::query_scalar("SELECT operation_count FROM game_spell_item_receipts WHERE cause_kind=5 AND feroxa_firing_id=encode($1,'hex')::uuid")
                    .bind(firing.as_slice()).fetch_all(&harness.pool).await?;
                assert!(counts.iter().all(|count| (1..=500).contains(count)));
                assert_eq!(counts.iter().sum::<i32>(), 577, "all32 corpses,32 backpacks,512 coins and the first drop retire");
            }
            owner.apply_feroxa_map_clock(firing, now).await.map_err(|_| "native map timer")?;
            for (corpse, loot) in &clear_corpses {
                for item in std::iter::once(corpse).chain(loot.iter()) {
                    let retired: (i16, i64) = sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                        .bind(item.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(retired, (2, 0), "source clear retires the actual corpse and its loot");
                }
                let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
                    .bind(corpse.as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(entries, 0);
            }
            if prove_spectators && now == 1_800_000 {
                let mut timers = crate::gameplay_transport::raid_live_world::FeroxaSpectators::default();
                let actor = deps.actor;
                let session = deps.admitted.game_session_id;
                let original = deps.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
                owner.apply_feroxa_spectators(firing, now, &mut timers).await.map_err(|_| "first spectator capture")?;
                move_spectator(&owner, actor, session, super::quest_world_callbacks::local(DESTINATION)).await?;
                owner.apply_feroxa_spectators(firing, now + 59_999, &mut timers).await.map_err(|_| "before spectator deadline")?;
                assert_eq!(deps.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
                    super::quest_world_callbacks::local(DESTINATION));
                owner.reset_epoch = 2;
                assert!(owner.apply_feroxa_spectators(firing, now + 60_000, &mut timers).await.is_err());
                owner.reset_epoch = 1;
                owner.apply_feroxa_spectators(firing, now + 60_000, &mut timers).await.map_err(|_| "first spectator return")?;
                assert_eq!(deps.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), original);
                move_spectator(&owner, actor, session, super::quest_world_callbacks::local(DESTINATION)).await?;
                owner.apply_feroxa_spectators(firing, now + 60_001, &mut timers).await.map_err(|_| "spectator replay")?;
                assert_eq!(deps.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
                    super::quest_world_callbacks::local(DESTINATION), "replay cannot return a later arrival");
                owner.apply_feroxa_spectators(firing, 2_700_000, &mut timers).await.map_err(|_| "second spectator capture")?;
                let (second, second_original) = extra_players[0];
                let second_session = crate::foundation::GameSessionId::decode(&id(81)).map_err(debug)?;
                move_spectator(&owner, second, second_session, super::quest_world_callbacks::local(DESTINATION)).await?;
                owner.apply_feroxa_spectators(firing, 2_760_000, &mut timers).await.map_err(|_| "second spectator return")?;
                assert_eq!(deps.runtime.lock().await.read_actor_position(second).map_err(debug)?.position(),
                    super::quest_world_callbacks::local(second_original));
                assert_eq!(deps.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
                    super::quest_world_callbacks::local(DESTINATION), "players outside the source floor are not captured");
                move_spectator(&owner, actor, session, original).await?;
            }
            if prove_clear && now == 1_800_000 {
                let after: (i16, i64) = sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(112).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(after, (2, 0), "source clear retires actual portable Ground quantity");
                seed_feroxa_clear_item(harness, &owner, id(113), 33428, 31536).await?;
            }
            let map = owner.bundle_map().ok_or("map")?;
            let tile = map.facts.phase_tile(at).map_err(debug)?.ok_or("published source phase")?;
            assert_eq!(tile.entries[0].facts.appearance_id, expected_ground);
            assert_eq!(map.zone_flags(at).ok_or("phase flags")?.0, expected_no_logout);
            let before = map.facts.object_revision(crate::map::view::placement_key(at, 0).ok_or("phase placement")?);
            owner.apply_feroxa_map_clock(firing, now).await.map_err(|_| "map replay")?;
            assert_eq!(map.facts.object_revision(crate::map::view::placement_key(at, 0).ok_or("phase placement")?), before);
            let still_pending = harness.root.pending_raid_effect(&harness.node, firing, owner.channel_id, generation)
                .await?.ok_or("announcement remains pending")?;
            assert_eq!(still_pending.identity, pending.identity);
            if prove_clear && now == 3_600_000 {
                let second: (i16, i64) = sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(113).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(second, (2, 0), "final map has its own clear, independent of middle-map replay");
                let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_audit_outbox a USING(transaction_id,event_id) WHERE r.cause_kind=5 AND r.feroxa_firing_id=encode($1,'hex')::uuid AND r.character_id IS NULL AND r.cost IS NULL")
                    .bind(firing.as_slice()).fetch_one(&harness.pool).await?;
                if large_tree {
                    assert!((3..=5).contains(&audits), "bounded partial trees have their own World-event audits");
                    let lines: i64 = sqlx::query_scalar("SELECT sum(operation_count)::bigint FROM game_spell_item_receipts WHERE cause_kind=5 AND feroxa_firing_id=encode($1,'hex')::uuid")
                        .bind(firing.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(lines, 578, "every physical identity is retired exactly once across both phases");
                    eprintln!("QN5_M52_LARGE_TREE_PG_EXECUTED elapsed={:?}", start.elapsed());
                } else if prove_batches {
                    assert_eq!(audits, 3, "two middle-map batches plus one final-map batch");
                    let lines: i64 = sqlx::query_scalar("SELECT sum(operation_count)::bigint FROM game_spell_item_receipts WHERE cause_kind=5 AND feroxa_firing_id=encode($1,'hex')::uuid")
                        .bind(firing.as_slice()).fetch_one(&harness.pool).await?;
                    assert_eq!(lines, 512, "all510 corpse/loot identities and both ordinary drops are audited");
                    eprintln!("QN5_M52_CROWDED_CLEAR_PG_EXECUTED elapsed={:?}", start.elapsed());
                } else {
                    assert_eq!(audits, 2, "each actual clear has a World-event audit, no Player or Ability impersonation");
                }
                assert_feroxa_recovery(harness, true).await?;
            }

            if prove_clear && now == 1_800_000 {
                let retained: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(113).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(retained, 17, "same phase replay preserves post-clear drops");
                let outside: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(114).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(outside, 17, "source clear must not widen into the loaded map's border");

            }

        }
        assert!(start.elapsed().as_secs_f64() > 0.01);
        eprintln!("QN5_M52_TIMED_NATIVE_MAPS_PG_EXECUTED elapsed={:?}", start.elapsed());
    }
    let now = i64::from(selected.definition.waves.first().ok_or("wave")?.delay_ms);
    for announcement_at in selected.definition.announcements.iter().map(|notice| i64::from(notice.delay_ms)) {
        let announcement = harness
            .root
            .advance_raid_run(
                &harness.node,
                firing,
                owner.channel_id,
                generation,
                selected.definition.clone(),
                announcement_at,
            )
            .await?
            .ok_or("announcement")?;
        assert!(matches!(
            announcement.action,
            CommittedRaidAction::Announcement { .. }
        ));
        harness
            .root
            .acknowledge_raid_effect(
                &harness.node,
                announcement.identity,
                vec![],
                announcement_at,
            )
            .await?;
    }
    let wave = harness
        .root
        .advance_raid_run(
            &harness.node,
            firing,
            owner.channel_id,
            generation,
            selected.definition.clone(),
            now,
        )
        .await?
        .ok_or("committed wave")?;
    let outcomes = owner
        .apply_raid_wave(&wave, &selected, bundle_digest)
        .await
        .map_err(|_| "physical raid wave")?;
    assert_eq!(outcomes.len(), expected);
    assert!(outcomes.iter().all(|outcome| outcome.placed != block_spawn));
    assert_eq!(
        deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(),
        super::quest_world_callbacks::local(DESTINATION),
        "source final() ejects current contenders before opening the portal"
    );
    if prove_stage {
        let stage = "oteryn:quest-callback/qn5/grimvale-encounter-entry";
        let response = super::quest_world_callbacks::wire(&owner, &mut admitted, 1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent(StepDirection::East)).await?;
        assert_eq!(oteryn_protocol_oteryn::world_spatial::decode_step_result(&response).map_err(debug)?,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved);
        let value: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/qn5/grimvale-entry'")
            .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(value, 1, "actual paced entry commits the authored stage");
        let copy = harness.root.read_character_quest_state(&authority,
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?).await.map_err(debug)?;
        assert!(!copy.states().get("oteryn:quest.grimvale_mini_world_change")
            .ok_or("persisted Grimvale state")?.completed,
            "entry alone must not complete the authored programme");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
            .bind(stage).fetch_one(&harness.pool).await?;
        assert_eq!(count, 1);
        super::quest_world_callbacks::wire(&owner, &mut admitted, 1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent(StepDirection::East)).await?;
        let replay_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
            .bind(stage).fetch_one(&harness.pool).await?;
        assert_eq!(replay_count, 1);
        eprintln!("QN5_M52_AUTHORED_STAGE_PG_EXECUTED elapsed={:?}", start.elapsed());
    }
    {
        let mut runtime = deps.runtime.lock().await;
        for (index, (actor, original)) in extra_players.iter().enumerate() {
            let expected = if index == 1 { *original } else { DESTINATION };
            assert_eq!(runtime.read_actor_position(*actor).map_err(debug)?.position(),
                super::quest_world_callbacks::local(expected),
                "only current in-range players move; existing landing occupants remain");
        }
        // A later arrival must not be ejected by replay of the same final wave.
        let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
        let closed = crate::map::floor::BundleDoors(owner.bundle_map().ok_or("map")?.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime, &cells, deps.actor, deps.admitted.game_session_id, before,
            super::quest_world_callbacks::local(Position { x: TRIGGER.x - 1, ..TRIGGER }),
            false, &closed,
        ).map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    assert_eq!(
        owner
            .apply_raid_wave(&wave, &selected, bundle_digest)
            .await
            .map_err(|_| "physical raid replay")?,
        outcomes
    );
    assert_eq!(deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(),
        super::quest_world_callbacks::local(Position { x: TRIGGER.x - 1, ..TRIGGER }),
        "final-wave replay does not eject later arrivals");
    let map = owner.bundle_map().ok_or("bundle map")?;
    let at = crate::map::overlay::TilePos {
        x: TRIGGER.x,
        y: TRIGGER.y,
        floor: TRIGGER.floor,
    };
    let source_key = crate::map::view::placement_key(at, 1).ok_or("source key")?;
    assert!(map.facts.object_revision(source_key) > 0);
    let revision = map.facts.object_revision(source_key);
    assert!(
        !crate::map::floor::is_teleport(
            &cells,
            crate::content::LogicalCell {
                x: i32::from(TRIGGER.x),
                y: i32::from(TRIGGER.y),
                z: 11
            }
        ),
        "native old teleport must yield to current-world quest callback after opening"
    );
    assert_eq!(
        owner
            .apply_raid_wave(&wave, &selected, bundle_digest)
            .await
            .map_err(|_| "replay")?,
        outcomes
    );
    assert_eq!(
        map.facts.object_revision(source_key),
        revision,
        "replay cannot retransform source"
    );
    let mut runtime = deps.runtime.lock().await;
    let statuses = runtime.raid_creatures(firing).map_err(debug)?;
    assert_eq!(statuses.len(), 1);
    if block_spawn {
        assert_eq!(statuses[0].outcome, oteryn_gameplay_entities::carrier::RaidSpawnOutcome::Failed);
        assert!(!statuses[0].alive, "failed placement must not fabricate a live boss");
        drop(runtime);
        harness.root.acknowledge_raid_effect(&harness.node, wave.identity, outcomes, now).await?;
        assert!(start.elapsed().as_secs_f64() > 0.01);
        eprintln!("QN5_FEROXA_BLOCKED_SPAWN_POSTGRES_COMPLETED elapsed={:?}", start.elapsed());
        return Ok(());
    }
    assert!(statuses[0].alive);
    let oteryn_gameplay_entities::carrier::RaidSpawnOutcome::Placed(actor) = statuses[0].outcome
    else {
        return Err("Feroxa was not placed".into());
    };
    assert_eq!(
        runtime
            .companion_snapshot(actor)
            .map_err(debug)?
            .state
            .policy
            .definition_key,
        "oteryn:creature.feroxa"
    );
    let original = runtime.companion_snapshot(actor).map_err(debug)?;
    let poffs = || map.facts.movement_effects_since(0).into_iter()
        .filter_map(|(_, at, effect)| (effect == 3).then_some(at)).collect::<Vec<_>>();
    let body_tile = crate::map::overlay::TilePos { x: 33380, y: 31537, floor: -11 };
    assert!(poffs().is_empty());
    assert_eq!(
        (original.health, original.maximum_health),
        (100_000, 100_000)
    );
    runtime
        .commit_monster_lab_damage(actor, b"qn5-feroxa-above-half", 49_999)
        .map_err(debug)?;
    crate::gameplay_transport::raid_live::advance_feroxa_forms(&mut runtime, firing, bundle_digest, map);
    assert!(
        runtime.contains_live_creature(actor),
        "above half does not replace the body"
    );
    assert!(poffs().is_empty(), "no transformation means no source poff");
    runtime
        .commit_monster_lab_damage(actor, b"qn5-feroxa-at-half", 1)
        .map_err(debug)?;
    crate::gameplay_transport::raid_live::advance_feroxa_forms(&mut runtime, firing, bundle_digest, map);
    let oteryn_gameplay_entities::carrier::RaidSpawnOutcome::Placed(second) =
        runtime.raid_creatures(firing).map_err(debug)?[0].outcome
    else {
        return Err("Feroxa2 missing".into());
    };
    assert_ne!(second, actor);
    let second_body = runtime.companion_snapshot(second).map_err(debug)?;
    assert_eq!(
        second_body.state.policy.definition_key,
        "oteryn:creature.feroxa2"
    );
    assert_eq!(
        (second_body.health, second_body.maximum_health),
        (50_000, 50_000)
    );
    assert_eq!(second_body.position, original.position);
    assert_eq!(poffs(), vec![body_tile], "first source handoff emits poff at its original cell");
    crate::gameplay_transport::raid_live::advance_feroxa_forms(&mut runtime, firing, bundle_digest, map);
    assert_eq!(poffs(), vec![body_tile], "replay cannot repeat a handoff's effect");
    runtime
        .commit_monster_lab_damage(second, b"qn5-feroxa-at-quarter", 25_000)
        .map_err(debug)?;
    crate::gameplay_transport::raid_live::advance_feroxa_forms(&mut runtime, firing, bundle_digest, map);
    let oteryn_gameplay_entities::carrier::RaidSpawnOutcome::Placed(third) =
        runtime.raid_creatures(firing).map_err(debug)?[0].outcome
    else {
        return Err("Feroxa3/4 missing".into());
    };
    assert_ne!(third, second);
    let third_body = runtime.companion_snapshot(third).map_err(debug)?;
    assert!(matches!(
        third_body.state.policy.definition_key.as_str(),
        "oteryn:creature.feroxa3" | "oteryn:creature.feroxa4"
    ));
    assert_eq!(
        (third_body.health, third_body.maximum_health),
        (130_000, 130_000)
    );
    assert_eq!(poffs(), vec![body_tile, body_tile], "second source handoff emits exactly one poff");
    runtime
        .commit_monster_lab_damage(third, b"qn5-feroxa-intermediate-lethal", 130_000)
        .map_err(debug)?;
    assert!(crate::gameplay_transport::raid_live::continue_feroxa_death(
        &mut runtime,
        third
    ));
    let oteryn_gameplay_entities::carrier::RaidSpawnOutcome::Placed(actor) =
        runtime.raid_creatures(firing).map_err(debug)?[0].outcome
    else {
        return Err("Feroxa5 missing".into());
    };
    assert_ne!(actor, third);
    let final_body = runtime.companion_snapshot(actor).map_err(debug)?;
    assert_eq!(
        final_body.state.policy.definition_key,
        "oteryn:creature.feroxa5"
    );
    assert_eq!(
        (final_body.health, final_body.maximum_health),
        (150_000, 150_000)
    );
    assert_eq!(final_body.position, original.position);
    assert_eq!(poffs(), vec![body_tile, body_tile], "the source death callback emits no poff");
    assert!(runtime.raid_creatures(firing).map_err(debug)?[0].alive);
    drop(runtime);
    // Placement acknowledgement opens the durable runner's lifetime boundary.
    harness
        .root
        .acknowledge_raid_effect(&harness.node, wave.identity, outcomes.clone(), now)
        .await?;
    assert!(
        harness
            .root
            .advance_raid_run(
                &harness.node,
                firing,
                owner.channel_id,
                generation,
                selected.definition.clone(),
                now + 72_000_000 - 1
            )
            .await?
            .is_none(),
        "a living Feroxa must not be cleaned up before twenty hours"
    );
    let expiry = harness
        .root
        .advance_raid_run(
            &harness.node,
            firing,
            owner.channel_id,
            generation,
            selected.definition.clone(),
            now + 72_000_000,
        )
        .await?
        .ok_or("Feroxa lifetime cleanup")?;
    assert!(matches!(
        expiry.action,
        CommittedRaidAction::Complete { cleanup: true }
    ));
    {
        let mut runtime = deps.runtime.lock().await;
        let stamp = runtime.issue_owner_work().map_err(debug)?;
        runtime
            .finish_raid_run(stamp, firing, true)
            .map_err(debug)?;
        assert!(
            runtime.companion_snapshot(actor).is_err(),
            "the current native boss is removed"
        );
    }
    harness
        .root
        .acknowledge_raid_effect(&harness.node, expiry.identity, vec![], now + 72_000_000)
        .await?;
    if prove_phases {
        let state: i16 = sqlx::query_scalar("SELECT state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid")
            .bind(firing.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(state, 2);
        assert!(harness.root.due_raid_firings(owner.channel_id).await?.iter().any(|(id, _)| *id == firing));
        assert!(crate::gameplay_transport::raid_live_world::project_grimvale_portal(map, false));
        owner.apply_feroxa_map_clock(firing, now + 72_000_000).await.map_err(|_| "completed map recovery")?;
        assert!(!crate::map::floor::is_teleport(&cells, crate::content::LogicalCell {
            x: i32::from(TRIGGER.x), y: i32::from(TRIGGER.y), z: 11,
        }), "ACKed final entry recovers without spawning or ejecting again");
    }
    assert!(crate::gameplay_transport::raid_live_world::project_grimvale_portal(map, false));
    assert!(
        crate::map::floor::is_teleport(
            &cells,
            crate::content::LogicalCell {
                x: i32::from(TRIGGER.x),
                y: i32::from(TRIGGER.y),
                z: 11
            }
        ),
        "closing restores original1949 teleport"
    );
    assert_eq!(receipt_count(harness).await?, 0);
    assert!(
        start.elapsed() > std::time::Duration::from_secs(1),
        "PG execution duration must exceed1s"
    );
    eprintln!(
        "WORLDCHG-Q PG EXECUTED: Grimvale native75min source wave, source Feroxa form handoffs, exact1949/22761 ingress, replay and close; elapsed={:?}",
        start.elapsed()
    );
    Ok(())
}

async fn move_spectator(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    destination: crate::foundation::MovementLocalPosition,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let closed = crate::map::floor::BundleDoors(owner.bundle_map().ok_or("map")?.facts.doors());
    let proof = crate::map::floor::prepare_quest_encounter_ejection(
        &runtime, owner.movement_cells(), actor, session, before, destination, &closed,
    ).map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn seed_feroxa_clear_item(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    item: [u8; 16], x: i32, y: i32,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let pin = runtime.content_pin();
    let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut position = Vec::new();
    position.extend_from_slice(&x.to_be_bytes()); position.extend_from_slice(&y.to_be_bytes());
    position.extend_from_slice(&11_i16.to_be_bytes());
    let mut tx = h.pool.begin().await?;
    // Trusted baseline fixture only. The tested retirement uses all ordinary guards.
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',17,1,encode($1,'hex')::uuid)")
        .bind(item.as_slice()).bind(owner.world_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,convert_to('qn5-ground-baseline','UTF8'),$5,$6,$7,nextval('game_item_ground_stack_ordinal_seq'))")
        .bind(item.as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .bind(position).bind(format!("sha256:{}",hex(pin.map_revision_digest())))
        .bind(format!("sha256:{}",hex(pin.server_artifact_digest())))
        .bind(pin.frame_binding_digest().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Trusted fixture baseline is minted through the real corpse and loot owners.
/// The tested World phase then retires those exact physical identities.
async fn mint_feroxa_clear_corpse(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    local: u32, entries: u32, nested: bool,
) -> TestResult<([u8; 16], Vec<[u8; 16]>)> {
    use crate::durability::item_mint::*;
    let runtime = owner.runtime.lock().await;
    let pin = runtime.content_pin();
    let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let ground = GroundPlacement {
        spatial_position: [33428_i32.to_be_bytes().as_slice(),31536_i32.to_be_bytes().as_slice(),11_i16.to_be_bytes().as_slice()].concat(),
        corpse_ref: id(119).to_vec(),
        map_revision: format!("sha256:{}", hex(pin.map_revision_digest())),
        content_revision: format!("sha256:{}", hex(pin.server_artifact_digest())),
        native_room_placement_context: pin.frame_binding_digest().to_vec(),
    };
    let generation = runtime.owner_fence().map_err(debug)?.generation();
    drop(runtime);
    let definition = |key: &str| TypedDefinitionRef { family: "Item".into(), production_key: key.into(), revision_ref: "definition-r1".into() };
    let cause = |purpose: &str, ordinal: u32| ItemMintCause::for_test(owner.world_id, owner.channel_id,
        generation, local, 1,
        TypedDefinitionRef { family: "LootTable".into(), production_key: "fixture:loot.feroxa-clear".into(), revision_ref: "definition-r1".into() },
        purpose.into(), ordinal);
    let mut candidate = h.root.freeze_item_mint(owner.character, &h.node, ItemMintRequest {
        cause: cause(CORPSE_MATERIALIZATION_PURPOSE_KEY, 0), item: definition("oteryn:item.tibia.i5973"),
        quantity: 1, ground, content_revision: "content-1".into(), ruleset_revision: "ruleset-1".into(), sim_revision: "sim-1".into(),
    }).await.map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("corpse mint permit")?;
    let corpse = match h.root.commit_corpse_mint(&permit, owner.character, &h.node, &mut candidate, id(41)).await.map_err(debug)? {
        ItemMintOutcome::Committed(v) => v.item_instance_id,
        other => return Err(debug(other)),
    };
    let mut loot = Vec::new();
    for ordinal in 1..=entries + if nested { 4 } else { 0 } {
    let mut candidate = h.root.freeze_corpse_loot_mint(owner.character, &h.node, CorpseLootMintRequest {
        cause: cause("fixture:purpose.feroxa-clear", ordinal),
        item: definition(if (nested && (ordinal==entries+1 || ordinal==entries+3)) || (!nested && entries==17 && ordinal==17) { "oteryn:item.tibia.i2854" } else { "oteryn:item.tibia.i3031" }),
        quantity: if (nested && (ordinal==entries+1 || ordinal==entries+3)) || (!nested && entries==17 && ordinal==17) { 1 } else { 17 },
        placement: CorpseContainerPlacement { corpse_item_instance_id: corpse, placement_ordinal: ordinal, fluid_subtype: None },
        content_revision: "content-1".into(), ruleset_revision: "ruleset-1".into(), sim_revision: "sim-1".into(),
    }).await.map_err(debug)?;
    let item = match h.root.commit_corpse_loot_mint(&permit, owner.character, &h.node, &mut candidate).await.map_err(debug)? {
        ItemMintOutcome::Committed(v) => v.item_instance_id,
        other => return Err(debug(other)),
    };
    loot.push(item);
    }
    if nested {
        let bag = loot.get(usize::try_from(entries)?).ok_or("nested bag")?;
        let child = loot.get(usize::try_from(entries+1)?).ok_or("nested gold")?;
        let mut tx = h.pool.begin().await?;
        // Trusted imported custody baseline only; the tested clear uses all guards.
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(child.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,minted_transaction_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(child.as_slice()).bind(bag.as_slice()).execute(&mut *tx).await?;
        // Keep the nested-corpse case above, and import a second actual backpack
        // as an ordinary Ground root with its own nested coin.
        let bag = loot.get(usize::try_from(entries+2)?).ok_or("ground bag")?;
        let child = loot.last().ok_or("ground bag gold")?;
        sqlx::query("DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(child.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,minted_transaction_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(child.as_slice()).bind(bag.as_slice()).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(bag.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) SELECT encode($1,'hex')::uuid,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,nextval('game_item_ground_stack_ordinal_seq') FROM game_item_ground_locations WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(bag.as_slice()).bind(corpse.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    Ok((corpse, loot))
}

async fn assert_feroxa_recovery(h: &Harness, reject_forged: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    crate::durability::spell_item_transaction::verify_spell_item_chain(&mut tx)
        .await
        .map_err(debug)?;
    tx.rollback().await?;
    if !reject_forged {
        return Ok(());
    }
    // Keep the common receipt/audit hashes coherent: each rollback-only mutation
    // must be rejected by the historical World-event proof itself.
    for (path, value) in [
        (vec!["phase"], "9"),
        (vec!["generation"], "18446744073709551615"),
        (
            vec!["removals", "0", "item_instance_id"],
            "00000000-0000-7000-8000-000000000001",
        ),
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let changed: String = sqlx::query_scalar(
            "WITH candidate AS (SELECT transaction_id,convert_to(jsonb_set(convert_from(intent,'UTF8')::jsonb,$1::text[],$2::jsonb)::text,'UTF8') AS intent FROM game_spell_item_receipts WHERE cause_kind=5 ORDER BY transaction_id LIMIT 1) UPDATE game_spell_item_receipts r SET intent=c.intent,binding=sha256(c.intent) FROM candidate c WHERE r.transaction_id=c.transaction_id RETURNING r.transaction_id::text",
        )
        .bind(&path)
        .bind(sqlx::types::Json(value))
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("UPDATE game_spell_item_audit_outbox a SET envelope=r.intent,envelope_sha256=r.binding FROM game_spell_item_receipts r WHERE r.transaction_id=$1::uuid AND a.transaction_id=r.transaction_id")
            .bind(changed)
            .execute(&mut *tx)
            .await?;
        assert!(
            matches!(
                crate::durability::spell_item_transaction::verify_spell_item_chain(&mut tx).await,
                Err(crate::durability::DurabilityError::InvalidStoredState)
            ),
            "history must reject a substituted Feroxa {path:?} even with matching hashes"
        );
        tx.rollback().await?;
    }
    Ok(())
}

// Trusted imported tree custody; every identity was minted by the real corpse owners.
async fn import_feroxa_large_tree(h: &Harness, corpses: &[([u8; 16], Vec<[u8; 16]>)]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    for (index, (corpse, loot)) in corpses.iter().enumerate() {
        let bag = loot.get(16).ok_or("large tree backpack")?;
        for (ordinal, coin) in loot.iter().take(16).enumerate() {
            sqlx::query("DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(coin.as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,$3::bigint,minted_transaction_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(coin.as_slice()).bind(bag.as_slice()).bind(i64::try_from(ordinal+1)?).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(bag.as_slice()).execute(&mut *tx).await?;
        if index == 0 {
            sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) SELECT encode($1,'hex')::uuid,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,nextval('game_item_ground_stack_ordinal_seq') FROM game_item_ground_locations WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(bag.as_slice()).bind(corpse.as_slice()).execute(&mut *tx).await?;
        } else {
            let parent = corpses.get((index-1)/2).and_then(|entry| entry.1.get(16)).ok_or("large tree parent")?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,$3::bigint,minted_transaction_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(bag.as_slice()).bind(parent.as_slice()).bind(17+i64::try_from((index-1)%2)?).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
