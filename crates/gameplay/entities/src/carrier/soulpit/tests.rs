//! Physical Instance-kernel qualification, not an activity admission fixture.
use super::super::runtime_actor_companion::{CompiledCreaturePolicy, CreatureFlags};
use super::*;
use oteryn_gameplay_domain::domain::condition::{ConditionType, ConditionValues, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionStore,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

type TestResult<T = ()> = Result<T, String>;
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
fn id(seed: u8) -> [u8; 16] {
    let mut value = [seed; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}

fn fixture(
    capacity: usize,
    instance: bool,
) -> TestResult<(
    ChannelActorCarrier,
    NamespaceContinuityGuard,
    ChannelContentPin,
    CompiledCreaturePolicies,
)> {
    let world = WorldId::decode(&id(1)).map_err(debug)?;
    let scope = if instance {
        RuntimeScopeRefV1::instance(world, id(2)).map_err(debug)?
    } else {
        RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(2)).map_err(debug)?)
    };
    let mut owner =
        NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
            runtime_scope: scope,
            scope_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        });
    let carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut owner, capacity).map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        [1; 32],
        [2; 32],
        [3; 32],
        [4; 32],
        (32375, 31151, 8),
    );
    let policies = CompiledCreaturePolicies::from_active_artifact(
        [1; 32],
        vec![CompiledCreaturePolicy {
            definition_key: "oteryn:creature.rat".into(),
            definition_revision: "fixture:1".into(),
            display_name: "Rat".into(),
            maximum_health: 101,
            forge_difficulty: None,
            base_speed: 110,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            defense: Some(1),
            armor: Some(1),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            blocked_by_nearby_players: false,
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        }],
    )
    .map_err(debug)?;
    Ok((carrier, owner, pin, policies))
}
fn request(wave: u8, ability: BossAbility) -> SoulpitWaveRequest {
    SoulpitWaveRequest {
        handoff_id: id(7),
        wave,
        species: "oteryn:creature.rat".into(),
        boss_ability: ability,
        now_micros: 4_000_000,
        positions: std::array::from_fn(|index| {
            if wave == 4 && index == 6 {
                MovementLocalPosition {
                    x: 32376,
                    y: 31144,
                    floor: 8,
                }
            } else {
                MovementLocalPosition {
                    x: 32363 + index as i32,
                    y: 31133,
                    floor: 8,
                }
            }
        }),
    }
}
fn hit(
    carrier: &mut ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    actor: ExactActorRef,
    occurrence: &str,
    damage: i64,
) -> TestResult<OwnerDamageResult> {
    let binding = format!("{occurrence}\0soulpit-test");
    carrier
        .commit_encounter_single_damage(
            owner,
            actor.0,
            OwnerDamageCommand {
                target: b"oteryn:creature.rat",
                occurrence: occurrence.as_bytes(),
                binding: binding.as_bytes(),
                damage,
            },
            None,
            false,
            None,
            Some(10_000),
        )
        .map_err(debug)
}

#[test]
fn instance_wave_attempts_preserve_partial_failure_and_replay_without_refill() -> TestResult {
    let (mut carrier, owner, pin, policies) = fixture(3, true)?;
    let request = request(1, BossAbility::Overpower);
    let first = carrier
        .spawn_soulpit_wave(&owner, &pin, &policies, request.clone(), |at| at.x != 32363)
        .map_err(debug)?;
    assert!(first[0].is_none());
    assert_eq!(
        first.iter().flatten().count(),
        3,
        "existing actor capacity records failed attempts"
    );
    let actor = first[1].ok_or("missing successful source slot")?.actor;
    assert_eq!(
        hit(&mut carrier, &owner, actor, "hit", 10)?.health_after,
        142
    );
    let mut retry = request.clone();
    retry.now_micros += 1_000_000;
    assert_eq!(
        carrier
            .spawn_soulpit_wave(&owner, &pin, &policies, retry, |_| true)
            .map_err(debug)?,
        first
    );
    let census = carrier.soulpit_actors(&owner, id(7)).map_err(debug)?;
    assert_eq!(
        census
            .iter()
            .find(|item| item.actor == actor)
            .ok_or("actor")?
            .health,
        142
    );
    let mut conflict = request;
    conflict.positions[0].x += 1;
    assert_eq!(
        carrier.spawn_soulpit_wave(&owner, &pin, &policies, conflict, |_| true),
        Err(CarrierError::PlanConflict)
    );
    let (mut channel, owner, pin, policies) = fixture(8, false)?;
    let before = channel.clone();
    assert_eq!(
        channel.spawn_soulpit_wave(
            &owner,
            &pin,
            &policies,
            self::request(1, BossAbility::Overpower),
            |_| true
        ),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(channel, before, "Channel cannot become a Soulpit scope");
    Ok(())
}

#[test]
fn later_stack_overflow_refuses_before_any_source_spawn_attempt() -> TestResult {
    let (mut carrier, owner, pin, policies) = fixture(8, true)?;
    let mut policy = policies.policies()[0].as_ref().clone();
    policy.maximum_health = i64::MAX / 4;
    let policies =
        CompiledCreaturePolicies::from_active_artifact([1; 32], vec![policy]).map_err(debug)?;
    let before = carrier.clone();
    assert_eq!(
        carrier.spawn_soulpit_wave(
            &owner,
            &pin,
            &policies,
            request(4, BossAbility::Overpower),
            |_| true
        ),
        Err(CarrierError::DamageOverflow)
    );
    assert_eq!(
        carrier, before,
        "a late boss overflow cannot leave earlier unreceipted creatures"
    );
    Ok(())
}

#[test]
fn retired_soulpit_generation_cannot_read_or_drive_a_replacement_boss() -> TestResult {
    let (mut carrier, owner, pin, policies) = fixture(7, true)?;
    let first = carrier
        .spawn_soulpit_wave(
            &owner,
            &pin,
            &policies,
            request(1, BossAbility::Oppressor),
            |_| true,
        )
        .map_err(debug)?;
    let retired = first[0].ok_or("first actor")?.actor;
    assert_eq!(carrier.retire_soulpit(&owner, id(7)).map_err(debug)?, 7);
    let next = carrier
        .spawn_soulpit_wave(
            &owner,
            &pin,
            &policies,
            request(4, BossAbility::Oppressor),
            |_| true,
        )
        .map_err(debug)?;
    let replacement = next
        .iter()
        .flatten()
        .find(|item| item.actor.0.actor_local_id == retired.0.actor_local_id)
        .ok_or("reused slot")?
        .actor;
    assert_ne!(
        retired.0.actor_local_generation,
        replacement.0.actor_local_generation
    );
    let replacement_tag = carrier
        .soulpit_tag(&owner, replacement)
        .map_err(debug)?
        .ok_or("replacement tag")?;
    assert_eq!(replacement_tag.stack, SoulpitStack::Boss);
    assert_eq!(
        carrier.soulpit_tag(&owner, retired),
        Err(CarrierError::StaleActorGeneration)
    );
    assert!(matches!(
        carrier.scoped_actor_conditions(&owner, retired, None),
        Err(CarrierError::StaleActorGeneration)
    ));
    let before = carrier.clone();
    assert!(matches!(
        carrier.drive_soulpit_oppressor(
            &owner,
            SoulpitOppressorCast {
                source: retired,
                current_target: None,
                now_micros: 6_000_000,
                draws: [25, 30, 15],
                decision_root: [1; 32],
            },
            |_, _, _, _| true,
            |_, _| Ok(0),
        ),
        Err(CarrierError::StaleActorGeneration)
    ));
    assert_eq!(
        carrier, before,
        "a retired caster cannot advance its replacement's cadence"
    );
    Ok(())
}

#[test]
fn source_enrage_uses_current_hp_once_and_overpower_precedes_stack_scaling() -> TestResult {
    let (mut carrier, owner, pin, policies) = fixture(8, true)?;
    let wave = carrier
        .spawn_soulpit_wave(
            &owner,
            &pin,
            &policies,
            request(4, BossAbility::Enrage),
            |_| true,
        )
        .map_err(debug)?;
    let boss = wave[6].ok_or("boss")?.actor;
    let tag = carrier
        .soulpit_tag(&owner, boss)
        .map_err(debug)?
        .ok_or("tag")?;
    assert_eq!(tag.experience(1000).map_err(debug)?, 28_000);
    assert_eq!(
        hit(&mut carrier, &owner, boss, "first", 150)?.health_after,
        593
    );
    assert_eq!(
        hit(&mut carrier, &owner, boss, "second", 100)?.health_after,
        503
    );
    assert!(!hit(&mut carrier, &owner, boss, "second", 100)?.applied);
    for (occurrence, expected) in [
        ("third", 413),
        ("fourth", 338),
        ("fifth", 263),
        ("sixth", 203),
        ("seventh", 143),
        ("eighth", 43),
    ] {
        assert_eq!(
            hit(&mut carrier, &owner, boss, occurrence, 100)?.health_after,
            expected
        );
    }
    assert_eq!(
        tag.incoming_health_change(100, 593, 743).map_err(debug)?,
        90,
        "source callback also reduces heals"
    );
    let overpower = SoulpitCreatureTag {
        boss: Some(SoulpitBossState {
            ability: BossAbility::Overpower,
            next_oppressor_at_micros: 0,
            ordinal: 0,
        }),
        ..tag
    };
    assert_eq!(
        overpower.outgoing_damage(101, 50).map_err(debug)?,
        (661, true)
    );
    assert_eq!(
        overpower.outgoing_damage(101, 51).map_err(debug)?,
        (530, false)
    );
    let summon = SoulpitCreatureTag {
        is_summon: true,
        boss: None,
        ..overpower
    };
    assert_eq!(summon.experience(1000).map_err(debug)?, 0);
    assert_eq!(summon.outgoing_damage(101, 1).map_err(debug)?, (530, false));
    assert_eq!(
        carrier.soulpit_defeated(&owner, boss),
        Err(CarrierError::CommittedLethalUnavailable)
    );
    hit(&mut carrier, &owner, boss, "lethal", 100)?;
    assert_eq!(carrier.soulpit_defeated(&owner, boss).map_err(debug)?, tag);
    Ok(())
}

#[test]
fn native_summon_inherits_stack_once_and_parent_cleanup_replays_without_stale_children()
-> TestResult {
    let (mut carrier, owner, pin, policies) = fixture(10, true)?;
    let wave = carrier
        .spawn_soulpit_wave(
            &owner,
            &pin,
            &policies,
            request(4, BossAbility::Overpower),
            |_| true,
        )
        .map_err(debug)?;
    let parent = wave[6].ok_or("parent")?.actor;
    let index = carrier.validate_ref(&owner, parent.0).map_err(debug)?;
    let Slot::CreatureOccupied {
        position: Some(position),
        ..
    } = carrier.slots[index]
    else {
        return Err("position".into());
    };
    let context = position.context;
    let child = ExactActorRef(
        carrier
            .admit_creature(&owner, ActorState(0), "oteryn:creature.rat", 101)
            .map_err(debug)?,
    );
    carrier
        .initialize_position(
            &owner,
            child.0,
            context,
            LocalPosition {
                x: 32377,
                y: 31144,
                floor: 8,
            },
        )
        .map_err(debug)?;
    carrier
        .native_summons
        .insert(super::super::native_summon::NativeSummonLink {
            parent,
            child,
            target: b"oteryn:creature.rat".as_slice().into(),
            origin: NativeSummonOrigin::Behavior { index: 0 },
            death: None,
        })
        .map_err(debug)?;
    let tag = carrier
        .inherit_soulpit_summon(&owner, parent, child)
        .map_err(debug)?
        .ok_or("summon tag")?;
    assert!(tag.is_summon);
    assert_eq!(tag.stack, SoulpitStack::Boss);
    assert_eq!(tag.boss, None);
    assert_eq!(tag.experience(1000).map_err(debug)?, 0);
    assert_eq!(
        hit(&mut carrier, &owner, child, "hurt-child", 1)?.health_after,
        742
    );
    assert_eq!(
        carrier
            .inherit_soulpit_summon(&owner, parent, child)
            .map_err(debug)?,
        Some(tag)
    );
    assert_eq!(
        hit(&mut carrier, &owner, child, "hurt-child-again", 1)?.health_after,
        741,
        "retry does not refill scaled HP"
    );
    hit(&mut carrier, &owner, parent, "master-lethal", 9999)?;
    carrier
        .cascade_soulpit_summon_deaths(&owner, id(7))
        .map_err(debug)?;
    assert_eq!(carrier.soulpit_defeated(&owner, child).map_err(debug)?, tag);
    carrier
        .cascade_soulpit_summon_deaths(&owner, id(7))
        .map_err(debug)?;
    assert_eq!(carrier.retire_soulpit(&owner, id(7)).map_err(debug)?, 8);
    assert_eq!(carrier.retire_soulpit(&owner, id(7)).map_err(debug)?, 0);
    assert!(
        carrier
            .soulpit_actors(&owner, id(7))
            .map_err(debug)?
            .is_empty()
    );
    Ok(())
}

#[test]
fn intense_hex_uses_its_own_key_composes_attributes_and_expires_at_three_seconds() -> TestResult {
    let mut conditions: ConditionStore<ExactActorRef> = ConditionStore::default();
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let facts = ApplicationFacts {
        now: 1_000_000,
        base_speed: 100,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes(id(8)),
    };
    let attributes = ConditionDefinition::new(
        "fixture:attributes",
        1,
        ConditionValues::Attributes {
            duration_ms: 10_000,
            critical_chance_percent: 17,
            critical_extra_percentage_points: 25,
            damage_dealt_percent: 71,
            incoming_reduction_percent: 20,
        },
    )
    .ok_or("attributes")?;
    let hex = ConditionDefinition::new(
        "oteryn:condition.soulpit.intense_hex",
        201189,
        ConditionValues::IntenseHex {
            duration_ms: 3000,
            damage_dealt_percent: 50,
            healing_received_percent: 50,
        },
    )
    .ok_or("hex")?;
    conditions
        .apply(
            &attributes,
            None,
            ConditionSourceKind::Creature,
            &[],
            &facts,
        )
        .map_err(debug)?;
    conditions
        .apply(&hex, None, ConditionSourceKind::Creature, &[], &facts)
        .map_err(debug)?;
    assert_eq!(hex.condition_type(), ConditionType::IntenseHex);
    assert_eq!(conditions.healing_received_percent_at(3_999_999), 50);
    assert!(matches!(
        conditions.attributes_at(3_999_999),
        Some(ConditionValues::Attributes {
            damage_dealt_percent: 36,
            critical_chance_percent: 17,
            critical_extra_percentage_points: 25,
            incoming_reduction_percent: 20,
            ..
        })
    ));
    assert_eq!(conditions.healing_received_percent_at(4_000_000), 100);
    assert!(conditions.expire_non_ticking(4_000_000));
    assert!(matches!(
        conditions.attributes_at(4_000_000),
        Some(ConditionValues::Attributes {
            damage_dealt_percent: 71,
            ..
        })
    ));
    Ok(())
}

#[test]
fn oppressor_commits_real_instance_player_conditions_and_source_holes() -> TestResult {
    let (mut carrier, owner, pin, policies) = fixture(12, true)?;
    let wave = carrier
        .spawn_soulpit_wave(
            &owner,
            &pin,
            &policies,
            request(4, BossAbility::Oppressor),
            |_| true,
        )
        .map_err(debug)?;
    let boss = wave[6].ok_or("boss")?.actor;
    let boss_index = carrier.validate_ref(&owner, boss.0).map_err(debug)?;
    let Slot::CreatureOccupied {
        position: Some(position),
        ..
    } = carrier.slots[boss_index]
    else {
        return Err("position".into());
    };
    let context = position.context;
    let mut players = Vec::new();
    // Root's source holes (+2,+3),(-1,+4) must remain empty, not circular fallbacks.
    for (ordinal, (dx, dy)) in [(1, 0), (5, 0), (2, 3), (-1, 4)].into_iter().enumerate() {
        let session = GameSessionId::decode(&id(10 + ordinal as u8)).map_err(debug)?;
        let lease = CharacterLease::new(
            CharacterId::decode(&id(20 + ordinal as u8)).map_err(debug)?,
            1,
        )
        .map_err(debug)?;
        let actor = carrier
            .admit_bound_test_attacker(&owner, session, lease)
            .map_err(debug)?;
        carrier
            .initialize_position(
                &owner,
                actor.0,
                context,
                LocalPosition {
                    x: 32376 + dx,
                    y: 31144 + dy,
                    floor: 8,
                },
            )
            .map_err(debug)?;
        players.push((actor, session));
    }
    let cast = SoulpitOppressorCast {
        source: boss,
        current_target: Some(players[0].0),
        now_micros: 6_000_000,
        draws: [25, 30, 15],
        decision_root: [1; 32],
    };
    let effects = carrier
        .drive_soulpit_oppressor(&owner, cast, |_, _, _, _| true, |_, _| Ok(0))
        .map_err(debug)?;
    assert_eq!(effects.len(), 2);
    let close = carrier
        .scoped_actor_conditions(&owner, players[0].0, Some(players[0].1))
        .map_err(debug)?;
    assert!(
        close
            .get(ConditionType::Status(StatusKind::Feared).conflict_key())
            .is_some()
    );
    assert_eq!(close.healing_received_percent_at(6_000_000), 50);
    let far = carrier
        .scoped_actor_conditions(&owner, players[1].0, Some(players[1].1))
        .map_err(debug)?;
    assert!(
        far.get(ConditionType::Status(StatusKind::Rooted).conflict_key())
            .is_some()
    );
    assert!(
        carrier
            .drive_soulpit_oppressor(&owner, cast, |_, _, _, _| true, |_, _| Ok(0))
            .map_err(debug)?
            .is_empty()
    );
    for (actor, session) in &players {
        assert_eq!(
            carrier
                .scoped_actor_conditions(&owner, *actor, Some(*session))
                .map_err(debug)?
                .healing_received_percent_at(9_000_000),
            100
        );
    }

    let (retired, session) = players[0];
    carrier.remove(&owner, retired.0).map_err(debug)?;
    let replacement = carrier
        .admit_bound_test_attacker(
            &owner,
            session,
            CharacterLease::new(CharacterId::decode(&id(20)).map_err(debug)?, 1).map_err(debug)?,
        )
        .map_err(debug)?;
    assert_eq!(retired.0.actor_local_id, replacement.0.actor_local_id);
    assert_ne!(
        retired.0.actor_local_generation,
        replacement.0.actor_local_generation
    );
    carrier
        .initialize_position(
            &owner,
            replacement.0,
            context,
            LocalPosition {
                x: 32377,
                y: 31144,
                floor: 8,
            },
        )
        .map_err(debug)?;
    assert!(matches!(
        carrier.scoped_actor_conditions(&owner, retired, Some(session)),
        Err(CarrierError::StaleActorGeneration)
    ));
    let retry = SoulpitOppressorCast {
        now_micros: 8_000_000,
        ..cast
    };
    let before = carrier.clone();
    assert!(matches!(
        carrier.drive_soulpit_oppressor(&owner, retry, |_, _, _, _| true, |_, _| Ok(0)),
        Err(CarrierError::StaleActorGeneration)
    ));
    assert_eq!(
        carrier, before,
        "a stale target cannot spend a valid caster's cadence"
    );
    let current = SoulpitOppressorCast {
        current_target: Some(replacement),
        ..retry
    };
    carrier
        .drive_soulpit_oppressor(&owner, current, |_, _, _, _| true, |_, _| Ok(0))
        .map_err(debug)?;
    assert_eq!(
        carrier
            .scoped_actor_conditions(&owner, replacement, Some(session))
            .map_err(debug)?
            .healing_received_percent_at(8_000_000),
        50
    );
    Ok(())
}
