//! Crystal 201189f8 grand_master_oberon_immunity.lua:3-7,
//! grand_master_oberon.lua:125-159, grand_master_oberon_functions.lua:1-60,
//! and library_area/creaturescripts_ghulosh.lua:19-60.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks_fixtures::{local, wire};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, ExactActorRef, GameSessionId,
    MovementPositionSnapshot, OwnerDamageCommand,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
    kill_reward::{KillSink, record_creature_kill},
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::{
    actor_spell,
    chat::{self, ChatDisposition, ChatIntent, ChatSpeechMode},
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
        encode_use_intent,
    },
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{Arc, atomic::Ordering};

const QUEST: &str = "oteryn:quest.fixture_library_boss";
const OBERON: &str = "oteryn:creature.grand_master_oberon";
const GHULOSH: &str = "oteryn:creature.ghulosh";
const DEATHGAZE: &str = "oteryn:creature.ghulosh_deathgaze";
const BOOK: &str = "oteryn:creature.the_book_of_death";
const DEATH: &str = "oteryn:creature.concentrated_death";
const LEVER: Position = at(100, 100);
const STAGING: Position = at(101, 100);
const SECOND_STAGE: Position = at(102, 100);
const LANDING: Position = at(103, 104);
const BOSS_AT: Position = at(104, 104);
const BOOK_AT: Position = at(104, 105);
const MIN: Position = at(102, 102);
const MAX: Position = at(107, 107);
const EXIT_PAD: Position = at(103, 103);
const EXIT: Position = at(99, 100);
const EXIT_CLEAR: Position = at(98, 100);

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -10 }
}

#[test]
fn oberon_scheduler_heals_and_summons_then_only_correct_say_removes_immunity() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        exercise(admin, OBERON, false).await
    })
}

#[test]
fn ghulosh_scheduler_transforms_with_current_hp_and_book_death_spawns_concentrated_death()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        exercise(admin, GHULOSH, false).await
    })
}

#[test]
fn library_lever_admits_two_staged_players_to_one_cell_and_both_step_out() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        exercise(admin, OBERON, true).await
    })
}

fn catalogue(boss: &str) -> TestResult<QuestStateCatalogue> {
    const OBERON_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_secret_library_quest/the_order_of_the_falcon/grand_master_oberon_immunity.lua:3-7";
    const GHULOSH_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_secret_library_quest/library_area/creaturescripts_ghulosh.lua:19-60";
    let mut monsters = vec![json!({"position": BOSS_AT, "creature": boss})];
    let (source, phases) = if boss == OBERON {
        (
            OBERON_SOURCE,
            json!({"speech_shield": {
                "creature": OBERON, "at_percent": 20, "lives": 3, "cooldown_ms": 1000,
                "prompts": [{"speech": "You appear like a worm among men!",
                    "answers": ["How appropriate, you look like something worms already got the better of!"]}],
                "summons": ["oteryn:creature.falcon_knight", "oteryn:creature.falcon_paladin"],
                "accepted_speech": "GRRRAAANNGH!", "rejected_speech": "HAHAHAHA!", "immunity_effect": 50
            }}),
        )
    } else {
        monsters.push(json!({"position": BOOK_AT, "creature": BOOK}));
        (
            GHULOSH_SOURCE,
            json!({
                "transformation": {"creature": GHULOSH, "replacement": DEATHGAZE,
                    "at_percent": 75, "speech": "FEEL MY WRATH!!"},
                "death_replacement": {"creature": BOOK, "replacement": DEATH}
            }),
        )
    };
    let data = json!({"schema": 1, "quests": [{
        "quest": QUEST, "recipe_sha256": Sha256::digest(source).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "sources": [source],
        "tracks": [{"key": "oteryn:quest-progress/fixture-library-boss/eligible",
            "initial": 1, "minimum": 1, "maximum": 1}],
        "stages": [{
            "key": "oteryn:quest-stage/fixture-library-boss/entry",
            "completes": false,
            "trigger": {"kind": "map_use", "callback": {
                "key": "oteryn:quest-callback/fixture-library-boss/entry",
                "quest": QUEST, "trigger": "use",
                "selector": {"position": LEVER, "ordinal": 1, "appearance": 2772},
                "storage": [], "lever": (boss == OBERON).then_some([2772, 2773]), "reversible": false,
                "effects": {"encounter": {
                    "players": [{"from": STAGING, "to": LANDING}, {"from": SECOND_STAGE, "to": LANDING}],
                    "monsters": monsters, "room_min": MIN, "room_max": MAX,
                    "minimum_level": if boss == OBERON { 0 } else { 1 }, "boss_phases": phases
                }},
                "passages": [], "offerings": [], "sources": [source]
            }}
        }]
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: String, boss: &str, party: bool) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "library_boss_phase", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("content generation")?;
    let caster_case = boss == OBERON && !party;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    if caster_case {
        deps.spells = native.spell_book().clone();
    }
    let catalogue = catalogue(boss)?;
    let callback = catalogue
        .authored_bindings()
        .ok_or("authored library boss")?
        .world_callbacks()
        .next()
        .ok_or("boss lever")?
        .clone();
    let world = super::quest_world_callbacks_fixtures::world_with_cells(
        &callback,
        LEVER,
        &[EXIT_PAD, EXIT, EXIT_CLEAR],
    )?;
    super::arena_match_fixtures::fixture::runtime_at(&mut deps, generation, &pin, &world, local(STAGING))?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue));
    if !caster_case {
        deps.initialize(&owner).await;
    }
    let peer = if party {
        let session = GameSessionId::decode(&id(51)).map_err(debug)?;
        let peer = {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    peer,
                    session,
                    CharacterLease::new(CharacterId::decode(&id(52)).map_err(debug)?, 1)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(peer, local(SECOND_STAGE))
                .map_err(debug)?;
            peer
        };
        deps.initialize_player(&owner, peer, session).await;
        let exit: Callback = serde_json::from_value(json!({
            "key": "oteryn:quest-callback/fixture-library-boss/exit", "quest": QUEST,
            "trigger": "step_in", "selector": {"position": EXIT_PAD, "ordinal": 0},
            "teleport": EXIT, "effects": {"teleport_effect": 11},
            "storage": [], "offerings": [], "passages": [], "reversible": false,
            "sources": callback.sources
        }))?;
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback, exit])
            .map_err(|_| "boss room callback fixture")?;
        Some((peer, session))
    } else {
        None
    };
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 7, 11, 15, 18])
            .ok_or("map USE and chat capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest session")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    if caster_case {
        let session = admitted.game_session_id;
        owner.admit_progression_session(&mut admitted).await;
        let progression = owner.progression_binding(session).ok_or("progression")?;
        let formula = native.training_formula().ok_or("training")?;
        let fresh = || super::death_ml_loss_composed_fixtures::current_fence(&owner, session);
        let druid =
            convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
        h.root
            .commit_character_build(
                &authority,
                &h.node,
                fresh().await?,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(91)).map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: druid.clone(),
                    pruned_stance: None,
                },
                formula,
            )
            .await
            .map_err(debug)?;
        h.root
            .commit_character_experience(
                &authority,
                &h.node,
                fresh().await?,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(92)).map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(98_800),
                    context: progression.context.clone(),
                    policy_revision: progression.policy_revision.clone(),
                    reward_revision: progression.reward_revision.clone(),
                    policy: progression.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        h.root
            .initialize_familiar_group(&authority, &h.node, fresh().await?)
            .await
            .map_err(debug)?;
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    deps.actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        level: 20,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, deps.actor, session, &druid));
        states
            .get_mut(&runtime, deps.actor, session)
            .ok_or("caster")?
            .enable_owned_training(&druid, formula)
            .map_err(debug)?;
    }
    owner.admit_quest_session(&admitted).await;
    owner
        .states
        .test_owner_time
        .store(1_000_001, Ordering::Relaxed);
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(LEVER.floor, LEVER.x, LEVER.y, 1)
        .ok_or("lever key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let request = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let result = wire(&owner, &mut admitted, 1, COMMAND_TYPE_USE_INTENT, request).await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    if let Some((peer, peer_session)) = peer {
        let participants = [(deps.actor, admitted.game_session_id), (peer, peer_session)];
        {
            let runtime = owner.runtime.lock().await;
            for (actor, _) in participants {
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    local(LANDING)
                );
            }
        }
        let permit = owner
            .spell_lane_permit()
            .await
            .ok_or("source step permit")?;
        for (actor, session) in participants {
            let (origin, entered) = move_to(&owner, actor, session, EXIT_PAD).await?;
            let after = owner
                .step_quest_callback(&permit, actor, session, origin, entered)
                .await;
            assert_eq!(after.position(), local(EXIT));
            if actor == deps.actor {
                move_to(&owner, actor, session, EXIT_CLEAR).await?;
            }
        }
        return Ok(());
    }
    let target = find(&owner, boss).await?;
    let initial = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(target)
        .map_err(debug)?
        .health;
    let reduced = if boss == OBERON {
        initial / 5
    } else {
        initial * 3 / 4
    };
    strike(
        &owner,
        &admitted,
        target,
        boss,
        initial - reduced,
        100,
        false,
    )
    .await?;
    owner.drain_quest_encounter_deadlines().await;
    if boss == OBERON {
        let shield = owner
            .runtime
            .lock()
            .await
            .companion_snapshot(target)
            .map_err(debug)?;
        assert_eq!(shield.health, initial);
        assert!(
            shield
                .state
                .policy
                .damage_immunities
                .iter()
                .any(|key| key == "physical")
        );
        assert_eq!(
            owner.runtime.lock().await.boss_speech_shield_effect(target),
            Some(50)
        );
        let field = oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry {
            definition:
                oteryn_gameplay_foundation::foundation::condition::ConditionDefinition::new(
                    "test.oberon.agony",
                    1,
                    oteryn_gameplay_domain::domain::condition::ConditionValues::Agony {
                        total_min: 10,
                        total_max: 10,
                        per_tick: 10,
                        interval_ms: 1000,
                        delayed: true,
                    },
                )
                .ok_or("agony condition")?,
            source: None,
            lineage: None,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(90)),
        };
        owner
            .runtime
            .lock()
            .await
            .commit_creature_field_entry(target, 2_000_000, Some(&field))
            .map_err(debug)?;
        owner
            .states
            .spell_states
            .lock()
            .await
            .next_monster_ai_pass_us = 0;
        owner
            .drain_monster_melee_at(
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(3_000_000),
            )
            .await;
        let after_tick = owner
            .runtime
            .lock()
            .await
            .companion_snapshot(target)
            .map_err(debug)?;
        assert_eq!(after_tick.health, initial);
        assert!(after_tick.state.conditions.instances().is_empty());
        let shield_cue = |(_, tile, effect): &(_, crate::map::overlay::TilePos, _)| {
            i32::from(tile.x) == after_tick.position.x
                && i32::from(tile.y) == after_tick.position.y
                && -i16::from(tile.floor) == after_tick.position.floor
                && *effect == 50
        };
        let periodic_effects = map.facts.movement_effects_since(0);
        assert_eq!(
            periodic_effects
                .iter()
                .filter(|cue| shield_cue(cue))
                .count(),
            1
        );
        let after_periodic_cue = periodic_effects.last().ok_or("periodic cue")?.0;
        let spell = (1..=deps.spells.source_len())
            .filter_map(|index| {
                u32::try_from(index)
                    .ok()
                    .and_then(std::num::NonZeroU32::new)
            })
            .find(|index| {
                deps.spells
                    .indexed(*index)
                    .is_some_and(|spell| spell.name == "Energy Strike")
            })
            .ok_or("energy strike")?;
        owner
            .states
            .test_owner_time
            .store(4_000_001, Ordering::Relaxed);
        let result = wire(
            &owner,
            &mut admitted,
            2,
            actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
            actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
                spell,
                target: actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                    x: after_tick.position.x,
                    y: after_tick.position.y,
                    floor: after_tick.position.floor,
                }),
                aim_at_target: false,
            }),
        )
        .await?;
        assert_eq!(
            actor_spell::decode_spell_cast_result(&result).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .companion_snapshot(target)
                .map_err(debug)?
                .health,
            initial
        );
        assert_eq!(
            map.facts
                .movement_effects_since(after_periodic_cue)
                .iter()
                .filter(|cue| shield_cue(cue))
                .count(),
            1
        );
        strike(&owner, &admitted, target, OBERON, initial, 101, false).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .companion_snapshot(target)
                .map_err(debug)?
                .health,
            initial
        );
        assert!(
            find(&owner, "oteryn:creature.falcon_knight").await.is_ok()
                || find(&owner, "oteryn:creature.falcon_paladin").await.is_ok()
        );
        assert!(
            map.facts
                .movement_speech_since(0)
                .iter()
                .any(|(_, line)| line.text == "You appear like a worm among men!")
        );
        owner.chat_open(admitted.game_session_id).await;
        for (id, us, text) in [
            (3, 10_000_000, "wrong"),
            (
                4,
                12_000_000,
                "HOW APPROPRIATE, YOU LOOK LIKE SOMETHING WORMS ALREADY GOT THE BETTER OF!",
            ),
        ] {
            owner
                .states
                .test_owner_time
                .store(us + 1, Ordering::Relaxed);
            let result = wire(
                &owner,
                &mut admitted,
                id,
                chat::COMMAND_TYPE_CHAT_INTENT,
                chat::encode_chat_intent(&ChatIntent::Say {
                    mode: ChatSpeechMode::Say,
                    text: text.into(),
                })
                .map_err(debug)?,
            )
            .await?;
            assert_eq!(
                chat::decode_chat_intent_result(&result)
                    .map_err(debug)?
                    .disposition,
                ChatDisposition::Ok
            );
            assert_eq!(
                owner.runtime.lock().await.boss_speech_shield_effect(target),
                if id == 3 { Some(50) } else { None }
            );
        }
        let lines = map.facts.movement_speech_since(0);
        for expected in ["HAHAHAHA!", "GRRRAAANNGH!"] {
            assert!(lines.iter().any(|(_, line)| line.text == expected));
        }
        strike(&owner, &admitted, target, OBERON, 1, 102, false).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .companion_snapshot(target)
                .map_err(debug)?
                .health,
            initial - 1
        );
    } else {
        let phase = owner
            .runtime
            .lock()
            .await
            .companion_snapshot(target)
            .map_err(debug)?;
        assert_eq!(phase.state.policy.definition_key, DEATHGAZE);
        assert_eq!(phase.health, reduced);
        assert_eq!(phase.position, local(BOSS_AT));
        assert!(
            map.facts
                .movement_speech_since(0)
                .iter()
                .any(|(_, line)| line.text == "FEEL MY WRATH!!")
        );
        let book = find(&owner, BOOK).await?;
        let hp = owner
            .runtime
            .lock()
            .await
            .companion_snapshot(book)
            .map_err(debug)?
            .health;
        strike(&owner, &admitted, book, BOOK, hp, 101, true).await?;
        let death = find(&owner, DEATH).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(death)
                .map_err(debug)?
                .position(),
            local(BOOK_AT)
        );
    }
    Ok(())
}

async fn move_to(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    destination: Position,
) -> TestResult<(MovementPositionSnapshot, MovementPositionSnapshot)> {
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let origin = runtime.read_actor_position(actor).map_err(debug)?;
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        origin,
        local(destination),
        false,
        &closed,
    )
    .map_err(debug)?;
    let [entered] = runtime
        .commit_source_steps_and_project(vec![proof], || true)
        .map_err(debug)?
        .as_slice()
        .try_into()
        .map_err(|_| "one admitted step")?;
    Ok((origin, entered))
}

async fn find(owner: &ComposedFreshAdmission<'_, '_, '_>, key: &str) -> TestResult<ExactActorRef> {
    let runtime = owner.runtime.lock().await;
    runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .find(|actor| runtime.matches_live_creature_identity(*actor, key.as_bytes()))
        .ok_or_else(|| format!("live creature {key}").into())
}

#[allow(clippy::too_many_arguments)]
async fn strike(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
    key: &str,
    damage: i64,
    command: u64,
    lethal: bool,
) -> TestResult {
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let lease = runtime
        .bound_attacker_lease(actor, admitted.game_session_id)
        .map_err(debug)?;
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            target,
            actor,
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(command).map_err(debug)?,
            ),
            0,
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence: b"library-phase",
                binding: b"library-phase-fixture",
                damage,
            },
        )
        .map_err(debug)?;
    if lethal {
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: Some(&states),
                activity: &[],
                queue: &queue,
                rewards: owner.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: admitted.game_session_id,
                actor,
            },
            now,
            Some(observed),
        );
    }
    Ok(())
}
