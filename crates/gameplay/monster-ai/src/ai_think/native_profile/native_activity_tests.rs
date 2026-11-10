//! Actual pinned native policies/current Instance census; no callback authority is fabricated.
#![allow(
    clippy::expect_used,
    reason = "source fixture assertions require explicit failures"
)]
use super::*;
use crate::ai_think::summon_plan::MonsterThinkReader;
use oteryn_gameplay_entities::{
    ActorConditionTransition, ApplicationFacts, ConditionDefinition, ConditionSource,
    ConditionSourceKind, ConditionValues, NativeSummonAdmissionSpec,
};

fn uuid(tag: u8) -> [u8; 16] {
    [1, 144, 0, 0, 0, tag, 112, 0, 128, 0, 0, 0, 0, 0, 0, tag]
}
fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}
fn run(
    case: impl FnOnce(&NativeGameplayState, ChannelRuntimeV1) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let world = crate::content::native_gameplay_test_world_id()
                .map_err(|error| format!("{error:?}"))?;
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                world, &manifest,
            );
            let scope = crate::foundation::RuntimeScopeRefV1::instance(world, uuid(2))
                .map_err(|error| format!("{error:?}"))?;
            let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
                scope,
                crate::foundation::NodeId::decode(&uuid(3)).map_err(|e| format!("{e:?}"))?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                16,
                crate::content::channel_content_pin::ChannelContentPin::from_activation(
                    world,
                    1,
                    native.source_digest(),
                    [2; 32],
                    [3; 32],
                    [4; 32],
                    (100, 100, 7),
                ),
            )
            .map_err(|error| format!("{error:?}"))?;
            runtime
                .install_companion_policies(native.policies().clone())
                .map_err(|error| format!("{error:?}"))?;
            case(&native, runtime)
        })
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "native activity test thread".to_owned())?
}
fn actor(
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    position: MovementLocalPosition,
) -> Result<ExactActorRef, String> {
    runtime
        .spawn_arena_opponent(key, position, |_, _| Some(true))
        .map_err(|error| format!("{error:?}"))
}
fn reference(
    native: &NativeGameplayState,
    key: &str,
) -> Result<crate::content::ProjectV2DefinitionRef, String> {
    native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == key)
        .map(|record| record.profile.target.clone())
        .ok_or_else(|| format!("actual native {key}"))
}
struct NoSummons {
    called: bool,
}
impl MonsterThinkReader for NoSummons {
    async fn summon_facts(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ThinkOccurrence,
        _: Option<ExactActorRef>,
        _: Option<&[MovementLocalPosition]>,
    ) -> Option<super::super::profile_schedule::MonsterSummonFacts> {
        self.called = true;
        None
    }
}
fn think(
    table: &mut CreatureAiTable,
    runtime: &mut ChannelRuntimeV1,
    players: &[super::super::targeting::TargetCandidate],
    now: u64,
) -> Result<Vec<(ExactActorRef, ThinkReport)>, String> {
    let root = super::super::think_tests::root();
    let revisions = super::super::think_tests::revisions();
    let batch = table
        .reserve_current_thinks(runtime, now)
        .ok_or("current source reserve")?;
    let mut reader = NoSummons { called: false };
    let polled = {
        let future = table.run_reserved_current_thinks(
            runtime,
            batch,
            &root,
            &revisions,
            players,
            &mut reader,
            |_, _, _, _, _| {},
        );
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        std::future::Future::poll(future.as_mut(), &mut context)
    };
    assert!(
        !reader.called,
        "the actual test creature has no summon schedule"
    );
    match polled {
        std::task::Poll::Ready(reports) => Ok(reports),
        std::task::Poll::Pending => Err("no source summon IO".into()),
    }
}

#[test]
fn srx3_fresh_native_empty_instance_has_no_invented_first_think() -> Result<(), String> {
    run(|native, mut runtime| {
        let rat = actor(&mut runtime, "oteryn:creature.rat", at(100, 100))?;
        let mut table = CreatureAiTable::default();
        table
            .admit_native_creature(
                &runtime,
                native,
                rat,
                &reference(native, "oteryn:creature.rat")?,
                0,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            table.due(rat),
            None,
            "Game.placeCreature has no player spectator and ordinary fresh monster starts idle"
        );
        assert!(
            table
                .reserve_current_thinks(&mut runtime, 0)
                .expect("source reserve")
                .occurrences()
                .is_empty()
        );
        Ok(())
    })
}

#[test]
fn srx3_native_monster_uses_symmetric_eleven_tile_source_perception() -> Result<(), String> {
    run(|native, mut runtime| {
        let rat = actor(&mut runtime, "oteryn:creature.rat", at(100, 100))?;
        let session =
            crate::foundation::GameSessionId::decode(&uuid(4)).map_err(|e| format!("{e:?}"))?;
        let reserved = runtime
            .reserve_fresh_session(session)
            .map_err(|e| format!("{e:?}"))?;
        let player = runtime
            .commit_fresh_session(reserved)
            .map_err(|e| format!("{e:?}"))?;
        runtime
            .initialize_pinned_test_position(player, at(111, 111))
            .map_err(|e| format!("{e:?}"))?;
        let mut table = CreatureAiTable::default();
        table
            .admit_native_creature(
                &runtime,
                native,
                rat,
                &reference(native, "oteryn:creature.rat")?,
                0,
            )
            .map_err(|e| format!("{e:?}"))?;
        let players = [super::super::targeting::TargetCandidate {
            actor: player,
            position: at(111, 111),
            eligible: true,
            invisible: false,
            health: 100,
            damage: 0,
        }];
        let reports = think(&mut table, &mut runtime, &players, 0)?;
        assert_eq!(reports.len(), 1);
        assert_eq!(
            reports[0].1.target,
            Some(player),
            "source Creature.canSee uses11x11, independently of client8x6 view"
        );
        assert!(!reports[0].1.idle);
        Ok(())
    })
}

#[test]
fn srx3_fresh_native_canonical_condition_prevents_idle_without_players() -> Result<(), String> {
    run(|native, mut runtime| {
        let rat = actor(&mut runtime, "oteryn:creature.rat", at(100, 100))?;
        let definition = ConditionDefinition::new(
            "source.native.activity",
            1,
            ConditionValues::Light {
                level: 4,
                duration_ms: 5000,
            },
        )
        .ok_or("source light")?;
        let root = super::super::think_tests::root();
        let plan = runtime
            .prepare_actor_condition(
                rat,
                None,
                ActorConditionTransition::Apply {
                    definition: &definition,
                    source: ConditionSource {
                        actor: rat,
                        session: None,
                        kind: ConditionSourceKind::SelfUse,
                    },
                    immunities: &[],
                    facts: ApplicationFacts {
                        now: 0,
                        base_speed: 220,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: false,
                        decision_root: &root,
                        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                            [3; 16],
                        ),
                    },
                },
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .map_err(|e| format!("{e:?}"))?;
        runtime
            .commit_actor_condition(
                &plan,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .map_err(|e| format!("{e:?}"))?;
        let mut table = CreatureAiTable::default();
        table
            .admit_native_creature(
                &runtime,
                native,
                rat,
                &reference(native, "oteryn:creature.rat")?,
                0,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(table.due(rat), Some(0));
        let reports = think(&mut table, &mut runtime, &[], 0)?;
        assert!(!reports[0].1.idle);
        assert_eq!(table.due(rat), Some(1_000_000));
        Ok(())
    })
}

#[test]
fn srx3_native_default_faction_intrinsic_summon_stays_active_without_players() -> Result<(), String>
{
    run(|native, mut runtime| {
        let parent = actor(&mut runtime, "oteryn:creature.demon", at(100, 100))?;
        let child_ref = reference(native, "oteryn:creature.fire_elemental")?;
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target == child_ref)
            .ok_or("actual fire elemental")?;
        let Data::Creature(cp) = &record.profile.data else {
            return Err("actual creature family".into());
        };
        let parent_ref = reference(native, "oteryn:creature.demon")?;
        let parent_record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target == parent_ref)
            .ok_or("actual demon")?;
        let Data::Behavior(behavior) = &parent_record
            .behavior
            .as_ref()
            .ok_or("demon behavior")?
            .data
        else {
            return Err("actual behavior family".into());
        };
        let summons = behavior.summons.as_ref().ok_or("actual demon summons")?;
        let (index, entry) = summons
            .entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.creature == child_ref)
            .ok_or("actual source summon slot")?;
        let spec = NativeSummonAdmissionSpec::qualified(
            &parent_ref.key,
            &child_ref.key,
            i64::try_from(cp.health.ok_or("native HP")?).map_err(|e| e.to_string())?,
            index,
            summons.max_summons,
            entry.count,
            native.source_digest(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let stamp = runtime.issue_owner_work().map_err(|e| format!("{e:?}"))?;
        let child = runtime
            .admit_current_native_summon(stamp, parent, &spec, at(101, 100))
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            runtime
                .native_summon_role(child)
                .map_err(|e| format!("{e:?}"))?,
            Some(parent)
        );
        let mut table = CreatureAiTable::default();
        table
            .admit_native_creature(&runtime, native, child, &child_ref, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(table.due(child), Some(0));
        let reports = think(&mut table, &mut runtime, &[], 0)?;
        assert!(
            !reports[0].1.idle,
            "default-faction source intrinsic summon is not an idle ordinary creature"
        );
        assert_eq!(table.due(child), Some(1_000_000));
        Ok(())
    })
}

#[test]
fn srx3_fresh_native_spawn_return_fact_keeps_off_home_creature_active() -> Result<(), String> {
    run(|native, mut runtime| {
        let rat = actor(&mut runtime, "oteryn:creature.rat", at(101, 100))?;
        let reference = reference(native, "oteryn:creature.rat")?;
        let profile = project_native(&runtime, native, &reference).map_err(|e| format!("{e:?}"))?;
        let mut state = CreatureAiState::new(rat, profile, at(100, 100));
        // A controlled AI input, not a map placement grant. The actual native
        // admission above populates this same private field exclusively from
        // runtime.creature_spawn_cell; the movement owner still owns all steps.
        state.spawn = Some(at(100, 100));
        state.creature_key = Some(reference.key);
        state.creature_revision = Some(reference.revision);
        let mut table = CreatureAiTable::default();
        table
            .admit_current_native_state(&runtime, state, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(table.due(rat), Some(0));
        let reports = think(&mut table, &mut runtime, &[], 0)?;
        assert!(matches!(reports[0].1.goal,MovementGoal::WalkBack{spawn} if spawn==at(100,100)));
        assert!(!reports[0].1.idle);
        assert_eq!(table.due(rat), Some(1_000_000));
        Ok(())
    })
}

#[test]
fn srx3_native_game_created_birth_position_is_not_a_cpp_spawn_zone() -> Result<(), String> {
    run(|native, mut runtime| {
        let rat = actor(&mut runtime, "oteryn:creature.rat", at(100, 100))?;
        let before = runtime
            .read_actor_position(rat)
            .map_err(|e| format!("{e:?}"))?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(rat, before, at(101, 100))
            .map_err(|e| format!("{e:?}"))?;
        let reference = reference(native, "oteryn:creature.rat")?;
        let mut table = CreatureAiTable::default();
        table
            .admit_native_creature(&runtime, native, rat, &reference, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            table.due(rat),
            None,
            "Crystal Monster::isInSpawnLocation returns true without spawnMonster; a Game-created birth position grants no walkback or initial onThink"
        );
        assert!(think(&mut table, &mut runtime, &[], 0)?.is_empty());
        Ok(())
    })
}

#[test]
fn srx3_plagirath_registered_spell_flag_selects_actual_native_target_without_profile_rewrite()
-> Result<(), String> {
    run(|native, mut runtime| {
        let key = "oteryn:creature.plagirath";
        let source = reference(native, key)?;
        let caster = actor(&mut runtime, key, at(100, 100))?;
        let session = crate::foundation::GameSessionId::decode(&uuid(77))
            .map_err(|error| format!("{error:?}"))?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(|error| format!("{error:?}"))?;
        let player = runtime
            .commit_fresh_session(reservation)
            .map_err(|error| format!("{error:?}"))?;
        runtime
            .initialize_pinned_test_position(player, at(101, 100))
            .map_err(|error| format!("{error:?}"))?;
        let mut table = CreatureAiTable::default();
        table
            .admit_native_creature(&runtime, native, caster, &source, 0)
            .map_err(|error| format!("{error:?}"))?;
        let players = [super::super::targeting::TargetCandidate {
            actor: player,
            position: at(101, 100),
            eligible: true,
            invisible: false,
            health: 100,
            damage: 0,
        }];
        let before = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target == source)
            .ok_or("actual native Plagirath")?
            .clone();
        let mut selected = false;
        for sequence in 0..1024_u64 {
            let reports = think(&mut table, &mut runtime, &players, sequence * 1_000_000)?;
            for (_, report) in reports {
                for proposal in report.proposals {
                    if proposal.ability.key == "oteryn:ability.spell.plagirath_bog" {
                        assert_eq!(
                            report.target,
                            Some(player),
                            "actual native AI has an independently selected combat target"
                        );
                        assert_eq!(
                            proposal.target, player,
                            "pinned plagirath_bog.lua65 registered needTarget(true) must reach actual proposal, not caster self"
                        );
                        selected = true;
                    }
                }
            }
            if selected {
                break;
            }
        }
        assert!(
            selected,
            "actual native shared schedule must select source Plagirath callback"
        );
        assert_eq!(
            native
                .creature_profiles()
                .records
                .iter()
                .find(|record| record.profile.target == source),
            Some(&before),
            "source callback targeting projection cannot rewrite any native profile/behavior bytes"
        );
        Ok(())
    })
}

#[test]
fn srx3_native_cross_floor_target_list_keeps_thinks_without_attack_target() -> Result<(), String> {
    run(|native, mut runtime| {
        let mut table = CreatureAiTable::default();
        let reference = reference(native, "oteryn:creature.rat")?;
        let mut candidates = Vec::new();
        let mut cases = Vec::new();
        for (ordinal, floor) in [13, 12, 11].into_iter().enumerate() {
            let x = 100 + i32::try_from(ordinal).map_err(|e| e.to_string())? * 100;
            let caster_at = MovementLocalPosition {
                x,
                y: 100,
                floor: 14,
            };
            let player_at = MovementLocalPosition { floor, ..caster_at };
            let caster = actor(&mut runtime, "oteryn:creature.rat", caster_at)?;
            let session = crate::foundation::GameSessionId::decode(&uuid(80 + ordinal as u8))
                .map_err(|e| format!("{e:?}"))?;
            let reservation = runtime
                .reserve_fresh_session(session)
                .map_err(|e| format!("{e:?}"))?;
            let player = runtime
                .commit_fresh_session(reservation)
                .map_err(|e| format!("{e:?}"))?;
            runtime
                .initialize_pinned_test_position(player, player_at)
                .map_err(|e| format!("{e:?}"))?;
            candidates.push(super::super::targeting::TargetCandidate {
                actor: player,
                position: player_at,
                eligible: true,
                invisible: false,
                health: 100,
                damage: 0,
            });
            table
                .admit_native_creature(&runtime, native, caster, &reference, 0)
                .map_err(|e| format!("{e:?}"))?;
            cases.push((caster, floor >= 12));
        }
        let reports = think(&mut table, &mut runtime, &candidates, 0)?;
        for (caster, perceived) in cases {
            let report = reports.iter().find(|(actor, _)| *actor == caster);
            if perceived {
                let report = &report
                    .ok_or("source +/-2 floor spectator must schedule actual think")?
                    .1;
                assert_eq!(
                    report.target, None,
                    "source isTarget separately requires the same floor"
                );
                assert!(
                    !report.idle,
                    "cross-floor source targetList keeps parked forms active"
                );
                assert_eq!(table.due(caster), Some(1_000_000));
            } else {
                assert!(
                    report.is_none(),
                    "a source floor difference of three grants no initial think"
                );
                assert_eq!(table.due(caster), None);
            }
        }
        Ok(())
    })
}

#[test]
fn bloodjaw_source_retarget_keeps_a_player_nine_tiles_north() -> Result<(), String> {
    run(|native, mut runtime| {
        const KEY: &str = "oteryn:creature.elder_bloodjaw";
        let bloodjaw = actor(&mut runtime, KEY, at(100, 100))?;
        let mut players = Vec::new();
        for (seed, position) in [(4, at(101, 100)), (5, at(100, 91))] {
            let session = crate::foundation::GameSessionId::decode(&uuid(seed))
                .map_err(|e| format!("{e:?}"))?;
            let reserved = runtime
                .reserve_fresh_session(session)
                .map_err(|e| format!("{e:?}"))?;
            let player = runtime
                .commit_fresh_session(reserved)
                .map_err(|e| format!("{e:?}"))?;
            runtime
                .initialize_pinned_test_position(player, position)
                .map_err(|e| format!("{e:?}"))?;
            players.push(player);
        }
        let mut table = CreatureAiTable::default();
        table
            .admit_native_creature(&runtime, native, bloodjaw, &reference(native, KEY)?, 0)
            .map_err(|e| format!("{e:?}"))?;
        let stamp = runtime.issue_owner_work().map_err(|e| format!("{e:?}"))?;
        assert!(
            table
                .set_current_bloodjaw_target(&runtime, stamp, bloodjaw, players[0], 0)
                .map_err(|e| format!("{e:?}"))?
        );
        // Crystal 201189f8 Creature::setAttackedCreature uses its eleven-tile view.
        assert!(!super::super::targeting::sees(at(100, 91), at(100, 100)));
        assert!(
            table
                .set_current_bloodjaw_target(&runtime, stamp, bloodjaw, players[1], 1)
                .map_err(|e| format!("{e:?}"))?
        );
        assert_eq!(
            table.state(bloodjaw).and_then(|state| state.target()),
            Some(players[1])
        );
        Ok(())
    })
}
