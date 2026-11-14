//! All 25 canonical charms, all three stages, through the live character-bound
//! evaluator. These tests need no PostgreSQL and do not claim database coverage.
//! Values: pinned Charm catalogue + Crystal201189f parity table; CipSoft news
//! 8140 (final Winter 2024 values), 8421 (intrinsic critical since Summer 2025).
//! Consumer completeness is separately recorded in the all-charms evidence table.
use crate::content::charm_source::CanonicalCharmCatalogue;
use crate::content::creature_policy::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags,
};
use crate::domain::charm::{BestiaryRaceKey, CharmKey, CharmStage};
use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CharacterLease, CommandId, CommandRef, ExactActorRef,
    GameSessionId, MovementLocalPosition,
};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::attack::charm_combat;
use crate::spell::cast::PlayerSpellState;
use crate::spell::combat_batch::LiveActorBinding;
use crate::spell::magnitude_owner::{
    MagnitudeOwnedAttributes, MagnitudeOwnerBinding, MagnitudePolicy, PreparedMagnitudeOwner,
};
use crate::spell::native_combat::{CombatPlan, Element, MagnitudePlan, NativeCombatPlan};
use oteryn_gameplay_progression::charm_effects::*;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RACE: &str = "oteryn:creature.rat";
const OTHER: &str = "oteryn:creature.orc";
const CASES: [(&str, [u32; 3]); 25] = [
    ("adrenaline_burst", [600, 900, 1200]),
    ("bless", [600, 900, 1200]),
    ("carnage", [1000, 2000, 2200]),
    ("cleanse", [600, 900, 1200]),
    ("cripple", [600, 900, 1200]),
    ("curse", [500, 1000, 1100]),
    ("divine_wrath", [500, 1000, 1100]),
    ("dodge", [500, 1000, 1100]),
    ("enflame", [500, 1000, 1100]),
    ("fatal_hold", [3000, 4500, 6000]),
    ("freeze", [500, 1000, 1100]),
    ("gut", [600, 900, 1200]),
    ("low_blow", [400, 800, 900]),
    ("numb", [600, 900, 1200]),
    ("overflux", [500, 1000, 1100]),
    ("overpower", [500, 1000, 1100]),
    ("parry", [500, 1000, 1100]),
    ("poison", [500, 1000, 1100]),
    ("savage_blow", [2000, 4000, 4400]),
    ("scavenge", [6000, 9000, 12000]),
    ("vampiric_embrace", [160, 240, 320]),
    ("void_inversion", [2000, 3000, 4000]),
    ("voids_call", [80, 120, 160]),
    ("wound", [500, 1000, 1100]),
    ("zap", [500, 1000, 1100]),
];
fn hit(source: CharmHitSource, lethal: bool) -> CharmHookEvent {
    CharmHookEvent::CommittedHit {
        source,
        attacker: CharmAttackerFacts {
            level: 100,
            max_health: 10_000,
            max_mana: 20_000,
        },
        creature_max_health: 10_000,
        health_before: 10_000,
        health_after: if lethal { 0 } else { 9_999 },
    }
}
fn event(name: &str) -> CharmHookEvent {
    match name {
        "low_blow" | "savage_blow" => CharmHookEvent::AttackDamageCalculation {
            source: CharmHitSource::CharacterAttack,
        },
        "adrenaline_burst" | "cleanse" | "numb" | "parry" => {
            CharmHookEvent::IncomingCreatureHit { base_damage: 1234 }
        }
        "dodge" => CharmHookEvent::IncomingCreatureAttack,
        "void_inversion" => CharmHookEvent::IncomingManaDrain { mana_drained: 1234 },
        "bless" => CharmHookEvent::CharacterDeath,
        "gut" => CharmHookEvent::CreatureLoot,
        "scavenge" => CharmHookEvent::Skinning,
        "carnage" => hit(CharmHitSource::CharacterAttack, true),
        _ => hit(CharmHitSource::CharacterAttack, false),
    }
}
fn expected(name: &str, value: u32) -> TestResult<CharmEffectOutcome> {
    use CharmEffectOutcome as O;
    let percent = CharmPercent::from_hundredths(value).ok_or("percent")?;
    let kind = |element, armor, ignore| CharmDamageKind {
        element,
        reduced_by_armor: armor,
        ignores_resistances: ignore,
    };
    Ok(match name {
        "wound" => O::ProcDamage {
            damage: kind(CharmElement::Physical, false, false),
            amount: 200,
        },
        "enflame" => O::ProcDamage {
            damage: kind(CharmElement::Fire, false, false),
            amount: 200,
        },
        "poison" => O::ProcDamage {
            damage: kind(CharmElement::Earth, false, false),
            amount: 200,
        },
        "freeze" => O::ProcDamage {
            damage: kind(CharmElement::Ice, false, false),
            amount: 200,
        },
        "zap" => O::ProcDamage {
            damage: kind(CharmElement::Energy, false, false),
            amount: 200,
        },
        "curse" => O::ProcDamage {
            damage: kind(CharmElement::Death, false, false),
            amount: 200,
        },
        "divine_wrath" => O::ProcDamage {
            damage: kind(CharmElement::Holy, false, false),
            amount: 200,
        },
        "overpower" | "overflux" => O::ProcDamage {
            damage: kind(CharmElement::Physical, false, true),
            amount: 500,
        },
        "carnage" => O::AreaDamageAroundKill {
            damage: kind(CharmElement::Physical, true, false),
            amount: 600,
        },
        "parry" => O::ReflectDamage {
            damage: kind(CharmElement::Physical, true, true),
            amount: 1234,
        },
        "cripple" | "numb" => O::ParalyseCreature {
            duration_ms: 10_000,
        },
        "adrenaline_burst" => O::Haste {
            duration_ms: 10_000,
        },
        "fatal_hold" => O::PreventCreatureFlee {
            duration_ms: 30_000,
        },
        "cleanse" => O::CleanseOneNegativeCondition,
        "dodge" => O::DodgeAttack,
        "void_inversion" => O::InvertManaDrain { mana_gained: 1234 },
        "bless" => O::DeathLossReduction { percent },
        "gut" => O::CreatureProductBonus { percent },
        "scavenge" => O::SkinningChanceBonus { percent },
        "low_blow" => O::CriticalHitChanceBonus { percent },
        "savage_blow" => O::CriticalExtraDamageBonus { percent },
        "vampiric_embrace" => O::LifeLeechBonus { percent },
        "voids_call" => O::ManaLeechBonus { percent },
        _ => return Err(format!("unlisted charm {name}").into()),
    })
}
struct Fixture {
    runtime: ChannelRuntimeV1,
    states: ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    character: CharacterId,
    catalogue: CanonicalCharmCatalogue,
    root: GameplayDecisionRoot,
}
fn fixture() -> TestResult<Fixture> {
    let (mut runtime, actor, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(66, 3);
    let character = CharacterId::decode(session.as_bytes()).map_err(|e| format!("{e:?}"))?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(character, 1).map_err(|e| format!("{e:?}"))?,
        )
        .map_err(|e| format!("{e:?}"))?;
    runtime
        .initialize_pinned_test_position(
            actor,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        )
        .map_err(|e| format!("{e:?}"))?;
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            actor,
            session,
            crate::gameplay_transport::actor_spell::tests::FACTS,
            (0, 0),
            SemanticTimeMicros::from_micros(100_000),
        )
        .ok_or("player")?;
    Ok(Fixture {
        runtime,
        states,
        actor,
        session,
        character,
        catalogue: CanonicalCharmCatalogue::embedded()?,
        root: GameplayDecisionRoot::from_bytes([42; 32]),
    })
}
fn assign(f: &mut Fixture, name: &str, stage: u8) -> TestResult {
    let key = CharmKey::new(format!("oteryn:charm.{name}")).map_err(|e| format!("{e:?}"))?;
    let state = CombatCharmState::new(
        crate::domain::CharacterId::from_bytes(*f.character.as_bytes())
            .map_err(|e| format!("{e:?}"))?,
        CharacterCharmState {
            unlocks: [(
                key.clone(),
                CharmStage::new(stage).map_err(|e| format!("{e:?}"))?,
            )]
            .into(),
            assignments: [(
                key,
                BestiaryRaceKey::new(RACE).map_err(|e| format!("{e:?}"))?,
            )]
            .into(),
        },
    );
    assert!(
        f.states
            .install_charm_state(&f.runtime, f.actor, f.session, state)
    );
    Ok(())
}
fn evaluate(f: &Fixture, race: &str, event: CharmHookEvent, seed: u32) -> Vec<CharmOutcome> {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&seed.to_le_bytes());
    charm_combat::evaluate(
        &f.states,
        &f.runtime,
        &f.catalogue,
        f.actor,
        f.session,
        race,
        &f.root,
        DecisionOccurrenceId::from_bytes(bytes),
        event,
    )
}
#[test]
fn all25_charms_all75_stages_use_canonical_values_bound_character_and_correct_effect() -> TestResult
{
    let mut f = fixture()?;
    assert_eq!(f.catalogue.catalogue().definitions().count(), CASES.len());
    for (name, values) in CASES {
        let mut prior = vec![false; 512];
        for stage in 1..=3 {
            assign(&mut f, name, stage)?;
            let key = format!("oteryn:charm.{name}");
            let definition = f.catalogue.charm(&key).ok_or("canonical charm")?;
            let value = values[usize::from(stage - 1)];
            assert_eq!(
                definition.stage(stage).ok_or("stage")?.hundredths(),
                value,
                "{name} stage {stage}"
            );
            let wanted = expected(name, value)?;
            let mut triggered = 0;
            for seed in 0..512 {
                let outcomes = evaluate(&f, RACE, event(name), seed);
                assert_eq!(outcomes.len(), 1, "{name} stage {stage}");
                assert_eq!(outcomes[0].stage, stage);
                assert_eq!(outcomes[0].charm_key, key);
                assert_eq!(
                    evaluate(&f, RACE, event(name), seed),
                    outcomes,
                    "deterministic replay"
                );
                let actual = charm_combat::effect(&outcomes[0]);
                if let Some(actual) = actual {
                    assert_eq!(actual, wanted, "{name} stage {stage} seed {seed}");
                    triggered += 1;
                } else {
                    assert_eq!(outcomes[0].result, CharmResult::NotTriggered, "{name}");
                }
                assert!(
                    !prior[seed as usize] || actual.is_some(),
                    "{name}: higher stage cannot lose the same trigger"
                );
                prior[seed as usize] = actual.is_some();
            }
            if definition.effect().stage_value() == CharmStageValue::TriggerChancePercent {
                assert!(
                    triggered > 0 && triggered < 512,
                    "{name}: observe both hit and miss"
                );
            } else {
                assert_eq!(triggered, 512, "{name}: passive bonus must not roll");
            }
            assert!(
                evaluate(&f, OTHER, event(name), 1).is_empty(),
                "{name}: unrelated race"
            );
            for lethal in [false, true] {
                assert!(
                    evaluate(&f, RACE, hit(CharmHitSource::CharmDamage, lethal), 1).is_empty(),
                    "{name}: no recursive proc"
                );
            }
        }
    }
    Ok(())
}
#[test]
fn all25_charms_wrong_hooks_and_secondary_auto_targets_do_not_gain_unrelated_effects() -> TestResult
{
    let mut f = fixture()?;
    for (name, _) in CASES {
        assign(&mut f, name, 3)?;
        for (event, allowed) in [
            (
                CharmHookEvent::AttackDamageCalculation {
                    source: CharmHitSource::CharacterAutoAttackOffTarget,
                },
                matches!(name, "low_blow" | "savage_blow"),
            ),
            (
                hit(CharmHitSource::CharacterAutoAttackOffTarget, false),
                matches!(name, "vampiric_embrace" | "voids_call"),
            ),
            (
                hit(CharmHitSource::CharacterAutoAttackOffTarget, true),
                matches!(name, "vampiric_embrace" | "voids_call"),
            ),
            (
                CharmHookEvent::LeechCalculation {
                    source: CharmHitSource::CharacterAttack,
                },
                matches!(name, "vampiric_embrace" | "voids_call"),
            ),
            (
                CharmHookEvent::IncomingManaDrain { mana_drained: 1234 },
                matches!(name, "void_inversion" | "numb" | "adrenaline_burst"),
            ),
            (CharmHookEvent::CreatureLoot, name == "gut"),
            (CharmHookEvent::Skinning, name == "scavenge"),
            (CharmHookEvent::CharacterDeath, name == "bless"),
        ] {
            assert_eq!(
                !evaluate(&f, RACE, event, 7).is_empty(),
                allowed,
                "{name} {event:?}"
            );
        }
    }
    Ok(())
}
fn add_targets(f: &mut Fixture) -> TestResult<Vec<LiveActorBinding>> {
    let policies = [RACE, OTHER].map(|key| CompiledCreaturePolicy {
        definition_key: key.into(),
        definition_revision: "test:all-charms".into(),
        display_name: key.into(),
        maximum_health: 10_000,
        forge_difficulty: None,
        base_speed: 100,
        outfit_look_type: 1,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        preferred_distance: Some(1),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
        condition_immunities: vec![],
        defense: Some(0),
        armor: Some(0),
        mitigation: Some(CreatureExactRatio {
            numerator: 0,
            denominator: 1,
        }),
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    });
    f.runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                f.runtime.content_pin().server_artifact_digest(),
                policies.to_vec(),
            )
            .map_err(|e| format!("{e:?}"))?,
        )
        .map_err(|e| format!("{e:?}"))?;
    [RACE, OTHER]
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            let actor = f
                .runtime
                .admit_pinned_monster_lab_creature(
                    MovementLocalPosition {
                        x: 11 + i as i32,
                        y: 10,
                        floor: 7,
                    },
                    key,
                    10_000,
                )
                .map_err(|e| format!("{e:?}"))?;
            f.runtime
                .install_creature_policy(actor, key)
                .map_err(|e| format!("{e:?}"))?;
            Ok(LiveActorBinding {
                source_id: 2 + i as u64,
                actor,
                target_atom: key.into(),
            })
        })
        .collect()
}
fn attributes(f: &Fixture, state: &PlayerSpellState) -> MagnitudeOwnedAttributes {
    MagnitudeOwnedAttributes {
        policy: MagnitudePolicy::BaselineTest,
        item_augments: vec![],
        owner_revisions: Some((1, 1, None)),
        binding: MagnitudeOwnerBinding {
            actor: f.actor,
            session: f.session,
            character: f.character,
            lease_generation: 1,
            player_revision: state.revision(),
            content_digest: f.runtime.content_pin().server_artifact_digest(),
        },
        base_critical_chance_permyriad: Some(500),
        base_critical_extra_permyriad: Some(1000),
        equipment_critical_chance_permyriad: Some(0),
        equipment_critical_extra_permyriad: Some(0),
        fatal_chance_permyriad: None,
        armor_penetration_permyriad: None,
        elemental_pierce_permyriad: None,
        wheel_effects: None,
        wheel_flat_damage: None,
        wheel_flat_healing: None,
        source_damage_multiplier_percent: None,
        source_healing_multiplier_percent: None,
        healing_dealt_percent: None,
        monster_armor_disabled: None,
        inactive_extra_stages: None,
    }
}
#[test]
fn all25_critical_charms_use_actual_assignment_with_intrinsic_critical_and_no_equipment()
-> TestResult {
    let mut f = fixture()?;
    let bindings = add_targets(&mut f)?;
    let plan = NativeCombatPlan::Combat(CombatPlan {
        element: Element::Physical,
        area: None,
        hits: bindings
            .iter()
            .map(|b| MagnitudePlan {
                target: b.source_id,
                magnitude: 10_000,
                bonus_percent: 0,
                side_percent: None,
                delay_ms: 0,
            })
            .collect(),
        cooldown_ms: None,
        shared_cooldown: None,
        reduce_all_spell_cooldowns_ms: 0,
        block_armor: false,
        use_weapon_charges: false,
        weapon_missile: false,
        dispel_paralysis_targets: vec![],
        resolve_critical_and_fatal_once: true,
    });
    let NativeCombatPlan::Combat(combat) = &plan else {
        return Err("combat".into());
    };
    for (name, bonuses) in [
        ("low_blow", [400, 800, 900]),
        ("savage_blow", [2000, 4000, 4400]),
    ] {
        for stage in 1..=3 {
            assign(&mut f, name, stage)?;
            let state = f
                .states
                .get(&f.runtime, f.actor, f.session)
                .ok_or("current player")?;
            let attrs = attributes(&f, state);
            let mut owner = PreparedMagnitudeOwner::qualify(
                &f.runtime,
                state,
                f.actor,
                f.session,
                Some(&attrs),
                &plan,
                &bindings,
                &[],
                100,
            )
            .map_err(|e| format!("qualify {e:?}"))?;
            let command = CommandRef::new(
                f.session,
                CommandId::new(u64::from(stage)).map_err(|e| format!("{e:?}"))?,
            );
            charm_combat::select_spell_critical(
                &f.states,
                &f.runtime,
                &f.catalogue,
                f.actor,
                command,
                &mut owner,
            )
            .map_err(|e| format!("projection {e:?}"))?;
            let bonus = bonuses[usize::from(stage - 1)];
            let roll = if name == "low_blow" {
                i64::from(500 + bonus)
            } else {
                1
            };
            let mut draws = 0;
            // Unassigned first: its base-critical result must not suppress the assigned species.
            let mut amounts = [0; 2];
            for i in [1, 0] {
                amounts[i] = owner
                    .finish(&plan, &combat.hits[i], &bindings[i], &mut |_, _| {
                        draws += 1;
                        roll
                    })
                    .map_err(|e| format!("finish {e:?}"))?;
            }
            let expected = if name == "low_blow" {
                [11_000, 10_000]
            } else {
                [11_000 + i64::from(bonus), 11_000]
            };
            assert_eq!(
                amounts, expected,
                "{name} stage {stage}: real assignment, innate critical, no Strike"
            );
            assert_eq!(draws, 1, "one AoE critical roll");
        }
    }
    Ok(())
}

fn enrage_fixture(base_health: i64) -> TestResult<(Fixture, ExactActorRef)> {
    use crate::foundation::{ChannelContentPin, NodeId, RuntimeScopeRefV1, WorldId};
    use oteryn_gameplay_domain::domain::soulpit::BossAbility;
    use oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveRequest;
    let id = |tag| [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
    let world = WorldId::decode(&id(72)).map_err(|e| format!("{e:?}"))?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        [1; 32],
        [2; 32],
        [3; 32],
        [4; 32],
        (32375, 31151, 8),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        RuntimeScopeRefV1::instance(world, id(73)).map_err(|e| format!("{e:?}"))?,
        NodeId::decode(&id(74)).map_err(|e| format!("{e:?}"))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(|e| format!("{e:?}"))?;
    let session = GameSessionId::decode(&id(75)).map_err(|e| format!("{e:?}"))?;
    let reservation = runtime
        .reserve_fresh_session(session)
        .map_err(|e| format!("{e:?}"))?;
    let actor = runtime
        .commit_fresh_session(reservation)
        .map_err(|e| format!("{e:?}"))?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(|e| format!("{e:?}"))?;
    let character = CharacterId::decode(&id(76)).map_err(|e| format!("{e:?}"))?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(character, 1).map_err(|e| format!("{e:?}"))?,
        )
        .map_err(|e| format!("{e:?}"))?;
    let policy = CompiledCreaturePolicy {
        definition_key: RACE.into(),
        definition_revision: "test:all-charms-enrage".into(),
        display_name: "Rat".into(),
        maximum_health: base_health,
        forge_difficulty: None,
        base_speed: 100,
        outfit_look_type: 1,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        preferred_distance: Some(1),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
        condition_immunities: vec![],
        defense: Some(0),
        armor: Some(0),
        mitigation: Some(CreatureExactRatio {
            numerator: 0,
            denominator: 1,
        }),
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    };
    let policies = CompiledCreaturePolicies::from_active_artifact(
        runtime.content_pin().server_artifact_digest(),
        vec![policy],
    )
    .map_err(|e| format!("{e:?}"))?;
    runtime
        .install_companion_policies(policies.clone())
        .map_err(|e| format!("{e:?}"))?;
    let actors = {
        let kernel = runtime
            .borrow_soulpit_kernel()
            .map_err(|e| format!("{e:?}"))?;
        kernel
            .carrier
            .spawn_soulpit_wave(
                kernel.continuity,
                kernel.content,
                &policies,
                SoulpitWaveRequest {
                    handoff_id: id(77),
                    wave: 4,
                    species: RACE.into(),
                    boss_ability: BossAbility::Enrage,
                    now_micros: 0,
                    positions: std::array::from_fn(|index| MovementLocalPosition {
                        x: if index == 6 {
                            32376
                        } else {
                            32363 + index as i32
                        },
                        y: if index == 6 { 31144 } else { 31133 },
                        floor: 8,
                    }),
                },
                |_| true,
            )
            .map_err(|e| format!("{e:?}"))?
    };
    let target = actors[6].ok_or("actual Soulpit boss")?.actor;
    assert_eq!(
        runtime
            .soulpit_variant(target)
            .and_then(|tag| tag.boss)
            .map(|boss| boss.ability),
        Some(BossAbility::Enrage)
    );
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            actor,
            session,
            crate::spell::cast::CharacterCastFacts {
                level: 20,
                ..crate::gameplay_transport::actor_spell::tests::FACTS
            },
            (0, 0),
            SemanticTimeMicros::from_micros(100_000),
        )
        .ok_or("Instance player state")?;
    states
        .get_mut(&runtime, actor, session)
        .ok_or("caster")?
        .restore_saved_vitals(20, 30);
    Ok((
        Fixture {
            runtime,
            states,
            actor,
            session,
            character,
            catalogue: CanonicalCharmCatalogue::embedded()?,
            root: GameplayDecisionRoot::from_bytes([42; 32]),
        },
        target,
    ))
}

#[test]
fn all25_leech_respects_real_enrage_owner_before_hp_cap_and_without_damage_prey() -> TestResult {
    use crate::ability::{AbilityOccurrence, RevisionSet};
    use crate::gameplay_transport::actor_spell::{commit_owner_batch, stage_player_batch};
    use crate::gameplay_transport::item_spell_augments::leech_projection;
    use crate::spell::cast::{CastContext, cast, v1_spell_book};
    use crate::spell::combat_batch::{OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect};
    use oteryn_protocol_oteryn::actor_spell::{SpellCastIntent, SpellTarget};
    use std::num::NonZeroU32;
    // health band, base species HP, Damage Prey, source healing, expected physical
    // pre-cap damage, life gain, mana gain (Void's Call III: 16% + 1.6%).
    for (health_percent, base_health, prey, heal_first, effective, life, mana) in [
        (30, 1000, 0, false, 60, 10, 11),
        (50, 1000, 0, false, 75, 12, 14),
        (70, 1000, 0, false, 90, 15, 16),
        (90, 1000, 0, false, 100, 16, 18),
        (30, 1000, 25, false, 75, 10, 11),
        (30, 20, 0, false, 60, 10, 11),
        (30, 1000, 0, true, 100, 16, 18),
    ] {
        let (mut f, target) = enrage_fixture(base_health)?;
        assign(&mut f, "voids_call", 3)?;
        let maximum = f
            .runtime
            .companion_snapshot(target)
            .map_err(|e| format!("{e:?}"))?
            .maximum_health;
        let health = maximum * health_percent / 100;
        f.runtime
            .set_test_creature_health(target, health)
            .map_err(|e| format!("{e:?}"))?;
        let before = f
            .states
            .get(&f.runtime, f.actor, f.session)
            .ok_or("player")?;
        let occurrence = AbilityOccurrence::new(
            "spell-cast:enrage-leech",
            RevisionSet::new("rules:1", "content:1", "world:1", "formula:1", "sim:1")
                .map_err(|e| format!("{e:?}"))?,
        )
        .map_err(|e| format!("{e:?}"))?;
        let atom = crate::spell::combat_execution::actor_atom(f.actor);
        let now = SemanticTimeMicros::from_micros(100_000);
        let paid = cast(
            &v1_spell_book().map_err(|e| format!("{e:?}"))?,
            before,
            &SpellCastIntent {
                spell: NonZeroU32::new(1).ok_or("spell")?,
                target: SpellTarget::None,
                aim_at_target: false,
            },
            CastContext {
                caster: &atom,
                owner_scope: "instance-owner",
                occurrence: occurrence.clone(),
                now,
                draw: &mut |_, max| max,
            },
        )
        .map_err(|e| format!("payment {e:?}"))?;
        let mut batch = OwnerCombatBatch {
            caster: f.actor,
            attacker: f.character,
            current_lease_generation: 1,
            command: CommandRef::new(f.session, CommandId::new(1).map_err(|e| format!("{e:?}"))?),
            occurrence: occurrence.into(),
            binding: b"qualified:enrage-leech-owner".to_vec(),
            anchor: Some(paid.payment_anchor_from(before).ok_or("payment anchor")?),
            now_us: Some(now),
            now_ms: 100,
            deferred: None,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: if heal_first {
                    OwnerCombatChange::DamageWithHealing {
                        target_atom: RACE.into(),
                        damage: 100,
                        healing: maximum,
                    }
                } else {
                    OwnerCombatChange::Damage {
                        target_atom: RACE.into(),
                        magnitude: 100,
                    }
                },
            }],
        };
        let inputs = leech_projection::capture(
            &f.runtime,
            &f.states,
            &f.catalogue,
            &batch,
            (1600, 1600),
            None,
            &f.root,
            DecisionOccurrenceId::from_bytes([9; 16]),
        )
        .map_err(|e| format!("capture {e:?}"))?;
        if let OwnerCombatChange::Damage { magnitude, .. } = &mut batch.effects[0].change {
            *magnitude = crate::gameplay_transport::prey::damage(*magnitude, prey);
        }
        let staged = f
            .runtime
            .stage_spell_batch(&batch)
            .map_err(|e| format!("physical {e:?}"))?;
        let mut proof = stage_player_batch(&f.runtime, &f.states, &batch, Some(paid.clone()))
            .map_err(|e| format!("player stage {e:?}"))?;
        proof
            .stage_item_spell_leech(&f.runtime, &staged, &inputs, &[])
            .map_err(|e| format!("leech stage {e:?}"))?;
        let receipt = commit_owner_batch(&mut f.runtime, &mut f.states, staged, Some(proof))
            .map_err(|e| format!("commit {e:?}"))?;
        assert!(receipt.applied);
        let expected_after = (if heal_first { maximum } else { health }) - effective;
        assert_eq!(
            f.runtime
                .companion_snapshot_including_dead(target)
                .map_err(|e| format!("{e:?}"))?
                .health,
            expected_after.max(0),
            "actual physical Enrage branch"
        );
        let after = f
            .states
            .get(&f.runtime, f.actor, f.session)
            .ok_or("after player")?
            .vitals();
        assert_eq!(
            (after.health, after.mana),
            (paid.vitals().health + life, paid.vitals().mana + mana),
            "Enrage {health_percent}% baseHP={base_health} Prey={prey} healing={heal_first}: mitigated pre-overkill leech"
        );
        let player = f
            .states
            .get(&f.runtime, f.actor, f.session)
            .ok_or("committed player")?
            .clone();
        let replay = f
            .runtime
            .stage_spell_batch(&batch)
            .map_err(|e| format!("replay {e:?}"))?;
        assert!(
            !commit_owner_batch(&mut f.runtime, &mut f.states, replay, None)
                .map_err(|e| format!("replay commit {e:?}"))?
                .applied
        );
        assert_eq!(f.states.get(&f.runtime, f.actor, f.session), Some(&player));
    }
    Ok(())
}

#[test]
fn all25_secondary_auto_leech_retains_only_assigned_minor_bonus_at_every_stage() -> TestResult {
    use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
    let mut f = fixture()?;
    for (name, values, life) in [
        ("vampiric_embrace", [160_u16, 240, 320], true),
        ("voids_call", [80_u16, 120, 160], false),
    ] {
        for stage in 1..=3 {
            assign(&mut f, name, stage)?;
            let value = values[usize::from(stage - 1)];
            for lethal in [false, true] {
                let outcomes = evaluate(
                    &f,
                    RACE,
                    hit(CharmHitSource::CharacterAutoAttackOffTarget, lethal),
                    1,
                );
                let base = EquippedOffence::from_sums(0, 0, 1600, 800);
                let mut secondary = base.clone();
                // Call the same projection that the production cleave drain calls,
                // not a test-local copy of its zeroing/bonus order.
                charm_combat::committed_hit_bonuses(&mut secondary, &outcomes, true);
                assert_eq!(
                    (secondary.life_leech, secondary.mana_leech),
                    if life { (value, 0) } else { (0, value) },
                    "D186 {name} stage {stage}, lethal={lethal}: keep only the qualifying charm"
                );
                let mut primary = base;
                charm_combat::committed_hit_bonuses(&mut primary, &outcomes, false);
                assert_eq!(
                    (primary.life_leech, primary.mana_leech),
                    if life {
                        (1600 + value, 800)
                    } else {
                        (1600, 800 + value)
                    }
                );
                let mut none = EquippedOffence::default();
                charm_combat::committed_hit_bonuses(&mut none, &outcomes, true);
                assert_eq!(
                    (none.life_leech, none.mana_leech),
                    (0, 0),
                    "no underlying skill is invented"
                );
                let mut unrelated = EquippedOffence::from_sums(0, 0, 1600, 800);
                let other = evaluate(
                    &f,
                    OTHER,
                    hit(CharmHitSource::CharacterAutoAttackOffTarget, lethal),
                    1,
                );
                charm_combat::committed_hit_bonuses(&mut unrelated, &other, true);
                assert_eq!(
                    (unrelated.life_leech, unrelated.mana_leech),
                    (0, 0),
                    "unassigned secondary species"
                );
            }
        }
    }
    Ok(())
}
