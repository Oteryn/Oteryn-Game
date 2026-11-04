//! M10 movement qualification through routed STEP and the production connection.

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::writer::*;
use std::time::Duration;

#[test]
fn m10_snow_step_out_real_dispatch_adds_one_snowbunny_progress_postgres() -> TestResult {
    on_thread(|| snow_scenario(false))
}

#[test]
fn m10_snow_step_at_9999_grants_snowbunny_achievement_postgres() -> TestResult {
    on_thread(|| snow_scenario(true))
}

#[test]
fn m10_trap_step_effect_reaches_client_presentation_frame_postgres() -> TestResult {
    on_thread(|| {
        trap_wire_case(true)?;
        trap_wire_case(false)
    })
}

fn on_thread(scenario: fn() -> TestResult) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("m10-movement-dispatch-postgres".into())
        .stack_size(256 << 20)
        .spawn(move || scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "M10 movement dispatch PostgreSQL thread panicked")??;
    Ok(())
}

fn movement_row() -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let manifest = Manifest {
        subareas: Default::default(),
        format: FORMAT.into(),
        min_reader_version: VERSION,
        projection_class: "server".into(),
        compiler_version: "m10-movement-dispatch-test".into(),
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
        palette: vec![
            PaletteEntry {
                key: "oteryn:terrain.tibia.i100".into(),
                family: Family::Terrain,
                id: 0,
                terrain: Some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(150),
                }),
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i799".into(),
                family: Family::Item,
                id: 0,
                terrain: None,
            },
            PaletteEntry {
                key: "oteryn:item.tibia.i2145".into(),
                family: Family::Item,
                id: 1,
                terrain: None,
            },
        ],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let tile = |x, special: Option<u32>| Tile {
        x,
        y: 0,
        flags: 0,
        house: 0,
        zones: vec![],
        items: [
            Some(Item {
                palette: 0,
                depth: 0,
                attrs: Attrs::default(),
            }),
            special.map(|palette| Item {
                palette,
                depth: 0,
                attrs: Attrs::default(),
            }),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };
    let bytes = write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![
                tile(1, None),
                tile(2, Some(1)),
                tile(3, None),
                tile(4, Some(2)),
            ],
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            production: false,
        },
    ))
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    mover: (
        crate::gameplay_transport::connection::AdmittedSession,
        ExactActorRef,
        GameSessionId,
    ),
    direction: StepDirection,
    x: i32,
) -> TestResult {
    let (admitted, actor, session) = mover;
    assert_eq!(admitted.game_session_id, session);
    let outcome = crate::gameplay_transport::connection::regression_routed_step(
        owner, admitted, actor, direction,
    )
    .await?;
    assert_eq!(outcome.disposition, StepDisposition::Moved, "step to x {x}");
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        MovementLocalPosition { x, y: 0, floor: 7 }
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("clock")?;
    Ok(())
}

async fn seed_dispatch_policy(
    h: &Harness,
    active: &oteryn_gameplay_content::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
) -> TestResult {
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
        .bind(id(42).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'m10-movement-fixture')")
        .bind(id(42).as_slice()).bind(1_i16).execute(&h.pool).await?;
    Ok(())
}

fn snow_scenario(threshold: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "m10_snow_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let channel = Dependencies::new()?.admitted.channel_id;
        let (bytes, pins) = movement_row()?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos {
                    x: 1,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            |key| {
                let id = [799, 2145]
                    .into_iter()
                    .position(|id| key == format!("oteryn:item.tibia.i{id}"))?;
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(u32::try_from(id).ok()? + 1)?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
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
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        seed_dispatch_policy(&h, active, &pin).await?;
        if threshold {
            sqlx::query("INSERT INTO game_character_achievement_progress(character_id,achievement_key,progress) VALUES(encode($1,'hex')::uuid,'oteryn:achievement/snowbunny',9999)")
                .bind(id(41).as_slice()).execute(&h.pool).await?;
        }
        let mover = (deps.admitted, deps.actor, deps.admitted.game_session_id);
        step(&mut owner, mover, StepDirection::East, 2).await?;
        step(&mut owner, mover, StepDirection::East, 3).await?;
        let progress: i64 = sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key='oteryn:achievement/snowbunny'")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(progress, if threshold { 10_000 } else { 1 });
        let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements a JOIN game_character_roots c ON c.account_id=a.account_id WHERE c.character_id=encode($1,'hex')::uuid AND a.achievement_key='oteryn:achievement/snowbunny'")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(grants, i64::from(threshold));
        owner.bundle_map().ok_or("bundle")?;
        let appearance = |x, ordinal| {
            use crate::map::view::MapFacts;
            let pos = TilePos { x, y: 0, floor: -7 };
            let tile = map.base().tile(x, 0, -7)?;
            let id = tile
                .ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, depth)| **depth == 0)
                .nth(usize::from(ordinal))
                .map(|(id, _)| *id)?;
            map.facts()
                .base_entry(pos, ordinal, id)
                .map(|item| item.appearance_id)
        };
        assert_eq!(appearance(2, 1), Some(6594));
        assert_eq!(map.facts().expire_movement_items(u64::MAX), 1);
        step(&mut owner, mover, StepDirection::West, 2).await?;
        step(&mut owner, mover, StepDirection::East, 3).await?;
        let progress: i64 = sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key='oteryn:achievement/snowbunny'")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(progress, if threshold { 10_000 } else { 2 });
        let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements a JOIN game_character_roots c ON c.account_id=a.account_id WHERE c.character_id=encode($1,'hex')::uuid AND a.achievement_key='oteryn:achievement/snowbunny'")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(grants, i64::from(threshold));
        if !threshold {
            step(&mut owner, mover, StepDirection::East, 4).await?;
            let unchanged: i64 = sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key='oteryn:achievement/snowbunny'")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(unchanged, 2);
        }
        Ok(())
    })
}

fn trap_wire_case(presentation: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "m10_trap_wire", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let channel = Dependencies::new()?.admitted.channel_id;
        let (bytes, pins) = movement_row()?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos {
                    x: 3,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            |key| {
                let id = [799, 2145]
                    .into_iter()
                    .position(|id| key == format!("oteryn:item.tibia.i{id}"))?;
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(u32::try_from(id).ok()? + 1)?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            (3, 0, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        seed_dispatch_policy(&h, active, &pin).await?;
        let offered = [
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 23,
                requires: &[6],
            },
        ];
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, if presentation { &[6, 23] } else { &[6] })
                .ok_or("test capabilities")?;
        let command = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: 1,
                command_type:
                    oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                payload: &oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                    StepDirection::East,
                ),
            },
        )
        .map_err(debug)?;
        let mut input = u32::try_from(command.len())?.to_be_bytes().to_vec();
        input.extend(command);
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        let mut reader = output.as_slice();
        let mut batches = Vec::new();
        while !reader.is_empty() {
            let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
                if delta.domain_id
                    == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION
                {
                    batches.push(
                        oteryn_protocol_oteryn::presentation::decode_presentation_batch(
                            delta.payload,
                        )
                        .map_err(debug)?,
                    );
                }
            }
        }
        if presentation {
            let [events] = batches.as_slice() else {
                return Err("expected one presentation batch".into());
            };
            assert!(events.iter().any(|event| matches!(
                event.kind,
                oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                    position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                        x: 4,
                        y: 0,
                        floor: 7
                    },
                    effect_id: 11,
                    ..
                }
            )));
        } else {
            assert!(batches.is_empty());
        }
        Ok(())
    })
}
