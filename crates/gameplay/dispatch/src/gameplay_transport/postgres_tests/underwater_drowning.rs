//! PAR-M10: underwater ground drowns on a real paced step, unless the head slot protects.
//!
//! Real activated content, a bundle row of underwater and dry ground, routed PostgreSQL
//! equipment custody and the production STEP, equipment-effects and condition owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::paced_movement::underwater_drown::is_environment_drown;
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_gameplay_content::content::item_abilities::AbilitySuppression;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::writer::*;
use std::time::Duration;

#[test]
fn fixm10_swimming_and_map_traps_postgres() -> TestResult {
    on_thread(|| {
        scenario(None, false)?;
        scenario_in_world(None, false, false)
    })
}

#[test]
fn fixm10_swimming_quest_relocation_postgres() -> TestResult {
    on_thread(|| {
        scenario(Some((0, 5, false)), false)?;
        scenario(Some((7, 4, false)), false)?;
        scenario(Some((6, 4, true)), false)
    })
}

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

/// One row at native floor -7: dry ground, underwater ground, swimming ground and a spike trap.
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
    let item = |key: &str, id| PaletteEntry {
        key: key.into(),
        family: Family::Item,
        id,
        terrain: None,
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
            ground("oteryn:terrain.tibia.i629", 3),
            item("oteryn:item.tibia.i2145", 0),
            item("oteryn:item.tibia.i3482", 1),
            item("oteryn:item.tibia.i3944", 2),
            item("oteryn:item.tibia.i2147", 3),
            item("oteryn:item.tibia.i44529", 4),
            item("oteryn:item.tibia.i2146", 5),
            item("oteryn:item.tibia.i2148", 6),
            item("oteryn:item.tibia.i3481", 7),
            item("oteryn:item.tibia.i3945", 8),
            item("oteryn:item.tibia.i44530", 9),
        ],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let tile = |x, palette, trap: bool| Tile {
        x,
        y: 0,
        flags: if x == 10 { 1 } else { 0 },
        house: 0,
        zones: vec![],
        items: [
            Some(Item {
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
            }),
            trap.then_some(Item {
                palette: match x {
                    8 | 11 => 5,
                    9 => 6,
                    12 => 7,
                    13 => 8,
                    _ => 4,
                },
                depth: 0,
                attrs: Attrs::default(),
            }),
            (x == 11).then_some(Item {
                palette: 5,
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
                tile(0, 0, false),
                tile(1, 0, false),
                tile(2, 1, false),
                tile(3, 2, false),
                tile(4, 0, false),
                tile(5, 3, false),
                tile(6, 0, true),
                tile(7, 3, false),
                tile(8, 3, true),
                tile(9, 0, true),
                tile(10, 0, true),
                tile(11, 0, true),
                tile(12, 0, true),
                tile(13, 0, true),
                tile(14, 0, false),
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
    let step = crate::gameplay_transport::connection::regression_routed_step(
        owner, admitted, actor, direction,
    )
    .await?;
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
    scenario_in_world(callback, portals, true)
}

fn scenario_in_world(
    callback: Option<(u16, u16, bool)>,
    portals: bool,
    optional: bool,
) -> TestResult {
    let started = std::time::Instant::now();
    let result = run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "underwater_drown", true, false).await?;
        eprintln!(
            "POSTGRES_STARTED PAR-M10 callback={callback:?} portals={portals} optional={optional}"
        );
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
            |key| {
                let ids = [2145, 3482, 3944, 2147, 44529, 2146, 2148, 3481, 3945, 44530];
                let index = ids
                    .iter()
                    .position(|id| key == format!("oteryn:item.tibia.i{id}"))?;
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(index as u32 + 1)?,
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
        sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'fixm10-fixture')")
            .bind(id(42).as_slice()).bind(if optional { 1_i16 } else { 2_i16 }).execute(&h.pool).await?;
        let (actor, session) = (deps.actor, deps.admitted.game_session_id);
        let mover = (deps.admitted, actor, session);
        if portals {
            install(&owner, actor, session, false).await?;
            step(&mut owner, mover, StepDirection::West, 2).await?;
            assert!(
                drowning(&owner, actor, session).await?,
                "dry portal lands underwater"
            );
            step(&mut owner, mover, StepDirection::East, 3).await?;
            step(&mut owner, mover, StepDirection::East, 1).await?;
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
                step(&mut owner, mover, StepDirection::East, 2).await?;
                assert!(drowning(&owner, actor, session).await?);
            }
            if trigger >= 6 {
                for x in 2..trigger {
                    step(&mut owner, mover, StepDirection::East, i32::from(x)).await?;
                }
            }
            if destination == 5 {
                seed_swimming_conditions(&owner, actor, session).await?;
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
                if trigger >= 6 {
                    trigger - 1
                } else if trigger == 2 {
                    1
                } else {
                    2
                }
            } else {
                destination
            };
            step(&mut owner, mover, direction, i32::from(final_x)).await?;
            assert_eq!(
                drowning(&owner, actor, session).await?,
                final_x == 2,
                "callback trigger {trigger}, destination {destination}, refused {refused}"
            );
            {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let state = states.get(&runtime, actor, session).ok_or("spell state")?;
                assert_eq!(
                    state.owned_conditions().outfit_at(owner.owner_now().get()),
                    matches!(final_x, 5 | 7 | 8).then_some(267),
                    "quest swimming hook"
                );
                if final_x == 5 {
                    assert!(
                        !state
                            .owned_conditions()
                            .instances()
                            .iter()
                            .any(|i| i.definition().key().starts_with("fixture.harm")),
                        "quest swimming cleanse"
                    );
                }
            }
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
        step(&mut owner, mover, StepDirection::East, 2).await?;
        assert!(
            drowning(&owner, actor, session).await?,
            "step into 5405 drowns"
        );
        step(&mut owner, mover, StepDirection::East, 3).await?;
        assert!(
            drowning(&owner, actor, session).await?,
            "5405 to 5406 keeps drowning"
        );
        step(&mut owner, mover, StepDirection::East, 4).await?;
        assert!(
            !drowning(&owner, actor, session).await?,
            "step out removes drown"
        );
        // A diving helmet's installed suppression refuses the StepIn drown.
        install(&owner, actor, session, true).await?;
        step(&mut owner, mover, StepDirection::West, 3).await?;
        assert!(
            !drowning(&owner, actor, session).await?,
            "diving helmet protects"
        );
        step(&mut owner, mover, StepDirection::West, 2).await?;
        assert!(!drowning(&owner, actor, session).await?, "still protected");
        // Taking it off restores the unprotected StepIn.
        install(&owner, actor, session, false).await?;
        step(&mut owner, mover, StepDirection::East, 3).await?;
        assert!(drowning(&owner, actor, session).await?, "unprotected again");
        step(&mut owner, mover, StepDirection::East, 4).await?;
        step(&mut owner, mover, StepDirection::East, 5).await?;
        let mut failures = Vec::new();
        let before_trap = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, actor, session).ok_or("spell state")?;
            assert_eq!(
                state.owned_conditions().outfit_at(owner.owner_now().get()),
                Some(267)
            );
            if !state.owned_conditions().instances().iter().any(|instance| {
                instance.definition().key() == "environment.swimming"
                    && instance.remaining_duration_ms(owner.owner_now().get()) == Some(-1)
            }) {
                failures.push("permanent outfit swimming.lua:3");
            }
            state.vitals().health
        };
        step(&mut owner, mover, StepDirection::East, 6).await?;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, actor, session).ok_or("spell state")?;
            assert!(state.vitals().health < before_trap, "trap applies damage");
            assert!(
                state
                    .owned_conditions()
                    .outfit_at(owner.owner_now().get())
                    .is_none()
            );
        }
        // trap.lua:35 transforms the map item; a private marker cannot prove this.
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
        if appearance(6, 1) != Some(2146) {
            failures.push("shared map close trap.lua:35");
        }
        step(&mut owner, mover, StepDirection::East, 7).await?;
        if appearance(6, 1) != Some(2145) {
            failures.push("stepout reopen trap.lua:64");
        }
        seed_swimming_conditions(&owner, actor, session).await?;
        let before = health(&owner, actor, session).await?;
        step(&mut owner, mover, StepDirection::East, 8).await?;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, actor, session).ok_or("spell state")?;
            if state
                .owned_conditions()
                .instances()
                .iter()
                .any(|i| i.definition().key().starts_with("fixture.harm"))
            {
                failures.push("swimming to swimming cleanse swimming.lua:32-36");
            }
        }
        if optional {
            if health(&owner, actor, session).await? != before || appearance(8, 1) != Some(3482) {
                failures.push("optional jaw immunity trap.lua:4,31-32");
            }
        } else {
            let damage = before.saturating_sub(health(&owner, actor, session).await?);
            if !(15..=30).contains(&damage) || appearance(8, 1) != Some(3481) {
                failures.push("open PvP jaw damage and close trap.lua:4,35,44");
            }
        }
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("spell state")?;
            state.install_equipped_element_protection(
                crate::content::ReferenceResistanceKind::Earth,
                Some(crate::content::ReferenceRationalPercent::new(100, 1).map_err(debug)?),
            );
        }
        let before = health(&owner, actor, session).await?;
        step(&mut owner, mover, StepDirection::East, 9).await?;
        if health(&owner, actor, session).await? != before || appearance(9, 1) != Some(3945) {
            failures.push("earth trap uses earth protection trap.lua:5");
        }
        step(&mut owner, mover, StepDirection::East, 10).await?;
        if !optional && appearance(8, 1) != Some(3481) {
            failures.push("jaw stays closed trap.lua:69");
        }
        if appearance(9, 1) != Some(3944) {
            failures.push("earth reopen trap.lua:69");
        }
        if health(&owner, actor, session).await? != before || appearance(10, 1) != Some(2145) {
            failures.push("PZ immunity trap.lua:23-24");
        }
        step(&mut owner, mover, StepDirection::East, 11).await?;
        if health(&owner, actor, session).await? != before
            || appearance(11, 1) != Some(3482)
            || appearance(11, 2) != Some(3482)
        {
            failures.push("multiple jaws suppress entry trap.lua:27-28");
        }
        // A blocked combat hit must still close/reopen every spike (trap.lua:35,44).
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .get_mut(&runtime, actor, session)
                .ok_or("spell state")?
                .install_equipped_element_protection(
                    crate::content::ReferenceResistanceKind::Physical,
                    Some(crate::content::ReferenceRationalPercent::new(100, 1).map_err(debug)?),
                );
        }
        for (x, closed, open) in [(12, 2148, 2147), (13, 44530, 44529)] {
            step(&mut owner, mover, StepDirection::East, x).await?;
            if appearance(x as u16, 1) != Some(closed) {
                failures.push("other spikes close trap.lua:3,7");
            }
            step(&mut owner, mover, StepDirection::East, x + 1).await?;
            if appearance(x as u16, 1) != Some(open) {
                failures.push("other spikes reopen trap.lua:69");
            }
            if x == 12 {
                step(&mut owner, mover, StepDirection::West, 12).await?;
            }
        }
        // A lethal trap must use the existing hit/death owner, not just assign zero HP.
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("spell state")?;
            state.install_equipped_element_protection(
                crate::content::ReferenceResistanceKind::Physical,
                None,
            );
            let (healed, _) = state.after_health_gain(u64::MAX).ok_or("restore health")?;
            let (low, _) = crate::spell::actor_conditions::stage_creature_hit(
                &healed,
                healed.vitals().health.saturating_sub(10),
                owner.owner_now().get(),
            )
            .map_err(debug)?;
            *state = low;
        }
        step(&mut owner, mover, StepDirection::West, 13).await?;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            if states
                .get(&runtime, actor, session)
                .ok_or("spell state")?
                .vitals()
                .health
                != 0
                || !states.is_dead(actor)
            {
                failures.push("lethal trap must record death through existing damage owner");
            }
        }
        assert!(failures.is_empty(), "FIXM10 regressions: {failures:?}");
        Ok(())
    });
    eprintln!(
        "POSTGRES_FINISHED PAR-M10 callback={callback:?} portals={portals} optional={optional} duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    result
}

async fn health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<u32> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    Ok(states
        .get(&runtime, actor, session)
        .ok_or("spell state")?
        .vitals()
        .health)
}

async fn seed_swimming_conditions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement, StatusKind};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let state = states
        .get_mut(&runtime, actor, session)
        .ok_or("spell state")?;
    if let Some((healed, _)) = state.after_health_gain(u64::MAX) {
        *state = healed;
    }
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let facts = ApplicationFacts {
        now: owner.owner_now().get(),
        base_speed: 100,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([77; 16]),
    };
    let mut values = vec![
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: 60000,
        },
        ConditionValues::Speed {
            paralysis: true,
            range: oteryn_gameplay_domain::domain::condition::SpeedRange {
                a_min: 0,
                b_min: -10000,
                a_max: 0,
                b_max: -10000,
            },
            duration_ms: 60000,
        },
    ];
    for element in [
        DotElement::Poison,
        DotElement::Fire,
        DotElement::Energy,
        DotElement::Bleeding,
        DotElement::Drown,
        DotElement::Freezing,
        DotElement::Dazzled,
        DotElement::Cursed,
    ] {
        values.push(ConditionValues::DamageOverTime {
            element,
            total_min: 100,
            total_max: 100,
            per_tick: 1,
            interval_ms: 60000,
            delayed: true,
        });
    }
    for (index, value) in values.into_iter().enumerate() {
        let definition = ConditionDefinition::new(&format!("fixture.harm{index}"), 1, value)
            .ok_or("condition definition")?;
        state
            .conditions
            .apply(
                &definition,
                Some("fixture:swim".to_owned()),
                ConditionSourceKind::Field,
                &[],
                &facts,
            )
            .map_err(debug)?;
    }
    Ok(())
}
