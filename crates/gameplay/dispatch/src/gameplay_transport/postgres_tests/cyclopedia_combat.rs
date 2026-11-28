//! P12c: real attack refresh -> fenced snapshot -> command 388 -> durable read-back.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::character_authority::CharacterAuthorityError;
use crate::durability::character_build::DurableBuildState;
use crate::durability::cyclopedia_character::combat::CombatSnapshot;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, ConnectionGeneration};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::combat::{CombatStat as Stat, CombatStats};
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, CharacterTab,
};
use sqlx::Row;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ElementalMeleeCase {
    BlockBudget,
    ImmuneTraining,
    SecondaryImmuneTraining,
    DamageHealing,
    WandDamageHealing,
    AlphaProficiency,
    SplitCritical,
    ProficiencyAttackPoints,
    PlainProficiencyAttackPoints,
    ConvertedCritical,
    AlphaCutoff,
    AlphaPrey,
    ResistedSecondaryTraining,
    SplitLethalAttribution,
}

#[test]
fn p12c_cyclopedia_combat_postgres_dispatch_snapshot_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, None, false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "P12c combat scenario panicked")??;
    Ok(())
}

#[test]
fn self_defence_tab_reports_weapon_base_when_extra_defence_is_unauthored() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, None, false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Hand defence scenario panicked")??;
    Ok(())
}

#[test]
fn self_defence_tab_retries_busy_combat_snapshot_holder() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario(true, None, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Busy Defence scenario panicked")??;
    Ok(())
}

#[test]
fn crystal_equipment_zero_primary_spends_creature_block_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::BlockBudget), false))
}

#[test]
fn crystal_equipment_immune_melee_awards_no_training_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::ImmuneTraining), false))
}

#[test]
fn crystal_equipment_secondary_immunity_preserves_damage_but_awards_no_tries_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::SecondaryImmuneTraining), false))
}

#[test]
fn crystal_equipment_elemental_melee_heals_mounted_thorn_knight_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::DamageHealing), false))
}

#[test]
fn crystal_equipment_wand_of_inferno_heals_lava_lurker_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::WandDamageHealing), false))
}

#[test]
fn crystal_equipment_melee_alpha_proficiency_follows_defence_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::AlphaProficiency), false))
}

#[test]
fn crystal_equipment_melee_critical_follows_intrinsic_split_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::SplitCritical), false))
}

#[test]
fn crystal_equipment_amber_slayer_proficiency_adds_physical_attack_points_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::ProficiencyAttackPoints), false))
}

#[test]
fn crystal_equipment_melee_converts_after_critical_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::ConvertedCritical), false))
}

#[test]
fn crystal_equipment_melee_plain_proficiency_adds_attack_points_before_roll_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::PlainProficiencyAttackPoints), false))
}

#[test]
fn crystal_equipment_alpha_proficiency_truncates_health_cutoff_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::AlphaCutoff), false))
}

#[test]
fn crystal_equipment_alpha_proficiency_follows_prey_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::AlphaPrey), false))
}

#[test]
fn crystal_equipment_resisted_secondary_refreshes_training_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::ResistedSecondaryTraining), false))
}

#[test]
fn crystal_equipment_split_lethal_reports_primary_first_postgres() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(false, Some(ElementalMeleeCase::SplitLethalAttribution), false))
}

fn scenario(hand_defence_only: bool, elemental_melee: Option<ElementalMeleeCase>, busy_holder: bool) -> TestResult {
    use super::cyclopedia_fixtures::support::exchange;
    use ElementalMeleeCase as Case;
    let alpha = matches!(elemental_melee, Some(Case::AlphaProficiency | Case::AlphaCutoff | Case::AlphaPrey));
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(
            admin,
            if hand_defence_only {
                "panel_hand_defence"
            } else {
                "p12c_combat"
            },
            true,
            false,
        )
        .await?;
        eprintln!("POSTGRES_STARTED P12C cyclopedia_combat_snapshot");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::equipment_mods_fixtures::activate(deps.admitted.world_id)?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        deps.room = pin.qualified_room().clone();
        deps.combat_room = Some(deps.room.clone());
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            deps.room.map_revision_digest(),
            (2, 11, -7),
        );
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.activation_sequence().to_string())
            .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
            .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let race = match elemental_melee {
            Some(ElementalMeleeCase::ImmuneTraining) => "oteryn:creature.percht_minion",
            Some(ElementalMeleeCase::SecondaryImmuneTraining | ElementalMeleeCase::SplitCritical
                | ElementalMeleeCase::AlphaProficiency) => "oteryn:creature.elf_overseer",
            Some(Case::AlphaCutoff | Case::AlphaPrey | Case::PlainProficiencyAttackPoints | Case::ConvertedCritical) => "oteryn:creature.lizard_chosen",
            Some(Case::ResistedSecondaryTraining) => "oteryn:creature.mounted_thorn_knight",
            Some(Case::SplitLethalAttribution) => "oteryn:creature.rat",
            Some(ElementalMeleeCase::DamageHealing) => "oteryn:creature.mounted_thorn_knight",
            Some(ElementalMeleeCase::WandDamageHealing) => "oteryn:creature.lava_lurker",
            Some(ElementalMeleeCase::ProficiencyAttackPoints) => "oteryn:creature.magma_crystal",
            _ => "oteryn:creature.rat",
        };
        let policy = runtime.companion_policy(race).map_err(debug)?;
        if elemental_melee == Some(ElementalMeleeCase::DamageHealing) {
            assert_eq!(policy.damage_healing("death", 1).map_err(debug)?, 1);
        }
        if elemental_melee == Some(ElementalMeleeCase::WandDamageHealing) {
            assert!(policy.damage_healing("fire", 100).map_err(debug)? > 0);
        }
        if elemental_melee == Some(ElementalMeleeCase::ImmuneTraining) {
            assert!(policy.damage_immunities.iter().any(|key| key == "physical"));
            assert!(policy.damage_immunities.iter().any(|key| key == "death"));
        }
        let maximum_health = u64::try_from(
            runtime
                .companion_policy(race)
                .map_err(debug)?
                .maximum_health,
        )?;
        runtime
            .realize_activation_spawns(vec![(
                vec![(1, 11, -7)],
                60_000,
                5_000,
                race.into(),
                maximum_health,
            )])
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.admitted.runtime_actor = Some(deps.actor);
        seed(&h, hand_defence_only, elemental_melee).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        if alpha || matches!(elemental_melee, Some(Case::ProficiencyAttackPoints | Case::PlainProficiencyAttackPoints)) {
            owner.proficiency_catalogue = Some(
                crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue::embedded()
                    .ok_or("proficiency catalogue")?,
            );
        }
        deps.initialize(&owner).await;
        let effects = owner
            .read_admission_effects(deps.admitted.game_session_id)
            .await
            .map_err(|_| "read effects")?
            .ok_or("effects")?;
        let armor = effects.1;
        {
            let runtime = deps.runtime.lock().await;
            if hand_defence_only {
                let build = DurableBuildState::new("monk", (0, 0), [(10, 0); 7])?;
                assert!(owner.states.spell_states.lock().await.load_owned_build(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    &build,
                ));
            }
            owner
                .install_admission_suppressions(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    Some(effects),
                )
                .await
                .ok_or("install equipment")?;
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let tab = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, tab, &[8, 0, 0, 1])
                .await?
                .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            row_count(&h).await?,
            0,
            "unselected capability cannot populate statistics"
        );
        let offers = [
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 17,
                requires: &[6],
            },
            OfferedCapability {
                id: CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                requires: &[],
            },
        ];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[6, 17, CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("capabilities")?;
        if hand_defence_only {
            if busy_holder {
                // Hold only the durability pass. Drive the named tab owner to its
                // first refusal before freeing it, with no wall-clock sleeps.
                use std::future::{Future, poll_fn};
                use std::task::Poll;
                let held = h.root.try_issue_semantic_pass()?;
                let mut request = std::pin::pin!(
                    owner.cyclopedia_character_wire(admitted, 2, tab, &[9, 0, 0, 20])
                );
                assert!(
                    poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx))).await.is_pending(),
                    "the tab must wait for the occupied holder during combat refresh"
                );
                drop(held);
                let response = request.await.map_err(|()| "Defence read after holder returned")?;
                let view = CombatStats::decode(&response).map_err(debug)?;
                assert_eq!(view.get(Stat::DefenceEquipment), Some(22));
            }
            let (status, bytes) = exchange(&owner, &mut admitted, 2, tab, &[9, 0, 0, 20]).await?;
            assert_eq!(status, CommandStatus::Accepted);
            let defence = CombatStats::decode(&bytes).map_err(debug)?;
            assert_eq!(defence.get(Stat::DefenceEquipment), Some(22));
            assert!(
                defence.get(Stat::Mitigation).is_some_and(|value| value > 0),
                "Twin Hooks must contribute to the Cyclopedia Mitigation stat"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let target = {
            let runtime = deps.runtime.lock().await;
            let visible = runtime.visible_entities();
            let row = visible.creatures.first().ok_or("rat")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: row.actor.placement_identity(),
                generation: row.generation,
            }
        };
        let intent = oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&target))
            .map_err(debug)?;
        let (status, result) = exchange(&owner, &mut admitted, 2, 11, &intent).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            result,
            oteryn_protocol_oteryn::attack::encode_attack_intent_result(
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            )
        );
        owner
            .refresh_attack_weapon(deps.actor, admitted.game_session_id)
            .await;
        assert_eq!(
            row_count(&h).await?,
            1,
            "the production weapon refresh persists statistics before the first tab query"
        );
        if let Some(case) = elemental_melee {
            let training_skill = match case {
                ElementalMeleeCase::ImmuneTraining => Some(oteryn_gameplay_progression::hit_training::SkillFamily::Sword),
                ElementalMeleeCase::SecondaryImmuneTraining => Some(oteryn_gameplay_progression::hit_training::SkillFamily::Axe),
                Case::ResistedSecondaryTraining => Some(oteryn_gameplay_progression::hit_training::SkillFamily::Club),
                _ => None,
            };
            use oteryn_gameplay_combat_rules::attack::AttackConstants;
            use oteryn_gameplay_progression::hit_training::HitBlock;
            let formula = native.training_formula().ok_or("training formula")?;
            let now = owner.owner_now();
            let starting_health = match case {
                ElementalMeleeCase::DamageHealing | ElementalMeleeCase::WandDamageHealing => i64::try_from(maximum_health / 2)?,
                ElementalMeleeCase::SecondaryImmuneTraining | ElementalMeleeCase::ProficiencyAttackPoints
                | ElementalMeleeCase::SplitCritical | ElementalMeleeCase::AlphaProficiency => {
                    i64::try_from(maximum_health)?
                }
                Case::AlphaCutoff => i64::try_from(maximum_health * 95 / 100)?,
                Case::SplitLethalAttribution => 10,
                _ => i64::try_from(maximum_health)?,
            };
            if alpha || matches!(case, Case::ProficiencyAttackPoints | Case::PlainProficiencyAttackPoints) {
                use crate::domain::CharacterRevision;
                use crate::durability::character_proficiency::{
                    DurableProficiencyState, StoredProficiencyTrack,
                };
                let (key, definition, level, perk) = match case {
                    Case::AlphaProficiency | Case::AlphaCutoff | Case::AlphaPrey => ("oteryn:item.tibia.i52338", "oteryn:proficiency.tibia.p457", 3, 0),
                    Case::PlainProficiencyAttackPoints => ("oteryn:item.tibia.i52338", "oteryn:proficiency.tibia.p457", 4, 1),
                    _ => ("oteryn:item.tibia.i47368", "oteryn:proficiency.tibia.p42", 5, 0),
                };
                let catalogue = owner.proficiency_catalogue.as_ref().ok_or("proficiency catalogue")?;
                let shape = catalogue.empty_track(key).ok_or("Amber Slayer proficiency")?.shape.track_shape();
                let mut selections = vec![None; usize::from(shape.perk_levels())];
                selections[level] = Some(perk);
                let stored = StoredProficiencyTrack {
                    state: DurableProficiencyState::new(
                        key, definition, "definition-r1",
                        u64::from(shape.final_progress()), selections,
                    )?,
                    committed_character_revision: CharacterRevision::new(1).map_err(debug)?,
                    last_proficiency_occurrence_id: [0; 16],
                    modifications: [None, None],
                };
                let track = catalogue.live_track(&stored).ok_or("selected attack proficiency")?;
                if alpha {
                    assert_eq!(track.effects.alpha_numerator * 10, track.effects.denominator);
                } else {
                    assert_eq!(track.effects.flat_attack, 1);
                }
                owner.states.proficiencies.lock().map_err(|_| "proficiencies")?
                    .entry(admitted.game_session_id).or_default().install_track(key, track);
                owner.states.spell_states.lock().await.analyser.snapshot(
                    deps.actor, admitted.game_session_id, 1, None,
                ).ok_or("analyser subscription")?;
            }
            if case == Case::SplitLethalAttribution {
                owner.states.spell_states.lock().await.analyser.snapshot(
                    deps.actor, admitted.game_session_id, 1, None,
                ).ok_or("analyser subscription")?;
            }
            let mut expected_damage = None;
            let target_actor;
            {
                let mut runtime = deps.runtime.lock().await;
                let creature = runtime.visible_entities().creatures.first().ok_or("target")?.actor;
                target_actor = creature;
                runtime.set_test_creature_health(creature, starting_health).map_err(debug)?;
                let mut states = owner.states.spell_states.lock().await;
                if case == Case::AlphaPrey {
                    use oteryn_gameplay_progression::prey::difficulty::PreyDefenseMode;
                    use oteryn_protocol_oteryn::prey as wire;
                    states.load_prey(
                        &runtime, deps.actor, admitted.game_session_id,
                        wire::Reply {
                            status: 0, wildcard_balance: 0,
                            slots: vec![wire::Slot {
                                slot: 1, state: 2, creatures: vec![race.into()], selected: race.into(),
                                bonus: 1, grade: 10, percent: 25, hunting_seconds: 7200, option: 0,
                                free_reroll_at: 0, chosen_tier: 1, allowed_tier: 5, selection_epoch: 1,
                            }],
                            list_size: 9, expansion_owned: false, offered_today: Vec::new(), offered_next: String::new(),
                        },
                        [true; 3], PreyDefenseMode::DealtOrAttacked,
                    );
                }
                let state = states
                    .get_mut(&runtime, deps.actor, admitted.game_session_id)
                    .ok_or("player state")?;
                state.facts.level = match case {
                    Case::SecondaryImmuneTraining | Case::ProficiencyAttackPoints | Case::SplitLethalAttribution => 800,
                    Case::ResistedSecondaryTraining => 1,
                    Case::AlphaPrey | Case::PlainProficiencyAttackPoints => 20,
                    _ => 330,
                };
                state.facts.vocation = if case == ElementalMeleeCase::WandDamageHealing {
                    state.restore_saved_vitals(state.vitals().health, state.facts.max_mana);
                    crate::spell::Vocation::Sorcerer
                } else {
                    crate::spell::Vocation::Knight
                };
                if case == Case::PlainProficiencyAttackPoints {
                    state.load_owned_build(&crate::durability::character_build::DurableBuildState::new(
                        "knight", (u16::try_from(state.facts.magic_level)?, 0), [(100, 0); 7],
                    ).map_err(debug)?).map_err(debug)?;
                }
                if alpha || matches!(case, Case::SplitCritical | Case::ConvertedCritical | Case::PlainProficiencyAttackPoints) {
                    use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
                    let mut offence = if matches!(case, Case::SplitCritical | Case::ConvertedCritical) {
                        EquippedOffence::from_sums(10_000, 0, 0, 0)
                    } else {
                        EquippedOffence::default()
                    };
                    if case == Case::ConvertedCritical {
                        offence.elemental_conversion = Some((crate::content::ProjectV2RangedElement::Fire, 50));
                    }
                    state.install_equipped_offence(offence);
                    let (level, expected) = expected_creature_damage(
                        &runtime, native, deps.actor, admitted.game_session_id, creature, state, now.get(), case,
                    )?;
                    state.facts.level = level;
                    expected_damage = Some(expected);
                }
                if let Some(skill) = training_skill {
                    let build = crate::durability::character_build::DurableBuildState::new(
                        "knight", (u16::try_from(state.facts.magic_level)?, 0),
                        [(if case == ElementalMeleeCase::SecondaryImmuneTraining { 100 } else { 50 }, 0); 7],
                    ).map_err(debug)?;
                    state.enable_owned_training(&build, formula).map_err(debug)?;
                    state.train_skill(skill, now.get(), formula, |hits| {
                        if case != Case::ResistedSecondaryTraining {
                            hits.melee_hit(HitBlock::Unblocked);
                        }
                        if case == ElementalMeleeCase::SecondaryImmuneTraining {
                            while hits.melee_hit(HitBlock::Armor) > 0 {}
                        }
                        0
                    }).map_err(debug)?;
                    assert!(!state.has_training_to_flush());
                }
            }
            owner.drain_auto_attacks().await;
            let runtime = deps.runtime.lock().await;
            let creature = target_actor;
            let health = runtime.companion_snapshot_including_dead(creature).map_err(debug)?.health;
            if let Some(expected) = expected_damage {
                assert_eq!(starting_health - health, expected, "rolled components settle through live creature defence");
            }
            if case == ElementalMeleeCase::SecondaryImmuneTraining {
                assert!(health < starting_health, "the non-immune physical component lands");
            }
            if case == ElementalMeleeCase::DamageHealing {
                assert!(health > starting_health, "the incoming death component heals the creature");
            }
            if case == ElementalMeleeCase::WandDamageHealing {
                let states = owner.states.spell_states.lock().await;
                let state = states.get(&runtime, deps.actor, admitted.game_session_id).ok_or("player state")?;
                assert!(state.vitals().mana < state.facts.max_mana, "the wand shot committed its mana cost");
                assert!(health > starting_health, "the fire shot heals the Lava Lurker");
            } else {
                owner.states.attack.lock().await.test_creature_blocks(creature).ok_or("melee swing reached block settlement")?;
            }
            if case == ElementalMeleeCase::ProficiencyAttackPoints {
                use oteryn_protocol_oteryn::analyser::{AnalyserFact, AnalyserImpact, decode_analyser_facts};
                use oteryn_protocol_oteryn::damage_element::DamageElement;
                let states = owner.states.spell_states.lock().await;
                let batch = states.analyser.next(deps.actor, admitted.game_session_id, 1)
                    .ok_or("committed damage facts")?;
                assert!(
                    decode_analyser_facts(&batch).map_err(debug)?.iter().any(|fact| matches!(
                        fact,
                        AnalyserFact::Impact(AnalyserImpact::Damage { element: DamageElement::Physical, value })
                            if value.get() >= 3
                    )),
                    "the physical component starts at the level / 5 minimum",
                );
            }
            if case == Case::SplitLethalAttribution {
                use oteryn_protocol_oteryn::analyser::{AnalyserFact, AnalyserImpact, decode_analyser_facts};
                use oteryn_protocol_oteryn::damage_element::DamageElement;
                let states = owner.states.spell_states.lock().await;
                let batch = states.analyser.next(deps.actor, admitted.game_session_id, 1)
                    .ok_or("committed damage facts")?;
                assert_eq!(health, 0);
                assert!(decode_analyser_facts(&batch).map_err(debug)?.iter().any(|fact| matches!(
                    fact, AnalyserFact::Impact(AnalyserImpact::Damage { element: DamageElement::Physical, value })
                        if value.get() == 10
                )), "primary damage consumes the creature's last 10 HP");
            }
            if training_skill.is_some() {
                let states = owner.states.spell_states.lock().await;
                let state = states.get(&runtime, deps.actor, admitted.game_session_id).ok_or("player state")?;
                if case == Case::ResistedSecondaryTraining {
                    assert_eq!(health, starting_health, "armour and resistance consume both components");
                    assert!(state.has_training_to_flush(), "the secondary callback awards a melee try before resistance");
                } else {
                    assert!(!state.has_training_to_flush(), "a final immunity block awards no melee tries");
                }
                if case == ElementalMeleeCase::SecondaryImmuneTraining {
                    let mut state = state.clone();
                    state.train_skill(oteryn_gameplay_progression::hit_training::SkillFamily::Axe, now.get(), formula, |hits| {
                        assert_eq!(hits.melee_hit(HitBlock::Armor), 1, "the landed primary refreshes the blood streak");
                        0
                    }).map_err(debug)?;
                }
            } else if case == ElementalMeleeCase::BlockBudget {
                let constants = AttackConstants::checked_in().ok_or("attack constants")?;
                let budget = owner.states.attack.lock().await.test_creature_blocks(creature).ok_or("melee swing reached block settlement")?;
                assert_eq!(budget, constants.block.max_blocks - 1);
            }
            drop(runtime);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        // The following connection exercises only the tab capability, so an independent
        // periodic attack observation cannot obscure the command-replay write count.
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("tab capability")?;
        let (status, offence_bytes) =
            exchange(&owner, &mut admitted, 3, tab, &[8, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let offence = CombatStats::decode(&offence_bytes).map_err(debug)?;
        assert_eq!(offence.get(Stat::CriticalChance), Some(1_050_000));
        assert_eq!(offence.get(Stat::LifeLeech), Some(500_000));
        assert_eq!(offence.get(Stat::Cleave), Some(1_000_000));
        assert_eq!(offence.get(Stat::PerfectShot1), Some(42));
        assert_eq!(offence.get(Stat::OnslaughtEquipment), Some(90_500));
        assert_eq!(offence.get(Stat::AttackTotal), None);
        let version = sample_revision(&h).await?;
        let (status, replay) = exchange(&owner, &mut admitted, 3, tab, &[8, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(offence_bytes, replay);
        assert_eq!(
            sample_revision(&h).await?,
            version,
            "wire replay cannot sample or write twice"
        );
        let (status, bytes) = exchange(&owner, &mut admitted, 4, tab, &[9, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let defence = CombatStats::decode(&bytes).map_err(debug)?;
        assert_eq!(defence.get(Stat::Armor), Some(i64::from(armor)));
        assert_eq!(defence.get(Stat::PhysicalReflection), Some(99));
        assert_eq!(defence.get(Stat::Dodge), None);
        assert_eq!(defence.get(Stat::Mitigation), None);
        let (status, bytes) = exchange(&owner, &mut admitted, 5, tab, &[10, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let misc = CombatStats::decode(&bytes).map_err(debug)?;
        assert_eq!(misc.get(Stat::Amplification), Some(574_000));
        assert_eq!(misc.get(Stat::Blessings), Some(2));
        assert_eq!(misc.get(Stat::MaximumBlessings), Some(7));
        assert!(
            misc.get(Stat::WealthDuplexSeconds)
                .is_some_and(|value| value > 0 && value <= 1800)
        );
        assert_eq!(misc.get(Stat::Momentum), None);
        assert_eq!(misc.get(Stat::OtherConcoctions), None);
        let version = sample_revision(&h).await?;
        let mut foreign = vec![8, 0, 0, 1];
        foreign.extend_from_slice(&id(60));
        for (command, bytes) in [
            (6, foreign.as_slice()),
            (7, &[11, 0, 0, 1][..]),
            (8, &[8, 0, 0][..]),
        ] {
            assert_eq!(
                exchange(&owner, &mut admitted, command, tab, bytes)
                    .await?
                    .0,
                CommandStatus::Rejected
            );
        }
        let mut stale_admitted = admitted;
        let mut stale_fence = fence;
        stale_fence.character_lease_generation -= 1;
        stale_admitted.item_fence = Some(stale_fence);
        assert!(
            owner
                .cyclopedia_character_wire(stale_admitted, 0, tab, &[8, 0, 0, 1])
                .await
                .is_err()
        );
        assert_eq!(
            sample_revision(&h).await?,
            version,
            "malformed, unconsented and stale requests write nothing"
        );

        // An owner restart reads the committed bytes, including their observation time and
        // revisions. Exact retries use the snapshot receipt rather than re-counting misc facts.
        let row = sqlx::query("SELECT observed_owner_micros::text,offence,defence,misc FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let equipment = crate::gameplay_transport::equipment_move::read_current_equipment(
            &owner,
            admitted.game_session_id,
        )
        .await
        .ok_or("equipment")?;
        let snapshot = CombatSnapshot {
            observed_owner_micros: row
                .try_get::<String, _>("observed_owner_micros")?
                .parse::<u64>()?
                + 1,
            equipment,
            offence: CombatStats::decode(&row.try_get::<Vec<u8>, _>("offence")?).map_err(debug)?,
            defence: CombatStats::decode(&row.try_get::<Vec<u8>, _>("defence")?).map_err(debug)?,
            misc: CombatStats::decode(&row.try_get::<Vec<u8>, _>("misc")?).map_err(debug)?,
        };
        let receipt = h
            .root
            .record_cyclopedia_combat(&authority, &h.node, fence, snapshot.clone())
            .await
            .map_err(debug)?;
        assert!(!receipt.replayed);
        let replay = h
            .root
            .record_cyclopedia_combat(&authority, &h.node, fence, snapshot.clone())
            .await
            .map_err(debug)?;
        assert!(replay.replayed);
        assert_eq!(replay.sample_revision, receipt.sample_revision);
        let mut changed = snapshot.clone();
        changed
            .offence
            .set(Stat::CriticalChance, 1)
            .map_err(debug)?;
        assert!(matches!(
            h.root
                .record_cyclopedia_combat(&authority, &h.node, fence, changed)
                .await,
            Err(CharacterAuthorityError::Conflict)
        ));
        let mut older = snapshot.clone();
        older.observed_owner_micros -= 1;
        assert!(matches!(
            h.root
                .record_cyclopedia_combat(&authority, &h.node, fence, older)
                .await,
            Err(CharacterAuthorityError::Conflict)
        ));
        let mut changed_equipment = snapshot.clone();
        changed_equipment.equipment.revision += 1;
        changed_equipment.offence.equipment_revision += 1;
        changed_equipment.defence.equipment_revision += 1;
        changed_equipment.misc.equipment_revision += 1;
        assert!(matches!(
            h.root
                .record_cyclopedia_combat(&authority, &h.node, fence, changed_equipment)
                .await,
            Err(CharacterAuthorityError::Conflict)
        ));
        let mut stale_connection = fence;
        stale_connection.connection_generation = ConnectionGeneration::new(999).map_err(debug)?;
        let mut foreign_fence = fence;
        foreign_fence.character_id =
            crate::domain::CharacterId::from_bytes(id(60)).map_err(debug)?;
        for refused in [stale_fence, stale_connection, foreign_fence] {
            assert!(
                h.root
                    .record_cyclopedia_combat(&authority, &h.node, refused, snapshot.clone())
                    .await
                    .is_err()
            );
        }
        assert_eq!(
            sample_revision(&h).await?,
            i64::try_from(receipt.sample_revision)?
        );
        assert_eq!(row_count(&h).await?, 1);
        let read = h
            .root
            .read_cyclopedia_character_tab(
                &authority,
                &h.node,
                fence,
                fence.character_id,
                CharacterTab::Offence,
                oteryn_protocol_oteryn::character_info::PageRequest::new(0, 1).ok_or("page")?,
                i64::try_from(snapshot.offence.observed_at_unix_ms)?,
            )
            .await
            .map_err(debug)?
            .encode()
            .map_err(debug)?;
        assert_eq!(CombatStats::decode(&read).map_err(debug)?, snapshot.offence);

        // A consented foreign target skips the self-refresh writer, so the read itself must
        // prove the viewer's current lease/connection. Reuse source-produced snapshot bytes
        // for this privileged peer fixture; the request never supplies a combat observation.
        super::fixture::pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_cyclopedia_combat(character_id,game_session_id,character_lease_generation,connection_generation,observed_owner_micros,observed_at_unix_ms,character_revision,equipment_revision,content_digest,input_sha256,sample_revision,offence,defence,misc) SELECT encode($2,'hex')::uuid,game_session_id,character_lease_generation,connection_generation,observed_owner_micros,observed_at_unix_ms,character_revision,equipment_revision,content_digest,input_sha256,sample_revision,offence,defence,misc FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) SELECT account_id,world_id,character_id,encode($2,'hex')::uuid,2,now()+interval '1 day' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (status, foreign_read) = exchange(&owner, &mut admitted, 9, tab, &foreign).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            CombatStats::decode(&foreign_read).map_err(debug)?,
            snapshot.offence
        );
        for refused in [stale_fence, stale_connection] {
            let mut stale_viewer = admitted;
            stale_viewer.item_fence = Some(refused);
            // Keep the admitted envelope internally consistent: the durable current fence,
            // rather than a local field mismatch, must reject this foreign read.
            stale_viewer.continuity.connection_generation = refused.connection_generation.get();
            assert!(
                owner
                    .cyclopedia_character_wire(stale_viewer, 0, tab, &foreign)
                    .await
                    .is_err()
            );
        }
        assert_eq!(
            sample_revision(&h).await?,
            i64::try_from(receipt.sample_revision)?
        );
        let peer_revision: i64 = sqlx::query_scalar("SELECT sample_revision FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(60).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(peer_revision, i64::try_from(receipt.sample_revision)?);
        assert_eq!(
            row_count(&h).await?,
            2,
            "foreign inspection never writes a sample"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        eprintln!("P12C POSTGRES_CASE_COMPLETED cyclopedia_combat_snapshot");
        h.cleanup().await
    })
}

#[allow(clippy::too_many_arguments, reason = "predict the held lineage against the same live fixture")]
fn expected_creature_damage(
    runtime: &ChannelRuntimeV1,
    native: &crate::content::native_gameplay::NativeGameplayState,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    target: crate::foundation::ExactActorRef,
    state: &crate::spell::cast::PlayerSpellState,
    now: u64,
    case: ElementalMeleeCase,
) -> Result<(u32, i64), Box<dyn std::error::Error>> {
    use oteryn_gameplay_combat_rules::{
        attack::{AttackConstants, FightMode},
        resolution::{
            self,
            equipped_offence::{CRITICAL_ROLL_MAX, CRITICAL_ROLL_MIN},
        },
    };
    let constants = AttackConstants::checked_in().ok_or("attack constants")?;
    let split_critical = case == ElementalMeleeCase::SplitCritical;
    use ElementalMeleeCase as Case;
    let key = if split_critical {
        "oteryn:item.tibia.i49523"
    } else {
        "oteryn:item.tibia.i52338"
    };
    let weapon = resolution::swing_weapon(native, |slot| {
        (slot == 5).then_some((key, "definition-r1"))
    })
    .ok_or("native weapon")?;
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
        runtime.content_pin().server_artifact_digest(),
    );
    let skill = state
        .live_skill(if split_critical { 3 } else { 2 }, now)
        .unwrap_or(constants.starting_skill);
    let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
    let defense = runtime.effective_creature_defense(&snapshot).map_err(debug)?;
    let policy = snapshot.state.policy;
    let armor = policy.armor.ok_or("target armor")?;
    let split = |amount| {
        weapon.split_damage(amount).map(|parts| parts.0).ok_or("intrinsic split")
    };
    let attack = weapon.attack(constants.fist.attack);
    // Choose a qualified skill/level fixture whose deterministic roll separates the two
    // orderings, since one particular roll may happen to round to the same result.
    let lineage = (*session.as_bytes(), 2);
    let roll = |tag, key, low, high| {
        resolution::swing_draw(
            &root, actor.placement_identity(), lineage, 0, tag, key, low, high,
        ).ok_or("held attack draw")
    };
    let critical_roll = u64::try_from(roll(
        b"critical-hit", "attack.critical-hit", CRITICAL_ROLL_MIN as i64, CRITICAL_ROLL_MAX as i64,
    )?)?;
    let offence = state.equipped_offence();
    let critical = |damage| -> Result<i64, std::num::TryFromIntError> {
        Ok(i64::from(offence.critical(u32::try_from(damage)?, critical_roll)))
    };
    let settle = |physical| {
        resolution::creature_mitigated(
            &root, constants, actor.placement_identity(), lineage, 0, physical, true, defense, armor,
        )
        .and_then(|blocked| resolution::creature_physical_response(&policy, blocked))
        .ok_or("mitigated swing")
    };
    for level in state.facts.level..state.facts.level + 300 {
        let draw = |attack_value| -> Result<i64, Box<dyn std::error::Error>> {
            let (low, high) = crate::combat_attack::player_melee_formula(constants)
                .bounds(&crate::spell::formula::FormulaInputs {
                    level,
                    attack_skill: skill,
                    attack_value,
                    attack_factor: constants.fight_mode(FightMode::Balanced).attack_factor,
                    magic_level: state.facts.magic_level,
                    base_power: None,
                    shielding_skill: 0,
                    shield_defense: None,
                }).map_err(debug)?;
            Ok(roll(b"damage-draw", "attack.damage", low, high)?)
        };
        let raw = draw(attack)?;
        let (expected, legacy) = match case {
            Case::SplitCritical => (
                settle(critical(split(raw)?)?)?,
                settle(split(critical(raw)?)?)?,
            ),
            Case::ConvertedCritical => {
                let total = |physical, elemental| -> Result<i64, Box<dyn std::error::Error>> {
                    Ok(settle(physical)? + resolution::creature_damage_response(&policy, elemental, "fire").ok_or("fire")?)
                };
                let converted = |physical| offence.convert_physical(physical, 0, crate::content::ProjectV2RangedElement::Physical).ok_or("conversion");
                let (primary, secondary, _) = converted(critical(raw)?)?;
                let (old_primary, old_secondary, _) = converted(raw)?;
                (total(primary, secondary)?, total(critical(old_primary)?, critical(old_secondary)?)?)
            }
            Case::PlainProficiencyAttackPoints => (settle(critical(draw(attack + 1)?)?)?, settle(critical(raw + 1)?)?),
            Case::AlphaCutoff => {
                let landed = settle(critical(raw)?)?;
                (landed + landed / 10, landed)
            }
            Case::AlphaPrey => {
                let landed = settle(critical(raw)?)?;
                // PREY-0 rounds the 25% contribution up before proficiency applies.
                let boost = |damage| damage + (damage + 3) / 4;
                let boosted = boost(landed);
                if landed % 4 == 0 {
                    continue;
                }
                (boosted + boosted / 10, boost(landed + landed / 10))
            }
            _ => {
                let landed = settle(critical(raw)?)?;
                (landed + landed / 10, settle(critical(raw + raw / 10)?)?)
            }
        };
        if expected != legacy {
            return Ok((level, expected));
        }
    }
    Err("no held swing distinguishes the reviewed combat orderings".into())
}

async fn seed(h: &Harness, hand_defence_only: bool, elemental: Option<ElementalMeleeCase>) -> TestResult {
    // Only custody/effect provisioning is fixture SQL. Every observed value passes through the
    // native content, equipment and combat owners before the new writer persists it.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    let weapon = if hand_defence_only {
        "oteryn:item.tibia.i10392"
    } else {
        "oteryn:item.tibia.i3264"
    };
    for (seed, key, slot, tier) in [
        (
            112_u8,
            weapon,
            6_i16,
            if hand_defence_only { 0_i16 } else { 10 },
        ),
        (114, "oteryn:item.tibia.i6529", 4, 10),
    ] {
        if elemental.is_some() && slot == 6 {
            continue;
        }
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(slot).bind(id(seed).as_slice()).execute(&mut *tx).await?;
        if tier != 0 {
            sqlx::query("INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,$2)")
                .bind(id(seed).as_slice()).bind(tier).execute(&mut *tx).await?;
        }
    }
    if let Some(case) = elemental {
        use ElementalMeleeCase as Case;
        let weapon = match case {
            ElementalMeleeCase::SecondaryImmuneTraining | ElementalMeleeCase::SplitCritical => "oteryn:item.tibia.i49523",
            Case::AlphaProficiency | Case::AlphaCutoff | Case::AlphaPrey | Case::PlainProficiencyAttackPoints | Case::ConvertedCritical => "oteryn:item.tibia.i52338",
            Case::SplitLethalAttribution => "oteryn:item.tibia.i49523",
            // The Flail's physical attack floor guarantees a nonzero roll through the same
            // healing path and leaves a physical component to settle after the source heal.
            Case::DamageHealing | Case::ResistedSecondaryTraining => "oteryn:item.tibia.i49525",
            ElementalMeleeCase::WandDamageHealing => "oteryn:item.tibia.i3071",
            _ => "oteryn:item.tibia.i47368",
        };
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(116).as_slice()).bind(id(42).as_slice()).bind(weapon).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,5,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(116).as_slice()).execute(&mut *tx).await?;
    }
    for key in [
        "oteryn:blessing.spiritual_shielding",
        "oteryn:blessing.embrace_of_tibia",
        "oteryn:blessing.twist_of_fate",
    ] {
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'fixture:p12c') ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).bind(key).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_concoction_effects(transaction_id,character_id,concoction,started_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'wealth_duplex',now()-interval '30 minutes',now()+interval '30 minutes')")
        .bind(id(115).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn row_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_cyclopedia_combat")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn sample_revision(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT sample_revision FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
