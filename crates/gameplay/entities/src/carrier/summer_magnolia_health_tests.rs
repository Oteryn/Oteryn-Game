//! Actual source-policy health component; no creature aliases or encounter flags.
use super::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use super::*;
use oteryn_gameplay_content::content::native_gameplay::{
    callback_source_native_test_policies, qualify_magnolia_second_life_test_source,
};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;
fn checked<T, E: std::fmt::Debug>(value: Result<T, E>) -> TestResult<T> {
    value.map_err(|e| format!("{e:?}").into())
}
fn id(tag: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}

#[test]
fn srv7_magnolia_actual_source_first_lethal_queues_second_life_on_same_actor() -> TestResult<()> {
    let world = checked(WorldId::decode(&id(1)))?;
    let mut runtime = checked(ChannelRuntimeV1::from_committed_scope_assignment(
        RuntimeScopeRefV1::Instance {
            world_id: world,
            instance_id: id(2),
        },
        checked(NodeId::decode(&id(3)))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    ))?;
    let creatures = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/creatures/definitions/spell-native-profiles.json"
    ))?;
    let presentations = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/presentations/definitions/spell-native-profiles.json"
    ))?;
    let policies = checked(callback_source_native_test_policies(
        &creatures,
        &presentations,
        [1; 32],
    ))?;
    let policy = policies
        .policies()
        .iter()
        .find(|p| p.definition_key == "oteryn:creature.the_moonsnow_magnolia")
        .ok_or("actual source Magnolia policy")?;
    assert_eq!(policy.maximum_health, 52_000);
    let foreign_policies = checked(callback_source_native_test_policies(
        &creatures,
        &presentations,
        [9; 32],
    ))?;
    assert!(
        runtime
            .install_companion_policies(foreign_policies)
            .is_err()
    );
    checked(runtime.install_companion_policies(policies))?;
    let session = checked(GameSessionId::decode(&id(4)))?;
    let reservation = checked(runtime.reserve_fresh_session(session))?;
    let player = checked(runtime.commit_fresh_session(reservation))?;
    checked(runtime.initialize_movement_test_position(
        player,
        MovementLocalPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    ))?;
    let actor = ExactActorRef(checked(runtime.carrier.admit_creature(
        &runtime.continuity,
        ActorState(0),
        "oteryn:creature.the_moonsnow_magnolia",
        52_000,
    ))?);
    let position_context = runtime.pinned_position_context();
    checked(runtime.carrier.initialize_position(
        &runtime.continuity,
        actor.0,
        position_context,
        LocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        },
    ))?;
    checked(runtime.install_creature_policy(actor, "oteryn:creature.the_moonsnow_magnolia"))?;
    for field in ["health", "experience", "body", "attack"] {
        let mut changed: serde_json::Value = serde_json::from_slice(&creatures)?;
        let rows = changed
            .get_mut("records")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("actual native records")?;
        let record = rows
            .iter_mut()
            .find(|record| {
                record["profile"]["target"]["key"] == "oteryn:creature.the_moonsnow_magnolia"
            })
            .ok_or("actual Magnolia record")?;
        let (pointer, value) = match field {
            "health" => ("/profile/data/profile/health", serde_json::json!(52_001)),
            "experience" => (
                "/profile/data/profile/experience",
                serde_json::json!(240_001),
            ),
            "body" => (
                "/profile/data/profile/details/corpse_item/key",
                serde_json::json!("oteryn:item.tibia.i3031"),
            ),
            _ => (
                "/behavior/data/profile/attacks/0/chance_ppm",
                serde_json::json!(999_999),
            ),
        };
        *record.pointer_mut(pointer).ok_or("actual source field")? = value;
        // Keep the existing Creature/Behavior closure internally valid. The
        // source seal must reject field drift after the native decoder accepts it.
        let mirrors = [record["profile"].clone(), record["behavior"].clone()];
        let bound_profiles = record
            .pointer_mut("/attack_definitions/profiles")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("actual attack definition closure")?;
        for mirror in mirrors {
            let bound = bound_profiles
                .iter_mut()
                .find(|profile| profile["target"] == mirror["target"])
                .ok_or("actual bound Creature/Behavior profile")?;
            *bound = mirror;
        }
        let changed_bytes = serde_json::to_vec(&changed)?;
        checked(callback_source_native_test_policies(
            &changed_bytes,
            &presentations,
            [1; 32],
        ))?;
        let qualified =
            qualify_magnolia_second_life_test_source(&changed_bytes, &presentations, [1; 32]);
        assert!(
            qualified.is_none(),
            "actual source {field} drift cannot qualify an override"
        );
    }
    let phase = qualify_magnolia_second_life_test_source(&creatures, &presentations, [1; 32])
        .ok_or("actual exact Magnolia source health qualification")?;
    let rat_policy = checked(runtime.companion_policy("oteryn:creature.rat"))?;
    let rat = ExactActorRef(checked(runtime.carrier.admit_creature(
        &runtime.continuity,
        ActorState(0),
        &rat_policy.definition_key,
        rat_policy.maximum_health,
    ))?);
    checked(runtime.carrier.initialize_position(
        &runtime.continuity,
        rat.0,
        position_context,
        LocalPosition {
            x: 2,
            y: 0,
            floor: 0,
        },
    ))?;
    checked(runtime.install_creature_policy(rat, "oteryn:creature.rat"))?;
    assert!(
        runtime.bind_magnolia_second_life(rat, &phase).is_err(),
        "another actual native Creature cannot acquire Magnolia maximum health"
    );
    assert_eq!(
        checked(runtime.companion_snapshot(rat))?.maximum_health,
        rat_policy.maximum_health
    );

    let command = checked(oteryn_gameplay_foundation::foundation::CommandId::new(1))?;
    let batch = OwnerCombatBatch {
        caster: player,
        attacker: checked(CharacterId::decode(&id(5)))?,
        current_lease_generation: 1,
        command: CommandRef::new(session, command),
        occurrence: SpellOccurrenceBinding {
            id: "spell-batch:qualified".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"actual-source-magnolia-health-component".to_vec(),
        anchor: None,
        now_us: None,
        now_ms: 100,
        effects: vec![OwnerCombatEffect {
            target: actor,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: checked(runtime.creature_spell_target_atom(actor))?,
                magnitude: 52_000,
            },
        }],
        deferred: None,
    };
    let staged = checked(runtime.stage_spell_batch(&batch))?;
    let result = checked(runtime.commit_spell_batch(staged))?;
    assert!(result.applied);
    let hit = result
        .effects
        .first()
        .and_then(|effect| effect.health)
        .ok_or("actual retained Magnolia root-hit health receipt")?;
    assert_eq!(
        (hit.health_before, hit.health_after),
        (52_000, 0),
        "ordinary drain consumers must retain the full first-life removed HP"
    );
    let after = checked(runtime.creature_combat_facts(actor, 100))?;
    assert_eq!(
        (after.health, after.maximum_health),
        (60_000, 60_000),
        "accepted RT0 first lethal prevents death then drains queued full heal"
    );
    assert_eq!(
        checked(runtime.companion_snapshot(actor))?.actor,
        actor,
        "one public creature identity survives the first life"
    );
    assert!(
        matches!(
            runtime
                .carrier
                .committed_lethal_receipt_inner(&runtime.continuity, actor.0),
            Err(CarrierError::CommittedLethalUnavailable)
        ),
        "first-life prevention has no publishable death"
    );
    let shares = checked(runtime.borrow_combat_death().reward_damage_shares(actor))?;
    assert_eq!(shares.iter().map(|share| share.total).sum::<u64>(), 52_000);
    checked(runtime.install_creature_policy(actor, "oteryn:creature.the_moonsnow_magnolia"))?;
    assert!(
        runtime
            .install_creature_policy(actor, "oteryn:creature.rakesh_moonfang")
            .is_err()
    );
    assert_eq!(
        checked(runtime.read_source_creature_health(
            actor,
            "oteryn:creature.the_moonsnow_magnolia",
            52_000
        ))?,
        60_000
    );
    let heals = checked(runtime.commit_source_creature_heal_batch(
        [1; 32],
        &[(
            actor,
            "oteryn:creature.the_moonsnow_magnolia".to_owned(),
            52_000,
            100,
        )],
    ))?;
    assert_eq!(
        heals.first().ok_or("actual heal result")?.health_after,
        60_000,
        "the native heal owner respects the sealed current maximum and never exceeds it"
    );
    let replay = checked(runtime.stage_spell_batch(&batch))?;
    assert!(!checked(runtime.commit_spell_batch(replay))?.applied);
    let replayed = checked(runtime.creature_combat_facts(actor, 100))?;
    assert_eq!((replayed.health, replayed.maximum_health), (60_000, 60_000));
    let mut final_batch = batch.clone();
    final_batch.command = CommandRef::new(
        session,
        checked(oteryn_gameplay_foundation::foundation::CommandId::new(2))?,
    );
    final_batch.binding = b"actual-source-magnolia-final-health-root".to_vec();
    let OwnerCombatChange::Damage { magnitude, .. } = &mut final_batch
        .effects
        .first_mut()
        .ok_or("actual final root effect")?
        .change
    else {
        return Err("damage effect".into());
    };
    *magnitude = 60_000;
    let staged = checked(runtime.stage_spell_batch(&final_batch))?;
    assert!(checked(runtime.commit_spell_batch(staged))?.applied);
    let final_health = checked(runtime.creature_combat_facts(actor, 100))?;
    assert_eq!(
        (final_health.health, final_health.maximum_health),
        (0, 60_000)
    );
    let final_death = checked(
        runtime
            .carrier
            .committed_lethal_receipt_inner(&runtime.continuity, actor.0),
    )?;
    assert_eq!(final_death.projection.occurrence.damage, 60_000);
    assert_eq!(final_death.projection.occurrence.health_before, 60_000);
    let replay = checked(runtime.stage_spell_batch(&batch))?;
    assert!(!checked(runtime.commit_spell_batch(replay))?.applied);
    assert_eq!(
        checked(runtime.creature_combat_facts(actor, 100))?.health,
        0,
        "old first-life replay cannot resurrect the actor or replace its final lethal"
    );
    let retained = checked(
        runtime
            .carrier
            .committed_lethal_receipt_inner(&runtime.continuity, actor.0),
    )?;
    assert_eq!(retained.projection.occurrence.damage, 60_000);
    assert_eq!(retained.projection.occurrence.health_before, 60_000);
    let shares = checked(runtime.borrow_combat_death().reward_damage_shares(actor))?;
    assert_eq!(shares.iter().map(|share| share.total).sum::<u64>(), 112_000);
    Ok(())
}

#[test]
fn srv7_magnolia_actual_unattributed_health_root_has_one_second_life() -> TestResult<()> {
    let world = checked(WorldId::decode(&id(1)))?;
    let mut runtime = checked(ChannelRuntimeV1::from_committed_scope_assignment(
        RuntimeScopeRefV1::Instance {
            world_id: world,
            instance_id: id(2),
        },
        checked(NodeId::decode(&id(3)))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        ChannelContentPin::test(world),
    ))?;
    let creatures = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/creatures/definitions/spell-native-profiles.json"
    ))?;
    let presentations = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/presentations/definitions/spell-native-profiles.json"
    ))?;
    let policies = checked(callback_source_native_test_policies(
        &creatures,
        &presentations,
        [1; 32],
    ))?;
    checked(runtime.install_companion_policies(policies))?;
    let actor = ExactActorRef(checked(runtime.carrier.admit_creature(
        &runtime.continuity,
        ActorState(0),
        "oteryn:creature.the_moonsnow_magnolia",
        52_000,
    ))?);
    let context = runtime.pinned_position_context();
    checked(runtime.carrier.initialize_position(
        &runtime.continuity,
        actor.0,
        context,
        LocalPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    ))?;
    checked(runtime.install_creature_policy(actor, "oteryn:creature.the_moonsnow_magnolia"))?;
    let target = checked(runtime.creature_spell_target_atom(actor))?;
    let pin = runtime.content_pin().clone();
    let first = checked(runtime.carrier.commit_creature_damage_with_pin_inner(
        &runtime.continuity,
        actor.0,
        OwnerDamageCommand {
            target: target.as_bytes(),
            occurrence: b"actual-unattributed-magnolia-first",
            binding: b"actual-unattributed-magnolia-first\0v1",
            damage: 52_000,
        },
        None,
        false,
        Some(&pin),
        None,
        Some(100),
    ))?;
    assert_eq!((first.health_before, first.health_after), (52_000, 0));
    let snapshot = checked(runtime.creature_combat_facts(actor, 100))?;
    assert_eq!((snapshot.health, snapshot.maximum_health), (60_000, 60_000));
    assert!(
        checked(runtime.borrow_combat_death().reward_damage_shares(actor))?.is_empty(),
        "no reward principal is invented for an unattributed root"
    );
    let final_hit = checked(runtime.carrier.commit_creature_damage_with_pin_inner(
        &runtime.continuity,
        actor.0,
        OwnerDamageCommand {
            target: target.as_bytes(),
            occurrence: b"actual-unattributed-magnolia-final",
            binding: b"actual-unattributed-magnolia-final\0v1",
            damage: 60_000,
        },
        None,
        false,
        Some(&pin),
        None,
        Some(200),
    ))?;
    assert_eq!(
        (final_hit.health_before, final_hit.health_after),
        (60_000, 0)
    );
    let replay = checked(runtime.carrier.commit_creature_damage_with_pin_inner(
        &runtime.continuity,
        actor.0,
        OwnerDamageCommand {
            target: target.as_bytes(),
            occurrence: b"actual-unattributed-magnolia-first",
            binding: b"actual-unattributed-magnolia-first\0v1",
            damage: 52_000,
        },
        None,
        false,
        Some(&pin),
        None,
        Some(300),
    ))?;
    assert!(!replay.applied);
    assert_eq!(
        checked(runtime.creature_combat_facts(actor, 300))?.health,
        0
    );
    let death = checked(
        runtime
            .carrier
            .committed_lethal_receipt_inner(&runtime.continuity, actor.0),
    )?;
    assert_eq!(
        (
            death.projection.occurrence.damage,
            death.projection.occurrence.health_before
        ),
        (60_000, 60_000)
    );
    Ok(())
}
