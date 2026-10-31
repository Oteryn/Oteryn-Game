use super::*;
use crate::carrier::runtime_actor_companion::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
};

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
fn at(x: i32, y: i32, floor: i16) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor }
}

fn fixture(capacity: usize) -> TestResult<(ChannelRuntimeV1, ExactActorRef, GameSessionId)> {
    fixture_with_forge_difficulty(capacity, None)
}

fn fixture_with_forge_difficulty(
    capacity: usize,
    forge_difficulty: Option<u8>,
) -> TestResult<(ChannelRuntimeV1, ExactActorRef, GameSessionId)> {
    let mut policy = fixture_policy("oteryn:creature.rat", "Rat", 101);
    policy.forge_difficulty = forge_difficulty;
    fixture_with_policies(capacity, vec![policy])
}

fn fixture_policy(key: &str, name: &str, maximum_health: i64) -> CompiledCreaturePolicy {
    CompiledCreaturePolicy {
        definition_key: key.into(),
        definition_revision: "fixture:1".into(),
        display_name: name.into(),
        maximum_health,
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
    }
}

fn fixture_with_policies(
    capacity: usize,
    policies: Vec<CompiledCreaturePolicy>,
) -> TestResult<(ChannelRuntimeV1, ExactActorRef, GameSessionId)> {
    let world = WorldId::decode(&id(1)).map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        [1; 32],
        [2; 32],
        [3; 32],
        [4; 32],
        (100, 100, 7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).map_err(debug)?,
        NodeId::decode(&id(3)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        capacity,
        pin,
    )
    .map_err(debug)?;
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact([1; 32], policies).map_err(debug)?,
        )
        .map_err(debug)?;
    let session = GameSessionId::decode(&id(4)).map_err(debug)?;
    let reserve = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reserve).map_err(debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(CharacterId::decode(&id(5)).map_err(debug)?, 1).map_err(debug)?,
        )
        .map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    Ok((runtime, actor, session))
}

fn ordinary(
    runtime: &mut ChannelRuntimeV1,
    position: MovementLocalPosition,
) -> TestResult<ExactActorRef> {
    ordinary_species(runtime, "oteryn:creature.rat", position)
}

fn ordinary_species(
    runtime: &mut ChannelRuntimeV1,
    species: &str,
    position: MovementLocalPosition,
) -> TestResult<ExactActorRef> {
    let maximum = runtime
        .companion_policy(species)
        .map_err(debug)?
        .maximum_health;
    let actor = runtime
        .admit_encounter_actor(species, maximum, position)
        .map_err(debug)?;
    runtime
        .install_creature_policy(actor, species)
        .map_err(debug)?;
    Ok(actor)
}

fn death(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    now_ms: u64,
) -> TestResult<EchoDeathInput> {
    let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
    let health = snapshot.health;
    runtime
        .carrier
        .current_owner_exact_commit(&runtime.continuity)
        .commit_damage(
            actor,
            OwnerDamageCommand {
                target: snapshot.state.policy.definition_key.as_bytes(),
                occurrence: b"echo-test-kill",
                binding: b"echo-test-kill\0echo-test-binding",
                damage: health,
            },
        )
        .map_err(debug)?;
    let mut owner = runtime.borrow_combat_death();
    let receipt = owner.committed_lethal_receipt(actor).map_err(debug)?;
    owner.project_committed_lethal(receipt).map_err(debug)?;
    let (death, _) = owner.projected_death(actor).map_err(debug)?;
    let (occurrence, _) = owner.reward_occurrence(actor, id(5)).map_err(debug)?;
    let bestiary_stars = oteryn_gameplay_domain::domain::soulpit::source_monster(
        &snapshot.state.policy.display_name,
    )
    .map(|monster| monster.stars);
    Ok(EchoDeathInput {
        death,
        occurrence,
        bestiary_stars,
        has_boss_race: false,
        draw: 100,
        now_ms,
    })
}

fn activate(
    runtime: &mut ChannelRuntimeV1,
    player: ExactActorRef,
    session: GameSessionId,
    now: u64,
) -> TestResult {
    let target = ordinary(runtime, at(100, 100, 7))?;
    let input = death(runtime, target, now)?;
    assert_eq!(
        runtime.record_echo_death(input).map_err(debug)?,
        EchoDeathEffect::SchedulePortal
    );
    runtime
        .drive_echo_wardens(now + 30_000, |_| true)
        .map_err(debug)?;
    assert!(
        runtime
            .activate_echo_portal(player, session, now + 30_000, 7)
            .map_err(debug)?
    );
    runtime
        .drive_echo_wardens(now + 30_000, |_| true)
        .map_err(debug)?;
    Ok(())
}

#[test]
fn real_death_creates_one_delayed_portal_and_ttl_starts_on_actual_creation() -> TestResult {
    let (mut runtime, _, _) = fixture(8)?;
    let target = ordinary(&mut runtime, at(100, 100, 7))?;
    let input = death(&mut runtime, target, 500)?;
    assert_eq!(
        runtime.record_echo_death(input).map_err(debug)?,
        EchoDeathEffect::SchedulePortal
    );
    assert_eq!(
        runtime.record_echo_death(input).map_err(debug)?,
        EchoDeathEffect::None
    );
    assert_eq!(runtime.next_echo_due_ms(), Some(30_500));
    assert!(
        !runtime
            .drive_echo_wardens(30_499, |_| true)
            .map_err(debug)?
    );
    assert_eq!(runtime.echo_portals(30_499).count(), 0);
    assert!(
        runtime
            .drive_echo_wardens(31_000, |_| true)
            .map_err(debug)?
    );
    assert_eq!(runtime.echo_portals(150_999).count(), 1);
    assert_eq!(runtime.next_echo_due_ms(), Some(151_000));
    assert!(
        runtime
            .drive_echo_wardens(151_000, |_| true)
            .map_err(debug)?
    );
    assert_eq!(runtime.echo_portals(151_000).count(), 0);
    assert_eq!(runtime.next_echo_due_ms(), None);
    let second = ordinary(&mut runtime, at(100, 100, 7))?;
    let input = death(&mut runtime, second, 200_000)?;
    runtime.record_echo_death(input).map_err(debug)?;
    assert!(
        !runtime
            .drive_echo_wardens(230_000, |_| false)
            .map_err(debug)?
    );
    assert_eq!(
        runtime.next_echo_due_ms(),
        None,
        "missing current ground consumes the delayed callback"
    );
    Ok(())
}

#[test]
fn committed_step_starts_one_warden_and_seven_staggered_minions_with_source_stats() -> TestResult {
    let (mut runtime, player, session) = fixture(20)?;
    let target = ordinary(&mut runtime, at(100, 100, 7))?;
    let input = death(&mut runtime, target, 0)?;
    runtime.record_echo_death(input).map_err(debug)?;
    runtime
        .drive_echo_wardens(30_000, |_| true)
        .map_err(debug)?;
    assert!(
        runtime
            .activate_echo_portal(player, session, 30_000, 6)
            .is_err()
    );
    assert_eq!(runtime.echo_portals(30_000).count(), 1);
    assert!(
        runtime
            .activate_echo_portal(player, session, 30_000, 7)
            .map_err(debug)?
    );
    assert!(
        !runtime
            .activate_echo_portal(player, session, 30_000, 7)
            .map_err(debug)?
    );
    runtime
        .drive_echo_wardens(30_000, |_| true)
        .map_err(debug)?;
    assert_eq!(
        runtime.echo_wardens.spawned.len(),
        2,
        "warden and first add are immediate"
    );
    runtime
        .drive_echo_wardens(30_399, |_| true)
        .map_err(debug)?;
    assert_eq!(runtime.echo_wardens.spawned.len(), 2);
    for step in 1..=6 {
        runtime
            .drive_echo_wardens(30_000 + step * 400, |_| true)
            .map_err(debug)?;
    }
    assert_eq!(runtime.echo_wardens.spawned.len(), 8);
    let mut wardens = 0;
    for actor in runtime.echo_wardens.spawned.clone() {
        let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
        assert_eq!(snapshot.position, at(100, 100, 7));
        match runtime.echo_variant(actor) {
            Some(EchoVariant::Warden) => {
                wardens += 1;
                assert_eq!(
                    (
                        snapshot.health,
                        snapshot.maximum_health,
                        runtime.echo_attack_percent(actor)
                    ),
                    (404, 404, 150)
                );
            }
            Some(EchoVariant::Minion) => {
                assert_eq!(
                    (
                        snapshot.health,
                        snapshot.maximum_health,
                        runtime.echo_attack_percent(actor)
                    ),
                    (151, 151, 100)
                );
            }
            None => return Err("untagged raid actor".into()),
        }
    }
    assert_eq!(wardens, 1);
    Ok(())
}

#[test]
fn aura_is_square_same_floor_permanent_and_does_not_reheal_or_restack() -> TestResult {
    let (mut runtime, player, session) = fixture(24)?;
    let inside = ordinary(&mut runtime, at(105, 105, 7))?;
    let outside = ordinary(&mut runtime, at(106, 100, 7))?;
    let below = ordinary(&mut runtime, at(100, 100, 8))?;
    activate(&mut runtime, player, session, 0)?;
    assert_eq!(runtime.echo_variant(inside), None);
    runtime
        .drive_echo_wardens(32_000, |_| true)
        .map_err(debug)?;
    assert_eq!(runtime.echo_variant(inside), Some(EchoVariant::Minion));
    assert_eq!(runtime.echo_variant(outside), None);
    assert_eq!(runtime.echo_variant(below), None);
    runtime
        .set_test_creature_health(inside, 50)
        .map_err(debug)?;
    runtime
        .drive_echo_wardens(34_000, |_| true)
        .map_err(debug)?;
    let snapshot = runtime.companion_snapshot(inside).map_err(debug)?;
    assert_eq!((snapshot.health, snapshot.maximum_health), (50, 151));
    let warden = runtime.echo_wardens.wardens.first().ok_or("warden")?.0;
    let input = death(&mut runtime, warden, 34_000)?;
    assert_eq!(
        runtime.record_echo_death(input).map_err(debug)?,
        EchoDeathEffect::FirstSpeciesReward
    );
    runtime
        .drive_echo_wardens(36_000, |_| true)
        .map_err(debug)?;
    assert_eq!(runtime.echo_variant(inside), Some(EchoVariant::Minion));
    let input = death(&mut runtime, inside, 36_000)?;
    assert_eq!(
        runtime.record_echo_death(input).map_err(debug)?,
        EchoDeathEffect::None
    );
    assert_eq!(runtime.echo_portals(66_000).count(), 0);
    Ok(())
}

#[test]
fn healing_uses_scaled_maximum_and_dead_raid_slots_survive_only_pending_settlement() -> TestResult {
    let (mut runtime, player, session) = fixture(20)?;
    activate(&mut runtime, player, session, 0)?;
    let warden = runtime.echo_wardens.wardens.first().ok_or("warden")?.0;
    runtime
        .set_test_creature_health(warden, 350)
        .map_err(debug)?;
    let healed = runtime
        .commit_source_creature_heal_batch(
            [1; 32],
            &[(warden, "oteryn:creature.rat".into(), 101, 100)],
        )
        .map_err(debug)?;
    assert_eq!(healed.first().ok_or("heal")?.health_after, 404);
    death(&mut runtime, warden, 31_000)?;
    runtime.pin_corpse_projection(warden).map_err(debug)?;
    runtime
        .drive_echo_wardens(32_000, |_| true)
        .map_err(debug)?;
    assert!(runtime.companion_snapshot_including_dead(warden).is_ok());
    runtime.unpin_corpse_projection(warden);
    runtime
        .drive_echo_wardens(32_400, |_| true)
        .map_err(debug)?;
    assert!(runtime.companion_snapshot_including_dead(warden).is_err());
    assert!(runtime.echo_variant(warden).is_none());
    Ok(())
}

#[test]
fn retired_generation_cannot_read_replacement_echo_soulpit_or_hex_state() -> TestResult {
    let (mut runtime, _, _) = fixture(2)?;
    let retired = ordinary(&mut runtime, at(100, 100, 7))?;
    runtime
        .apply_echo_variant(retired, EchoVariant::Minion)
        .map_err(debug)?;
    runtime.remove_encounter_actor(retired).map_err(debug)?;
    assert_eq!(runtime.echo_variant(retired), None);
    assert_eq!(
        runtime.carrier.soulpit_tag(&runtime.continuity, retired),
        Err(CarrierError::StaleActorGeneration)
    );

    let replacement = ordinary(&mut runtime, at(100, 100, 7))?;
    assert_eq!(retired.0.actor_local_id, replacement.0.actor_local_id);
    assert_ne!(
        retired.0.actor_local_generation,
        replacement.0.actor_local_generation
    );
    runtime
        .apply_echo_variant(replacement, EchoVariant::Warden)
        .map_err(debug)?;
    assert_eq!(runtime.echo_variant(replacement), Some(EchoVariant::Warden));
    assert_eq!(runtime.echo_attack_percent(replacement), 150);
    assert_eq!(runtime.echo_variant(retired), None);
    assert_eq!(runtime.echo_attack_percent(retired), 100);
    assert_eq!(
        runtime.carrier.soulpit_tag(&runtime.continuity, retired),
        Err(CarrierError::StaleActorGeneration)
    );
    assert_eq!(
        runtime
            .carrier
            .soulpit_tag(&runtime.continuity, replacement)
            .map_err(debug)?,
        None
    );
    assert!(matches!(
        runtime
            .carrier
            .scoped_actor_conditions(&runtime.continuity, retired, None),
        Err(CarrierError::StaleActorGeneration)
    ));
    assert_eq!(
        runtime.soulpit_hex_delta(retired, 0),
        Err(CarrierError::StaleActorGeneration)
    );
    assert!(
        runtime
            .carrier
            .scoped_actor_conditions(&runtime.continuity, replacement, None)
            .is_ok()
    );
    assert!(runtime.soulpit_hex_delta(replacement, 0).is_ok());
    Ok(())
}

#[test]
fn natural_minion_and_forge_keep_source_order_rounding_and_fill_health_once() -> TestResult {
    use oteryn_gameplay_domain::domain::forge_creature::ForgeCreature;

    for echo_first in [true, false] {
        let (mut runtime, player, session) = fixture_with_forge_difficulty(24, Some(4))?;
        runtime
            .begin_activation_spawn_sources(vec![(
                vec![(
                    (105, 105, 7),
                    60_000,
                    "oteryn:creature.rat".into(),
                    101,
                    false,
                )],
                5_000,
            )])
            .map_err(debug)?;
        runtime.realize_activation_spawn_window(0).map_err(debug)?;
        let natural = runtime
            .spawn_point_creatures()
            .into_iter()
            .find_map(|(_, _, actor)| actor)
            .ok_or("natural spawn")?;
        activate(&mut runtime, player, session, 0)?;
        if !echo_first {
            runtime.drive_forge_population(30_000_000).map_err(debug)?;
            let forged = runtime.companion_snapshot(natural).map_err(debug)?;
            assert_eq!((forged.health, forged.maximum_health), (364, 364));
        }
        runtime
            .drive_echo_wardens(32_000, |_| true)
            .map_err(debug)?;
        assert_eq!(runtime.echo_variant(natural), Some(EchoVariant::Minion));
        if echo_first {
            let minion = runtime.companion_snapshot(natural).map_err(debug)?;
            assert_eq!((minion.health, minion.maximum_health), (151, 151));
            runtime
                .set_test_creature_health(natural, 50)
                .map_err(debug)?;
            runtime.drive_forge_population(32_000_000).map_err(debug)?;
        }
        let combined = runtime.companion_snapshot(natural).map_err(debug)?;
        assert_eq!(combined.forge, Some(ForgeCreature::Fiendish));
        let expected = if echo_first { 544 } else { 546 };
        assert_eq!(
            (combined.health, combined.maximum_health),
            (expected, expected)
        );
        runtime
            .set_test_creature_health(natural, 50)
            .map_err(debug)?;
        runtime
            .drive_echo_wardens(34_000, |_| true)
            .map_err(debug)?;
        runtime.drive_forge_population(42_000_000).map_err(debug)?;
        let retained = runtime.companion_snapshot(natural).map_err(debug)?;
        assert_eq!((retained.health, retained.maximum_health), (50, expected));
    }
    Ok(())
}

#[test]
fn renamed_source_deaths_spawn_and_reward_base_species_and_enlist_same_name_variants() -> TestResult
{
    for (base_key, base_name, variants) in [
        (
            "oteryn:creature.nomad",
            "Nomad",
            [
                ("oteryn:creature.nomad_blue", "Nomad Blue"),
                ("oteryn:creature.nomad_female", "Nomad Female"),
            ],
        ),
        (
            "oteryn:creature.horse",
            "Horse",
            [
                ("oteryn:creature.brown_horse", "Brown Horse"),
                ("oteryn:creature.grey_horse", "Grey Horse"),
            ],
        ),
    ] {
        for slain_ordinal in 0..2 {
            let (mut runtime, player, session) = fixture_with_policies(
                24,
                vec![
                    fixture_policy(base_key, base_name, 101),
                    fixture_policy(variants[0].0, variants[0].1, 201),
                    fixture_policy(variants[1].0, variants[1].1, 301),
                ],
            )?;
            let slain = ordinary_species(&mut runtime, variants[slain_ordinal].0, at(100, 100, 7))?;
            let other =
                ordinary_species(&mut runtime, variants[1 - slain_ordinal].0, at(105, 105, 7))?;
            let base = ordinary_species(&mut runtime, base_key, at(105, 100, 7))?;
            let input = death(&mut runtime, slain, 0)?;
            assert_eq!(
                runtime.record_echo_death(input).map_err(debug)?,
                EchoDeathEffect::SchedulePortal
            );
            runtime
                .drive_echo_wardens(30_000, |_| true)
                .map_err(debug)?;
            assert_eq!(
                runtime
                    .echo_portals(30_000)
                    .next()
                    .ok_or("portal")?
                    .creature,
                base_key
            );
            assert!(
                runtime
                    .activate_echo_portal(player, session, 30_000, 7)
                    .map_err(debug)?
            );
            runtime
                .drive_echo_wardens(30_000, |_| true)
                .map_err(debug)?;
            let warden = runtime.echo_wardens.wardens.first().ok_or("warden")?.0;
            let spawned = runtime.companion_snapshot(warden).map_err(debug)?;
            assert_eq!(spawned.state.policy.definition_key, base_key);
            assert_eq!(
                spawned.maximum_health, 404,
                "the donor resolves the base type, not the slain variant's health"
            );
            runtime
                .drive_echo_wardens(32_000, |_| true)
                .map_err(debug)?;
            assert_eq!(runtime.echo_variant(other), Some(EchoVariant::Minion));
            assert_eq!(runtime.echo_variant(base), Some(EchoVariant::Minion));
            let input = death(&mut runtime, warden, 32_000)?;
            assert_eq!(
                runtime.record_echo_death(input).map_err(debug)?,
                EchoDeathEffect::FirstSpeciesReward
            );
            assert_eq!(
                runtime
                    .companion_snapshot_including_dead(warden)
                    .map_err(debug)?
                    .state
                    .policy
                    .definition_key,
                base_key,
                "the actual death/Charm settlement reads one canonical base species for all variants"
            );
        }
    }
    Ok(())
}

#[test]
fn renamed_source_portal_refuses_missing_or_ambiguous_native_base_without_guessing() -> TestResult {
    for ambiguous in [false, true] {
        let mut policies = vec![fixture_policy(
            "oteryn:creature.nomad_blue",
            "Nomad Blue",
            201,
        )];
        if ambiguous {
            policies.push(fixture_policy("oteryn:creature.nomad", "Nomad", 101));
            policies.push(fixture_policy(
                "oteryn:creature.nomad_duplicate",
                "Nomad",
                111,
            ));
        }
        let (mut runtime, _, _) = fixture_with_policies(8, policies)?;
        let slain = ordinary_species(&mut runtime, "oteryn:creature.nomad_blue", at(100, 100, 7))?;
        let input = death(&mut runtime, slain, 0)?;
        assert!(matches!(
            runtime.record_echo_death(input),
            Err(CarrierError::InvalidCreatureTarget)
        ));
        assert_eq!(runtime.next_echo_due_ms(), None);
        assert!(runtime.echo_wardens.spawned.is_empty());
    }
    Ok(())
}
