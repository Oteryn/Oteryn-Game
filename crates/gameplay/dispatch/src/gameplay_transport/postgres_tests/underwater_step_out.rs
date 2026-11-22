use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, MovementLocalPosition, WorldId};
use crate::gameplay_transport::connection::regression_routed_step;
use crate::gameplay_transport::paced_movement::underwater_drown::is_environment_drown;
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_gameplay_domain::domain::condition::{
    ConditionType, ConditionValues, ConflictKey, DotElement, SpeedRange,
};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use oteryn_world_bundle::sector::{Attrs, Item, Tile};
use oteryn_world_bundle::writer::{
    BuildClass, Extent, FORMAT, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
    TerrainKind, VERSION, write,
};
use std::time::Duration;

#[test]
fn m10_underwater_step_out_clears_drowning_and_positive_haste() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, false)
}

#[test]
fn m10_underwater_step_out_preserves_paralysis() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, true)
}

fn on_thread(admin: String, paralysis: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(Box::pin(async {
                    let h = Harness::create_with_admission_seed(
                        admin,
                        "underwater_step_out",
                        true,
                        false,
                    )
                    .await?;
                    let result = exercise(&h, paralysis).await;
                    h.cleanup().await?;
                    result
                }))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "underwater StepOut PostgreSQL thread panicked")??;
    Ok(())
}

async fn exercise(h: &Harness, paralysis: bool) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let channel = Dependencies::new()?.admitted.channel_id;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active generation")?;
    let (bytes, bundle) = underwater_row()?;
    let base = crate::map::load(&bytes, &bundle).map_err(debug)?;
    let map = boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle,
            start: TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        world,
        channel,
        |_| None,
    )
    .map_err(debug)?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        map.frame_binding_digest(world),
        map.map_revision_digest(),
        (1, 0, 7),
    );
    let mut deps = Dependencies::with_content(
        channel_pin,
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let cells = map
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    deps.combat_room = Some(
        pin.qualified_room()
            .bundle_view(
                cells.clone(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .ok_or("qualified underwater bundle room")?,
    );
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;

    let entry = regression_routed_step(&owner, deps.admitted, actor, StepDirection::East).await?;
    assert_eq!(entry.disposition, StepDisposition::Moved);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("step pacing clock")?;
    let speed = {
        let runtime = deps.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 2,
                y: 0,
                floor: 7
            }
        );
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("spell state")?;
        assert!(
            state
                .owned_conditions()
                .instances()
                .iter()
                .any(|condition| is_environment_drown(condition.provenance()))
        );

        let (kind, target_speed) = if paralysis {
            (ConditionType::Paralysis, 70)
        } else {
            (ConditionType::Haste, 160)
        };
        let definition = ConditionDefinition::new(
            if paralysis {
                "fixture.underwater.paralysis"
            } else {
                "fixture.underwater.haste"
            },
            1,
            ConditionValues::Speed {
                paralysis,
                range: SpeedRange {
                    a_min: 0,
                    b_min: target_speed,
                    a_max: 0,
                    b_max: target_speed,
                },
                duration_ms: 60_000,
            },
        )
        .ok_or("speed definition")?;
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let facts = ApplicationFacts {
            now: owner.owner_now().get(),
            base_speed: 100,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
        };
        state
            .conditions
            .apply(&definition, None, ConditionSourceKind::SelfUse, &[], &facts)
            .map_err(debug)?;
        let speed = state
            .owned_conditions()
            .active_at(ConflictKey::Speed, facts.now)
            .ok_or("active speed condition")?;
        assert_eq!(speed.definition().condition_type(), kind);
        assert_eq!(
            speed.speed_delta(),
            i64::from(target_speed) - i64::from(facts.base_speed)
        );
        speed.clone()
    };

    let exit = regression_routed_step(&owner, deps.admitted, actor, StepDirection::West).await?;
    assert_eq!(exit.disposition, StepDisposition::Moved);
    let runtime = deps.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 7
        }
    );
    let states = owner.states.spell_states.lock().await;
    let conditions = states
        .get(&runtime, actor, session)
        .ok_or("spell state")?
        .owned_conditions();
    assert!(
        conditions
            .get(ConflictKey::Element(DotElement::Drown))
            .is_none()
    );
    assert_eq!(
        conditions.get(ConflictKey::Speed),
        paralysis.then_some(&speed)
    );
    Ok(())
}

fn underwater_row() -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    let manifest = Manifest {
        format: FORMAT.into(),
        min_reader_version: VERSION,
        projection_class: "server".into(),
        compiler_version: "underwater-step-out-test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: [100, 5405]
            .into_iter()
            .enumerate()
            .map(|(id, appearance)| PaletteEntry {
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: Family::Terrain,
                id: id as u32,
                terrain: Some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(150),
                    blocks_projectile: None,
                }),
            })
            .collect(),
        subareas: Default::default(),
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let sector = Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles: [(1, 0), (2, 1)]
            .into_iter()
            .map(|(x, palette)| Tile {
                x,
                y: 0,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![Item {
                    palette,
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            })
            .collect(),
    };
    let bytes = write(&manifest, &[sector], &Default::default()).map_err(debug)?;
    let digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: manifest.identity.project_format_version,
            world_schema_version: manifest.identity.world_schema_version,
            content_revision: manifest.identity.content_revision,
            production: false,
        },
    ))
}
