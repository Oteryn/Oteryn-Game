//! PAR-M10: underwater ground drowns on a real paced step, unless the head slot protects.
//!
//! Real activated content, a bundle row of underwater and dry ground, routed PostgreSQL
//! equipment custody and the production STEP, equipment-effects and condition owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::paced_movement::underwater_drown::is_environment_drown;
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_gameplay_content::content::item_abilities::AbilitySuppression;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::writer::*;
use std::time::Duration;

#[test]
fn underwater_drowning_postgres_step_in_drowns_helmets_protect_step_out_removes() -> TestResult {
    on_thread(|| scenario(None, false))
}

#[test]
fn underwater_drowning_postgres_quest_relocation_enters_and_leaves_water() -> TestResult {
    on_thread(|| {
        scenario(Some((0, 2, false)), false)?;
        scenario(Some((3, 4, false)), false)
    })
}

#[test]
fn underwater_drowning_postgres_refused_quest_step_restores_water_state() -> TestResult {
    on_thread(|| {
        scenario(Some((2, 1, true)), false)?;
        scenario(Some((1, 2, true)), false)
    })
}

#[test]
fn underwater_drowning_postgres_portal_landings_enter_and_leave_water() -> TestResult {
    on_thread(|| scenario(None, true))
}

fn on_thread(scenario: fn() -> TestResult) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("underwater-drowning-postgres".into())
        .stack_size(256 << 20)
        .spawn(move || scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "underwater drowning PostgreSQL thread panicked")??;
    Ok(())
}

/// One row at native floor -7: dry ground at x 1 and x 4, underwater ground (5405, 5406) at
/// x 2 and x 3. The ground keys are real appearance ids, as the real World routes them.
fn underwater_row(portals: bool) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let ground = |key: &str, id| PaletteEntry {
        key: key.into(),
        family: Family::Terrain,
        id,
        terrain: Some(Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(150),
        }),
    };
    let manifest = Manifest {
        subareas: Default::default(),
        format: FORMAT.into(),
        min_reader_version: VERSION,
        projection_class: "server".into(),
        compiler_version: "underwater-drowning-test".into(),
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
            ground("oteryn:terrain.tibia.i100", 0),
            ground("oteryn:terrain.tibia.i5405", 1),
            ground("oteryn:terrain.tibia.i5406", 2),
        ],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let tile = |x, palette| Tile {
        x,
        y: 0,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![Item {
            palette,
            depth: 0,
            attrs: Attrs {
                teleport: if portals {
                    match x {
                        0 => Some((2, 0, (-7_i8).to_ne_bytes()[0])),
                        4 => Some((1, 0, (-7_i8).to_ne_bytes()[0])),
                        _ => None,
                    }
                } else {
                    None
                },
                ..Attrs::default()
            },
        }],
    };
    let bytes = write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![tile(0, 0), tile(1, 0), tile(2, 1), tile(3, 2), tile(4, 0)],
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

async fn drowning(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<bool> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states.get(&runtime, actor, session).ok_or("spell state")?;
    Ok(state
        .owned_conditions()
        .instances()
        .iter()
        .any(|instance| is_environment_drown(instance.provenance())))
}

/// One production paced step, then the owner clock moves past its pacing window.
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: StepDirection,
    x: i32,
) -> TestResult {
    let (step, _) = owner.paced_step(actor, session, direction).await;
    assert_eq!(step.disposition, StepDisposition::Moved, "step to x {x}");
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

/// Reads the persisted equipment through the production admission owner and installs its
/// effects. `diving_helmet` adds the `SUPPRESS_DROWN` suppression that the same read yields for
/// a worn Item with that modifier (`equipment_effects_owner_uses_primary_slot_and_extra_never_
/// suppresses`): the three diving helmets are not yet native item policies, so no character
/// can hold one in this content.
async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    diving_helmet: bool,
) -> TestResult {
    let mut effects = owner
        .read_admission_effects(session)
        .await
        .map_err(|_| "admission effects")?
        .ok_or("effects")?;
    assert!(effects.0.suppressions.is_empty(), "the head slot is empty");
    if diving_helmet {
        effects.0.suppressions.insert(AbilitySuppression::Drown);
    }
    let runtime = owner.runtime.lock().await;
    owner
        .install_admission_suppressions(&runtime, actor, session, Some(effects))
        .await
        .ok_or("install")?;
    Ok(())
}

fn scenario(callback: Option<(u16, u16, bool)>, portals: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "underwater_drown", true, false).await?;
        eprintln!("POSTGRES_STARTED PAR-M10");
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
        // Activated content digests (custody reads pin them), the test row's map binding.
        let channel = Dependencies::new()?.admitted.channel_id;
        let (bytes, pins) = underwater_row(portals)?;
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
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        assert_eq!(deps.admitted.channel_id, channel);
        // The activated room scopes the cells to the activated content digest.
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
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let (actor, session) = (deps.actor, deps.admitted.game_session_id);
        if portals {
            install(&owner, actor, session, false).await?;
            step(&mut owner, actor, session, StepDirection::West, 2).await?;
            assert!(
                drowning(&owner, actor, session).await?,
                "dry portal lands underwater"
            );
            step(&mut owner, actor, session, StepDirection::East, 3).await?;
            step(&mut owner, actor, session, StepDirection::East, 1).await?;
            assert!(
                !drowning(&owner, actor, session).await?,
                "wet actor lands on dry ground"
            );
            return Ok(());
        }
        if let Some((trigger, destination, refused)) = callback {
            use oteryn_gameplay_quests::quest::world_callbacks::{
                Callback, Position, Selector, Storage, Trigger,
            };
            let position = |x| Position { x, y: 0, floor: -7 };
            // The production callback selector and movement proof operate on real bundle
            // ground. An unknown storage track refuses before any callback mutation.
            let fixture = Callback {
                key: "fixture:underwater-callback".into(),
                quest: "fixture:underwater".into(),
                trigger: Trigger::StepIn,
                selector: Selector {
                    position: Some(position(trigger)),
                    unique: None,
                    action: None,
                    appearance: None,
                },
                lever: None,
                reversible: false,
                passages: vec![],
                teleport: Some(position(destination)),
                storage: if refused {
                    vec![Storage {
                        track: "fixture:unknown-track".into(),
                        minimum: 0,
                        maximum: 0,
                        set: 1,
                    }]
                } else {
                    vec![]
                },
                offerings: vec![],
                minimum_level: None,
                experience: None,
                monk_reward: None,
                item_on_target: None,
                return_on_failure: refused,
                failure_message: None,
                sources: vec![],
            };
            install(&owner, actor, session, false).await?;
            // Outward callbacks and returns into water start from a real wet step.
            if trigger == 3 || (trigger == 1 && refused) {
                step(&mut owner, actor, session, StepDirection::East, 2).await?;
                assert!(drowning(&owner, actor, session).await?);
            }
            owner
                .states
                .quest_callbacks
                .fixture
                .set(vec![fixture])
                .map_err(|_| "callback fixture already installed")?;
            let direction = if trigger <= 1 {
                StepDirection::West
            } else {
                StepDirection::East
            };
            let final_x = if refused {
                if trigger == 2 { 1 } else { 2 }
            } else {
                destination
            };
            step(&mut owner, actor, session, direction, i32::from(final_x)).await?;
            assert_eq!(
                drowning(&owner, actor, session).await?,
                final_x == 2,
                "callback trigger {trigger}, destination {destination}, refused {refused}"
            );
            return Ok(());
        }
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 1,
                y: 0,
                floor: 7
            }
        );
        // Unprotected: StepIn drowns, underwater to underwater keeps drowning, StepOut removes.
        install(&owner, actor, session, false).await?;
        assert!(!drowning(&owner, actor, session).await?);
        step(&mut owner, actor, session, StepDirection::East, 2).await?;
        assert!(
            drowning(&owner, actor, session).await?,
            "step into 5405 drowns"
        );
        step(&mut owner, actor, session, StepDirection::East, 3).await?;
        assert!(
            drowning(&owner, actor, session).await?,
            "5405 to 5406 keeps drowning"
        );
        step(&mut owner, actor, session, StepDirection::East, 4).await?;
        assert!(
            !drowning(&owner, actor, session).await?,
            "step out removes drown"
        );
        // A diving helmet's installed suppression refuses the StepIn drown.
        install(&owner, actor, session, true).await?;
        step(&mut owner, actor, session, StepDirection::West, 3).await?;
        assert!(
            !drowning(&owner, actor, session).await?,
            "diving helmet protects"
        );
        step(&mut owner, actor, session, StepDirection::West, 2).await?;
        assert!(!drowning(&owner, actor, session).await?, "still protected");
        // Taking it off restores the unprotected StepIn.
        install(&owner, actor, session, false).await?;
        step(&mut owner, actor, session, StepDirection::East, 3).await?;
        assert!(drowning(&owner, actor, session).await?, "unprotected again");
        Ok(())
    })
}
