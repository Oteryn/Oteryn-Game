//! Admitted native profiles: missing executable ability bodies remain Unknown.
use super::*;
use crate::content::ProjectV2AuthoringProfileData as Data;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::ChannelRuntimeV1;

pub(crate) fn project_native(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    creature: &crate::content::ProjectV2DefinitionRef,
) -> Result<CreatureBehaviourProfile, behaviour_profile::ProfileRefusal> {
    use behaviour_profile::ProfileRefusal;
    if native.source_digest() != runtime.content_pin().server_artifact_digest() {
        return Err(ProfileRefusal::Invalid);
    }
    let mut records = native
        .creature_profiles()
        .records
        .iter()
        .filter(|record| &record.profile.target == creature);
    let record = records.next().ok_or(ProfileRefusal::Missing)?;
    if records.next().is_some() {
        return Err(ProfileRefusal::Invalid);
    }
    if matches!(
        runtime.binding().runtime_scope(),
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) && let Some(source) = native.soul_war_source_attacks(creature)
    {
        return project_native_parts(record, source.behavior(), source.abilities());
    }
    if let Some(source) = native.omrafir_source_attacks(creature) {
        return project_native_parts(record, source.behavior(), source.abilities());
    }
    project_native_record(record)
}

/// The projection of one admitted native creature record (the source digest already checked).
pub(crate) fn project_native_record(
    record: &crate::content::native_gameplay::CreatureProfileRecord,
) -> Result<CreatureBehaviourProfile, behaviour_profile::ProfileRefusal> {
    use behaviour_profile::ProfileRefusal;
    let Some(Data::Behavior(behavior)) = record.behavior.as_ref().map(|p| &p.data) else {
        return Err(ProfileRefusal::Missing);
    };
    project_native_parts(record, behavior, &BTreeMap::new())
}

fn project_native_parts(
    record: &crate::content::native_gameplay::CreatureProfileRecord,
    behavior: &crate::content::ProjectV2BehaviorAuthoring,
    added: &BTreeMap<
        crate::content::ProjectV2DefinitionRef,
        crate::content::ProjectV2AbilityAuthoring,
    >,
) -> Result<CreatureBehaviourProfile, behaviour_profile::ProfileRefusal> {
    use behaviour_profile::ProfileRefusal;
    let mut abilities = added.clone();
    for profile in
        record
            .source_item_abilities
            .iter()
            .chain(record.attack_definitions.iter().flat_map(|definitions| {
                definitions
                    .profiles
                    .iter()
                    .filter(|profile| matches!(profile.data, Data::Ability(_)))
            }))
    {
        let Data::Ability(ability) = &profile.data else {
            return Err(ProfileRefusal::Invalid);
        };
        // Source-item operations may also be present in the complete canonical attack
        // closure. A shared exact body is one profile; conflicting bodies still refuse.
        if abilities
            .insert(profile.target.clone(), ability.clone())
            .is_some_and(|previous| previous != *ability)
        {
            return Err(ProfileRefusal::Invalid);
        }
    }
    // Preserve every source schedule slot, even when another slot's callback has no executable
    // closure. Missing bodies cannot be manufactured from names, presentation or magnitude.
    let mut projected = behavior.clone();
    // A resolved native melee body still uses the existing separately qualified swing
    // owner. Its source bounds supply the magnitude required by the think projection;
    // no Ability body or damage formula is inferred from a reference.
    if let Some(melee) = &record.monster_melee {
        for entry in &mut projected.attacks {
            if entry.ability == melee.ability && abilities.contains_key(&entry.ability) {
                entry.magnitude = Some(crate::content::ProjectV2Magnitude {
                    minimum: u64::from(melee.minimum),
                    maximum: u64::from(melee.maximum),
                });
            }
        }
    }
    CreatureBehaviourProfile::project_native(Some(&projected), &abilities)
}

impl CreatureAiTable {
    /// Preflight the exact native Soulpit closure before an encounter debits its core.
    /// Crystal 201189f soulpit_fight.lua:67,87-88,131 accepts passive species too;
    /// src/creatures/monsters/monster.cpp:2341-2348,2752-2759 changes stack/HP/rewards,
    /// never hostility. This grants neither an actor nor an AI registration.
    pub fn qualify_soulpit_creature(
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        creature: &crate::content::ProjectV2DefinitionRef,
    ) -> Result<(), behaviour_profile::ProfileRefusal> {
        project_native(runtime, native, creature).map(|_| ())
    }

    /// Re-read a source setType on the same exact physical incarnation. This
    /// changes no schedule clock, target, home, sequence, due time or RNG result.
    pub fn refresh_current_native_creature(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        actor: ExactActorRef,
    ) -> Result<bool, behaviour_profile::ProfileRefusal> {
        use crate::monster_summon::{CurrentSourceCreatureRole, current_source_creature_role};
        use behaviour_profile::ProfileRefusal;
        if native.source_digest() != runtime.content_pin().server_artifact_digest() {
            return Err(ProfileRefusal::Invalid);
        }
        let role =
            current_source_creature_role(runtime, actor).map_err(|_| ProfileRefusal::Invalid)?;
        if !matches!(
            role,
            CurrentSourceCreatureRole::Ordinary
                | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)
        ) {
            return Err(ProfileRefusal::Invalid);
        }
        let snapshot = runtime
            .companion_snapshot(actor)
            .map_err(|_| ProfileRefusal::Invalid)?;
        let creature = crate::content::ProjectV2DefinitionRef {
            family: crate::content::ProjectV2Family::Creature,
            key: snapshot.state.policy.definition_key.clone(),
            revision: snapshot.state.policy.definition_revision.clone(),
        };
        let state = &self.entry(actor).ok_or(ProfileRefusal::Missing)?.state;
        if state.creature_key.as_deref() == Some(creature.key.as_str())
            && state.creature_revision.as_deref() == Some(creature.revision.as_str())
        {
            return Ok(false);
        }
        let profile = project_native(runtime, native, &creature)?;
        if role.is_ordinary() && (!profile.targeting().hostile || !profile.targeting().can_target) {
            return Err(ProfileRefusal::Invalid);
        }
        self.entry_mut(actor)
            .ok_or(ProfileRefusal::Missing)?
            .state
            .rebind_native_profile(profile, &creature);
        Ok(true)
    }

    pub fn admit_native_creature(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        actor: ExactActorRef,
        creature: &crate::content::ProjectV2DefinitionRef,
        now: u64,
    ) -> Result<(), behaviour_profile::ProfileRefusal> {
        use crate::monster_summon::{CurrentSourceCreatureRole, current_source_creature_role};
        let role = current_source_creature_role(runtime, actor)
            .map_err(|_| behaviour_profile::ProfileRefusal::Invalid)?;
        if !matches!(
            role,
            CurrentSourceCreatureRole::Ordinary
                | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)
        ) {
            return Err(behaviour_profile::ProfileRefusal::Invalid);
        }
        let profile = project_native(runtime, native, creature)?;
        // Crystal 201189f src/lua/functions/core/game/game_functions.cpp:288-310
        // creates Soulpit variants without replacing the source type's passive targeting.
        // Only this current actor's owner-verified Instance tag admits the passive profile.
        if role.is_ordinary()
            && runtime.soulpit_variant(actor).is_none()
            && (!profile.targeting().hostile || !profile.targeting().can_target)
        {
            return Err(behaviour_profile::ProfileRefusal::Invalid);
        }
        if !runtime.matches_live_creature_identity(actor, creature.key.as_bytes()) {
            return Err(behaviour_profile::ProfileRefusal::Invalid);
        }
        let anchor = runtime
            .read_actor_position(actor)
            .map_err(|_| behaviour_profile::ProfileRefusal::Invalid)?
            .position();
        let mut state = CreatureAiState::new(actor, profile, anchor);
        state.spawn = runtime.creature_spawn_cell(actor);
        state.is_summon = !role.is_ordinary();
        state.creature_key = Some(creature.key.clone());
        state.creature_revision = Some(creature.revision.clone());
        self.admit_current_native_state(runtime, state, now)
    }

    fn admit_current_native_state(
        &mut self,
        runtime: &ChannelRuntimeV1,
        state: CreatureAiState,
        now: u64,
    ) -> Result<(), behaviour_profile::ProfileRefusal> {
        use behaviour_profile::ProfileRefusal;
        let actor = state.actor();
        let position = runtime
            .read_actor_position(actor)
            .map_err(|_| ProfileRefusal::Invalid)?;
        let has_condition = !runtime
            .actor_conditions(actor, None)
            .map_err(|_| ProfileRefusal::Invalid)?
            .instances()
            .is_empty();
        let mut has_player = false;
        for (player, at, session) in runtime
            .positioned_actor_census()
            .map_err(|_| ProfileRefusal::Invalid)?
        {
            let Some(session) = session else {
                continue;
            };
            if at.context() != position.context()
                || !targeting::sees_native(position.position(), at.position())
            {
                continue;
            }
            let invisible = runtime
                .actor_conditions(player, Some(session))
                .map_err(|_| ProfileRefusal::Invalid)?
                .invisible_at(now);
            if !invisible || state.profile.targeting().sense_invisible {
                has_player = true;
                break;
            }
        }
        let returning = state.spawn.is_some_and(|home| home != position.position());
        let active = has_player
            || has_condition
            || returning
            || (state.is_summon && state.profile.default_source_faction());
        self.admit(state, now);
        if !active && let Some(entry) = self.entry_mut(actor) {
            entry.due_us = None;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "native_profile_tests.rs"]
mod tests;

#[cfg(test)]
mod soulpit_preflight_tests {
    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};

    #[test]
    fn native_soulpit_preflight_qualifies_without_spawning_or_admitting_an_actor()
    -> Result<(), String> {
        fn debug(error: impl std::fmt::Debug) -> String {
            format!("{error:?}")
        }
        fn id(seed: u8) -> [u8; 16] {
            let mut id = [seed; 16];
            id[6] = 0x70;
            id[8] = 0x80;
            id
        }
        let world = WorldId::decode(&id(1)).map_err(debug)?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native")?;
        let runtime = |digest| {
            ChannelRuntimeV1::from_committed_assignment(
                world,
                ChannelId::decode(&id(2)).map_err(debug)?,
                NodeId::decode(&id(3)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                8,
                ChannelContentPin::from_activation(
                    world,
                    1,
                    digest,
                    [2; 32],
                    [3; 32],
                    [4; 32],
                    (100, 100, 7),
                ),
            )
            .map_err(debug)
        };
        let runtime = runtime(native.source_digest())?;
        let rat = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == "oteryn:creature.rat")
            .ok_or("native rat")?
            .profile
            .target
            .clone();
        assert!(runtime.positioned_actor_census().map_err(debug)?.is_empty());
        CreatureAiTable::qualify_soulpit_creature(&runtime, native, &rat).map_err(debug)?;
        assert!(runtime.positioned_actor_census().map_err(debug)?.is_empty());
        let mut missing = rat.clone();
        missing.revision = "unadmitted-revision".into();
        assert_eq!(
            CreatureAiTable::qualify_soulpit_creature(&runtime, native, &missing),
            Err(behaviour_profile::ProfileRefusal::Missing)
        );
        let mut mismatch = native.source_digest();
        mismatch[0] ^= 1;
        let other = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(debug)?,
            NodeId::decode(&id(3)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::from_activation(
                world,
                1,
                mismatch,
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .map_err(debug)?;
        assert_eq!(
            CreatureAiTable::qualify_soulpit_creature(&other, native, &rat),
            Err(behaviour_profile::ProfileRefusal::Invalid)
        );
        Ok(())
    }

    #[test]
    fn soulpit_passive_chocolate_blob_keeps_native_targeting_while_ordinary_admission_refuses()
    -> Result<(), String> {
        std::thread::Builder::new()
            .name("soulpit-passive-profile".into())
            .stack_size(64 << 20)
            .spawn(passive_chocolate_blob_scenario)
            .map_err(|error| format!("{error:?}"))?
            .join()
            .map_err(|_| "passive Soulpit profile scenario thread failed".to_string())?
    }

    fn passive_chocolate_blob_scenario() -> Result<(), String> {
        use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
        use oteryn_gameplay_domain::domain::soulpit::BossAbility;
        use oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveRequest;
        fn debug(error: impl std::fmt::Debug) -> String {
            format!("{error:?}")
        }
        fn id(seed: u8) -> [u8; 16] {
            let mut id = [seed; 16];
            id[6] = 0x70;
            id[8] = 0x80;
            id
        }
        let world = WorldId::decode(&id(11)).map_err(debug)?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native")?;
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == "oteryn:creature.chocolate_blob")
            .ok_or("native chocolate blob")?;
        let creature = &record.profile.target;
        let Data::Creature(profile) = &record.profile.data else {
            return Err("creature profile".into());
        };
        assert_eq!(
            profile
                .details
                .as_ref()
                .and_then(|details| details.soul_core_item.as_ref())
                .map(|core| core.key.as_str()),
            Some("oteryn:item.tibia.i49087")
        );
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            native.source_digest(),
            [2; 32],
            [3; 32],
            [4; 32],
            (32375, 31151, 8),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            RuntimeScopeRefV1::instance(world, id(12)).map_err(debug)?,
            NodeId::decode(&id(13)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin,
        )
        .map_err(debug)?;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        CreatureAiTable::qualify_soulpit_creature(&runtime, native, creature).map_err(debug)?;
        let at = MovementLocalPosition {
            x: 32376,
            y: 31144,
            floor: 8,
        };
        let ordinary = runtime
            .admit_pinned_monster_lab_creature(
                at,
                &creature.key,
                i64::try_from(profile.health.ok_or("native health")?).map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(ordinary, &creature.key)
            .map_err(debug)?;
        assert!(runtime.matches_live_creature_identity(ordinary, creature.key.as_bytes()));
        let mut ai = CreatureAiTable::default();
        assert_eq!(
            ai.admit_native_creature(&runtime, native, ordinary, creature, 0),
            Err(behaviour_profile::ProfileRefusal::Invalid)
        );
        let spawned = {
            let kernel = runtime.borrow_soulpit_kernel().map_err(debug)?;
            kernel
                .carrier
                .spawn_soulpit_wave(
                    kernel.continuity,
                    kernel.content,
                    native.policies(),
                    SoulpitWaveRequest {
                        handoff_id: id(14),
                        wave: 1,
                        species: creature.key.clone(),
                        boss_ability: BossAbility::Overpower,
                        now_micros: 0,
                        positions: std::array::from_fn(|index| {
                            Some(MovementLocalPosition {
                                x: 32363 + index as i32,
                                y: 31133,
                                floor: 8,
                            })
                        }),
                    },
                    |_| true,
                )
                .map_err(debug)?
        };
        let soulpit = spawned[0].ok_or("Soulpit chocolate blob")?.actor;
        ai.admit_native_creature(&runtime, native, soulpit, creature, 0)
            .map_err(debug)?;
        let state = ai.state(soulpit).ok_or("admitted Soulpit AI")?;
        assert!(!state.profile().targeting().hostile);
        assert!(state.profile().targeting().can_target);
        assert_eq!(state.target(), None);
        assert!(!targeting::is_eligible(
            state.profile().targeting(),
            at,
            &targeting::TargetCandidate {
                actor: ordinary,
                position: at,
                eligible: true,
                invisible: false,
                health: 100,
                damage: 1,
            }
        ));
        runtime.remove_test_actor(soulpit).map_err(debug)?;
        assert_eq!(
            ai.admit_native_creature(&runtime, native, soulpit, creature, 1),
            Err(behaviour_profile::ProfileRefusal::Invalid)
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "native_profile/native_activity_tests.rs"]
mod native_activity_tests;

#[cfg(test)]
mod source_overlay_schedule_tests {
    use super::super::profile_schedule::{AttackTarget, NativeAttackFacts, ProfileScheduleInput};
    use super::super::think_tests::{actor_n, revisions, root};
    use super::*;

    #[test]
    fn srx3_soul_war_source_vectors_preserve_shared_clock_phase_and_gap() -> Result<(), String> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let world = crate::content::native_gameplay_test_world_id()
                    .map_err(|e| format!("{e:?}"))?;
                let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                    world, &path,
                );
                for (name, index, maximum) in [
                    ("goshnar_s_cruelty", 5, 7000_u64),
                    ("goshnar_s_megalomania_green", 7, 35000),
                    ("goshnar_s_megalomania_purple", 7, 35000),
                    ("goshnar_s_megalomania_blue", 1, 30000),
                ] {
                    let record = native
                        .creature_profiles()
                        .records
                        .iter()
                        .find(|r| r.profile.target.key == format!("oteryn:creature.{name}"))
                        .ok_or_else(|| format!("actual source {name}"))?;
                    let source = native
                        .soul_war_source_attacks(&record.profile.target)
                        .ok_or_else(|| format!("source overlay {name}"))?;
                    let projected =
                        project_native_parts(record, source.behavior(), source.abilities())
                            .map_err(|e| format!("projection {name}: {e:?}"))?;
                    let behavior = projected.behavior();
                    assert_eq!(
                        behavior.attacks.iter().map(|e| e.interval_ms).max(),
                        Some(maximum)
                    );
                    let expected = behavior.attacks[index].ability.clone();
                    assert!(source.accepts_ability(&expected));
                    let actor = actor_n(1);
                    let mut state = ProfileScheduleState::new(actor);
                    let revisions = revisions();
                    let root = root();
                    let mut selected = 0;
                    for sequence in 1..=210_u64 {
                        let occurrence = ThinkOccurrence { actor, sequence };
                        // A large semantic-time gap must not invent skipped source thinks.
                        let now_us =
                            sequence * 1_000_000 + if sequence >= 8 { 90_000_000 } else { 0 };
                        let plan = state
                            .prepare_native_with_summons(
                                ProfileScheduleInput {
                                    occurrence,
                                    behavior,
                                    abilities: projected.abilities(),
                                    target: Some(AttackTarget {
                                        actor: actor_n(2),
                                        same_floor_distance: Some(1),
                                    }),
                                    revisions: &revisions,
                                    root: &root,
                                },
                                None,
                                NativeAttackFacts {
                                    now_us,
                                    fleeing: false,
                                    target_disappeared: false,
                                },
                            )
                            .map_err(|e| format!("schedule {name}:{sequence}: {e:?}"))?;
                        let phase = ((sequence - 1) * 1000) % maximum + 1000;
                        assert_eq!(plan.evaluated_attack_ticks_ms, phase, "{name}:{sequence}");
                        assert_eq!(
                            plan.attack_ticks_after_ms,
                            if phase == maximum { 0 } else { phase },
                            "{name}:{sequence}"
                        );
                        selected += plan
                            .proposals
                            .iter()
                            .filter(|p| p.ability == expected && p.entry_index == index)
                            .count();
                    }
                    assert!(selected > 0, "actual selected source slot {name}:{index}");
                }
                Ok(())
            })
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| "source overlay schedule thread".to_owned())?
    }
    #[test]
    fn srx3_omrafir_actual_native_projection_retains_both_omitted_source_slots()
    -> Result<(), String> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = crate::content::native_gameplay_test_world_id()
                    .map_err(|error| format!("{error:?}"))?;
                let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                    world, &manifest,
                );
                let record = native
                    .creature_profiles()
                    .records
                    .iter()
                    .find(|record| record.profile.target.key == "oteryn:creature.omrafir")
                    .ok_or("actual Omrafir")?;
                let uuid = |tag: u8| [1, 144, 0, 0, 0, tag, 112, 0, 128, 0, 0, 0, 0, 0, 0, tag];
                let runtime = ChannelRuntimeV1::from_committed_assignment(
                    world,
                    crate::foundation::ChannelId::decode(&uuid(2)).map_err(|e| format!("{e:?}"))?,
                    crate::foundation::NodeId::decode(&uuid(3)).map_err(|e| format!("{e:?}"))?,
                    1,
                    1,
                    1,
                    "runtime-scope-assignment:1",
                    1,
                    crate::content::channel_content_pin::ChannelContentPin::from_activation(
                        world,
                        1,
                        native.source_digest(),
                        [2; 32],
                        [3; 32],
                        [4; 32],
                        (0, 0, 7),
                    ),
                )
                .map_err(|error| format!("{error:?}"))?;
                // This is only the actual production profile projection. No actor,
                // map mutation or Instance admission is granted by this fixture.
                let profile = project_native(&runtime, &native, &record.profile.target)
                    .map_err(|error| format!("{error:?}"))?;
                assert_eq!(
                    profile.behavior().attacks[2].ability.key,
                    "oteryn:ability.spell.omrafir_beam"
                );
                assert_eq!(
                    profile.behavior().defenses[2].ability.key,
                    "oteryn:ability.spell.omrafir_healing_2"
                );
                assert_eq!(
                    (
                        profile.behavior().attacks.len(),
                        profile.behavior().defenses.len()
                    ),
                    (8, 3)
                );
                Ok(())
            })
            .map_err(|error| error.to_string())?
            .join()
            .map_err(|_| "Omrafir projection test thread".to_owned())?
    }
}
