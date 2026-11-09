//! Retained, exact native hit provenance for deferred objective credit.
use super::*;

/// Minted only by the current native owner from its retained damage ledger.
/// A retry reads the same occurrence; it must use this identity for durable deduplication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommittedNativeHit {
    attacker: ExactActorRef,
    target: ExactActorRef,
    origin: DamageOrigin,
    ordinal: u64,
    health_loss: std::num::NonZeroU64,
}

impl CommittedNativeHit {
    pub const fn attacker(self) -> ExactActorRef {
        self.attacker
    }
    pub const fn target(self) -> ExactActorRef {
        self.target
    }
    pub const fn origin(self) -> DamageOrigin {
        self.origin
    }
    pub const fn ordinal(self) -> u64 {
        self.ordinal
    }
    pub const fn health_loss(self) -> std::num::NonZeroU64 {
        self.health_loss
    }
}

impl CurrentOwnerExactActorCommit<'_> {
    /// Resolve an attributed hit only while its exact target and bound player lease
    /// remain current. Unattributed/reflected damage cannot qualify. Evicted receipts
    /// fail closed, so callers retain the witness before leaving the commit turn.
    pub fn committed_hit_for_bound_attacker(
        &self,
        target: ExactActorRef,
        attacker: ExactActorRef,
        command: CommandRef,
        sub_ordinal: u16,
    ) -> Result<Option<CommittedNativeHit>, CarrierError> {
        let lease = self.bound_attacker_lease(attacker, command)?;
        let origin = AttackerCommand::new(
            lease.character_id(),
            lease.generation(),
            command,
            sub_ordinal,
        )
        .origin();
        let index = self.carrier.validate_ref(self.continuity, target.0)?;
        let Slot::CreatureOccupied {
            generation,
            committed,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != target.0.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        Ok(committed.entries.iter().find_map(|record| {
            if record.origin != Some(origin) || record.reflection.is_some() {
                return None;
            }
            Some(CommittedNativeHit {
                attacker,
                target,
                origin,
                ordinal: record.ordinal,
                health_loss: record.result.committed_health_loss()?,
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn engenc_native_hit_retains_exact_identity_across_retry() {
        let id = |tag| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
        let world = WorldId::decode(&id(1)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).expect("channel"),
            NodeId::decode(&id(3)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .expect("runtime");
        let session = GameSessionId::decode(&id(4)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reservation");
        let player = runtime.commit_fresh_session(reservation).expect("player");
        let lease =
            CharacterLease::new(CharacterId::decode(&id(5)).expect("character"), 1).expect("lease");
        runtime
            .bind_attacker_lease(player, session, lease)
            .expect("bind");
        let target = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 2,
                    y: 1,
                    floor: 7,
                },
                "engenc:training",
                100,
            )
            .expect("target");
        let command = CommandRef::new(
            session,
            oteryn_gameplay_foundation::foundation::CommandId::new(1).expect("command"),
        );
        let damage = || OwnerDamageCommand {
            target: b"engenc:training",
            occurrence: &[],
            binding: b"engenc:hit",
            damage: 5,
        };
        let mut owner = runtime.borrow_exact_actor_commit();
        assert!(
            owner
                .commit_damage(
                    target,
                    OwnerDamageCommand {
                        target: b"engenc:training",
                        occurrence: b"environment",
                        binding: b"environment\0hit",
                        damage: 3,
                    }
                )
                .expect("unattributed hit")
                .applied
        );

        assert_eq!(
            owner
                .committed_hit_for_bound_attacker(target, player, command, 0)
                .expect("before"),
            None
        );
        assert!(
            owner
                .commit_damage_for_bound_attacker(target, player, command, 0, damage())
                .expect("hit")
                .applied
        );
        let hit = owner
            .committed_hit_for_bound_attacker(target, player, command, 0)
            .expect("read")
            .expect("witness");
        assert_eq!(hit.attacker(), player);
        assert_eq!(hit.target(), target);
        assert_eq!(hit.health_loss().get(), 5);
        assert_eq!(hit.ordinal(), 1);
        let mut stale_target = target;
        stale_target.0.actor_local_generation =
            ActorLocalGeneration(target.0.actor_local_generation.0 + 1);
        assert_eq!(
            owner.committed_hit_for_bound_attacker(stale_target, player, command, 0),
            Err(CarrierError::StaleActorGeneration)
        );
        assert_eq!(
            hit.origin(),
            AttackerCommand::new(lease.character_id(), lease.generation(), command, 0).origin()
        );
        assert!(
            !owner
                .commit_damage_for_bound_attacker(target, player, command, 0, damage())
                .expect("replay")
                .applied
        );
        assert_eq!(
            owner
                .committed_hit_for_bound_attacker(target, player, command, 0)
                .expect("retry"),
            Some(hit)
        );
        assert_eq!(
            owner
                .committed_hit_for_bound_attacker(target, player, command, 1)
                .expect("other ordinal"),
            None
        );
        let stale = CommandRef::new(
            GameSessionId::decode(&id(9)).expect("other session"),
            command.command_id(),
        );
        assert_eq!(
            owner.committed_hit_for_bound_attacker(target, player, stale, 0),
            Err(CarrierError::SupersededAttackerSession)
        );
    }
}
