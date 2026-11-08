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
    for (floor, x, y) in [(-11, 33389, 31539), (-11, 33457, 31531)] {
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
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-grimvale-producer", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "worldchg_grimvale_producer",
                false,
                false,
            )
            .await?;
            let result = producer_scenario(&h).await;
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

async fn producer_scenario(harness: &Harness) -> TestResult {
    use crate::foundation::{NodeId, ScopeOwnershipGeneration};
    use crate::map::boot::BundleCollisionIndexExt;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let key = "raid:edron.feroxa";
    let expected = 1;
    let start = std::time::Instant::now();
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
    assert_eq!(selected.definition.waves[0].delay_ms, 61_000);
    let first_area = selected
        .areas
        .get(&selected.definition.waves[0].entries[0].area_anchor)
        .ok_or("first source spawn area")?;
    assert_eq!(
        first_area.from,
        RaidPosition {
            x: 33389,
            y: 31539,
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
        expected + 1,
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
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.combat_room = Some(room);
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.reset_epoch = 1;
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
    let now = i64::from(selected.definition.waves.first().ok_or("wave")?.delay_ms);
    for announcement_at in [1_000, 30_000, 60_000] {
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
    assert!(outcomes.iter().all(|outcome| outcome.placed));
    assert_eq!(
        owner
            .apply_raid_wave(&wave, &selected, bundle_digest)
            .await
            .map_err(|_| "physical raid replay")?,
        outcomes
    );
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
    let runtime = deps.runtime.lock().await;
    let statuses = runtime.raid_creatures(firing).map_err(debug)?;
    assert_eq!(statuses.len(), 1);
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
    // Placement acknowledgement opens the durable runner's lifetime boundary.
    harness.root.acknowledge_raid_effect(&harness.node, wave.identity, outcomes.clone(), now).await?;
    assert!(harness.root.advance_raid_run(&harness.node, firing, owner.channel_id, generation,
        selected.definition.clone(), now + 72_000_000 - 1).await?.is_none(),
        "a living Feroxa must not be cleaned up before twenty hours");
    let expiry = harness.root.advance_raid_run(&harness.node, firing, owner.channel_id, generation,
        selected.definition.clone(), now + 72_000_000).await?.ok_or("Feroxa lifetime cleanup")?;
    assert!(matches!(expiry.action, CommittedRaidAction::Complete { cleanup: true }));
    {
        let mut runtime = deps.runtime.lock().await;
        let stamp = runtime.issue_owner_work().map_err(debug)?;
        runtime.finish_raid_run(stamp, firing, true).map_err(debug)?;
        assert!(runtime.companion_snapshot(actor).is_err(), "the current native boss is removed");
    }
    harness.root.acknowledge_raid_effect(&harness.node, expiry.identity, vec![], now + 72_000_000).await?;
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
        "WORLDCHG-Q PG EXECUTED: Grimvale native61s final wave, Feroxa actor, exact1949/22761 ingress, replay and close; elapsed={:?}",
        start.elapsed()
    );
    Ok(())
}
