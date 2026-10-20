//! A2: bound attacker slots and write fences through Ability and Combat.

use crate::ability::commit::{
    OwnerCharmDamagePlan, OwnerCommitError, commit_exact_owner_charm_damage,
    commit_exact_owner_damage, commit_exact_owner_primary_damage,
};
use crate::ability::exact_actor_resolution::{
    ExactActorProposal, ResolvedExactActor, resolve_exact_actor,
};
use crate::ability::{
    AbilityIntent, AbilityOccurrence, CalculationStage, CommitGroup, Effect, EffectPlan,
    ProposalSource, RevisionSet,
};
use crate::foundation::runtime_actor_carrier::*;
use crate::foundation::*;

const SUPERSEDED: Result<OwnerDamageResult, CarrierError> =
    Err(CarrierError::SupersededAttackerSession);

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

fn session(seed: u8) -> GameSessionId {
    GameSessionId::decode(&id(seed)).expect("session")
}

fn lease(character: u8, generation: u64) -> CharacterLease {
    CharacterLease::new(
        CharacterId::decode(&id(character)).expect("character"),
        generation,
    )
    .expect("lease")
}

fn command(session_seed: u8, sequence: u64) -> CommandRef {
    CommandRef::new(
        session(session_seed),
        crate::foundation::CommandId::new(sequence).expect("command"),
    )
}

struct Fixture {
    continuity: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    creature: ExactActorRef,
    /// Session 1 of Character 1, bound at lease generation 1.
    attacker: ExactActorRef,
}

fn fixture(seed: u64) -> Fixture {
    let mut continuity =
        NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant::fixture(
            WorldId::decode(&id(seed as u8)).expect("world"),
            ChannelId::decode(&id(seed as u8 + 1)).expect("channel"),
            ScopeOwnershipGeneration::new(1).expect("scope"),
        ));
    let mut carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 3).expect("carrier");
    let creature = ExactActorRef(
        carrier
            .admit_creature(&continuity, ActorState(1), "target:one", 100)
            .expect("creature"),
    );
    let attacker = carrier
        .admit_bound_test_attacker(&continuity, session(1), lease(1, 1))
        .expect("bound attacker");
    Fixture {
        continuity,
        carrier,
        creature,
        attacker,
    }
}

impl Fixture {
    fn hit(
        &mut self,
        attacker: ExactActorRef,
        command: CommandRef,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let binding = format!("hit:{}", command.command_id().get());
        self.carrier
            .current_owner_exact_commit(&self.continuity)
            .commit_damage_for_bound_attacker(
                self.creature,
                attacker,
                command,
                0,
                OwnerDamageCommand {
                    target: b"target:one",
                    occurrence: &[],
                    binding: binding.as_bytes(),
                    damage: 1,
                },
            )
    }

    fn health(&self) -> i64 {
        match &self.carrier.slots[0] {
            Slot::CreatureOccupied { health, .. } => Some(*health),
            _ => None,
        }
        .expect("the creature is slot 0")
    }

    fn fence(&mut self, token: WriteFenceToken) -> Result<WriteFenceSet, CarrierError> {
        self.carrier
            .fence_player_writes(&self.continuity, self.attacker.0, session(1), token)
    }

    fn lift(&mut self, token: WriteFenceToken) -> Result<bool, CarrierError> {
        self.carrier
            .lift_player_fence(&self.continuity, self.attacker.0, session(1), token)
    }

    fn lose_control(&mut self, epoch: u64) {
        self.carrier
            .record_player_control_loss(
                &self.continuity,
                self.attacker.0,
                session(1),
                ControlLossMark {
                    epoch,
                    grace_deadline: 100,
                },
            )
            .expect("control loss");
    }
}

#[test]
fn a_write_between_fence_and_commit_is_refused_and_only_the_exact_token_lifts() {
    let mut f = fixture(10);
    assert!(f.hit(f.attacker, command(1, 1)).is_ok());
    let token = WriteFenceToken::Transition(7);
    assert_eq!(f.fence(token), Ok(WriteFenceSet::Fenced));
    let before = f.carrier.slots.clone();
    assert_eq!(f.hit(f.attacker, command(1, 2)), SUPERSEDED);
    assert_eq!(f.carrier.slots, before);
    // A lift with any other token is a no-op.
    assert_eq!(f.lift(WriteFenceToken::Transition(8)), Ok(false));
    assert_eq!(f.lift(WriteFenceToken::ControlLoss(7)), Ok(false));
    assert_eq!(f.hit(f.attacker, command(1, 2)), SUPERSEDED);
    // NotApplicable with the session still holding the lease: the exact token lifts.
    assert_eq!(f.lift(token), Ok(true));
    assert_eq!(f.lift(token), Ok(false));
    assert!(f.hit(f.attacker, command(1, 2)).is_ok());
    assert_eq!(f.health(), 98);
}

#[test]
fn one_transition_per_session_joins_its_retry_and_never_replaces_a_fence() {
    let mut f = fixture(20);
    f.lose_control(3);
    let grace = WriteFenceToken::ControlLoss(3);
    assert_eq!(f.fence(grace), Ok(WriteFenceSet::Fenced));
    // An unknown durable outcome keeps the fence; the retry joins it.
    assert_eq!(f.fence(grace), Ok(WriteFenceSet::Joined));
    // An overlapping logout or revocation waits for the first transition's step (c).
    assert_eq!(
        f.fence(WriteFenceToken::Transition(1)),
        Err(CarrierError::WriteFenceBusy)
    );
    assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
    // After a lift, the second transition starts again at step (a) with its own token.
    assert_eq!(f.lift(grace), Ok(true));
    assert_eq!(
        f.fence(WriteFenceToken::Transition(1)),
        Ok(WriteFenceSet::Fenced)
    );
    assert_eq!(f.fence(grace), Err(CarrierError::WriteFenceBusy));
    // After a terminal settle, the second transition finds nothing to do.
    f.carrier
        .remove_terminal_player(&f.continuity, session(1), f.attacker)
        .expect("terminal settle");
    assert!(f.fence(WriteFenceToken::Transition(2)).is_err());
    assert!(f.lift(WriteFenceToken::Transition(1)).is_err());
    assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
}

#[test]
fn grace_expiry_fences_only_its_marked_epoch_and_a_resume_lifts_it() {
    let mut f = fixture(30);
    // No mark, or another epoch: fencing refuses and the transition does not start.
    assert_eq!(
        f.fence(WriteFenceToken::ControlLoss(1)),
        Err(CarrierError::ControlLossConflict)
    );
    f.lose_control(1);
    assert_eq!(
        f.fence(WriteFenceToken::ControlLoss(2)),
        Err(CarrierError::ControlLossConflict)
    );
    assert_eq!(
        f.fence(WriteFenceToken::ControlLoss(1)),
        Ok(WriteFenceSet::Fenced)
    );
    assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
    // The same-session resume lifts the fence together with the mark of its epoch.
    f.carrier
        .restore_player_control(&f.continuity, f.attacker.0, session(1), 1)
        .expect("resume");
    assert!(f.hit(f.attacker, command(1, 1)).is_ok());
    // A resume never lifts another transition's fence.
    f.lose_control(2);
    assert_eq!(
        f.fence(WriteFenceToken::Transition(5)),
        Ok(WriteFenceSet::Fenced)
    );
    f.carrier
        .restore_player_control(&f.continuity, f.attacker.0, session(1), 2)
        .expect("resume");
    assert_eq!(f.hit(f.attacker, command(1, 2)), SUPERSEDED);
}

#[test]
fn a_terminal_session_is_never_unfenced_and_its_slot_binding_goes_with_it() {
    let mut f = fixture(40);
    let token = WriteFenceToken::Transition(1);
    assert_eq!(f.fence(token), Ok(WriteFenceSet::Fenced));
    f.carrier
        .remove_terminal_player(&f.continuity, session(1), f.attacker)
        .expect("terminal settle");
    assert!(f.lift(token).is_err());
    assert!(
        f.carrier
            .bind_attacker_lease(&f.continuity, f.attacker.0, session(1), lease(1, 1))
            .is_err()
    );
    assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
    assert!(f.carrier.attackers.is_empty());
    // A new session in the recycled slot inherits neither the fence nor the binding.
    let reservation = f
        .carrier
        .reserve_player(&f.continuity, session(2))
        .expect("reserve");
    let next = f
        .carrier
        .commit_reserved_player(&f.continuity, reservation)
        .expect("commit");
    assert_eq!(f.hit(next, command(2, 1)), SUPERSEDED);
    f.carrier
        .bind_attacker_lease(&f.continuity, next.0, session(2), lease(1, 2))
        .expect("bind");
    assert_eq!(
        f.carrier
            .bind_attacker_lease(&f.continuity, next.0, session(2), lease(1, 3)),
        Err(CarrierError::AttackerBindingConflict)
    );
    assert!(f.hit(next, command(2, 1)).is_ok());
}

#[test]
fn a_rebind_drops_the_old_fence_and_leaves_the_rebound_slot_unfenced() {
    let mut f = fixture(50);
    f.lose_control(4);
    assert_eq!(
        f.fence(WriteFenceToken::ControlLoss(4)),
        Ok(WriteFenceSet::Fenced)
    );
    for (successor, refused) in [
        (lease(1, 1), "an equal lease generation"),
        (lease(2, 2), "another Character"),
    ] {
        assert_eq!(
            f.carrier.rebind_player_session(
                &f.continuity,
                f.attacker.0,
                session(1),
                session(2),
                successor
            ),
            Err(CarrierError::AttackerBindingConflict),
            "{refused}"
        );
    }
    f.carrier
        .rebind_player_session(
            &f.continuity,
            f.attacker.0,
            session(1),
            session(2),
            lease(1, 2),
        )
        .expect("post-grace rebind");
    assert_eq!(f.hit(f.attacker, command(1, 1)), SUPERSEDED);
    assert!(f.hit(f.attacker, command(2, 1)).is_ok());
    assert!(f.fence(WriteFenceToken::ControlLoss(4)).is_err());
    assert_eq!(
        f.carrier
            .player_control_loss(&f.continuity, f.attacker.0, session(2)),
        Ok(None)
    );
}

#[test]
fn a_same_session_continuation_reconciles_before_it_binds() {
    let mut f = fixture(60);
    let reservation = f
        .carrier
        .reserve_player(&f.continuity, session(3))
        .expect("reserve");
    let slot = f
        .carrier
        .commit_reserved_player(&f.continuity, reservation)
        .expect("reconstructed slot");
    // No write is admitted before the binding exists.
    assert_eq!(f.hit(slot, command(3, 1)), SUPERSEDED);
    for refused in [
        ContinuationReconcile::Terminal,
        ContinuationReconcile::Unprovable,
    ] {
        assert_eq!(
            f.carrier
                .bind_continuation(&f.continuity, slot.0, session(3), refused),
            Err(CarrierError::ContinuationRefused)
        );
        assert_eq!(f.hit(slot, command(3, 1)), SUPERSEDED);
    }
    f.carrier
        .bind_continuation(
            &f.continuity,
            slot.0,
            session(3),
            ContinuationReconcile::Proven(lease(3, 1)),
        )
        .expect("proven continuation binds unfenced");
    assert!(f.hit(slot, command(3, 1)).is_ok());
    assert_eq!(
        f.carrier.bind_continuation(
            &f.continuity,
            slot.0,
            session(3),
            ContinuationReconcile::Proven(lease(3, 2)),
        ),
        Err(CarrierError::ContinuationRefused)
    );
}

fn primary_plan(f: &Fixture) -> (ResolvedExactActor, EffectPlan) {
    let occurrence = AbilityOccurrence::new(
        "attack:rebind",
        RevisionSet::new(
            "rules:1",
            "content:1",
            "world:1",
            "formula:1",
            "simulation:1",
        )
        .expect("revisions"),
    )
    .expect("occurrence");
    let resolved = resolve_exact_actor(
        &f.carrier.current_owner_exact_lookup(&f.continuity),
        &occurrence,
        ExactActorProposal::client(f.creature),
    )
    .expect("current creature");
    let plan = EffectPlan::immediate(
        occurrence,
        AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:one"])
            .expect("intent"),
        vec![Effect::damage("target:one", 3).expect("damage")],
        vec![],
        CommitGroup::atomic("scope:fixture", "group:one").expect("group"),
    )
    .expect("plan");
    (resolved, plan)
}

fn charm_plan(parent: &EffectPlan) -> EffectPlan {
    EffectPlan::ordered_sequential(
        parent.occurrence().clone(),
        parent.intent().clone(),
        vec![
            parent.effects()[0].clone(),
            Effect::damage("target:one", 9).expect("generated"),
        ],
        vec![CalculationStage::new("charm:generated").expect("stage")],
        parent.commit_group().owner_scope(),
        parent.commit_group().group_id(),
    )
    .expect("charm plan")
}

#[test]
fn a_rebind_during_an_in_flight_charm_or_descriptor_commit_settles_exactly_once() {
    for charm_before_rebind in [false, true] {
        let mut f = fixture(70);
        let (resolved, plan) = primary_plan(&f);
        let parent = commit_exact_owner_primary_damage(
            &mut f.carrier.current_owner_exact_commit(&f.continuity),
            &resolved,
            &plan,
            f.attacker,
            command(1, 1),
        )
        .expect("primary under the bound lease");
        let frozen = OwnerCharmDamagePlan::prepare(&parent, charm_plan(&plan))
            .expect("prepare")
            .expect("live parent");
        let charm = |f: &mut Fixture| {
            commit_exact_owner_charm_damage(
                &mut f.carrier.current_owner_exact_commit(&f.continuity),
                &frozen,
                f.attacker,
                command(1, 1),
            )
            .map(|charm| charm.result)
        };
        if charm_before_rebind {
            assert!(charm(&mut f).expect("descendant").applied);
        }
        let settled = f.health();
        assert_eq!(settled, if charm_before_rebind { 88 } else { 97 });
        f.carrier
            .rebind_player_session(
                &f.continuity,
                f.attacker.0,
                session(1),
                session(2),
                lease(1, 2),
            )
            .expect("rebind");
        // Neither the descendant nor a replay of the primary applies again under the old
        // session: each occurrence settled exactly once, before the rebind or never.
        assert_eq!(
            charm(&mut f),
            Err(OwnerCommitError::Owner(
                CarrierError::SupersededAttackerSession
            ))
        );
        assert_eq!(
            commit_exact_owner_damage(
                &mut f.carrier.current_owner_exact_commit(&f.continuity),
                &resolved,
                &plan,
                f.attacker,
                command(1, 1),
            ),
            Err(OwnerCommitError::Owner(
                CarrierError::SupersededAttackerSession
            ))
        );
        assert_eq!(f.health(), settled);
    }
}

#[test]
fn a_fenced_slot_refuses_every_bridge_before_the_owner_write() {
    let mut f = fixture(80);
    let (resolved, plan) = primary_plan(&f);
    f.lose_control(1);
    assert_eq!(
        f.fence(WriteFenceToken::ControlLoss(1)),
        Ok(WriteFenceSet::Fenced)
    );
    let before = f.carrier.slots.clone();
    assert_eq!(
        commit_exact_owner_primary_damage(
            &mut f.carrier.current_owner_exact_commit(&f.continuity),
            &resolved,
            &plan,
            f.attacker,
            command(1, 1),
        )
        .map(|_| ()),
        Err(OwnerCommitError::Owner(
            CarrierError::SupersededAttackerSession
        ))
    );
    // A command of a session the slot does not hold is refused the same way.
    assert_eq!(
        commit_exact_owner_damage(
            &mut f.carrier.current_owner_exact_commit(&f.continuity),
            &resolved,
            &plan,
            f.attacker,
            command(9, 1),
        ),
        Err(OwnerCommitError::Owner(
            CarrierError::SupersededAttackerSession
        ))
    );
    assert_eq!(f.carrier.slots, before);
}

impl Fixture {
    fn swing(
        &mut self,
        lineage: CommandRef,
        ordinal: u16,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let binding = format!("swing:{}:{ordinal}", lineage.command_id().get());
        self.carrier
            .current_owner_exact_commit(&self.continuity)
            .commit_swing_damage_for_bound_attacker(
                self.creature,
                self.attacker,
                lineage,
                ordinal,
                OwnerDamageCommand {
                    target: b"target:one",
                    occurrence: &[],
                    binding: binding.as_bytes(),
                    damage: 1,
                },
            )
    }
}

/// ATTACK-1b swing identity: many swings of one lineage commit on the same creature, a
/// replay of a retained swing is idempotent, a lower or equal unretained ordinal is refused,
/// the session fence still holds and the high-water mark never lowers.
#[test]
fn swings_of_one_lineage_commit_in_order_and_never_rewind() {
    let mut f = fixture(40);
    let lineage = command(1, 5);
    for ordinal in 0..4 {
        let hit = f.swing(lineage, ordinal).expect("swing commits");
        assert!(hit.applied);
        assert_eq!(hit.health_before - hit.health_after, 1);
    }
    assert_eq!(f.health(), 96);
    // An exact replay of a retained swing returns its receipt without a second write.
    let replay = f.swing(lineage, 3).expect("replay");
    assert!(!replay.applied);
    assert_eq!(f.health(), 96);
    // A different-bytes swing under an already committed ordinal is not a new swing.
    let before = f.carrier.slots.clone();
    let binding = b"swing:other";
    assert!(
        f.carrier
            .current_owner_exact_commit(&f.continuity)
            .commit_swing_damage_for_bound_attacker(
                f.creature,
                f.attacker,
                lineage,
                3,
                OwnerDamageCommand {
                    target: b"target:one",
                    occurrence: &[],
                    binding,
                    damage: 1,
                },
            )
            .is_err()
    );
    assert_eq!(f.carrier.slots, before);
    // A later lineage (a new `ATTACK_TARGET`) starts again at ordinal 0.
    let next = command(1, 6);
    assert!(f.swing(next, 0).expect("new lineage").applied);
    assert!(f.swing(next, 1).expect("new lineage").applied);
    assert_eq!(f.health(), 94);
    // Once the retained receipt is gone, a lower or equal ordinal of the current lineage is
    // stale, never a fresh write.
    if let Slot::CreatureOccupied { committed, .. } = &mut f.carrier.slots[0] {
        committed.entries.clear();
    }
    let before = f.carrier.slots.clone();
    assert_eq!(f.swing(next, 1), Err(CarrierError::StaleAttackerSequence));
    assert_eq!(f.swing(next, 0), Err(CarrierError::StaleAttackerSequence));
    assert_eq!(f.carrier.slots, before);
    // A spell command of the same session after the swings keeps the mark; the lineage
    // continues and the mark is never lowered by it.
    assert!(f.hit(f.attacker, command(1, 7)).expect("spell").applied);
    assert!(f.swing(next, 2).expect("lineage continues").applied);
    assert_eq!(
        f.hit(f.attacker, command(1, 7)).map(|r| r.applied),
        Ok(false)
    );
    // The bound-attacker session fence still refuses a superseded session.
    assert_eq!(f.swing(command(9, 8), 0), SUPERSEDED);
    let token = WriteFenceToken::Transition(3);
    assert_eq!(f.fence(token), Ok(WriteFenceSet::Fenced));
    assert_eq!(f.swing(next, 3), SUPERSEDED);
}

/// Codex P2 4180149935: the lineage's last swing ordinal (`u16::MAX`) is admitted, and a
/// replay of it stays idempotent.
#[test]
fn the_final_u16_swing_ordinal_commits() {
    let mut f = fixture(40);
    let lineage = command(1, 5);
    assert!(f.swing(lineage, u16::MAX - 1).expect("penultimate").applied);
    let last = f.swing(lineage, u16::MAX).expect("final ordinal commits");
    assert!(last.applied);
    assert_eq!(f.health(), 98);
    assert!(!f.swing(lineage, u16::MAX).expect("replay").applied);
    assert_eq!(f.health(), 98);
}

#[test]
fn fence_tokens_keep_their_kind() {
    assert_eq!(
        ChannelRuntimeV1::transition_fence(2),
        WriteFenceToken::Transition(2)
    );
    assert_eq!(
        ChannelRuntimeV1::grace_expiry_fence(2),
        WriteFenceToken::ControlLoss(2)
    );
    assert_ne!(
        ChannelRuntimeV1::transition_fence(2),
        ChannelRuntimeV1::grace_expiry_fence(2)
    );
}

fn hit_at(
    f: &mut Fixture,
    command: CommandRef,
    now_ms: u64,
    damage: i64,
) -> Result<OwnerDamageResult, CarrierError> {
    let binding = format!("hit:{}", command.command_id().get());
    f.carrier
        .current_owner_exact_commit_at(&f.continuity, Some(now_ms))
        .commit_damage_for_bound_attacker(
            f.creature,
            f.attacker,
            command,
            0,
            OwnerDamageCommand {
                target: b"target:one",
                occurrence: &[],
                binding: binding.as_bytes(),
                damage,
            },
        )
}

fn principal(f: &mut Fixture) -> TopDamagePrincipal {
    f.carrier
        .current_owner_combat_death(&f.continuity)
        .top_damage_contributor(f.creature)
        .expect("live creature")
}

/// §1.3: no tracked contributor means the lethal-attacker fallback; a tracked winner whose
/// bound slot is still present carries its session, slot and latest damage clock; a winner
/// whose slot is gone is reported as gone.
#[test]
fn the_top_damage_principal_is_untracked_present_or_gone() {
    let mut f = fixture(70);
    assert_eq!(principal(&mut f), TopDamagePrincipal::Untracked);
    assert!(hit_at(&mut f, command(1, 1), 1_000, 1).is_ok());
    assert!(f.hit(f.attacker, command(1, 2)).is_ok());
    assert!(hit_at(&mut f, command(1, 3), 900, 1).is_ok());
    let character = CharacterId::decode(&id(1)).expect("character");
    assert_eq!(
        principal(&mut f),
        TopDamagePrincipal::Present(TopDamageContributor {
            character,
            lease_generation: 1,
            session: session(1),
            actor: f.attacker,
            last_damage_at_ms: Some(1_000),
        })
    );
    f.carrier
        .remove(&f.continuity, f.attacker.0)
        .expect("attacker leaves");
    assert_eq!(principal(&mut f), TopDamagePrincipal::Gone);
}

/// P2 4180039074: the projected death time is the lethal hit's owner clock and an
/// idempotent replay at a later clock does not move it.
#[test]
fn the_projected_death_time_is_the_lethal_hit_clock_and_survives_a_replay() {
    let mut f = fixture(72);
    let context = PreProductionPositionContext {
        world_id: WorldId::decode(&id(72)).expect("world"),
        channel_id: ChannelId::decode(&id(73)).expect("channel"),
        scope_generation: ScopeOwnershipGeneration::new(1).expect("scope"),
        coordinate_frame_marker: 41,
        map_revision_marker: 42,
        content_generation_marker: 43,
    };
    f.carrier
        .initialize_position(
            &f.continuity,
            f.creature.0,
            context,
            CombatDeathFixture::POSITION,
        )
        .expect("position");
    assert!(hit_at(&mut f, command(1, 1), 5_000, 100).is_ok());
    assert_eq!(f.health(), 0);
    assert!(hit_at(&mut f, command(1, 1), 9_000, 100).is_ok());
    let mut death = f.carrier.current_owner_combat_death(&f.continuity);
    crate::combat::project_fixed_one_creature_death(&mut death, f.creature)
        .expect("projected death");
    assert_eq!(death.projected_death_at_ms(f.creature), Ok(Some(5_000)));
}
