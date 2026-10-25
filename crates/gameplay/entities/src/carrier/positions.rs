use super::*;

impl MovementPositionSnapshot {
    pub const fn facing(self) -> Option<MovementFacing> {
        self.0.version.facing
    }
    pub const fn position(self) -> MovementLocalPosition {
        let position = self.0.version.position;
        MovementLocalPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }
    }

    pub const fn context(self) -> MovementPositionContext {
        MovementPositionContext(self.0.version.context)
    }

    pub const fn revision(self) -> u64 {
        self.0.version.revision
    }

    pub const fn world_id(self) -> WorldId {
        self.0.version.context.world_id
    }
}

impl CurrentOwnerMovementPosition<'_> {
    /// Turn in place through the same exact-snapshot owner fence as a cardinal step.
    pub fn commit_turn(
        &mut self,
        expected: MovementPositionSnapshot,
        facing: MovementFacing,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let index = self
            .carrier
            .validate_ref(self.continuity, expected.0.actor_ref)?;
        let mut committed = self.carrier.compare_commit_position(
            self.continuity,
            expected.0,
            expected.0.version.context,
            expected.0.version.position,
        )?;
        committed.version.facing = Some(facing);
        match &mut self.carrier.slots[index] {
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                *position = Some(committed.version);
            }
            _ => unreachable!("position commit proved occupied exact slot"),
        }
        Ok(MovementPositionSnapshot(committed))
    }

    pub fn commit_spell_relocation<P: SpellRelocationProof>(
        &mut self,
        proof: P,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let (actor, expected, next) = proof.parts();
        if actor.0 != expected.0.actor_ref {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let snapshot = self.carrier.compare_commit_position(
            self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )?;
        Ok(MovementPositionSnapshot(snapshot))
    }
    pub fn read(&self, actor: ExactActorRef) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_movement_position(self.continuity, actor.0)
    }

    pub fn commit_cardinal(
        &mut self,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let before = expected.position();
        let east = before.x.checked_add(1).is_some_and(|x| x == next.x) && next.y == before.y;
        let west = before.x.checked_sub(1).is_some_and(|x| x == next.x) && next.y == before.y;
        let south = before.y.checked_add(1).is_some_and(|y| y == next.y) && next.x == before.x;
        let north = before.y.checked_sub(1).is_some_and(|y| y == next.y) && next.x == before.x;
        if next.floor != before.floor || !(east || west || south || north) {
            return Err(CarrierError::MovementNonCardinal);
        }
        self.carrier.commit_movement_position(
            self.continuity,
            expected.0,
            LocalPosition {
                x: next.x,
                y: next.y,
                floor: next.floor,
            },
        )
    }
}

#[cfg(any(test, feature = "test-support"))]
impl PreProductionPositionContext {
    /// Test builds only: a synthetic position context for the named scope. Production
    /// contexts come only from the runtime's pinned assignment.
    pub const fn for_test(
        world_id: WorldId,
        channel_id: ChannelId,
        scope_generation: ScopeOwnershipGeneration,
        coordinate_frame_marker: u64,
        map_revision_marker: u64,
        content_generation_marker: u64,
    ) -> Self {
        Self {
            world_id,
            channel_id,
            scope_generation,
            coordinate_frame_marker,
            map_revision_marker,
            content_generation_marker,
        }
    }
}

impl ChannelRuntimeV1 {
    /// The Movement owner context of this runtime's fixed Content pin; a step is eligible only
    /// for a position initialized under exactly this context.
    pub fn pinned_movement_context(&self) -> MovementPositionContext {
        MovementPositionContext(self.pinned_position_context())
    }
    /// Current-owner census (#162 5868482467, shared-lease P1 repair r4121956127): the local
    /// positions of every committed, currently-present player actor under this runtime's pinned
    /// Movement context — the same per-slot position this type's own `read()` reads for one
    /// actor, read here for all of them in one Channel-owner work item. No parallel tracking of
    /// actor positions is introduced: this reads the existing `slots` store directly, exactly as
    /// the existing test-only census methods below do, and reveals no slot index or generation to
    /// its caller. A world-object occupancy check intersects these positions' cells with a
    /// LocalObject's own collision footprint; it must read them under the same runtime lock and
    /// the same work item as the transition it gates, so the check is TOCTOU-free.
    /// Actual fixed-slot occupancy, including living creatures. Missing/foreign-context
    /// positions are unknown and refuse traversal; they are never interpreted as clear.
    pub fn position_occupied_by_other(
        &self,
        actor: ExactActorRef,
        destination: MovementLocalPosition,
    ) -> Result<bool, CarrierError> {
        let source_index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let context = self.pinned_position_context();
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            if index == source_index {
                continue;
            }
            let spell_combat = match slot {
                Slot::Occupied { spell_combat, .. }
                | Slot::CreatureOccupied { spell_combat, .. } => Some(spell_combat),
                _ => None,
            };
            if spell_combat
                .and_then(|state| state.pending_owner.as_ref())
                .and_then(|reservation| reservation.destination())
                .is_some_and(|(relocated, reserved)| relocated != actor && reserved == destination)
            {
                return Ok(true);
            }
            let version = match slot {
                Slot::Occupied {
                    committed: true,
                    position,
                    ..
                } => *position,
                Slot::CreatureOccupied {
                    health, position, ..
                } if *health > 0 => *position,
                Slot::CreatureReserved { planned, .. } => match planned.as_ref() {
                    Slot::CreatureOccupied { position, .. } => *position,
                    _ => return Err(CarrierError::PlanConflict),
                },
                Slot::NpcOccupied { position, .. } => Some(*position),
                _ => continue,
            }
            .ok_or(CarrierError::PositionSnapshotMismatch)?;
            if version.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            if version.position
                == (LocalPosition {
                    x: destination.x,
                    y: destination.y,
                    floor: destination.floor,
                })
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Whether `actor`'s own unresolved spell reservation relocates somebody into `cell`.
    /// `position_occupied_by_other` skips the issuer's slot, so a world-object gate that must
    /// not close a tile onto the issuer's pending destination asks this as well.
    pub fn reserved_into_by_own_pending(
        &self,
        actor: ExactActorRef,
        cell: MovementLocalPosition,
    ) -> Result<bool, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        Ok(match self.carrier.slots.get(index) {
            Some(
                Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. },
            ) => spell_combat
                .pending_owner
                .as_ref()
                .and_then(|reservation| reservation.destination())
                .is_some_and(|(_, reserved)| reserved == cell),
            _ => false,
        })
    }
    pub fn committed_player_positions(&self) -> Vec<MovementLocalPosition> {
        let context = self.pinned_position_context();
        self.carrier
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.context == context => Some(MovementLocalPosition {
                    x: version.position.x,
                    y: version.position.y,
                    floor: version.position.floor,
                }),
                _ => None,
            })
            .collect()
    }
    /// The carrier's slots: every actor of this runtime, of any kind, takes one. A ledger with
    /// at most one entry per slot is bounded by it.
    pub fn actor_slot_capacity(&self) -> usize {
        self.carrier.slots.len()
    }
    /// AI-4 (GAME-AI-01 slice §4.6): true only while `actor` names a live (`health > 0`)
    /// creature generation of this runtime. A player, a stale or dead generation, a vacant slot
    /// or another scope is false.
    pub fn contains_live_creature(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        matches!(
            &self.carrier.slots[index],
            Slot::CreatureOccupied { generation, health, .. }
                if *generation == actor.0.actor_local_generation.0 && *health > 0
        )
    }
    /// AI-4 (§4.6): the current position of a live creature or committed player actor, read in
    /// the owner work item that uses it. A value snapshot, never an authority token.
    pub fn read_actor_position(
        &self,
        actor: ExactActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier
            .read_position(&self.continuity, actor.0)
            .map(MovementPositionSnapshot)
    }
    /// Complete bounded census of the owner's positioned committed players and
    /// live creatures. These current value snapshots grant no mutation authority;
    /// each consumer must revalidate exact actors, session and positions at commit.
    pub fn positioned_actor_census(
        &self,
    ) -> Result<
        Vec<(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )>,
        CarrierError,
    > {
        self.census(false)
    }
    /// ATTACK0-RL-03: the creature melee census. It also lists committed players whose control
    /// was lost and who stay positioned; the consumer admits only those still in fight.
    pub fn positioned_melee_census(
        &self,
    ) -> Result<
        Vec<(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )>,
        CarrierError,
    > {
        self.census(true)
    }
    fn census(
        &self,
        include_control_lost: bool,
    ) -> Result<
        Vec<(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )>,
        CarrierError,
    > {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        // SPAWN-CEILINGS-1 (Codex 4242381774): the census reads the occupied index in slot order,
        // so a pass costs the actors present, not the capacity, and its order is unchanged.
        let mut indexes = Vec::new();
        indexes
            .try_reserve_exact(self.carrier.occupied.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        indexes.extend_from_slice(&self.carrier.occupied);
        indexes.sort_unstable();
        let mut result = Vec::new();
        result
            .try_reserve(indexes.len())
            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        for index in indexes {
            let index =
                usize::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            let Some(slot) = self.carrier.slots.get(index) else {
                continue;
            };
            let (generation, session) = match slot {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: true,
                    lifecycle,
                    position: Some(_),
                    ..
                } if include_control_lost || lifecycle.control_loss.is_none() => {
                    (*generation, Some(*session))
                }
                Slot::CreatureOccupied {
                    generation,
                    health,
                    position: Some(_),
                    ..
                } if *health > 0 => (*generation, None),
                _ => continue,
            };
            let actor = ExactActorRef(ActorRef::from_owner_parts(
                self.binding.world_id,
                self.binding.channel_id,
                self.binding.scope_generation,
                ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(generation),
            ));
            let position = self.read_actor_position(actor)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            result.push((actor, position, session));
        }
        Ok(result)
    }
    /// Revalidate a previously staged complete census without allocating after COMMIT.
    pub fn validate_positioned_actor_census(
        &self,
        expected: &[(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )],
    ) -> Result<(), CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        let mut cursor = 0usize;
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            let (generation, session) = match slot {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: true,
                    lifecycle,
                    position: Some(_),
                    ..
                } if lifecycle.control_loss.is_none() => (*generation, Some(*session)),
                Slot::CreatureOccupied {
                    generation,
                    health,
                    position: Some(_),
                    ..
                } if *health > 0 => (*generation, None),
                _ => continue,
            };
            let actor = ExactActorRef(ActorRef::from_owner_parts(
                self.binding.world_id,
                self.binding.channel_id,
                self.binding.scope_generation,
                ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(generation),
            ));
            let position = self.read_actor_position(actor)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            if expected.get(cursor) != Some(&(actor, position, session)) {
                return Err(CarrierError::PlayerReservationMismatch);
            }
            cursor += 1;
        }
        if cursor != expected.len() {
            return Err(CarrierError::PlayerReservationMismatch);
        }
        Ok(())
    }
    /// Local, bounded owner lookup for an independently resolved current GameSession.
    /// Names and privacy permission are resolved by their owners before using this read.
    pub fn positioned_player_for_session(
        &self,
        session: GameSessionId,
    ) -> Result<Option<(ExactActorRef, MovementPositionSnapshot)>, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let mut result = None;
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            let Slot::Occupied {
                generation,
                game_session_id: Some(current),
                committed: true,
                lifecycle,
                position: Some(_),
                ..
            } = slot
            else {
                continue;
            };
            if lifecycle.control_loss.is_some() || *current != session {
                continue;
            }
            if result.is_some() {
                return Err(CarrierError::PlayerReservationMismatch);
            }
            let actor = ExactActorRef(ActorRef::from_owner_parts(
                self.binding.world_id,
                self.binding.channel_id,
                self.binding.scope_generation,
                ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(*generation),
            ));
            let position = self.read_actor_position(actor)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            result = Some((actor, position));
        }
        Ok(result)
    }
    /// The compact position context of this runtime's fixed Content pin.
    pub(super) fn pinned_position_context(&self) -> PreProductionPositionContext {
        let prefix = |digest: &[u8; 32]| {
            u64::from_be_bytes([
                digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6],
                digest[7],
            ])
        };
        PreProductionPositionContext {
            world_id: self.binding.world_id,
            channel_id: self.binding.channel_id,
            scope_generation: self.binding.scope_generation,
            coordinate_frame_marker: prefix(&self.content.frame_binding_digest()),
            map_revision_marker: prefix(&self.content.map_revision_digest()),
            content_generation_marker: self.content.activation_sequence(),
        }
    }
    /// #935 first-entry position initialization: the Channel owner writes the
    /// pinned generation's start cell for one committed, unpositioned player
    /// actor. The context comes only from this runtime's assignment binding and
    /// Content pin. An exact retry reconciles the completed initialization
    /// without another write (later Movement is not undone); a stale or
    /// recycled actor, or one positioned under another context, gets no write.
    pub fn initialize_first_entry_position(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<FirstEntryPosition, CarrierError> {
        let context = self.pinned_position_context();
        match self.carrier.initialize_position(
            &self.continuity,
            actor.0,
            context,
            self.content.entry_start(),
        ) {
            Ok(snapshot) => Ok(FirstEntryPosition::Initialized(MovementPositionSnapshot(
                snapshot,
            ))),
            Err(CarrierError::PositionAlreadyInitialized) => {
                let current = self.carrier.read_position(&self.continuity, actor.0)?;
                if current.version.context == context {
                    Ok(FirstEntryPosition::Reconciled(MovementPositionSnapshot(
                        current,
                    )))
                } else {
                    Err(CarrierError::PositionAlreadyInitialized)
                }
            }
            Err(error) => Err(error),
        }
    }
}

impl ChannelActorCarrier {
    pub(super) fn current_owner_movement_position<'a>(
        &'a mut self,
        continuity: &'a NamespaceContinuityGuard,
    ) -> CurrentOwnerMovementPosition<'a> {
        CurrentOwnerMovementPosition {
            carrier: self,
            continuity,
        }
    }
    /// GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.5 ("Creature slots become
    /// Movement-capable actors of the same Movement owner"): a creature slot with a live
    /// (`health > 0`) generation reads exactly like a player slot here. `validate_ref` already
    /// rejects a stale generation, a vacant slot or an exhausted one before this is reached, and
    /// `read_position`/`compare_commit_position` already branch on `Slot::CreatureOccupied`
    /// identically to `Slot::Occupied` for position facts. NPC-BEHAVIOUR-0 §4.4 also admits
    /// the existing NPC slot to this Movement read; the separate combat/AI position read
    /// continues to exclude NPCs.
    pub(super) fn read_movement_position(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        if let Slot::NpcOccupied {
            generation,
            position,
            ..
        } = &self.slots[index]
        {
            if *generation != actor_ref.actor_local_generation.0 {
                return Err(CarrierError::StaleActorGeneration);
            }
            if position.actor_local_id != actor_ref.actor_local_id
                || position.actor_local_generation != actor_ref.actor_local_generation
            {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            return Ok(MovementPositionSnapshot(PositionSnapshot {
                actor_ref,
                version: *position,
            }));
        }
        self.read_position(continuity, actor_ref)
            .map(MovementPositionSnapshot)
    }
    /// See [`Self::read_movement_position`]: a creature slot commits its cardinal step through
    /// the same compare-commit path a player's does (§4.5). The NPC branch writes that
    /// actor's existing position only after current-owner, exact-snapshot and cardinal
    /// checks. It does not grant NPCs the spell relocation or combat position paths.
    fn commit_movement_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next: LocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, expected.actor_ref)?;
        if let Slot::NpcOccupied {
            generation,
            position,
            ..
        } = &self.slots[index]
        {
            self.validate_position_context(expected.version.context)?;
            if *generation != expected.actor_ref.actor_local_generation.0
                || position.actor_local_id != expected.actor_ref.actor_local_id
                || position.actor_local_generation != expected.actor_ref.actor_local_generation
                || *position != expected.version
            {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            let facing = match (
                next.floor == position.position.floor,
                next.x.checked_sub(position.position.x),
                next.y.checked_sub(position.position.y),
            ) {
                (true, Some(1), Some(0)) => MovementFacing::East,
                (true, Some(-1), Some(0)) => MovementFacing::West,
                (true, Some(0), Some(1)) => MovementFacing::South,
                (true, Some(0), Some(-1)) => MovementFacing::North,
                _ => return Err(CarrierError::MovementNonCardinal),
            };
            let version = VersionedPosition {
                position: next,
                revision: position
                    .revision
                    .checked_add(1)
                    .ok_or(CarrierError::PositionRevisionExhausted)?,
                facing: Some(facing),
                ..*position
            };
            let Slot::NpcOccupied { position, .. } = &mut self.slots[index] else {
                unreachable!("exclusively borrowed NPC slot remains current");
            };
            *position = version;
            return Ok(MovementPositionSnapshot(PositionSnapshot {
                actor_ref: expected.actor_ref,
                version,
            }));
        }
        // An exclusive carrier borrow prevents a slot replacement between this check and
        // the existing private exact-snapshot/revision compare-commit.
        self.compare_commit_position(continuity, expected, expected.version.context, next)
            .map(MovementPositionSnapshot)
    }
    pub(super) fn validate_position_context(
        &self,
        context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        if context.world_id != self.world_id
            || context.channel_id != self.channel_id
            || context.scope_generation != self.scope_generation
            || context.coordinate_frame_marker == 0
            || context.map_revision_marker == 0
            || context.content_generation_marker == 0
        {
            return Err(CarrierError::InvalidPreProductionPositionContext);
        }
        Ok(())
    }
    /// Private preproduction-only initial binding; existing actor admission
    /// cannot silently invent a position or a Content activation context.
    pub fn initialize_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        context: PreProductionPositionContext,
        position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        self.assert_slot_spell_unreserved(index)?;
        self.validate_position_context(context)?;
        match &mut self.slots[index] {
            Slot::CreatureOccupied {
                generation,
                health: 0,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::CreatureNotActionable)
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: stored @ None,
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: stored @ None,
                health: 1..,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                let version = VersionedPosition {
                    actor_local_id: actor_ref.actor_local_id,
                    actor_local_generation: actor_ref.actor_local_generation,
                    context,
                    position,
                    revision: 1,
                    facing: None,
                };
                *stored = Some(version);
                Ok(PositionSnapshot { actor_ref, version })
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(_),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(_),
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::PositionAlreadyInitialized)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }
    /// One direct occupied-slot read under the independently current owner.
    pub fn read_position(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(version),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(version),
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                if version.actor_local_id != actor_ref.actor_local_id
                    || version.actor_local_generation != actor_ref.actor_local_generation
                {
                    return Err(CarrierError::PositionSnapshotMismatch);
                }
                Ok(PositionSnapshot {
                    actor_ref,
                    version: *version,
                })
            }
            Slot::Occupied {
                generation,
                committed: true,
                position: None,
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: None,
                ..
            } if *generation == actor_ref.actor_local_generation.0 => {
                Err(CarrierError::PositionUnavailable)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }
    /// Compare and commit exactly one replacement in the same actor slot.
    /// All fallible checks run before the sole mutation and no alternate store
    /// or movement timing authority participates.
    pub fn compare_commit_position(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next_context: PreProductionPositionContext,
        next_position: LocalPosition,
    ) -> Result<PositionSnapshot, CarrierError> {
        self.compare_commit_position_with_summon_lifecycle(
            continuity,
            expected,
            next_context,
            next_position,
            false,
        )
    }
    pub(super) fn compare_commit_position_with_summon_lifecycle(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        expected: PositionSnapshot,
        next_context: PreProductionPositionContext,
        next_position: LocalPosition,
        allow_self_removal: bool,
    ) -> Result<PositionSnapshot, CarrierError> {
        let index = self.validate_ref(continuity, expected.actor_ref)?;
        self.assert_slot_spell_unreserved(index)?;
        if matches!(
            &self.slots[index],
            Slot::CreatureOccupied {
                generation,
                health: 0,
                ..
            } if *generation == expected.actor_ref.actor_local_generation.0
        ) {
            return Err(CarrierError::CreatureNotActionable);
        }
        self.validate_position_context(next_context)?;
        if next_context != expected.version.context {
            return Err(CarrierError::PositionContextMismatch);
        }
        let (generation, current) = match &self.slots[index] {
            Slot::Occupied {
                generation,
                committed: true,
                position: Some(current),
                ..
            }
            | Slot::CreatureOccupied {
                generation,
                position: Some(current),
                ..
            } => (*generation, *current),
            _ => return Err(CarrierError::PositionSnapshotMismatch),
        };
        if generation != expected.actor_ref.actor_local_generation.0
            || current.actor_local_id != expected.actor_ref.actor_local_id
            || current.actor_local_generation != expected.actor_ref.actor_local_generation
            || current != expected.version
        {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let separated_summons =
            self.summons_separated_by_move(continuity, expected.actor_ref, next_position)?;
        if !allow_self_removal
            && separated_summons
                .iter()
                .flatten()
                .any(|child| child.0 == expected.actor_ref)
        {
            return Err(CarrierError::MovementCreatureUnavailable);
        }
        // Restore every store administrative removal touches, including attacker ownership
        // (removal no longer writes the spawn cells, so they need no snapshot).
        let summon_rollback = if separated_summons.iter().any(Option::is_some) {
            Some((
                self.slots.clone(),
                self.free_head,
                self.occupied.clone(),
                self.native_summons.clone(),
                self.native_summon_deaths,
                self.native_summon_death_cursor,
                self.attackers.clone(),
            ))
        } else {
            None
        };
        let revision = current
            .revision
            .checked_add(1)
            .ok_or(CarrierError::PositionRevisionExhausted)?;
        let version = VersionedPosition {
            actor_local_id: expected.actor_ref.actor_local_id,
            actor_local_generation: expected.actor_ref.actor_local_generation,
            context: next_context,
            position: next_position,
            revision,
            facing: match (
                next_position.floor == current.position.floor,
                next_position.x.checked_sub(current.position.x),
                next_position.y.checked_sub(current.position.y),
            ) {
                (true, Some(1), Some(0)) => Some(MovementFacing::East),
                (true, Some(-1), Some(0)) => Some(MovementFacing::West),
                (true, Some(0), Some(1)) => Some(MovementFacing::South),
                (true, Some(0), Some(-1)) => Some(MovementFacing::North),
                _ => current.facing,
            },
        };
        match &mut self.slots[index] {
            Slot::Occupied { position, .. } | Slot::CreatureOccupied { position, .. } => {
                *position = Some(version)
            }
            _ => unreachable!("validated occupied slot"),
        }
        if let Err(error) = self.remove_separated_summons(continuity, separated_summons) {
            if let Some((slots, free, occupied, links, deaths, cursor, attackers)) = summon_rollback
            {
                self.slots = slots;
                self.free_head = free;
                self.occupied = occupied;
                self.native_summons = links;
                self.native_summon_deaths = deaths;
                self.native_summon_death_cursor = cursor;
                self.attackers = attackers;
            }
            return Err(error);
        }
        Ok(PositionSnapshot {
            actor_ref: expected.actor_ref,
            version,
        })
    }
}

impl ChannelRuntimeV1 {
    /// Native occupancy veto under the SAME owner turn. Conservatively include all positioned
    /// actors (even dead/uncontrolled or differently pinned); never infer emptiness from timers.
    pub fn current_cell_occupied_except(
        &self,
        cell: MovementLocalPosition,
        except: ExactActorRef,
    ) -> bool {
        let Ok(excluded) = self.carrier.validate_ref(&self.continuity, except.0) else {
            return true;
        };
        self.carrier.slots.iter().enumerate().any(|(i, slot)| {
            if i == excluded {
                return false;
            }
            let position = match slot {
                Slot::Occupied {
                    position: Some(p), ..
                }
                | Slot::CreatureOccupied {
                    position: Some(p), ..
                } => p.position,
                _ => return false,
            };
            position.x == cell.x && position.y == cell.y && position.floor == cell.floor
        })
    }
}
