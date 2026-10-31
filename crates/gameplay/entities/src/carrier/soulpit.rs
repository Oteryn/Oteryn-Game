//! Soulpit's physical variants in the existing actor carrier. Every admission requires
//! the shared activity owner's actual Instance continuity; this module allocates no scope.
use super::runtime_actor_companion::{CompanionState, CompiledCreaturePolicies};
use super::*;
use oteryn_gameplay_domain::domain::soulpit::{
    BossAbility, MONSTERS_PER_WAVE, OPPRESSOR_INTERVAL_MICROS, SoulpitStack,
    enrage_retained_percent, overpower_damage, wave_stacks,
};
use oteryn_gameplay_foundation::foundation::condition::ConditionStore;

#[path = "soulpit/oppressor.rs"]
mod oppressor;
pub use oppressor::{SoulpitOppressorCast, SoulpitOppressorTarget};
#[cfg(test)]
#[path = "soulpit/tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulpitBossState {
    pub ability: BossAbility,
    pub next_oppressor_at_micros: u64,
    pub ordinal: u64,
}

/// A physical actor's source role, retained in its native combat slot. Logical monster
/// identity is persisted by Soulpit's lifecycle writer, never supplied by a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulpitCreatureTag {
    pub handoff_id: [u8; 16],
    pub wave: u8,
    pub monster_id: u64,
    pub stack: SoulpitStack,
    pub is_summon: bool,
    pub boss: Option<SoulpitBossState>,
}

impl SoulpitCreatureTag {
    pub fn experience(self, base: u64) -> Result<u64, CarrierError> {
        self.stack
            .experience(base, self.is_summon)
            .map_err(|_| CarrierError::DamageOverflow)
    }

    /// combat.cpp:2849-2865 truncates critical damage before monster.cpp's stack multiplier.
    pub fn outgoing_damage(self, raw: u64, critical_roll: u8) -> Result<(u64, bool), CarrierError> {
        let (raw, critical) = if self
            .boss
            .is_some_and(|boss| boss.ability == BossAbility::Overpower)
        {
            let outcome =
                overpower_damage(raw, critical_roll).map_err(|_| CarrierError::InvalidDamage)?;
            (outcome.amount, outcome.critical)
        } else {
            (raw, false)
        };
        let scaled = u128::from(raw) * u128::from(self.stack.attack_percent()) / 100;
        Ok((
            u64::try_from(scaled).map_err(|_| CarrierError::DamageOverflow)?,
            critical,
        ))
    }

    /// The donor health-change callback applies to both incoming damage and healing.
    /// Its literal reversed <=20% interval is deliberately preserved by the domain rule.
    pub fn incoming_health_change(
        self,
        amount: i64,
        health: i64,
        maximum: i64,
    ) -> Result<i64, CarrierError> {
        if amount < 0 {
            return Err(CarrierError::InvalidDamage);
        }
        if !self
            .boss
            .is_some_and(|boss| boss.ability == BossAbility::Enrage)
        {
            return Ok(amount);
        }
        let retained = enrage_retained_percent(
            u64::try_from(health).map_err(|_| CarrierError::InvalidCreatureHealth)?,
            u64::try_from(maximum).map_err(|_| CarrierError::InvalidCreatureHealth)?,
        )
        .map_err(|_| CarrierError::InvalidCreatureHealth)?;
        i64::try_from(i128::from(amount) * i128::from(retained) / 100)
            .map_err(|_| CarrierError::DamageOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulpitWaveRequest {
    pub handoff_id: [u8; 16],
    pub wave: u8,
    pub species: String,
    /// Runtime positions use the source z; native map positions use its checked negation.
    pub positions: [MovementLocalPosition; MONSTERS_PER_WAVE],
    pub boss_ability: BossAbility,
    pub now_micros: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulpitWaveActor {
    pub actor: ExactActorRef,
    pub monster_id: u64,
    pub stack: SoulpitStack,
}

#[derive(Debug, Clone)]
pub struct SoulpitActorFacts {
    pub actor: ExactActorRef,
    pub tag: SoulpitCreatureTag,
    pub species: String,
    pub health: i64,
    pub position: MovementLocalPosition,
}

#[derive(Debug, Clone)]
pub struct SoulpitDeathFacts {
    pub actor: ExactActorRef,
    pub tag: SoulpitCreatureTag,
    pub principal: TopDamagePrincipal,
    pub damage_shares: Vec<RewardDamageShare>,
    pub death_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WaveReceipt {
    request: SoulpitWaveRequest,
    content: [u8; 32],
    actors: [Option<SoulpitWaveActor>; MONSTERS_PER_WAVE],
}

/// Physical admission receipts only. The durable SoulpitEncounter owns phases, survivors,
/// deadlines and rewards. Keeping failed attempts prevents a retry from creating extra adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulpitPhysicalState {
    handoff_id: [u8; 16],
    waves: Vec<WaveReceipt>,
    next_summon_id: u64,
}

/// Test-only physical kernel below activity admission, matching the existing carrier
/// fixtures. No production grant constructor or private continuity field is exposed.
#[cfg(any(test, feature = "test-support"))]
pub fn physical_test_kernel(
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    capacity: usize,
) -> Result<(ChannelActorCarrier, NamespaceContinuityGuard), CarrierError> {
    let mut continuity =
        NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
            runtime_scope: scope,
            scope_generation: generation,
        });
    let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, capacity)?;
    Ok((carrier, continuity))
}

impl ChannelActorCarrier {
    pub(super) fn require_soulpit_instance(
        &self,
        continuity: &NamespaceContinuityGuard,
    ) -> Result<(), CarrierError> {
        self.validate_current_continuity(continuity)?;
        if !matches!(self.runtime_scope, RuntimeScopeRefV1::Instance { .. }) {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }

    pub fn soulpit_tag(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<Option<SoulpitCreatureTag>, CarrierError> {
        let index = self.validate_summon_actor(continuity, actor)?;
        match &self.slots[index] {
            Slot::CreatureOccupied { spell_combat, .. } => Ok(spell_combat.soulpit),
            _ => Err(CarrierError::NotCreature),
        }
    }

    pub fn soulpit_actors(
        &self,
        continuity: &NamespaceContinuityGuard,
        handoff_id: [u8; 16],
    ) -> Result<Vec<SoulpitActorFacts>, CarrierError> {
        self.require_soulpit_instance(continuity)?;
        let mut actors = Vec::new();
        for (index, slot) in self.slots.iter().enumerate() {
            let Slot::CreatureOccupied {
                generation,
                target_identity,
                health,
                spell_combat,
                position: Some(position),
                ..
            } = slot
            else {
                continue;
            };
            let Some(tag) = spell_combat
                .soulpit
                .filter(|tag| tag.handoff_id == handoff_id)
            else {
                continue;
            };
            actors.push(SoulpitActorFacts {
                actor: ExactActorRef(ActorRef::from_scoped_owner_parts(
                    self.runtime_scope,
                    self.scope_generation,
                    ActorLocalId(
                        u32::try_from(index + 1)
                            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                    ),
                    ActorLocalGeneration(*generation),
                )),
                tag,
                species: std::str::from_utf8(target_identity)
                    .map_err(|_| CarrierError::InvalidCreatureTarget)?
                    .to_owned(),
                health: *health,
                position: MovementLocalPosition {
                    x: position.position.x,
                    y: position.position.y,
                    floor: position.position.floor,
                },
            });
        }
        Ok(actors)
    }

    pub fn capture_soulpit_death(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<SoulpitDeathFacts, CarrierError> {
        let tag = self.soulpit_defeated(continuity, actor)?;
        let index = self.validate_ref(continuity, actor.0)?;
        let Slot::CreatureOccupied {
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        let damage_shares = damage_contributors
            .entries
            .iter()
            .map(|entry| RewardDamageShare {
                character: entry.character,
                total: entry.total,
                lease_generation: entry.high_water.map(|(lease, _, _, _)| lease),
                session: entry.high_water.map(|(_, session, _, _)| session),
                last_damage_at_ms: entry.last_damage_at_ms,
            })
            .collect();
        let death_at_ms = self
            .committed_lethal_receipt_inner(continuity, actor.0)?
            .projection
            .death_at_ms;
        Ok(SoulpitDeathFacts {
            actor,
            tag,
            principal: self.top_damage_contributor_inner(continuity, actor.0)?,
            damage_shares,
            death_at_ms,
        })
    }

    /// Executes each donor spawn attempt once. A blocked cell or full existing carrier is
    /// recorded as None, matching Game.createSoulPitMonster's skipped failure; no redraw.
    pub fn spawn_soulpit_wave(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        content: &ChannelContentPin,
        policies: &CompiledCreaturePolicies,
        request: SoulpitWaveRequest,
        mut ground_allows: impl FnMut(MovementLocalPosition) -> bool,
    ) -> Result<[Option<SoulpitWaveActor>; MONSTERS_PER_WAVE], CarrierError> {
        self.require_soulpit_instance(continuity)?;
        if request.handoff_id == [0; 16]
            || content.world_id() != self.runtime_scope.world_id()
            || policies.source_digest() != content.server_artifact_digest()
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let stacks = wave_stacks(request.wave).map_err(|_| CarrierError::InvalidCommitBinding)?;
        let next_oppressor = request
            .now_micros
            .checked_add(OPPRESSOR_INTERVAL_MICROS)
            .ok_or(CarrierError::DamageOverflow)?;
        let policy = policies
            .policies()
            .iter()
            .find(|policy| policy.definition_key == request.species)
            .cloned()
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        if let Some(state) = &self.soulpit {
            if state.handoff_id != request.handoff_id {
                return Err(CarrierError::PlanConflict);
            }
            if let Some(prior) = state
                .waves
                .iter()
                .find(|receipt| receipt.request.wave == request.wave)
            {
                let mut replay = request.clone();
                replay.now_micros = prior.request.now_micros;
                return if prior.request == replay && prior.content == policies.source_digest() {
                    Ok(prior.actors)
                } else {
                    Err(CarrierError::PlanConflict)
                };
            }
        }
        // Validate every source stack before admitting the first actor. A later overflow
        // must not leave earlier attempts unreceipted and create them again on retry.
        let base_health = u64::try_from(policy.maximum_health)
            .map_err(|_| CarrierError::InvalidCreatureHealth)?;
        let mut maximum_health = [0; MONSTERS_PER_WAVE];
        for (maximum, stack) in maximum_health.iter_mut().zip(stacks) {
            *maximum = stack
                .scaled_health(base_health)
                .ok()
                .and_then(|value| i64::try_from(value).ok())
                .ok_or(CarrierError::DamageOverflow)?;
        }
        for (position, stack) in request.positions.iter().zip(stacks) {
            if position.floor != 8
                || !(32362..=32390).contains(&position.x)
                || !(31132..=31153).contains(&position.y)
                || (stack == SoulpitStack::Boss && (position.x, position.y) != (32376, 31144))
            {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
        }
        let marker = |digest: [u8; 32]| {
            u64::from_be_bytes([
                digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6],
                digest[7],
            ])
        };
        let context = PreProductionPositionContext {
            scope: positions::PositionScope::new(self.runtime_scope)?,
            scope_generation: self.scope_generation,
            coordinate_frame_marker: marker(content.frame_binding_digest()),
            map_revision_marker: marker(content.map_revision_digest()),
            content_generation_marker: content.activation_sequence(),
        };
        self.validate_position_context(context)?;
        let state = self.soulpit.get_or_insert_with(|| {
            Box::new(SoulpitPhysicalState {
                handoff_id: request.handoff_id,
                waves: Vec::new(),
                next_summon_id: 29,
            })
        });
        state
            .waves
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut actors = [None; MONSTERS_PER_WAVE];
        for (ordinal, (position, stack)) in
            request.positions.iter().copied().zip(stacks).enumerate()
        {
            let local = LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            };
            if !ground_allows(position) || self.cell_occupied(context, local) {
                continue;
            }
            let maximum = maximum_health[ordinal];
            let actor = match self.admit_creature(
                continuity,
                ActorState(0),
                &policy.definition_key,
                maximum,
            ) {
                Ok(actor) => actor,
                Err(CarrierError::CapacityExceeded | CarrierError::AllocationFailed) => continue,
                Err(error) => return Err(error),
            };
            if let Err(error) = self.initialize_position(continuity, actor, context, local) {
                self.remove(continuity, actor)?;
                return Err(error);
            }
            let monster_id = u64::from(request.wave - 1) * 7 + ordinal as u64 + 1;
            let index = self.validate_ref(continuity, actor)?;
            if let Slot::CreatureOccupied {
                spell_combat,
                companion,
                ..
            } = &mut self.slots[index]
            {
                spell_combat.soulpit = Some(SoulpitCreatureTag {
                    handoff_id: request.handoff_id,
                    wave: request.wave,
                    monster_id,
                    stack,
                    is_summon: false,
                    boss: (stack == SoulpitStack::Boss).then_some(SoulpitBossState {
                        ability: request.boss_ability,
                        next_oppressor_at_micros: next_oppressor,
                        ordinal: 0,
                    }),
                });
                *companion = Some(Box::new(CompanionState {
                    policy: Arc::clone(&policy),
                    master: None,
                    creation_speed_delta: 0,
                    conditions: ConditionStore::default(),
                    outfit_look_type: policy.outfit_look_type,
                    object_look_type: policy.object_look_type,
                    expires_at_unix: None,
                    lifecycle_epoch: 0,
                    party_protection: false,
                    created_at_us: Some(request.now_micros),
                    familiar_defense: None,
                }));
            }
            actors[ordinal] = Some(SoulpitWaveActor {
                actor: ExactActorRef(actor),
                monster_id,
                stack,
            });
        }
        let state = self.soulpit.as_mut().ok_or(CarrierError::PlanConflict)?;
        state.waves.push(WaveReceipt {
            request,
            content: policies.source_digest(),
            actors,
        });
        Ok(actors)
    }

    /// Called by the existing native summon admission after it establishes the parent link.
    /// The summon inherits the stack and no boss ability, XP, ordinary loot or skill-loss flag.
    pub fn inherit_soulpit_summon(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        parent: ExactActorRef,
        child: ExactActorRef,
    ) -> Result<Option<SoulpitCreatureTag>, CarrierError> {
        let Some(parent_tag) = self.soulpit_tag(continuity, parent)? else {
            return Ok(None);
        };
        self.require_soulpit_instance(continuity)?;
        if self
            .native_summons
            .child(child)
            .is_none_or(|link| link.parent != parent)
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let index = self.validate_ref(continuity, child.0)?;
        self.assert_slot_spell_unreserved(index)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if let Some(tag) = spell_combat.soulpit {
            return if tag.handoff_id == parent_tag.handoff_id
                && tag.stack == parent_tag.stack
                && tag.is_summon
            {
                Ok(Some(tag))
            } else {
                Err(CarrierError::PlanConflict)
            };
        }
        if *health <= 0 || spell_combat.forge.is_some() || spell_combat.echo.is_some() {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let maximum = parent_tag
            .stack
            .scaled_health(
                u64::try_from(spell_combat.maximum_health)
                    .map_err(|_| CarrierError::InvalidCreatureHealth)?,
            )
            .ok()
            .and_then(|value| i64::try_from(value).ok())
            .ok_or(CarrierError::DamageOverflow)?;
        let state = self.soulpit.as_mut().ok_or(CarrierError::PlanConflict)?;
        if state.handoff_id != parent_tag.handoff_id {
            return Err(CarrierError::PlanConflict);
        }
        let next = state
            .next_summon_id
            .checked_add(1)
            .ok_or(CarrierError::DamageOverflow)?;
        let tag = SoulpitCreatureTag {
            monster_id: state.next_summon_id,
            is_summon: true,
            boss: None,
            ..parent_tag
        };
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &mut self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        *health = maximum;
        spell_combat.maximum_health = maximum;
        spell_combat.soulpit = Some(tag);
        state.next_summon_id = next;
        Ok(Some(tag))
    }

    /// A durable Defeated acknowledgement is emitted only for the real lethal HP receipt.
    pub fn soulpit_defeated(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<SoulpitCreatureTag, CarrierError> {
        self.require_soulpit_instance(continuity)?;
        let tag = self
            .soulpit_tag(continuity, actor)?
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        self.committed_lethal_receipt_inner(continuity, actor.0)?;
        Ok(tag)
    }

    /// Source master death removes its intrinsic children. Use the existing HP receipt
    /// owner under Instance continuity, avoiding the legacy Channel-only summon death key.
    pub fn cascade_soulpit_summon_deaths(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        handoff_id: [u8; 16],
    ) -> Result<(), CarrierError> {
        self.require_soulpit_instance(continuity)?;
        let mut children = Vec::new();
        for parent in self
            .soulpit_actors(continuity, handoff_id)?
            .into_iter()
            .filter(|actor| actor.health == 0 && !actor.tag.is_summon)
        {
            for link in self.native_summons.children(parent.actor) {
                children.push((parent.actor, link.child));
            }
        }
        for (parent, child) in children {
            let index = self.validate_ref(continuity, child.0)?;
            let Slot::CreatureOccupied {
                health,
                target_identity,
                ..
            } = &self.slots[index]
            else {
                return Err(CarrierError::NotCreature);
            };
            if *health == 0 {
                continue;
            }
            let damage = *health;
            let target = copy_bounded_binding(target_identity)?;
            let occurrence = format!(
                "soulpit-master-death:{}:{}",
                parent.0.actor_local_id.0, parent.0.actor_local_generation.0
            );
            let binding = format!("{occurrence}\0SOULPIT-master-lethal");
            self.commit_encounter_single_damage(
                continuity,
                child.0,
                OwnerDamageCommand {
                    target: &target,
                    occurrence: occurrence.as_bytes(),
                    binding: binding.as_bytes(),
                    damage,
                },
                None,
                false,
                None,
                None,
            )?;
        }
        Ok(())
    }

    pub fn retire_soulpit_defeated(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        self.soulpit_defeated(continuity, actor)?;
        self.remove(continuity, actor.0)?;
        Ok(())
    }

    pub fn retire_soulpit(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        handoff_id: [u8; 16],
    ) -> Result<usize, CarrierError> {
        self.require_soulpit_instance(continuity)?;
        let mut actors = Vec::new();
        for (index, slot) in self.slots.iter().enumerate() {
            if let Slot::CreatureOccupied {
                generation,
                spell_combat,
                ..
            } = slot
                && spell_combat
                    .soulpit
                    .is_some_and(|tag| tag.handoff_id == handoff_id)
            {
                actors.push(ActorRef::from_scoped_owner_parts(
                    self.runtime_scope,
                    self.scope_generation,
                    ActorLocalId(
                        u32::try_from(index + 1)
                            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                    ),
                    ActorLocalGeneration(*generation),
                ));
            }
        }
        for actor in &actors {
            // Removing a master also retires its children through the existing carrier.
            // validate_ref proves authority and bounds, not slot occupancy. Only a
            // same-generation vacant slot is a child already removed by this pass.
            let index = self.validate_ref(continuity, *actor)?;
            match &self.slots[index] {
                Slot::CreatureOccupied { generation, .. }
                    if *generation == actor.actor_local_generation.0 =>
                {
                    self.remove(continuity, *actor)?;
                }
                Slot::VacantReusable { generation, .. }
                    if *generation == actor.actor_local_generation.0 => {}
                _ => return Err(CarrierError::StaleActorGeneration),
            }
        }
        Ok(actors.len())
    }
}

impl ChannelRuntimeV1 {
    /// Projects only Soulpit's combat tag/health into an existing real creature fixture
    /// so the current magnitude consumer can be tested before #2769 composes its Instance
    /// wrapper. This is not activity or Instance admission; the real Instance admission
    /// positive/negative tests exercise the unchanged production guards separately.
    #[cfg(any(test, feature = "test-support"))]
    pub fn project_soulpit_oppressor_for_test(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(actor)?;
        let snapshot = self.companion_snapshot(actor)?;
        let maximum = SoulpitStack::Boss
            .scaled_health(
                u64::try_from(snapshot.state.policy.maximum_health)
                    .map_err(|_| CarrierError::InvalidCreatureHealth)?,
            )
            .map_err(|_| CarrierError::DamageOverflow)?;
        let maximum = i64::try_from(maximum).map_err(|_| CarrierError::DamageOverflow)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if spell_combat.soulpit.is_some() {
            return Err(CarrierError::PlanConflict);
        }
        *health = maximum;
        spell_combat.maximum_health = maximum;
        spell_combat.soulpit = Some(SoulpitCreatureTag {
            handoff_id: [0x79; 16],
            wave: 4,
            monster_id: 28,
            stack: SoulpitStack::Boss,
            is_summon: false,
            boss: Some(SoulpitBossState {
                ability: BossAbility::Oppressor,
                next_oppressor_at_micros: OPPRESSOR_INTERVAL_MICROS,
                ordinal: 0,
            }),
        });
        Ok(())
    }

    pub fn soulpit_hex_delta(
        &self,
        actor: ExactActorRef,
        now_micros: u64,
    ) -> Result<oteryn_gameplay_foundation::foundation::condition::ConditionBuffDelta, CarrierError>
    {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let session = match &self.carrier.slots[index] {
            Slot::Occupied {
                committed: true,
                game_session_id: Some(session),
                ..
            } => Some(*session),
            Slot::CreatureOccupied { .. } => None,
            _ => return Err(CarrierError::InvalidActorIdentity),
        };
        let store = self
            .carrier
            .scoped_actor_conditions(&self.continuity, actor, session)?;
        if !store.accepts_time(now_micros) {
            return Err(CarrierError::PlanConflict);
        }
        Ok(store.intense_hex_delta_at(now_micros))
    }

    pub fn soulpit_hex_percent(
        &self,
        actor: ExactActorRef,
        now_micros: u64,
    ) -> Result<(i32, u8), CarrierError> {
        let delta = self.soulpit_hex_delta(actor, now_micros)?;
        Ok((
            100 + delta.damage_dealt,
            (100 + delta.healing_received) as u8,
        ))
    }

    pub fn soulpit_variant(&self, actor: ExactActorRef) -> Option<SoulpitCreatureTag> {
        self.carrier
            .soulpit_tag(&self.continuity, actor)
            .ok()
            .flatten()
    }

    /// Existing melee and spell consumers share raw source scaling. The current Channel
    /// owner cannot admit a Soulpit variant; the shared Instance runtime reuses this carrier.
    pub fn encounter_source_damage(
        &self,
        actor: ExactActorRef,
        raw: u64,
        critical_roll: u8,
    ) -> Result<(u64, bool), CarrierError> {
        if let Some(tag) = self.soulpit_variant(actor) {
            return tag.outgoing_damage(raw, critical_roll);
        }
        let scaled = u128::from(raw) * u128::from(self.echo_attack_percent(actor)) / 100;
        Ok((
            u64::try_from(scaled).map_err(|_| CarrierError::DamageOverflow)?,
            false,
        ))
    }
}
