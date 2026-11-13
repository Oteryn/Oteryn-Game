use super::*;
use crate::{SourceActorReservationProof, source_reservation_seal};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn id(n: u8) -> [u8; 16] {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

struct UseProof {
    player: ExactActorRef,
    session: GameSessionId,
    boss: [ExactActorRef; 1],
    event: [u8; 16],
}
impl source_reservation_seal::Sealed for UseProof {}
impl SourceActorReservationProof for UseProof {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        (self.player, self.session)
    }
    fn event_identity(&self) -> [u8; 16] {
        self.event
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        [8; 32]
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        &self.boss
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.owner_fence().is_ok()
            && runtime
                .player_control_facts(self.player, self.session)
                .is_ok_and(|facts| facts.control_loss.is_none())
    }
}

fn fixture() -> TestResult<(ChannelRuntimeV1, UseProof)> {
    let world = WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
    let scope = RuntimeScopeRefV1::instance(world, id(2)).map_err(|error| format!("{error:?}"))?;
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        scope,
        NodeId::decode(&id(3)).map_err(|error| format!("{error:?}"))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::test(world),
    )?;
    let mut policies = super::super::soul_war_mirror_runtime::tests::policies();
    policies.bind_qualified_outer_artifact(runtime.content_pin().server_artifact_digest())?;
    runtime.install_companion_policies(policies)?;
    let session = GameSessionId::decode(&id(4)).map_err(|error| format!("{error:?}"))?;
    let reservation = runtime.reserve_fresh_session(session)?;
    let player = runtime.commit_fresh_session(reservation)?;
    runtime.initialize_first_entry_position(player)?;
    let maximum = runtime.companion_policy(CRUELTY)?.maximum_health;
    let boss = ExactActorRef(runtime.admit_encounter_fixture_actor(
        ActorState(1),
        CRUELTY,
        maximum,
        Some(LocalPosition {
            x: 33856,
            y: 31866,
            floor: 7,
        }),
    )?);
    runtime.install_creature_policy(boss, CRUELTY)?;
    Ok((
        runtime,
        UseProof {
            player,
            session,
            boss: [boss],
            event: id(5),
        },
    ))
}

#[test]
fn srx3_cruelty_maw_known_receipt_releases_and_decreases_exactly_once() -> TestResult {
    let (mut runtime, proof) = fixture()?;
    let initial = runtime.companion_snapshot(proof.boss[0])?;
    let base = runtime.effective_creature_defense(&initial)?;
    assert_eq!(runtime.soul_war_cruelty_drain(&initial)?, None);
    assert_eq!(runtime.increase_soul_war_cruelty(&initial, None)?, 3);
    let current = runtime.companion_snapshot(proof.boss[0])?;
    assert_eq!(runtime.effective_creature_defense(&current)?, base + 2);
    let mut reservation = runtime.prepare_source_actor_reservation(&proof)?;
    runtime.reserve_source_actors(&mut reservation, &proof)?;
    assert!(
        runtime
            .increase_soul_war_cruelty(&current, Some(3))
            .is_err()
    );
    assert!(runtime.remove_encounter_actor(proof.boss[0]).is_err());
    assert!(
        runtime
            .record_control_loss(
                proof.player,
                proof.session,
                ControlLossMark {
                    epoch: 1,
                    grace_deadline: 100
                }
            )
            .is_err()
    );
    assert!(
        runtime
            .complete_soul_war_cruelty_maw(
                reservation.clone(),
                &proof,
                Some((&current, base, Some(3))),
            )
            .is_err()
    );
    assert!(runtime.actor_spell_reserved(proof.player));
    assert!(runtime.actor_spell_reserved(proof.boss[0]));
    runtime.complete_soul_war_cruelty_maw(
        reservation.clone(),
        &proof,
        Some((&current, base + 2, Some(3))),
    )?;
    let after = runtime.companion_snapshot(proof.boss[0])?;
    assert_eq!(after.health, initial.health);
    assert_eq!(after.maximum_health, initial.maximum_health);
    assert_eq!(after.state.policy, initial.state.policy);
    assert_eq!(runtime.effective_creature_defense(&after)?, base);
    assert_eq!(runtime.soul_war_cruelty_drain(&after)?, Some(2));
    assert!(!runtime.actor_spell_reserved(proof.player));
    assert!(!runtime.actor_spell_reserved(proof.boss[0]));
    assert!(
        runtime
            .complete_soul_war_cruelty_maw(reservation, &proof, Some((&after, base, Some(2))),)
            .is_err(),
        "a replayed receipt cannot consume the same physical reservation twice"
    );
    assert_eq!(runtime.soul_war_cruelty_drain(&after)?, Some(2));
    Ok(())
}

#[test]
fn srx3_cruelty_maw_at_base_preserves_absent_drain_and_known_refusal_changes_nothing() -> TestResult
{
    let (mut runtime, mut proof) = fixture()?;
    let initial = runtime.companion_snapshot(proof.boss[0])?;
    let base = runtime.effective_creature_defense(&initial)?;
    let mut reservation = runtime.prepare_source_actor_reservation(&proof)?;
    runtime.reserve_source_actors(&mut reservation, &proof)?;
    runtime.complete_soul_war_cruelty_maw(reservation, &proof, Some((&initial, base, None)))?;
    assert_eq!(runtime.effective_creature_defense(&initial)?, base);
    assert_eq!(runtime.soul_war_cruelty_drain(&initial)?, None);
    runtime.increase_soul_war_cruelty(&initial, None)?;
    proof.event = id(6);
    let mut reservation = runtime.prepare_source_actor_reservation(&proof)?;
    runtime.reserve_source_actors(&mut reservation, &proof)?;
    runtime.complete_soul_war_cruelty_maw(reservation, &proof, None)?;
    assert_eq!(runtime.effective_creature_defense(&initial)?, base + 2);
    assert_eq!(runtime.soul_war_cruelty_drain(&initial)?, Some(3));
    assert!(!runtime.actor_spell_reserved(proof.boss[0]));
    Ok(())
}
