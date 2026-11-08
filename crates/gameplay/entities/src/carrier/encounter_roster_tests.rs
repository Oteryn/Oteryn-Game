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
fn worldchg_preflight_retains_actor_slots_and_movement_for_the_real_commit() -> TestResult {
    let mut runtime = runtime(1)?;
    let step = player(&mut runtime)?;
    let before = runtime.encounter_fixture_snapshot();
    let monsters = [("creature:rat", 20, position(111))];
    checked(runtime.preflight_quest_encounter_entry(vec![step], &monsters))?;
    assert_eq!(runtime.encounter_fixture_snapshot(), before);
    assert_eq!(
        checked(runtime.read_actor_position(step.actor))?,
        step.expected
    );
    assert!(runtime.visible_entities().creatures.is_empty());
    let (players, actors) =
        checked(runtime.commit_quest_encounter_entry_with_roster(vec![step], &monsters, || true))?;
    assert_eq!(players[0].position(), step.destination);
    assert_eq!(actors.len(), 1);
    assert!(runtime.contains_live_creature(actors[0]));
    Ok(())
}
