//! Actual source-policy health component; no creature aliases or encounter flags.
use super::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use super::*;

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
    let policies = checked(
        oteryn_gameplay_content::content::native_gameplay::callback_source_native_test_policies(
            &creatures,
            &presentations,
            [1; 32],
        ),
    )?;
    let policy = policies
        .policies()
        .iter()
        .find(|p| p.definition_key == "oteryn:creature.the_moonsnow_magnolia")
        .ok_or("actual source Magnolia policy")?;
    assert_eq!(policy.maximum_health, 52_000);
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
    let replay = checked(runtime.stage_spell_batch(&batch))?;
    assert!(!checked(runtime.commit_spell_batch(replay))?.applied);
    let replayed = checked(runtime.creature_combat_facts(actor, 100))?;
    assert_eq!((replayed.health, replayed.maximum_health), (60_000, 60_000));
    Ok(())
}
