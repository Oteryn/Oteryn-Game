//! EQMOD2: neutral, non-recursive equipment damage with independent source replay fencing.
use super::*;
use sha2::{Digest, Sha256};

const REFLECTION_STREAMS_MAX: usize = 1024;

impl ChannelRuntimeV1 {
    /// Commit a source-qualified reflected hit. The incoming owner admits the source stream
    /// and strictly increasing clock; this owner independently fences both actors, retains a
    /// bounded high-water mark, attributes only removed HP, and never consumes player commands.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_equipment_reflection(
        &mut self,
        source: ExactActorRef,
        reflector: ExactActorRef,
        session: GameSessionId,
        stream: &[u8],
        sequence: u64,
        magnitude: u32,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_source_damage(
            source,
            reflector,
            session,
            stream,
            sequence,
            magnitude,
            sequence / 1_000,
        )
    }

    /// Independently fenced leaf damage for an existing owner occurrence. The sequence is
    /// replay identity; `now_ms` timestamps attribution without changing that identity.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_source_damage(
        &mut self,
        source: ExactActorRef,
        reflector: ExactActorRef,
        session: GameSessionId,
        stream: &[u8],
        sequence: u64,
        magnitude: u32,
        now_ms: u64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_source_damage_with_vitals(
            source,
            reflector,
            session,
            stream,
            sequence,
            magnitude,
            now_ms,
            |_, result| Some(result),
        )?
        .ok_or(CarrierError::PlanConflict)
    }

    /// Prepare the existing leaf-damage successor before invoking the player owner.
    /// The callback has immutable runtime access, so a refused player commit leaves both
    /// owners unchanged and a successful callback is followed by allocation-free publication.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_source_damage_with_vitals<R>(
        &mut self,
        source: ExactActorRef,
        reflector: ExactActorRef,
        session: GameSessionId,
        stream: &[u8],
        sequence: u64,
        magnitude: u32,
        now_ms: u64,
        commit: impl FnOnce(&Self, OwnerDamageResult) -> Option<R>,
    ) -> Result<Option<R>, CarrierError> {
        let (index, next, result) = self.prepare_source_damage(
            source, reflector, session, stream, sequence, magnitude, now_ms,
        )?;
        let Some(receipt) = commit(self, result) else {
            return Ok(None);
        };
        if let Some(next) = next {
            self.carrier.slots[index] = next;
        }
        Ok(Some(receipt))
    }

    /// Compose distinct existing source owners with one immutable player-owner callback.
    /// Every receipt and attribution allocation is prepared before the player can write.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_source_damage_batch_with_vitals<R>(
        &mut self,
        sources: &[(ExactActorRef, u32)],
        reflector: ExactActorRef,
        session: GameSessionId,
        stream: &[u8],
        sequence: u64,
        now_ms: u64,
        commit: impl FnOnce(&Self, &[OwnerDamageResult]) -> Option<R>,
    ) -> Result<Option<R>, CarrierError> {
        if sources.len() > self.actor_slot_capacity() {
            return Err(CarrierError::AllocationFailed);
        }
        let mut plans = Vec::new();
        let mut results = Vec::new();
        plans
            .try_reserve(sources.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        results
            .try_reserve(sources.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (source, magnitude) in sources {
            let (index, next, result) = self.prepare_source_damage(
                *source, reflector, session, stream, sequence, *magnitude, now_ms,
            )?;
            if plans.iter().any(|(other, _)| *other == index) {
                return Err(CarrierError::PlanConflict);
            }
            plans.push((index, next));
            results.push(result);
        }
        let Some(receipt) = commit(self, &results) else {
            return Ok(None);
        };
        for (index, next) in plans {
            if let Some(next) = next {
                self.carrier.slots[index] = next;
            }
        }
        Ok(Some(receipt))
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_source_damage(
        &mut self,
        source: ExactActorRef,
        reflector: ExactActorRef,
        session: GameSessionId,
        stream: &[u8],
        sequence: u64,
        magnitude: u32,
        now_ms: u64,
    ) -> Result<(usize, Option<Slot>, OwnerDamageResult), CarrierError> {
        let lease = self.bound_attacker_lease(reflector, session)?;
        // The incoming owner may still hit a player held through disconnect. The bound
        // lease/session fence is required; a live client connection is not.
        self.player_control_facts(reflector, session)?;
        let target = self
            .creature_spell_target_atom(source)
            .map_err(|_| CarrierError::NotCreature)?;
        let key: [u8; 32] = Sha256::new()
            .chain_update(reflector.placement_identity())
            .chain_update(stream)
            .finalize()
            .into();
        let owner = self.borrow_exact_actor_commit_at(now_ms);
        let index = owner.carrier.validate_ref(owner.continuity, source.0)?;
        let Slot::CreatureOccupied {
            committed,
            damage_contributors,
            ..
        } = &owner.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if let Some(mark) = committed.reflection_marks.get(&key) {
            if sequence <= *mark {
                let result = committed
                    .entries
                    .iter()
                    .find(|record| record.reflection == Some((key, sequence)))
                    .ok_or(CarrierError::StaleAttackerSequence)
                    .and_then(|record| {
                        if record.damage != i64::from(magnitude) {
                            return Err(CarrierError::PlanConflict);
                        }
                        Ok(OwnerDamageResult {
                            applied: false,
                            ..record.result
                        })
                    })?;
                return Ok((index, None, result));
            }
        } else if committed.reflection_marks.len() >= REFLECTION_STREAMS_MAX {
            return Err(CarrierError::DamageReceiptCapacityExceeded);
        }
        if damage_contributors
            .high_water(lease.character_id())
            .is_some_and(|mark| {
                mark.0 > lease.generation() || (mark.0 == lease.generation() && mark.1 != session)
            })
        {
            return Err(CarrierError::SupersededAttackerSession);
        }
        let occurrence = format!("equipment-reflection:{key:x?}:{sequence}");
        let mut binding = occurrence.as_bytes().to_vec();
        binding.push(0);
        binding.extend_from_slice(&magnitude.to_be_bytes());
        let (_, next, result) = owner.carrier.prepare_creature_damage_inner_bounded(
            owner.continuity,
            source.0,
            OwnerDamageCommand {
                target: target.as_bytes(),
                occurrence: occurrence.as_bytes(),
                binding: &binding,
                damage: i64::from(magnitude),
            },
            None,
            false,
            None,
            false,
            Some(now_ms),
        )?;
        let Some(mut next) = next else {
            return Ok((index, None, result));
        };
        let Slot::CreatureOccupied {
            committed,
            damage_contributors,
            ..
        } = &mut next
        else {
            return Err(CarrierError::NotCreature);
        };
        let record = committed
            .entries
            .last_mut()
            .ok_or(CarrierError::PlanConflict)?;
        record.reflection = Some((key, sequence));
        let ordinal = record.ordinal;
        committed.reflection_marks.insert(key, sequence);
        let mark = damage_contributors
            .high_water(lease.character_id())
            .filter(|mark| mark.0 == lease.generation() && mark.1 == session)
            .unwrap_or((lease.generation(), session, 0, 0));
        damage_contributors.record(
            lease.character_id(),
            result
                .health_before
                .saturating_sub(result.health_after)
                .unsigned_abs(),
            ordinal,
            Some(mark),
            Some(now_ms),
        );
        Ok((index, Some(next), result))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn fixture() -> (
        ChannelRuntimeV1,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
    ) {
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
        runtime
            .bind_attacker_lease(
                player,
                session,
                CharacterLease::new(CharacterId::decode(&id(5)).expect("character"), 1)
                    .expect("lease"),
            )
            .expect("bind");
        runtime
            .initialize_movement_test_position(
                player,
                MovementLocalPosition {
                    x: 1,
                    y: 1,
                    floor: 7,
                },
            )
            .expect("position");
        let creature = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 2,
                    y: 1,
                    floor: 7,
                },
                "eqmod2:creature",
                10_000,
            )
            .expect("creature");
        (runtime, player, session, creature)
    }

    #[test]
    fn incoming_order_reflection_refused_player_commit_preserves_source_and_receipt() {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                let (mut runtime, player, session, creature) = fixture();
                let refused = runtime
                    .commit_source_damage_with_vitals(
                        creature,
                        player,
                        session,
                        b"incoming-refused",
                        1,
                        100,
                        0,
                        |runtime, result| {
                            assert_eq!(runtime.live_creature_health(creature), Some(10_000));
                            assert_eq!(result.health_after, 9_900);
                            None::<()>
                        },
                    )
                    .expect("prepared source successor");
                assert!(refused.is_none());
                assert_eq!(runtime.live_creature_health(creature), Some(10_000));
                let committed = runtime
                    .commit_source_damage(creature, player, session, b"incoming-refused", 1, 100, 0)
                    .expect("same occurrence retry");
                assert!(committed.applied);
                assert_eq!(runtime.live_creature_health(creature), Some(9_900));
                let replay = runtime
                    .commit_source_damage(creature, player, session, b"incoming-refused", 1, 100, 0)
                    .expect("replay");
                assert!(!replay.applied);
            })
            .expect("thread")
            .join()
            .expect("scenario");
    }

    #[test]
    fn eqmod2_reflection_replays_evicted_clocks_and_normal_commands_are_independent() {
        let (mut runtime, player, session, creature) = fixture();
        for sequence in 1..=24 {
            let hit = runtime
                .commit_equipment_reflection(creature, player, session, b"melee", sequence, 1)
                .expect("reflection");
            assert!(hit.applied);
            let replay = runtime
                .commit_equipment_reflection(creature, player, session, b"melee", sequence, 1)
                .expect("replay");
            assert!(!replay.applied);
            assert_eq!(replay.health_after, hit.health_after);
            assert!(
                runtime
                    .commit_equipment_reflection(creature, player, session, b"melee", sequence, 2)
                    .is_err()
            );
        }
        assert!(matches!(
            runtime.commit_equipment_reflection(creature, player, session, b"melee", 1, 1),
            Err(CarrierError::StaleAttackerSequence)
        ));
        assert_eq!(runtime.live_creature_health(creature), Some(9976));
        let command = CommandRef::new(
            session,
            oteryn_gameplay_foundation::foundation::CommandId::new(1).expect("command"),
        );
        let hit = runtime
            .borrow_exact_actor_commit()
            .commit_damage_for_bound_attacker(
                creature,
                player,
                command,
                0,
                OwnerDamageCommand {
                    target: b"eqmod2:creature",
                    occurrence: &[],
                    binding: b"ordinary-player-command",
                    damage: 1,
                },
            )
            .expect("ordinary command after autonomous effects");
        assert!(hit.applied);
        let shares = runtime
            .borrow_combat_death()
            .reward_damage_shares(creature)
            .expect("shares");
        assert_eq!(shares[0].total, 25);
        assert_eq!(shares[0].session, Some(session));
    }

    #[test]
    fn eqmod2_reflection_stream_boundary_and_stale_session_refuse_before_hp() {
        let (mut runtime, player, session, creature) = fixture();
        for stream in 0..REFLECTION_STREAMS_MAX {
            assert!(
                runtime
                    .commit_equipment_reflection(
                        creature,
                        player,
                        session,
                        &stream.to_be_bytes(),
                        1,
                        1
                    )
                    .expect("at capacity")
                    .applied
            );
        }
        let before = runtime.live_creature_health(creature);
        assert!(matches!(
            runtime.commit_equipment_reflection(creature, player, session, b"extra", 1, 1),
            Err(CarrierError::DamageReceiptCapacityExceeded)
        ));
        assert_eq!(runtime.live_creature_health(creature), before);
        assert!(
            runtime
                .commit_equipment_reflection(
                    creature,
                    player,
                    session,
                    &0_usize.to_be_bytes(),
                    2,
                    1
                )
                .expect("existing stream at capacity")
                .applied
        );
        let mut wrong = *session.as_bytes();
        wrong[15] ^= 1;
        let before = runtime.live_creature_health(creature);
        assert!(
            runtime
                .commit_equipment_reflection(
                    creature,
                    player,
                    GameSessionId::decode(&wrong).expect("session"),
                    b"extra",
                    2,
                    1
                )
                .is_err()
        );
        assert_eq!(runtime.live_creature_health(creature), before);
    }
}
