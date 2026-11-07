//! Native source groups reuse the carrier clone/rollback preparation of
//! qengine2 encounter_access::commit_quest_encounter_entry and the existing
//! CreatureReserved slots. No actor is visible before its durable receipt.
use super::*;
use std::sync::atomic::{AtomicU8, Ordering};

// Retain equality facts, never clone the production-only activation capability.
fn pinned_content(pin: &ChannelContentPin) -> ([u8; 32], [u8; 32]) {
    use sha2::{Digest, Sha256};
    let start = pin.entry_start();
    let binding = Sha256::new()
        .chain_update(b"oteryn.native-actor-batch.pin.v1\0")
        .chain_update(pin.world_id().as_bytes())
        .chain_update(pin.activation_sequence().to_be_bytes())
        .chain_update(pin.server_artifact_digest())
        .chain_update(pin.client_artifact_digest())
        .chain_update(pin.frame_binding_digest())
        .chain_update(pin.map_revision_digest())
        .chain_update(start.x.to_be_bytes())
        .chain_update(start.y.to_be_bytes())
        .chain_update(start.floor.to_be_bytes())
        .finalize()
        .into();
    (pin.server_artifact_digest(), binding)
}

#[derive(Debug)]
pub struct PreparedNativeActorBatch {
    owner: ExactActorRef,
    session: GameSessionId,
    character: CharacterId,
    lease_generation: u64,
    content: ([u8; 32], [u8; 32]),
    binding: [u8; 32],
    actors: Vec<ExactActorRef>,
    keys: Vec<String>,
    positions: Vec<MovementLocalPosition>,
    plans: Vec<(usize, Slot)>,
    before: Option<ChannelActorCarrier>,
    state: Arc<AtomicU8>,
}
#[derive(Debug, Clone)]
pub struct NativeActorBatchProof {
    owner: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease_generation: u64,
    content: [u8; 32],
    binding: [u8; 32],
    actors: Vec<ExactActorRef>,
    keys: Vec<String>,
    positions: Vec<MovementLocalPosition>,
    state: Arc<AtomicU8>,
}
impl NativeActorBatchProof {
    /// Opaque physical reservation proof. The source owner separately derives the
    /// command binding and exact allowed source pads inside its durable writer.
    #[allow(
        clippy::too_many_arguments,
        reason = "independent source and current fence components"
    )]
    pub fn authenticates(
        &self,
        character: [u8; 16],
        session: GameSessionId,
        lease: u64,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        content: [u8; 32],
        binding: [u8; 32],
        key: &str,
        count: usize,
        pads: &[MovementLocalPosition],
    ) -> bool {
        self.state.load(Ordering::Acquire) == 1
            && self.character == character
            && self.session == session
            && self.lease_generation == lease
            && self.owner.runtime_scope() == scope
            && self.owner.scope_generation() == generation
            && self.content == content
            && self.binding == binding
            && self.actors.len() == count
            && self.keys.iter().all(|actual| actual == key)
            && self
                .positions
                .iter()
                .all(|position| pads.contains(position))
            && self
                .positions
                .iter()
                .enumerate()
                .all(|(i, position)| !self.positions[..i].contains(position))
    }
}
impl PreparedNativeActorBatch {
    /// The same reservation state is invalidated on publication or definite
    /// rollback, so a retained proof cannot authorize another native group.
    pub fn proof(&self) -> Result<NativeActorBatchProof, CarrierError> {
        if self.state.load(Ordering::Acquire) != 1 {
            return Err(CarrierError::PlanConflict);
        }
        Ok(NativeActorBatchProof {
            owner: self.owner,
            session: self.session,
            character: *self.character.as_bytes(),
            lease_generation: self.lease_generation,
            content: self.content.0,
            binding: self.binding,
            actors: self.actors.clone(),
            keys: self.keys.clone(),
            positions: self.positions.clone(),
            state: Arc::clone(&self.state),
        })
    }
}
impl ChannelRuntimeV1 {
    /// Prepare every actual native payload before SQL, restoring the exact prior
    /// carrier on refusal. Health comes exclusively from the installed policy.
    pub fn prepare_native_actor_batch(
        &mut self,
        owner: ExactActorRef,
        session: GameSessionId,
        binding: [u8; 32],
        monsters: &[(&str, MovementLocalPosition)],
    ) -> Result<PreparedNativeActorBatch, CarrierError> {
        self.owner_fence()?;
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let lease = self.bound_attacker_lease(owner, session)?;
        if monsters.is_empty() || monsters.len() > 8 || binding == [0; 32] {
            return Err(CarrierError::PlanConflict);
        }
        self.positioned_actor_census()?;
        let before = self.carrier.clone();
        let mut actors = Vec::with_capacity(monsters.len());
        let mut keys = Vec::with_capacity(monsters.len());
        let mut positions = Vec::with_capacity(monsters.len());
        let mut plans = Vec::with_capacity(monsters.len());
        let result = (|| {
            for (key, position) in monsters {
                let policy = self.companion_policy(key)?;
                if policy.is_familiar || !self.companion_creation_available(*position)? {
                    return Err(CarrierError::PlanConflict);
                }
                let actor = self.admit_encounter_actor(
                    &policy.definition_key,
                    policy.maximum_health,
                    *position,
                )?;
                self.install_creature_policy(actor, &policy.definition_key)?;
                let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
                plans.push((index, self.carrier.slots[index].clone()));
                actors.push(actor);
                keys.push(policy.definition_key.clone());
                positions.push(*position);
            }
            Ok(())
        })();
        self.carrier = before.clone();
        result?;
        Ok(PreparedNativeActorBatch {
            owner,
            session,
            character: lease.character_id(),
            lease_generation: lease.generation(),
            content: pinned_content(&self.content),
            binding,
            actors,
            keys,
            positions,
            plans,
            before: Some(before),
            state: Arc::new(AtomicU8::new(0)),
        })
    }
    /// Reserve the existing fixed slots invisibly. An ambiguous SQL COMMIT keeps
    /// these reservations in the existing unresolved SpellCommitWindow.
    pub fn reserve_native_actor_batch(
        &mut self,
        prepared: &mut PreparedNativeActorBatch,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        self.carrier
            .player_slot_index(&self.continuity, prepared.owner.0, prepared.session)?;
        if prepared.state.load(Ordering::Acquire) != 0
            || prepared.content != pinned_content(&self.content)
            || prepared.before.as_ref() != Some(&self.carrier)
        {
            return Err(CarrierError::PlanConflict);
        }
        let before = self.carrier.clone();
        let result = (|| {
            for ((index, plan), actor) in prepared.plans.iter().zip(&prepared.actors) {
                let entry =
                    u32::try_from(*index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
                let Slot::VacantReusable { next_free, .. } = self.carrier.slots[*index] else {
                    return Err(CarrierError::PlanConflict);
                };
                if self.carrier.free_head != Some(entry) {
                    return Err(CarrierError::PlanConflict);
                }
                self.carrier.slots[*index] = Slot::CreatureReserved {
                    generation: actor.actor_local_generation(),
                    owner: prepared.owner,
                    session: prepared.session,
                    planned: Box::new(plan.clone()),
                };
                self.carrier.free_head = next_free;
                self.carrier.occupied.push(entry);
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.carrier = before;
            return Err(error);
        }
        prepared.before = None;
        prepared.state.store(1, Ordering::Release);
        Ok(())
    }
    fn validate_native_actor_batch(
        &self,
        prepared: &PreparedNativeActorBatch,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        // Natural source actors have no player master. Historical receipt
        // resolution may publish/release the same reservation after its creator
        // leaves, under this independently current namespace and content pin.
        if prepared.state.load(Ordering::Acquire) != 1
            || prepared.content != pinned_content(&self.content)
        {
            return Err(CarrierError::PlanConflict);
        }
        for ((index, plan), actor) in prepared.plans.iter().zip(&prepared.actors) {
            match &self.carrier.slots[*index] {
                Slot::CreatureReserved {
                    generation,
                    owner,
                    session,
                    planned,
                } if *generation == actor.actor_local_generation()
                    && *owner == prepared.owner
                    && *session == prepared.session
                    && planned.as_ref() == plan => {}
                _ => return Err(CarrierError::PlanConflict),
            }
        }
        Ok(())
    }
    /// All validation precedes publication; assignments cannot partially fail.
    pub fn install_native_actor_batch(
        &mut self,
        prepared: &PreparedNativeActorBatch,
    ) -> Result<(), CarrierError> {
        self.validate_native_actor_batch(prepared)?;
        self.carrier
            .player_slot_index(&self.continuity, prepared.owner.0, prepared.session)?;
        if !self
            .bound_attacker_lease(prepared.owner, prepared.session)
            .is_ok_and(|lease| {
                lease.character_id() == prepared.character
                    && lease.generation() == prepared.lease_generation
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        for ((index, _), actor) in prepared.plans.iter().zip(&prepared.actors) {
            let reserved = std::mem::replace(
                &mut self.carrier.slots[*index],
                Slot::Exhausted {
                    generation: actor.actor_local_generation(),
                },
            );
            let Slot::CreatureReserved { planned, .. } = reserved else {
                unreachable!("validated native source reservation");
            };
            self.carrier.slots[*index] = *planned;
        }
        prepared.state.store(2, Ordering::Release);
        Ok(())
    }
    /// Only a definite uncommitted result may release these exact reservations.
    pub fn rollback_native_actor_batch(
        &mut self,
        prepared: &PreparedNativeActorBatch,
    ) -> Result<(), CarrierError> {
        self.validate_native_actor_batch(prepared)?;
        for ((index, _), actor) in prepared.plans.iter().zip(&prepared.actors).rev() {
            let entry =
                u32::try_from(*index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            self.carrier.slots[*index] = Slot::VacantReusable {
                generation: actor.actor_local_generation(),
                next_free: self.carrier.free_head,
            };
            self.carrier.free_head = Some(entry);
            self.carrier.unindex_occupied(entry);
        }
        prepared.state.store(3, Ordering::Release);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "bounded native actor test fixture setup"
    )]
    use super::*;
    use crate::{CompiledCreaturePolicy, CreatureFlags};
    fn id(n: u8) -> [u8; 16] {
        [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
    }
    fn policy(name: &str) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: format!("creature:{name}"),
            definition_revision: "test-1".into(),
            display_name: name.into(),
            maximum_health: 812,
            forge_difficulty: None,
            base_speed: 220,
            outfit_look_type: 991,
            object_look_type: None,
            summonable: true,
            convinceable: true,
            mana_cost: Some(250),
            is_familiar: false,
            condition_immunities: Vec::new(),
            preferred_distance: Some(1),
            reward_boss: Some(false),
            blocked_by_nearby_players: false,
            defense: Some(1),
            armor: Some(10),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        }
    }
    fn owner() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        owner_with_policies(vec![policy("rat")])
    }
    fn owner_with_policies(
        records: Vec<CompiledCreaturePolicy>,
    ) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        let world = oteryn_gameplay_foundation::foundation::WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            oteryn_gameplay_foundation::foundation::ChannelId::decode(&id(2)).unwrap(),
            oteryn_gameplay_foundation::foundation::NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
        )
        .unwrap();
        let session =
            oteryn_gameplay_foundation::foundation::GameSessionId::decode(&id(4)).unwrap();
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        runtime
            .install_companion_policies(
                oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicies::from_active_artifact([1; 32], records).unwrap(),
            )
            .unwrap();
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(CharacterId::decode(&id(5)).unwrap(), 1).unwrap(),
            )
            .unwrap();
        (runtime, actor, session)
    }

    fn pair() -> [(&'static str, MovementLocalPosition); 2] {
        [
            (
                "creature:rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
            ),
            (
                "creature:rat",
                MovementLocalPosition {
                    x: 2,
                    y: 0,
                    floor: 0,
                },
            ),
        ]
    }
    #[test]
    fn reserved_native_group_stays_invisible_and_occupied_until_final_resolution() {
        let (mut runtime, actor, session) = owner();
        let mut batch = runtime
            .prepare_native_actor_batch(actor, session, [3; 32], &pair())
            .unwrap();
        assert!(runtime.visible_entities().creatures.is_empty());
        runtime.reserve_native_actor_batch(&mut batch).unwrap();
        let proof = batch.proof().unwrap();
        assert!(
            runtime.visible_entities().creatures.is_empty(),
            "unknown COMMIT cannot publish speculative actors"
        );
        for (_, position) in pair() {
            assert!(
                runtime.position_occupied_by_other(actor, position).unwrap(),
                "reserved capacity blocks every other placement"
            );
        }
        assert!(proof.authenticates(
            id(5),
            session,
            1,
            actor.runtime_scope(),
            actor.scope_generation(),
            runtime.content.server_artifact_digest(),
            [3; 32],
            "creature:rat",
            2,
            &pair().map(|(_, position)| position)
        ));
        assert!(
            !proof.authenticates(
                id(5),
                session,
                1,
                actor.runtime_scope(),
                actor.scope_generation(),
                runtime.content.server_artifact_digest(),
                [4; 32],
                "creature:rat",
                2,
                &pair().map(|(_, position)| position)
            ),
            "another command binding cannot reuse the reservation"
        );
        runtime.install_native_actor_batch(&batch).unwrap();
        assert_eq!(runtime.visible_entities().creatures.len(), 2);
        assert!(runtime.visible_entities().creatures.iter().all(|creature| {
            runtime
                .companion_snapshot(creature.actor)
                .unwrap()
                .state
                .master
                .is_none()
        }));
        assert!(
            runtime.install_native_actor_batch(&batch).is_err(),
            "receipt replay cannot publish two more actors"
        );
        assert!(batch.proof().is_err());
    }
    #[test]
    fn source_group_known_rollback_and_partial_preparation_publish_nothing() {
        let (mut runtime, actor, session) = owner();
        let before = runtime.carrier.clone();
        let same = [pair()[0], pair()[0]];
        assert!(
            runtime
                .prepare_native_actor_batch(actor, session, [3; 32], &same)
                .is_err()
        );
        assert_eq!(
            runtime.carrier, before,
            "partial pair admission restores original fixed slots"
        );
        let mut batch = runtime
            .prepare_native_actor_batch(actor, session, [3; 32], &pair())
            .unwrap();
        runtime.reserve_native_actor_batch(&mut batch).unwrap();
        runtime.rollback_native_actor_batch(&batch).unwrap();
        assert!(runtime.visible_entities().creatures.is_empty());
        for (_, position) in pair() {
            assert!(!runtime.position_occupied_by_other(actor, position).unwrap());
        }
        assert!(
            batch.proof().is_err(),
            "definite rollback invalidates all retained proofs"
        );
    }
    #[test]
    fn stale_source_creator_cannot_publish_a_reserved_natural_group() {
        let (mut runtime, actor, session) = owner();
        let mut batch = runtime
            .prepare_native_actor_batch(actor, session, [3; 32], &pair())
            .unwrap();
        runtime.reserve_native_actor_batch(&mut batch).unwrap();
        runtime.remove_terminal_session(session, actor).unwrap();
        assert!(runtime.install_native_actor_batch(&batch).is_err());
        assert!(runtime.visible_entities().creatures.is_empty());
        runtime.rollback_native_actor_batch(&batch).unwrap();
    }
}
