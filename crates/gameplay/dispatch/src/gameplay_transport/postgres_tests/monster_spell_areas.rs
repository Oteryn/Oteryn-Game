//! Native monster spells through the production monster Think drain.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::native_gameplay::{
    CreatureProfilesDocument, NativeGameplayInput, PinnedGameplayBytes, test_support,
};
use crate::foundation::{ChannelRuntimeV1, GameSessionId, MovementLocalPosition, WorldId};
use crate::gameplay_transport::last_position::AdmissionPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use sha2::{Digest, Sha256};
use std::time::Duration;

const CANDY_HORROR: &str = "oteryn:creature.candy_horror";
const RECIPIENT: &str = "oteryn:creature.fire_elemental";

#[test]
fn candy_horror_wave_executes_both_directional_earth_masks_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "candy_horror_wave", CANDY_HORROR, None)
}

#[test]
fn heal_monster_restores_a_nearby_masterless_monster_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(
        admin,
        "heal_monster_area",
        "oteryn:creature.orc_cult_priest",
        Some((1, 100, 300)),
    )
}

#[test]
fn heal_monster_9x9_reaches_beyond_the_short_heal_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(
        admin,
        "heal_monster_large_area",
        "oteryn:creature.mezlon_the_defiler_stop",
        Some((5, 1, 1000)),
    )
}

fn run_case(
    admin: String,
    tag: &'static str,
    caster: &'static str,
    heal: Option<(i32, i64, i64)>,
) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run(tag, move || {
        run(async move |_| {
            let harness = Harness::create_with_admission_seed(admin, tag, false, false).await?;
            let result = scenario(&harness, caster, heal).await;
            let cleanup = harness.cleanup().await;
            result?;
            cleanup?;
            Ok(())
        })
    })
}

fn pin_bytes(bytes: Vec<u8>) -> PinnedGameplayBytes {
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    PinnedGameplayBytes { sha256, bytes }
}

fn fixture_input(caster: &str) -> TestResult<NativeGameplayInput> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut input = NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
        .map_err(debug)?;
    input.source_world = Some(pin_bytes(std::fs::read(
        root.join("tools/content-schema/native-gameplay/canary-thalom-world.json"),
    )?));
    let mut profiles: CreatureProfilesDocument =
        serde_json::from_slice(&input.creature_profiles.bytes)?;
    use crate::content::ProjectV2AuthoringProfileData as Data;
    for key in [caster, RECIPIENT] {
        let record = profiles
            .records
            .iter_mut()
            .find(|row| row.profile.target.key == key)
            .ok_or("source creature profile")?;
        let behavior_root = record.behavior.as_mut().ok_or("source behavior")?;
        let Data::Behavior(behavior) = &mut behavior_root.data else {
            return Err("source behavior family".into());
        };
        let definitions = record
            .attack_definitions
            .as_mut()
            .ok_or("source ability closure")?;
        // Isolate the exercised spell and hold positions. Keep source cast schedules and formulas.
        behavior.movement.can_walk = false;
        behavior.attacks.clear();
        record.monster_melee = None;
        if key == CANDY_HORROR {
            // Crystal 201189f data-global/monster/magicals/candy_horror.lua and
            // data/scripts/spells/attack/candy_horror_wave.lua. The content lane's
            // parent association is provided here as a complete source cast slot.
            let schedule: crate::content::ProjectV2AbilitySchedule = serde_json::from_value(
                serde_json::json!({
                    "ability": {"family":"Ability","key":"oteryn:ability.spell.candy_horror_wave","revision":"definition-r1"},
                    "interval_ms":2000,"chance_ppm":200000,
                    "magnitude":{"minimum":0,"maximum":352}
                }),
            )?;
            let parent = schedule.ability.clone();
            behavior.attacks.push(schedule);
            let Data::Creature(creature) = &mut record.profile.data else {
                return Err("source creature family".into());
            };
            creature.abilities.push(parent.clone());
            definitions.profiles.push(serde_json::from_value(serde_json::json!({
                "target":parent,
                "data":{"kind":"Ability","profile":{"details":{
                    "kind":"Spell","range_tiles":0,"needs_target":false,"needs_direction":true,
                    "encounter":{"family":"Encounter","key":"oteryn:encounter.source_core_candy_horror","revision":"definition-r1"}
                }}}
            }))?);
            definitions.records.push(serde_json::from_value(serde_json::json!({
                "kind":"Ability","identity":{"family":"Ability","key":parent.key,"revision":parent.revision},
                "effects":[]
            }))?);
        }
        for root in [&record.profile, &*behavior_root] {
            *definitions
                .profiles
                .iter_mut()
                .find(|profile| profile.target == root.target)
                .ok_or("source closure root")? = root.clone();
        }
    }
    input.creature_profiles = pin_bytes(serde_json::to_vec(&profiles)?);
    Ok(input)
}

async fn scenario(harness: &Harness, caster: &str, heal: Option<(i32, i64, i64)>) -> TestResult {
    use crate::map::{boot::BundleCollisionIndexExt, facts::CreatureVisualKind};

    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        test_support::try_activate_input(world, &fixture_input(caster)?).map_err(debug)?;
    let active = controller.active().ok_or("active native generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let mut deps = Dependencies::new()?;
    deps.room = pin.qualified_room().clone();
    deps.combat_room = Some(deps.room.clone());
    let content = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        deps.room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        content,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    let y = if heal.is_some() { 11 } else { 12 };
    runtime
        .initialize_source_pinned_lab_player_position(
            deps.actor,
            session,
            MovementLocalPosition { x: 3, y, floor: -7 },
        )
        .map_err(debug)?;
    let far_wave_player = if heal.is_none() {
        let session = GameSessionId::decode(&id(51)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_source_pinned_lab_player_position(
                actor,
                session,
                MovementLocalPosition { x: 4, y, floor: -7 },
            )
            .map_err(debug)?;
        Some((actor, session))
    } else {
        None
    };
    let caster_x = if heal.is_some() { 0 } else { 1 };
    let mut placements = vec![(caster_x, caster)];
    if heal.is_some() {
        placements.extend([(1, RECIPIENT), (5, RECIPIENT)]);
    }
    let spawns = placements
        .into_iter()
        .map(|(x, key)| {
            let health =
                u64::try_from(runtime.companion_policy(key).map_err(debug)?.maximum_health)?;
            Ok(((x, y, -7), 60_000, key.to_owned(), health, false))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let count = spawns.len();
    runtime
        .begin_activation_spawn_sources(vec![(spawns, 5_000)])
        .map_err(debug)?;
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .map_err(debug)?
            .realized,
        count
    );
    let mut recipients = std::collections::BTreeMap::new();
    for entry in runtime.visible_entities().creatures {
        if runtime.matches_live_creature_identity(entry.actor, RECIPIENT.as_bytes()) {
            let at = runtime
                .read_actor_position(entry.actor)
                .map_err(debug)?
                .position();
            runtime
                .set_test_creature_health(entry.actor, 1)
                .map_err(debug)?;
            recipients.insert(at.x, entry.actor);
        }
    }
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        deps.room.movement_cells().index()
    else {
        return Err("source world bundle".into());
    };
    let max_health = 100_000;
    let initial_health = if heal.is_some() {
        max_health / 2
    } else {
        max_health
    };
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        for (actor, session) in [(deps.actor, session)].into_iter().chain(far_wave_player) {
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        max_health,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("initialize player spell state")?;
            states
                .get_mut(&runtime, actor, session)
                .ok_or("current player")?
                .set_health_for_test(initial_health);
        }
    }
    let fence = owner
        .current_monk_fence(session)
        .await
        .map_err(debug)?
        .ok_or("current player fence")?;
    assert!(owner.track_restored(session, deps.actor, &AdmissionPosition::FirstLogin, fence));
    owner.clock_origin -= Duration::from_secs(10);
    for _ in 0..64 {
        owner.drain_monster_melee().await;
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let player_health = states
            .get(&runtime, deps.actor, session)
            .ok_or("current player")?
            .vitals()
            .health;
        if let Some((x, minimum, maximum)) = heal {
            let health_at = |x| -> TestResult<i64> {
                let actor = recipients.get(&x).ok_or("source heal recipient")?;
                Ok(runtime
                    .companion_snapshot(*actor)
                    .map_err(debug)?
                    .state
                    .health)
            };
            let health = health_at(x)?;
            assert_eq!(
                player_health, initial_health,
                "area heals skip wounded players"
            );
            if x == 1 {
                assert_eq!(
                    health_at(5)?,
                    1,
                    "the short heal cannot reach the far monster"
                );
            }
            if health > 1 {
                assert!((minimum..=maximum).contains(&(health - 1)));
                return Ok(());
            }
        } else {
            // Facing east from (1,12), (3,12) belongs only to DRAWBLOOD
            // and (4,12) belongs only to GREEN_RINGS.
            let mut drawblood = false;
            let mut green_rings = false;
            for (_, cue) in index.map().facts.creature_visuals_since(0) {
                if let CreatureVisualKind::Effect { position, effect } = cue {
                    if (position.x, position.y, effect) == (3, 12, 1) {
                        drawblood = true;
                    }
                    if (position.x, position.y, effect) == (4, 12, 9) {
                        green_rings = true;
                    }
                    assert_ne!((position.x, position.y, effect), (2, 12, 9));
                }
            }
            let (far_actor, far_session) = far_wave_player.ok_or("far wave player")?;
            let far_health = states
                .get(&runtime, far_actor, far_session)
                .ok_or("current far player")?
                .vitals()
                .health;
            if player_health < max_health && far_health < max_health && drawblood && green_rings {
                return Ok(());
            }
        }
        drop(states);
        drop(runtime);
        owner.clock_origin -= Duration::from_secs(2);
    }
    Err("native Think never published the expected spell effects".into())
}
