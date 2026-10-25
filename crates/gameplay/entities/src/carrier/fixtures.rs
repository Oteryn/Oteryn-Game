use super::*;

impl ChannelRuntimeV1 {
    /// Synthetic context is confined to tests. A committed session still goes through
    /// the actual runtime reservation and carrier position initialization checks.
    #[cfg(any(test, feature = "test-support"))]
    pub fn initialize_movement_test_position(
        &mut self,
        actor: ExactActorRef,
        position: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let context = self.test_position_context();
        self.carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }
    /// Test only: `initialize_movement_test_position` under the active pin's position context,
    /// for consumers that read occupancy or commit steps beside pinned creatures.
    #[cfg(any(test, feature = "test-support"))]
    pub fn initialize_pinned_test_position(
        &mut self,
        actor: ExactActorRef,
        position: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let context = self.pinned_position_context();
        self.carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }
    #[cfg(any(test, feature = "test-support"))]
    const fn test_position_context(&self) -> PreProductionPositionContext {
        PreProductionPositionContext {
            world_id: self.binding.world_id,
            channel_id: self.binding.channel_id,
            scope_generation: self.binding.scope_generation,
            coordinate_frame_marker: 11,
            map_revision_marker: 12,
            content_generation_marker: 13,
        }
    }
    /// Explicit Game-owned qualification declaration. The caller independently holds
    /// current qualified tile and SQL Item collision proofs through this owner turn.
    /// Health and policy come from this active runtime, not fixture health constants.
    #[cfg(any(test, feature = "test-support"))]
    pub fn realize_native_qualification_spawn(
        &mut self,
        expected_pin: ChannelContentPin,
        definition_key: &str,
        position: MovementLocalPosition,
    ) -> Result<runtime_actor_companion::CompanionSnapshot, CarrierError> {
        self.owner_fence()?;
        if self.spawn_activation.is_some() {
            return Err(CarrierError::PlanConflict);
        }
        if self.content != expected_pin {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let policy = self.companion_policy(definition_key)?;
        if policy.is_familiar {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        // The census refuses foreign physical contexts; reserved/pending destinations
        // are checked separately so qualification cannot displace another owner.
        self.positioned_actor_census()?;
        let context = self.pinned_position_context();
        let cell = LocalPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        if self.carrier.cell_occupied(context, cell)
            || self.carrier.slots.iter().any(|slot| match slot {
                Slot::Occupied {
                    committed: true,
                    position: None,
                    ..
                }
                | Slot::CreatureOccupied { position: None, .. }
                | Slot::Occupied {
                    committed: false, ..
                } => true,
                Slot::Occupied { spell_combat, .. }
                | Slot::CreatureOccupied { spell_combat, .. } => spell_combat
                    .pending_owner
                    .as_ref()
                    .and_then(|p| p.destination())
                    .is_some_and(|(_, destination)| destination == position),
                _ => false,
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        let definition = SpawnDefinition::new(
            vec![(position.x, position.y, position.floor)],
            1,
            60_000_000,
            5_000_000,
            policy.definition_key.clone(),
            policy.maximum_health,
        )?;
        let slots_before = self.carrier.slots.clone();
        let free_before = self.carrier.free_head;
        let occupied_before = self.carrier.occupied.clone();
        let spawns_before = self.carrier.spawns.clone();
        let result = (|| {
            self.carrier.realize_spawn(
                &self.continuity,
                SpawnSourceId(u16::MAX),
                definition,
                context,
            )?;
            let actor = self
                .carrier
                .spawn_cell_state(SpawnSourceId(u16::MAX), 0)?
                .live
                .ok_or(CarrierError::PlanConflict)?;
            self.install_creature_policy(actor, definition_key)?;
            self.companion_snapshot(actor)
        })();
        if result.is_err() {
            self.carrier.slots = slots_before;
            self.carrier.free_head = free_before;
            self.carrier.occupied = occupied_before;
            self.carrier.spawns = spawns_before;
        }
        result
    }
    /// Test only: one live creature at `position`, under the same synthetic context as
    /// [`Self::initialize_movement_test_position`]. Spawn realization is AI-2's carrier path.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_test_creature(
        &mut self,
        position: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        self.admit_monster_lab_creature(position, "test:creature", 20)
    }
    /// Test-only parameterized admission for the native monster laboratory.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_monster_lab_creature(
        &mut self,
        position: MovementLocalPosition,
        creature_key: &str,
        health: i64,
    ) -> Result<ExactActorRef, CarrierError> {
        let context = self.test_position_context();
        let actor =
            self.carrier
                .admit_creature(&self.continuity, ActorState(0), creature_key, health)?;
        self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )?;
        Ok(ExactActorRef(actor))
    }
    /// Test only: actual fixed-slot creature admission with the active pin's position
    /// context, for consumers that independently require qualified current policy reads.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_pinned_test_creature(
        &mut self,
        position: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        let context = self.pinned_position_context();
        let actor =
            self.carrier
                .admit_creature(&self.continuity, ActorState(0), "test:creature", 20)?;
        self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )?;
        Ok(ExactActorRef(actor))
    }
    /// Test only: despawn one actor, so its generation goes stale.
    #[cfg(any(test, feature = "test-support"))]
    pub fn remove_test_actor(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub(super) const TEST_ALLOCATION_FAILURE_CAPACITY: usize = u32::MAX as usize;

/// Foundation-only factory. Path-included Foundation integration crates do not import Content;
/// the Movement library tests supply real Content claims outside this module.
#[cfg(any(test, feature = "test-support"))]
pub struct MovementActorFixture {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ExactActorRef,
}

#[cfg(any(test, feature = "test-support"))]
impl MovementActorFixture {
    pub fn new(position: MovementLocalPosition, creature: bool) -> Result<Self, CarrierError> {
        fn uuid(seed: u8) -> [u8; 16] {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        }
        let grant = PreProductionContinuityGrant {
            world_id: WorldId::decode(&uuid(10)).map_err(|_| CarrierError::WrongScope)?,
            channel_id: ChannelId::decode(&uuid(11)).map_err(|_| CarrierError::WrongScope)?,
            scope_generation: ScopeOwnershipGeneration::new(1)
                .map_err(|_| CarrierError::WrongScope)?,
        };
        let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant);
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1)?;
        let actor = if creature {
            carrier.admit_creature(&owner, ActorState(1), "engineering:creature", 20)?
        } else {
            carrier.admit(&owner, ActorState(1))?
        };
        let context = PreProductionPositionContext {
            world_id: owner.world_id,
            channel_id: owner.channel_id,
            scope_generation: owner.current_generation,
            coordinate_frame_marker: 11,
            map_revision_marker: 12,
            content_generation_marker: 13,
        };
        carrier.initialize_position(
            &owner,
            actor,
            context,
            LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )?;
        Ok(Self {
            owner,
            carrier,
            actor: ExactActorRef(actor),
        })
    }

    pub const fn world_id(&self) -> WorldId {
        self.owner.world_id
    }
    pub const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    pub fn current_position(&self) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(&self.owner, self.actor.0)
    }

    pub fn raw_position(&self) -> Result<(MovementLocalPosition, u64), CarrierError> {
        let version = self
            .carrier
            .read_position(&self.owner, self.actor.0)?
            .version;
        Ok((
            MovementLocalPosition {
                x: version.position.x,
                y: version.position.y,
                floor: version.position.floor,
            },
            version.revision,
        ))
    }

    pub fn borrow_position(&mut self) -> CurrentOwnerMovementPosition<'_> {
        self.carrier.current_owner_movement_position(&self.owner)
    }

    pub fn wrong_context(&self) -> Result<MovementPositionContext, CarrierError> {
        let mut context = self
            .carrier
            .read_position(&self.owner, self.actor.0)?
            .version
            .context;
        context.coordinate_frame_marker += 1;
        Ok(MovementPositionContext(context))
    }

    pub fn intervening_write(&mut self) -> Result<(), CarrierError> {
        let expected = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.compare_commit_position(
            &self.owner,
            expected,
            expected.version.context,
            expected.version.position,
        )?;
        Ok(())
    }

    pub fn recycle(&mut self) -> Result<(), CarrierError> {
        let old = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.remove(&self.owner, self.actor.0)?;
        let actor = self.carrier.admit(&self.owner, ActorState(2))?;
        self.carrier.initialize_position(
            &self.owner,
            actor,
            old.version.context,
            old.version.position,
        )?;
        self.actor = ExactActorRef(actor);
        Ok(())
    }

    pub fn become_creature(&mut self) -> Result<(), CarrierError> {
        let old = self.carrier.read_position(&self.owner, self.actor.0)?;
        self.carrier.remove(&self.owner, self.actor.0)?;
        let actor =
            self.carrier
                .admit_creature(&self.owner, ActorState(3), "engineering:creature", 20)?;
        self.carrier.initialize_position(
            &self.owner,
            actor,
            old.version.context,
            old.version.position,
        )?;
        self.actor = ExactActorRef(actor);
        Ok(())
    }

    pub fn advance_owner(&mut self) -> Result<(), CarrierError> {
        self.owner.advance(PreProductionContinuityGrant {
            world_id: self.owner.world_id,
            channel_id: self.owner.channel_id,
            scope_generation: ScopeOwnershipGeneration::new(2)
                .map_err(|_| CarrierError::WrongScope)?,
        })
    }

    pub fn exhaust_position_revision(&mut self) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.owner, self.actor.0)?;
        if let Slot::Occupied {
            position: Some(version),
            ..
        } = &mut self.carrier.slots[index]
        {
            version.revision = u64::MAX;
            Ok(())
        } else {
            Err(CarrierError::MovementCreatureUnavailable)
        }
    }
}

/// Test-only D1 death source (`VSL_COMBAT_FIXTURE_PROFILE`): one positioned
/// fixture creature in a pre-production Channel carrier of the given scope
/// generation. A death exists only through the owner's committed lethal path
/// and Combat's projection; the fixture returns the death key that projection
/// names and never constructs one. Nothing here exists outside `cfg(test)`.
#[cfg(any(test, feature = "test-support"))]
pub struct CombatDeathFixture {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ExactActorRef,
    /// D4: `strike_by` derives one fixture command per distinct occurrence text, so the same
    /// text replays as the same `(session, sequence)` command (test-only bookkeeping).
    strike_commands: Vec<(String, u64)>,
    /// ATTACK-1b: the bound player attacker of [`Self::swing`], when built with one.
    attacker: Option<ExactActorRef>,
}

#[cfg(any(test, feature = "test-support"))]
impl CombatDeathFixture {
    /// Fixture creature health: test/evidence only, not Reference behavior.
    pub const HEALTH: i64 = 20;
    const TARGET: &'static str = "fixture:vsl-combat.creature";
    pub const POSITION: LocalPosition = LocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };

    pub fn new(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
    ) -> Result<Self, CarrierError> {
        Self::new_with_health(world_id, channel_id, scope_generation, Self::HEALTH)
    }

    /// Test-only source-health variant; default fixture and all production paths stay intact.
    pub fn new_with_health(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        health: i64,
    ) -> Result<Self, CarrierError> {
        Self::build(world_id, channel_id, scope_generation, health, 1)
    }

    /// ATTACK-1b: the fixture creature plus one bound player attacker (Character `character`,
    /// lease generation 1) of the fixture swing session, for [`Self::swing`].
    pub fn new_with_bound_attacker(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        character: CharacterId,
    ) -> Result<Self, CarrierError> {
        let mut fixture = Self::build(world_id, channel_id, scope_generation, Self::HEALTH, 2)?;
        let lease =
            CharacterLease::new(character, 1).map_err(|_| CarrierError::InvalidActorIdentity)?;
        let attacker = fixture.carrier.admit_bound_test_attacker(
            &fixture.owner,
            Self::swing_session()?,
            lease,
        )?;
        fixture.attacker = Some(attacker);
        Ok(fixture)
    }

    fn build(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        health: i64,
        capacity: usize,
    ) -> Result<Self, CarrierError> {
        let mut owner =
            NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
                world_id,
                channel_id,
                scope_generation,
            });
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, capacity)?;
        let actor = carrier.admit_creature(&owner, ActorState(1), Self::TARGET, health)?;
        let context = PreProductionPositionContext {
            world_id,
            channel_id,
            scope_generation,
            coordinate_frame_marker: 41,
            map_revision_marker: 42,
            content_generation_marker: 43,
        };
        carrier.initialize_position(&owner, actor, context, Self::POSITION)?;
        Ok(Self {
            owner,
            carrier,
            actor: ExactActorRef(actor),
            strike_commands: Vec::new(),
            attacker: None,
        })
    }

    fn swing_session() -> Result<GameSessionId, CarrierError> {
        let mut session = [0_u8; 16];
        session[6] = 0x70;
        session[8] = 0x80;
        session[15] = 2;
        GameSessionId::decode(&session).map_err(|_| CarrierError::InvalidActorIdentity)
    }

    /// ATTACK-1b: one auto-attack swing of the bound attacker, committed under the swing
    /// identity `(lineage command `lineage`, swing_ordinal)` exactly as the Channel owner's
    /// auto-attack drain commits it.
    pub fn swing(
        &mut self,
        lineage: u64,
        swing_ordinal: u16,
        damage: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let attacker = self.attacker.ok_or(CarrierError::InvalidActorIdentity)?;
        let lineage = CommandRef::new(
            Self::swing_session()?,
            oteryn_gameplay_foundation::foundation::CommandId::new(lineage)
                .map_err(|_| CarrierError::InvalidActorIdentity)?,
        );
        let binding = format!(
            "fixture:attack.swing.v1\0{}\0{swing_ordinal}",
            lineage.command_id().get()
        );
        self.carrier
            .current_owner_exact_commit(&self.owner)
            .commit_swing_damage_for_bound_attacker(
                self.actor,
                attacker,
                lineage,
                swing_ordinal,
                OwnerDamageCommand {
                    target: Self::TARGET.as_bytes(),
                    occurrence: &[],
                    binding: binding.as_bytes(),
                    damage,
                },
            )
    }

    pub const fn actor(&self) -> ExactActorRef {
        self.actor
    }

    /// One owner-committed damage occurrence. An identical replay returns the
    /// recorded transition without a second write.
    pub fn strike(
        &mut self,
        occurrence: &str,
        damage: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let mut binding = occurrence.as_bytes().to_vec();
        binding.extend_from_slice(b"\0fixture:vsl-combat.strike.v1");
        self.carrier
            .current_owner_exact_commit(&self.owner)
            .commit_damage(
                self.actor,
                OwnerDamageCommand {
                    target: Self::TARGET.as_bytes(),
                    occurrence: occurrence.as_bytes(),
                    binding: &binding,
                    damage,
                },
            )
    }

    /// MAP-ITEM-REF-1 Part B: [`ChannelRuntimeV1::bind_corpse_item`] for the fixture's corpse.
    pub fn bind_corpse_item(
        &mut self,
        item_instance_id: [u8; 16],
        family: &str,
        production_key: &str,
        revision_ref: &str,
    ) -> Result<(), CarrierError> {
        self.carrier.bind_corpse_item_inner(
            self.actor,
            RuntimeCorpseItem {
                item_instance_id,
                family: family.to_owned(),
                production_key: production_key.to_owned(),
                revision_ref: revision_ref.to_owned(),
            },
        )
    }

    /// MAP-ITEM-REF-1 Part B: the fixture's bound corpse Item, if bound.
    pub fn bound_corpse_item_instance_id(&self) -> Option<[u8; 16]> {
        self.carrier
            .bound_corpses(None)
            .into_iter()
            .find(|corpse| corpse.identity == self.actor.corpse_identity())
            .map(|corpse| corpse.item.item_instance_id)
    }

    /// D132/D3-3 + D4: the same owner-committed damage occurrence as [`Self::strike`],
    /// additionally attributed to `attacker`'s running per-creature damage total. The fixture
    /// stands in for the future transport: it maps each distinct `occurrence` text to one
    /// monotonic `CommandId` of one fixed fixture `GameSessionId`, so the same text replays as
    /// the same command and a new text is a new, higher-sequence command.
    pub fn strike_by(
        &mut self,
        occurrence: &str,
        damage: i64,
        attacker: CharacterId,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let sequence = match self
            .strike_commands
            .iter()
            .find(|(text, _)| text == occurrence)
        {
            Some((_, sequence)) => *sequence,
            None => {
                let sequence = u64::try_from(self.strike_commands.len())
                    .map_err(|_| CarrierError::CapacityArithmeticOverflow)?
                    + 1;
                self.strike_commands.push((occurrence.to_owned(), sequence));
                sequence
            }
        };
        let mut session = [0_u8; 16];
        session[6] = 0x70;
        session[8] = 0x80;
        session[15] = 1;
        let command = CommandRef::new(
            GameSessionId::decode(&session).map_err(|_| CarrierError::InvalidActorIdentity)?,
            oteryn_gameplay_foundation::foundation::CommandId::new(sequence)
                .map_err(|_| CarrierError::InvalidActorIdentity)?,
        );
        let mut binding = occurrence.as_bytes().to_vec();
        binding.extend_from_slice(b"\0fixture:vsl-combat.strike.v1");
        self.carrier
            .current_owner_exact_commit(&self.owner)
            .commit_damage_for_attacker(
                self.actor,
                AttackerCommand::new(attacker, 1, command, 0),
                OwnerDamageCommand {
                    target: Self::TARGET.as_bytes(),
                    occurrence: occurrence.as_bytes(),
                    binding: &binding,
                    damage,
                },
            )
    }

    /// D132/D3-3: the current deterministic top-damage `CharacterId` tracked for this fixture's
    /// creature, [`CurrentOwnerCombatDeath::top_damage_character`].
    pub fn top_damage_character(&mut self) -> Result<Option<CharacterId>, CarrierError> {
        self.carrier
            .current_owner_combat_death(&self.owner)
            .top_damage_character(self.actor)
    }

    /// Administrative despawn: the creature leaves without a semantic death.
    pub fn despawn(&mut self) -> Result<(), CarrierError> {
        self.carrier.remove(&self.owner, self.actor.0).map(|_| ())
    }

    /// D2b: the same current-owner Combat-death capability
    /// [`ChannelRuntimeV1::borrow_combat_death`] grants, for
    /// `reward_occurrence` tests.
    pub fn borrow_combat_death(&mut self) -> CurrentOwnerCombatDeath<'_> {
        self.carrier.current_owner_combat_death(&self.owner)
    }

    /// The scope moved: owner continuity advances to `next`, while this
    /// carrier still serves its original generation.
    pub fn advance_owner(&mut self, next: ScopeOwnershipGeneration) -> Result<(), CarrierError> {
        self.owner.advance(PreProductionContinuityGrant {
            world_id: self.owner.world_id,
            channel_id: self.owner.channel_id,
            scope_generation: next,
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
impl ChannelActorCarrier {
    /// Test only: a committed player slot for `session` with `lease` bound, as fresh admission
    /// leaves it (A2).
    pub fn admit_bound_test_attacker(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        session: GameSessionId,
        lease: CharacterLease,
    ) -> Result<ExactActorRef, CarrierError> {
        let reservation = self.reserve_player(continuity, session)?;
        let actor = self.commit_reserved_player(continuity, reservation)?;
        self.bind_attacker_lease(continuity, actor.0, session, lease)?;
        Ok(actor)
    }

    /// Test only: the committed player slot of `session`.
    pub fn test_player_actor(&self, session: GameSessionId) -> Option<ExactActorRef> {
        self.slots
            .iter()
            .enumerate()
            .find_map(|(index, slot)| match slot {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(bound),
                    committed: true,
                    ..
                } if *bound == session => Some(ExactActorRef(ActorRef::from_owner_parts(
                    self.world_id,
                    self.channel_id,
                    self.scope_generation,
                    ActorLocalId(u32::try_from(index + 1).ok()?),
                    ActorLocalGeneration(*generation),
                ))),
                _ => None,
            })
    }
}

impl ChannelRuntimeV1 {
    /// Test-only native damage writer for source-specific lab fixtures. Target identity is
    /// read from the actual current slot; caller cannot supply an alternate target binding.
    #[cfg(any(test, feature = "test-support"))]
    pub fn commit_monster_lab_damage(
        &mut self,
        actor: ExactActorRef,
        occurrence: &[u8],
        damage: i64,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let idx = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let target = match &self.carrier.slots[idx] {
            Slot::CreatureOccupied {
                generation,
                target_identity,
                ..
            } if *generation == actor.0.actor_local_generation.0 => target_identity.to_vec(),
            _ => return Err(CarrierError::StaleActorGeneration),
        };
        let mut binding = occurrence.to_vec();
        binding.extend_from_slice(b"\0monster-lab-native-damage-v1\0");
        binding.extend_from_slice(&damage.to_be_bytes());
        self.carrier
            .current_owner_exact_commit(&self.continuity)
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: &target,
                    occurrence,
                    binding: &binding,
                    damage,
                },
            )
    }
    /// Test-only player geometry under its independently selected current ContentPin.
    /// Session identity and one-time initialization use the real carrier checks.
    #[cfg(any(test, feature = "test-support"))]
    pub fn initialize_source_pinned_lab_player_position(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        at: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        let context = self.pinned_position_context();
        self.carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }
    /// Test-only admission through the REAL current ContentPin position context. Existing
    /// movement-test fixture context is intentionally unchanged for its separate test contract.
    #[cfg(any(test, feature = "test-support"))]
    pub fn admit_source_pinned_lab_creature(
        &mut self,
        at: MovementLocalPosition,
        key: &str,
        health: i64,
    ) -> Result<ExactActorRef, CarrierError> {
        let context = self.pinned_position_context();
        let actor = self
            .carrier
            .admit_creature(&self.continuity, ActorState(0), key, health)?;
        self.carrier.initialize_position(
            &self.continuity,
            actor,
            context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        )?;
        Ok(ExactActorRef(actor))
    }
}

#[cfg(any(test, feature = "test-support"))]
impl crate::carrier::CombatDeathFixture {
    /// Combat's idempotent projection of the committed lethal occurrence and
    /// the durable death key it names, with the corpse position.
    pub fn project_death(
        &mut self,
    ) -> Result<
        (
            crate::carrier::CreatureDeathOccurrenceKey,
            crate::carrier::MovementLocalPosition,
        ),
        CarrierError,
    > {
        let actor = self.actor();
        let mut combat = self.borrow_combat_death();
        let projection = project_fixed_one_creature_death(&mut combat, actor)?;
        Ok((projection.occurrence().death_key(), projection.position()))
    }
}
