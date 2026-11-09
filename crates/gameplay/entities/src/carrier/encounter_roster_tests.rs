//! M30: exact actor identities leave the native entry owner only with the whole commit.
use super::*;
use crate::{CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn checked<T, E: std::fmt::Debug>(value: Result<T, E>) -> TestResult<T> {
    value.map_err(|error| format!("{error:?}").into())
}

fn id(n: u8) -> [u8; 16] {
    [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
}

fn runtime(generation: u64) -> TestResult<ChannelRuntimeV1> {
    let world = checked(WorldId::decode(&id(1)))?;
    let mut runtime = checked(ChannelRuntimeV1::from_committed_assignment(
        world,
        checked(ChannelId::decode(&id(2)))?,
        checked(NodeId::decode(&id(3)))?,
        1,
        generation,
        generation,
        &format!("runtime-scope-assignment:{generation}"),
        8,
        ChannelContentPin::test(world),
    ))?;
    let policy = CompiledCreaturePolicy {
        definition_key: "creature:rat".into(),
        definition_revision: "test-1".into(),
        display_name: "rat".into(),
        maximum_health: 20,
        forge_difficulty: None,
        base_speed: 220,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: Vec::new(),
        preferred_distance: Some(1),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
        defense: Some(1),
        armor: Some(0),
        mitigation: None,
        resistances: Vec::new(),
        damage_immunities: Vec::new(),
        healing_from_damage: Vec::new(),
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    };
    checked(runtime.install_companion_policies(checked(
        CompiledCreaturePolicies::from_active_artifact(
            runtime.content_pin().server_artifact_digest(),
            vec![policy],
        ),
    )?))?;
    Ok(runtime)
}

#[derive(Clone, Copy)]
struct Step {
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
}
impl super::super::runtime_actor_source_step::source_step_seal::Sealed for Step {}
impl SourceStepCommitProof for Step {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        runtime.owner_fence()?;
        if runtime.read_actor_position(self.actor)? != self.expected {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        Ok(())
    }
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    ) {
        (
            self.actor,
            self.expected,
            self.destination,
            MovementFacing::East,
        )
    }
}
fn position(x: i32) -> MovementLocalPosition {
    MovementLocalPosition {
        x,
        y: 100,
        floor: 7,
    }
}
fn player(runtime: &mut ChannelRuntimeV1) -> TestResult<Step> {
    let session = checked(GameSessionId::decode(&id(5)))?;
    let reservation = checked(runtime.reserve_fresh_session(session))?;
    let actor = checked(runtime.commit_fresh_session(reservation))?;
    checked(runtime.initialize_pinned_test_position(actor, position(100)))?;
    Ok(Step {
        actor,
        expected: checked(runtime.read_actor_position(actor))?,
        destination: position(110),
    })
}
#[test]
fn engenc_encounter_roster_returns_exact_policy_bound_defenders_and_rejects_stale_replay()
-> TestResult {
    let mut runtime = runtime(1)?;
    let step = player(&mut runtime)?;
    let monsters = [
        ("creature:rat", 20, position(111)),
        ("creature:rat", 20, position(112)),
    ];
    let (players, defenders) =
        checked(runtime.commit_quest_encounter_entry_with_roster(vec![step], &monsters, || true))?;
    assert_eq!(players.len(), 1);
    assert_eq!(
        players[0],
        checked(runtime.read_actor_position(step.actor))?
    );
    assert_eq!(players[0].position(), step.destination);
    assert_eq!(
        defenders.len(),
        2,
        "the old entry API discarded the admitted identities"
    );
    assert_ne!(defenders[0], defenders[1]);
    for (actor, (_, health, at)) in defenders.iter().zip(&monsters) {
        assert!(runtime.contains_live_creature(*actor));
        assert_eq!(
            checked(runtime.read_actor_position(*actor))?.position(),
            *at
        );
        let snapshot = checked(runtime.companion_snapshot(*actor))?;
        assert_eq!(snapshot.health, *health);
        assert_eq!(snapshot.state.policy.definition_key, "creature:rat");
    }
    let before = runtime.encounter_fixture_snapshot();
    let mut projected = false;
    assert!(
        runtime
            .commit_quest_encounter_entry_with_roster(vec![step], &monsters, || {
                projected = true;
                true
            })
            .is_err()
    );
    assert!(!projected);
    assert_eq!(
        runtime.encounter_fixture_snapshot(),
        before,
        "a stale entry cannot publish another roster"
    );
    Ok(())
}
#[test]
fn engenc_encounter_roster_rolls_back_late_spawn_and_map_projection_failures() -> TestResult {
    let mut runtime = runtime(1)?;
    let step = player(&mut runtime)?;
    let before = runtime.encounter_fixture_snapshot();
    let mut projected = false;
    let invalid = [
        ("creature:rat", 20, position(111)),
        ("creature:missing", 20, position(112)),
    ];
    assert!(
        runtime
            .commit_quest_encounter_entry_with_roster(vec![step], &invalid, || {
                projected = true;
                true
            })
            .is_err()
    );
    assert!(!projected);
    assert_eq!(
        runtime.encounter_fixture_snapshot(),
        before,
        "a later policy failure retracts the first admitted defender"
    );
    let monsters = [
        ("creature:rat", 20, position(111)),
        ("creature:rat", 20, position(112)),
    ];
    assert!(
        runtime
            .commit_quest_encounter_entry_with_roster(vec![step], &monsters, || false)
            .is_err()
    );
    assert_eq!(
        runtime.encounter_fixture_snapshot(),
        before,
        "a rejected map CAS retracts all creatures and player movement"
    );
    assert_eq!(runtime.visible_entities().creatures.len(), 0);
    Ok(())
}

#[test]
fn qm50_platform_displaces_every_occupant_and_rolls_back_failed_projection() -> TestResult {
    let mut runtime = runtime(1)?;
    let rider = player(&mut runtime)?;
    let creatures = checked(runtime.commit_quest_creature_spawn(&[
        ("creature:rat", 20, position(100)),
        ("creature:rat", 20, position(100)),
        ("creature:rat", 20, position(101)),
    ]))?;
    let before = runtime.encounter_fixture_snapshot();
    assert!(
        runtime
            .commit_quest_platform_displacement(
                position(100),
                position(110),
                |_, _, _| true,
                || false,
            )
            .is_err()
    );
    assert_eq!(runtime.encounter_fixture_snapshot(), before);
    let moved = checked(runtime.commit_quest_platform_displacement(
        position(100),
        position(110),
        |_, _, _| true,
        || true,
    ))?;
    assert_eq!(moved.len(), 3);
    for actor in [rider.actor, creatures[0], creatures[1]] {
        assert_eq!(
            checked(runtime.read_actor_position(actor))?.position(),
            position(110)
        );
    }
    assert_eq!(
        checked(runtime.read_actor_position(creatures[2]))?.position(),
        position(101)
    );
    Ok(())
}

#[test]
fn qm50_platform_checks_all_destinations_before_projecting_or_moving() -> TestResult {
    let mut runtime = runtime(1)?;
    let rider = player(&mut runtime)?;
    let creatures =
        checked(runtime.commit_quest_creature_spawn(&[("creature:rat", 20, position(100))]))?;
    let before = runtime.encounter_fixture_snapshot();
    let mut projected = false;
    assert!(
        runtime
            .commit_quest_platform_displacement(
                position(100),
                position(110),
                |actor, _, _| actor == rider.actor,
                || {
                    projected = true;
                    true
                },
            )
            .is_err()
    );
    assert!(!projected);
    assert_eq!(runtime.encounter_fixture_snapshot(), before);
    assert_eq!(
        checked(runtime.read_actor_position(creatures[0]))?.position(),
        position(100)
    );
    Ok(())
}

#[test]
fn engenc_defender_completion_is_exact_participant_fenced_and_idempotent() -> TestResult {
    let mut runtime = runtime(1)?;
    let step = player(&mut runtime)?;
    let defenders = checked(runtime.commit_quest_creature_spawn(&[
        ("creature:rat", 20, position(111)),
        ("creature:rat", 20, position(112)),
        ("creature:rat", 20, position(113)),
    ]))?;
    let mut roster = checked(EncounterDefenders::new(&[step.actor], &defenders[..2]))?;
    assert_eq!(roster.participants(), &[step.actor]);
    assert!(!roster.complete_for(step.actor));
    let death = |actor: ExactActorRef| CreatureDeathOccurrenceKey { actor: actor.0 };
    assert_eq!(roster.record_death(death(defenders[2])), None);
    assert_eq!(roster.record_death(death(defenders[0])), Some(0));
    assert_eq!(roster.record_death(death(defenders[0])), None);
    assert!(!roster.complete_for(step.actor));
    let mut stale = defenders[1];
    stale.0.actor_local_generation.0 += 1;
    assert_eq!(roster.record_death(death(stale)), None);
    assert_eq!(roster.record_death(death(defenders[1])), Some(1));
    assert!(roster.complete_for(step.actor));
    assert!(!roster.complete_for(defenders[2]));
    assert_eq!(roster.record_death(death(defenders[1])), None);
    assert!(EncounterDefenders::new(&[step.actor], &[]).is_err());
    assert!(EncounterDefenders::new(&[step.actor], &[defenders[0], defenders[0]]).is_err());
    assert!(EncounterDefenders::new(&[step.actor], &[step.actor]).is_err());
    let mut foreign = defenders[0];
    foreign
        .0
        .set_scope_generation(checked(ScopeOwnershipGeneration::new(2))?);
    assert!(EncounterDefenders::new(&[step.actor], &[foreign]).is_err());
    Ok(())
}

#[test]
fn qm50_platform_rider_keeps_other_occupants_and_rolls_back_failed_map_batch() -> TestResult {
    let mut runtime = runtime(1)?;
    let rider = player(&mut runtime)?;
    let creatures =
        checked(runtime.commit_quest_creature_spawn(&[("creature:rat", 20, position(100))]))?;
    let before = runtime.encounter_fixture_snapshot();
    assert!(
        runtime
            .commit_quest_platform_rider(rider, || false)
            .is_err()
    );
    assert_eq!(runtime.encounter_fixture_snapshot(), before);
    let landed = checked(runtime.commit_quest_platform_rider(rider, || true))?;
    assert_eq!(landed.position(), rider.destination);
    assert_eq!(
        checked(runtime.read_actor_position(creatures[0]))?.position(),
        position(100)
    );
    Ok(())
}

#[test]
fn m49_arrival_spawn_rolls_back_when_the_world_readset_fence_fails() -> TestResult {
    let mut runtime = runtime(1)?;
    let before = runtime.encounter_fixture_snapshot();
    let monsters = [("creature:rat", 20, position(110))];
    assert!(
        runtime
            .commit_quest_creature_spawn_with_projection(&monsters, || false)
            .is_err()
    );
    assert_eq!(runtime.encounter_fixture_snapshot(), before);
    let actors = checked(runtime.commit_quest_creature_spawn_with_projection(&monsters, || true))?;
    assert_eq!(actors.len(), 1);
    assert_eq!(
        checked(runtime.read_actor_position(actors[0]))?.position(),
        position(110)
    );
    Ok(())
}
