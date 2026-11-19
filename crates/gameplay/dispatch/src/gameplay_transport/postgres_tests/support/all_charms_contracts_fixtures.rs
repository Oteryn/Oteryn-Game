use crate::content::charm_source::CanonicalCharmCatalogue;

use crate::content::creature_policy::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags,
};

use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CharacterLease, ExactActorRef,
    GameSessionId, MovementLocalPosition,
};

use crate::gameplay_transport::actor_spell::ChannelSpellStates;

use crate::gameplay_transport::attack::charm_combat;


use oteryn_gameplay_progression::charm_effects::*;

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) const RACE: &str = "oteryn:creature.rat";

pub(super) fn expected(name: &str, value: u32) -> TestResult<CharmEffectOutcome> {
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

pub(super) struct Fixture {
    pub(super) runtime: ChannelRuntimeV1,
    pub(super) states: ChannelSpellStates,
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    pub(super) character: CharacterId,
    pub(super) catalogue: CanonicalCharmCatalogue,
    pub(super) root: GameplayDecisionRoot,
}

pub(super) fn fixture() -> TestResult<Fixture> {
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

pub(super) fn enrage_fixture_with_identity(
    base_health: i64,
    identity: Option<(crate::foundation::WorldId, GameSessionId, CharacterLease)>,
) -> TestResult<(Fixture, ExactActorRef)> {
    use crate::foundation::{ChannelContentPin, NodeId, RuntimeScopeRefV1, WorldId};
    use oteryn_gameplay_domain::domain::soulpit::BossAbility;
    use oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveRequest;
    let id = |tag| [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
    let world = identity.map_or_else(
        || WorldId::decode(&id(72)).map_err(|e| format!("{e:?}")),
        |(world, _, _)| Ok(world),
    )?;
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
    let session = identity.map_or_else(
        || GameSessionId::decode(&id(75)).map_err(|e| format!("{e:?}")),
        |(_, session, _)| Ok(session),
    )?;
    let reservation = runtime
        .reserve_fresh_session(session)
        .map_err(|e| format!("{e:?}"))?;
    let actor = runtime
        .commit_fresh_session(reservation)
        .map_err(|e| format!("{e:?}"))?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(|e| format!("{e:?}"))?;
    let lease = identity.map_or_else(
        || {
            CharacterLease::new(
                CharacterId::decode(&id(76)).map_err(|e| format!("{e:?}"))?,
                1,
            )
            .map_err(|e| format!("{e:?}"))
        },
        |(_, _, lease)| Ok(lease),
    )?;
    let character = lease.character_id();
    runtime
        .bind_attacker_lease(actor, session, lease)
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
                    positions: std::array::from_fn(|index| {
                        Some(MovementLocalPosition {
                            x: if index == 6 {
                                32376
                            } else {
                                32363 + index as i32
                            },
                            y: if index == 6 { 31144 } else { 31133 },
                            floor: 8,
                        })
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

pub(super) fn charm_gaps_periodic_fixture(
    mode: &str,
    (mut f, source): (Fixture, ExactActorRef),
) -> TestResult {
    use crate::foundation::condition::{
        ApplicationFacts, ConditionDefinition, DamageSchedule, DamageSegment, TickFacts,
    };
    use oteryn_gameplay_domain::domain::condition::{ConflictKey, DotElement};
    let maximum = f
        .runtime
        .companion_snapshot(source)
        .map_err(|e| format!("{e:?}"))?
        .maximum_health;
    let health = if mode == "numb" {
        maximum * 3 / 10
    } else {
        maximum * 41 / 100
    };
    f.runtime
        .set_test_creature_health(source, health)
        .map_err(|e| format!("{e:?}"))?;
    f.states
        .get_mut(&f.runtime, f.actor, f.session)
        .ok_or("player")?
        .restore_saved_vitals(if mode == "cleanse" { 15 } else { 185 }, 30);
    let names: &[&str] = if mode == "cleanse" {
        &["cleanse"]
    } else {
        &["parry", "numb"]
    };
    let catalogue = super::all_charms_incoming_order_fixtures::assign_charms(
        &mut f.states,
        &f.runtime,
        source,
        f.actor,
        f.session,
        names,
    );
    let amount = if mode == "cleanse" {
        10
    } else if mode == "numb" {
        u32::try_from(health * 4 / 3)?
    } else {
        u32::try_from(maximum / 10)?
    };
    let definition = ConditionDefinition::new_damage_schedule(
        "fixture:gaps-poison",
        1,
        DotElement::Poison,
        DamageSchedule::Fixed {
            delayed: true,
            segments: vec![DamageSegment {
                count: 3,
                interval_ms: 1000,
                amount,
            }],
        },
    )
    .ok_or("poison")?;
    let facts = ApplicationFacts {
        now: 100_000,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &f.root,
        occurrence: DecisionOccurrenceId::from_bytes([92; 16]),
    };
    assert!(f.states.install_owned_source_condition_fixture(
        &f.runtime,
        f.actor,
        f.session,
        source,
        &[definition],
        &facts
    ));
    let now = 2_100_000;
    let result = f
        .states
        .tick_source_player_conditions_with_charms(
            &mut f.runtime,
            f.actor,
            f.session,
            TickFacts::default(),
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
            Some(&catalogue),
        )
        .ok_or("periodic settlement")?;
    if mode == "cleanse" {
        assert_eq!(result.damage, 10);
        assert!(result.death.is_none());
        let state = f
            .states
            .get(&f.runtime, f.actor, f.session)
            .ok_or("player")?;
        assert_eq!(state.vitals().health, 5);
        assert!(
            state
                .owned_conditions()
                .cleanse_immunity_remaining(ConflictKey::Element(DotElement::Poison), now)
                > 0
        );
    } else {
        let tag = f.runtime.soulpit_variant(source).ok_or("Enrage")?;
        let first = tag
            .incoming_health_change(i64::from(amount), health, maximum)
            .map_err(|e| format!("{e:?}"))?;
        let after_first = health - first;
        let expected = if mode == "numb" {
            after_first
        } else {
            after_first
                - tag
                    .incoming_health_change(i64::from(amount), after_first, maximum)
                    .map_err(|e| format!("{e:?}"))?
        };
        assert_eq!(
            f.runtime
                .companion_snapshot_including_dead(source)
                .map_err(|e| format!("{e:?}"))?
                .health,
            expected
        );
        assert!(
            f.runtime
                .actor_active_speed_delta(source, None, SemanticTimeMicros::from_micros(now))
                .map_err(|e| format!("{e:?}"))?
                < 0,
            "Numb follows authoritative source survival"
        );
    }
    Ok(())
}

pub(super) fn charm_gaps_native_fixture(
    native_bite: bool,
    raw_lethal: bool,
    (mut f, source): (Fixture, ExactActorRef),
) -> TestResult {
    use crate::player_lethal::{PlayerHitKind, PlayerLethalVitals, SourceHitEquipment};
    use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
    let maximum = f
        .runtime
        .companion_snapshot(source)
        .map_err(|e| format!("{e:?}"))?
        .maximum_health;
    let health = if raw_lethal {
        maximum * 3 / 10
    } else if native_bite {
        maximum * 41 / 100
    } else {
        maximum * 405 / 1000
    };
    f.runtime
        .set_test_creature_health(source, health)
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = super::all_charms_incoming_order_fixtures::assign_charms(
        &mut f.states,
        &f.runtime,
        source,
        f.actor,
        f.session,
        &["parry", "numb"],
    );
    let parry = u32::try_from(if raw_lethal {
        health * 4 / 3
    } else {
        maximum / 10
    })?;
    let offence = EquippedOffence {
        reflect_percent: 10_000,
        ..Default::default()
    };
    let equipment = offence.reflected(10_000, true, 1, u64::try_from(maximum)?);
    f.states
        .get_mut(&f.runtime, f.actor, f.session)
        .ok_or("player")?
        .install_equipped_offence(offence);
    let tag = f.runtime.soulpit_variant(source).ok_or("Enrage")?;
    let mut expected = health;
    for amount in if native_bite {
        [parry, equipment]
    } else {
        [equipment, parry]
    } {
        expected -= tag
            .incoming_health_change(i64::from(amount), expected, maximum)
            .map_err(|e| format!("{e:?}"))?;
    }
    assert!(expected > 0);
    let mut attack = crate::gameplay_transport::attack::ChannelAttackStates::default();
    let mut vitals = charm_combat::source_vitals::CharmSourceVitals {
        states: &mut f.states,
        attack: &mut attack,
        catalogue: &catalogue,
        root: GameplayDecisionRoot::from_bytes(f.runtime.content_pin().server_artifact_digest()),
    };
    let now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000);
    vitals
        .apply_source_incoming_hit(
            &mut f.runtime,
            source,
            f.actor,
            f.session,
            10,
            parry,
            None,
            PlayerHitKind::Mitigated,
            "charm-gaps-native",
            now,
            None,
            Some(SourceHitEquipment {
                native_bite,
                magnitude: 10_000,
                element: Some("physical"),
                stream: b"charm-gaps-native",
            }),
        )
        .ok_or("native hit")?;
    assert_eq!(
        f.runtime
            .companion_snapshot(source)
            .map_err(|e| format!("{e:?}"))?
            .health,
        expected,
        "each occurrence uses the preceding reduced successor"
    );
    assert!(
        f.runtime
            .actor_active_speed_delta(source, None, SemanticTimeMicros::from_micros(now.get()))
            .map_err(|e| format!("{e:?}"))?
            < 0,
        "living source receives Numb"
    );
    Ok(())
}

pub(super) fn charm_gaps_durable_player(
    world: crate::foundation::WorldId,
    session: GameSessionId,
    lease: CharacterLease,
    mode: &str,
) -> TestResult {
    let base_health = if mode.starts_with("native") {
        10_000
    } else {
        100
    };
    let fixture = enrage_fixture_with_identity(base_health, Some((world, session, lease)))?;
    if mode.starts_with("native") {
        charm_gaps_native_fixture(
            mode != "native-equipment-first",
            mode == "native-raw-lethal",
            fixture,
        )
    } else {
        charm_gaps_periodic_fixture(mode, fixture)
    }
}
