use super::*;

impl ChannelRuntimeV1 {
    /// Facts of the committed player actor bound to exactly this GameSession.
    pub fn player_control_facts(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Result<PlayerControlFacts, CarrierError> {
        let control_loss =
            self.carrier
                .player_control_loss(&self.continuity, actor.0, game_session_id)?;
        Ok(PlayerControlFacts {
            placement_identity: actor.placement_identity(),
            placement_revision: actor.0.actor_local_generation.0,
            control_loss,
        })
    }
    /// A2: the current bound lease of `actor`'s player slot held by `game_session_id`, read
    /// without a write (ATTACK-1b Charm hooks). Same checks as the commit-time read.
    pub fn bound_attacker_lease(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Result<CharacterLease, CarrierError> {
        self.carrier
            .bound_attacker_lease(&self.continuity, actor.0, game_session_id)
    }
    /// Mirror one committed durable control loss onto the still-present player actor.
    /// Recording the identical decision again is a no-op; a different one conflicts.
    pub fn record_control_loss(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        mark: ControlLossMark,
    ) -> Result<(), CarrierError> {
        self.carrier
            .record_player_control_loss(&self.continuity, actor.0, game_session_id, mark)
    }
    /// Clear the exact recorded loss once the durable same-session recovery committed:
    /// the still-present actor is controlled again. Idempotent when already restored; a
    /// different recorded epoch is a conflict.
    pub fn restore_control(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        epoch: u64,
    ) -> Result<(), CarrierError> {
        self.carrier
            .restore_player_control(&self.continuity, actor.0, game_session_id, epoch)
    }
    /// The grace-expiry token of a control-loss epoch.
    pub const fn grace_expiry_fence(epoch: u64) -> WriteFenceToken {
        WriteFenceToken::ControlLoss(epoch)
    }
    /// The token of a transition id from [`Self::mint_transition_fence`].
    pub const fn transition_fence(id: u64) -> WriteFenceToken {
        WriteFenceToken::Transition(id)
    }
    /// A fresh transition id for a transition other than grace expiry.
    pub fn mint_transition_fence(&mut self) -> Result<u64, CarrierError> {
        let next = self
            .carrier
            .fence_transitions
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        self.carrier.fence_transitions = next;
        Ok(next)
    }
    /// CHARM-DESC-FENCE-LEASE step (a): fence the slot's damage writes for `game_session_id`,
    /// before the durable transition. A grace-expiry token needs the slot's control-loss mark of
    /// that epoch. One transition per session: another token is `WriteFenceBusy`.
    pub fn fence_player_writes(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<WriteFenceSet, CarrierError> {
        self.carrier
            .fence_player_writes(&self.continuity, actor.0, game_session_id, token)
    }
    /// Step (c) for a durable outcome that left the session holding the lease: lift the fence
    /// with its exact token. Any other token is a no-op; returns whether the fence was lifted.
    pub fn lift_player_fence(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<bool, CarrierError> {
        self.carrier
            .lift_player_fence(&self.continuity, actor.0, game_session_id, token)
    }
    /// D295 item 4: the post-grace in-place rebind. One operation detaches the terminal session,
    /// drops its fence and control-loss mark, and binds the newly authorized session and its
    /// newer lease of the same Character. The actor reference is unchanged.
    #[allow(
        dead_code,
        reason = "the post-grace takeover is not composed into the transport yet"
    )]
    pub fn rebind_player_session(
        &mut self,
        actor: ExactActorRef,
        terminal: GameSessionId,
        successor: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
        self.carrier
            .rebind_player_session(&self.continuity, actor.0, terminal, successor, lease)
    }
    /// FND-04B §22: bind a reconstructed slot of a same-session continuation only after its
    /// durable reconcile. Terminal or unprovable refuses and leaves the slot unbound, so no
    /// damage write is admitted for the session.
    #[allow(
        dead_code,
        reason = "process-replacement continuation is not composed yet; the carrier gate is"
    )]
    pub fn bind_continuation(
        &mut self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        reconcile: ContinuationReconcile,
    ) -> Result<(), CarrierError> {
        self.carrier
            .bind_continuation(&self.continuity, actor.0, game_session_id, reconcile)
    }
    /// Test-only census: (committed player actors, pending player reservations).
    #[cfg(any(test, feature = "test-support"))]
    pub fn player_slot_counts(&self) -> (usize, usize) {
        self.carrier
            .slots
            .iter()
            .fold((0, 0), |(committed, pending), slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    ..
                } => (committed + 1, pending),
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: false,
                    ..
                } => (committed, pending + 1),
                _ => (committed, pending),
            })
    }
    /// Test-only census: the position revisions of committed player actors that
    /// stand at the pinned start cell under the pinned context, ascending.
    #[cfg(any(test, feature = "test-support"))]
    pub fn entry_start_player_revisions(&self) -> Vec<u64> {
        let mut revisions: Vec<u64> = self
            .carrier
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.position == self.content.entry_start()
                    && version.context == self.pinned_position_context() =>
                {
                    Some(version.revision)
                }
                _ => None,
            })
            .collect();
        revisions.sort_unstable();
        revisions
    }
    /// Test-only census: the recorded control-loss epochs of committed players, ascending.
    #[cfg(any(test, feature = "test-support"))]
    pub fn player_control_loss_epochs(&self) -> Vec<u64> {
        let mut epochs: Vec<u64> = self
            .carrier
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    lifecycle,
                    ..
                } => lifecycle.control_loss.map(|mark| mark.epoch),
                _ => None,
            })
            .collect();
        epochs.sort_unstable();
        epochs
    }
    /// Test-only census: committed player actors positioned at the pinned
    /// start cell under the pinned context by one initialization (revision 1).
    #[cfg(any(test, feature = "test-support"))]
    pub fn players_positioned_at_entry_start(&self) -> usize {
        self.carrier
            .slots
            .iter()
            .filter(|slot| {
                matches!(slot, Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.position == self.content.entry_start()
                    && version.revision == 1
                    && version.context == self.pinned_position_context())
            })
            .count()
    }
}

impl ChannelActorCarrier {
    pub fn reserve_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, CarrierError> {
        let actor_ref = self.admit_inner(
            continuity,
            ActorState(0),
            Some(game_session_id),
            false,
            None,
            false,
        )?;
        Ok(PlayerActorReservation {
            game_session_id,
            actor_ref,
        })
    }
    pub fn commit_reserved_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        reservation: PlayerActorReservation,
    ) -> Result<ExactActorRef, CarrierError> {
        let index = self.validate_ref(continuity, reservation.actor_ref)?;
        // SPELL-LOCK-2 §1.3: a slot reserved by a pending spell batch refuses retryably.
        self.assert_slot_spell_unreserved(index)?;
        match &mut self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(game_session_id),
                committed,
                ..
            } if *generation == reservation.actor_ref.actor_local_generation.0
                && *game_session_id == reservation.game_session_id
                && !*committed =>
            {
                *committed = true;
                Ok(ExactActorRef(reservation.actor_ref))
            }
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }
    pub(super) fn rollback_reserved_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        reservation: PlayerActorReservation,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, reservation.actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(game_session_id),
                committed,
                ..
            } if *generation == reservation.actor_ref.actor_local_generation.0
                && *game_session_id == reservation.game_session_id
                && !*committed => {}
            _ => return Err(CarrierError::PlayerReservationMismatch),
        }
        self.remove(continuity, reservation.actor_ref).map(|_| ())
    }
    pub fn remove_terminal_player(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, actor.0)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(stored_session),
                committed,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && *stored_session == game_session_id
                && *committed => {}
            _ => return Err(CarrierError::PlayerReservationMismatch),
        }
        self.remove(continuity, actor.0).map(|_| ())
    }
    pub(super) fn contains_committed_player(
        &self,
        continuity: &NamespaceContinuityGuard,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
    ) -> bool {
        let Ok(index) = self.validate_ref(continuity, actor.0) else {
            return false;
        };
        matches!(
            &self.slots[index],
            Slot::Occupied {
                generation,
                game_session_id: Some(stored_session),
                committed: true,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && *stored_session == game_session_id
        )
    }
    pub(super) fn player_slot_index(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<usize, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        match &self.slots[index] {
            Slot::Occupied {
                generation,
                game_session_id: Some(bound),
                committed: true,
                ..
            } if *generation == actor_ref.actor_local_generation.0 && *bound == game_session_id => {
                Ok(index)
            }
            Slot::Occupied { generation, .. } | Slot::CreatureOccupied { generation, .. }
                if *generation == actor_ref.actor_local_generation.0 =>
            {
                Err(CarrierError::PlayerReservationMismatch)
            }
            _ => Err(CarrierError::StaleActorGeneration),
        }
    }
    pub fn player_control_loss(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<Option<ControlLossMark>, CarrierError> {
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        match &self.slots[index] {
            Slot::Occupied { lifecycle, .. } => Ok(lifecycle.control_loss),
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }
    pub fn record_player_control_loss(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        mark: ControlLossMark,
    ) -> Result<(), CarrierError> {
        if mark.epoch == 0 {
            return Err(CarrierError::ControlLossConflict);
        }
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        self.assert_slot_spell_unreserved(index)?;
        match &mut self.slots[index] {
            Slot::Occupied { lifecycle, .. } => match lifecycle.control_loss {
                None => {
                    lifecycle.control_loss = Some(mark);
                    Ok(())
                }
                Some(existing) if existing == mark => Ok(()),
                _ => Err(CarrierError::ControlLossConflict),
            },
            _ => Err(CarrierError::ControlLossConflict),
        }
    }
    pub fn restore_player_control(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        epoch: u64,
    ) -> Result<(), CarrierError> {
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        self.assert_slot_spell_unreserved(index)?;
        match &mut self.slots[index] {
            Slot::Occupied { lifecycle, .. } => match lifecycle.control_loss {
                Some(mark) if mark.epoch == epoch => {
                    lifecycle.control_loss = None;
                    // CHARM-DESC-FENCE-LEASE §3 item 3: a resume lifts the fence of its epoch.
                    let generation = actor_ref.actor_local_generation.0;
                    if let Some(entry) = self
                        .attackers
                        .iter_mut()
                        .find(|entry| entry.index == index && entry.generation == generation)
                        && entry.authority.fence == Some(WriteFenceToken::ControlLoss(epoch))
                    {
                        entry.authority.fence = None;
                    }
                    Ok(())
                }
                None => Ok(()),
                _ => Err(CarrierError::ControlLossConflict),
            },
            _ => Err(CarrierError::ControlLossConflict),
        }
    }
    pub(super) fn bound_attacker_lease(
        &self,
        continuity: &NamespaceContinuityGuard,
        attacker: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<CharacterLease, CarrierError> {
        // The three checks, in order: the slot holds the command's session; it is not fenced;
        // its lease is bound.
        let index =
            self.player_slot_index(continuity, attacker, game_session_id)
                .map_err(|error| match error {
                    CarrierError::PlayerReservationMismatch
                    | CarrierError::StaleActorGeneration => CarrierError::SupersededAttackerSession,
                    other => other,
                })?;
        let authority = self
            .attacker_authority(index, attacker.actor_local_generation.0)
            .unwrap_or_default();
        if authority.fence.is_some() {
            return Err(CarrierError::SupersededAttackerSession);
        }
        authority
            .lease
            .ok_or(CarrierError::SupersededAttackerSession)
    }
    fn attacker_authority(&self, index: usize, generation: u64) -> Option<PlayerAttackerAuthority> {
        self.attackers
            .iter()
            .find(|entry| entry.index == index && entry.generation == generation)
            .map(|entry| entry.authority)
    }
    /// The authority entry of the committed player slot `actor_ref` bound to `game_session_id`,
    /// inserted empty when absent.
    fn attacker_authority_mut(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
    ) -> Result<&mut PlayerAttackerAuthority, CarrierError> {
        let index = self.player_slot_index(continuity, actor_ref, game_session_id)?;
        let generation = actor_ref.actor_local_generation.0;
        let at = match self.attackers.iter().position(|entry| entry.index == index) {
            Some(at) => {
                // A recycled slot's stale entry; `remove` normally prunes it first.
                if self.attackers[at].generation != generation {
                    self.attackers[at] = PlayerAttackerEntry {
                        index,
                        generation,
                        authority: PlayerAttackerAuthority::default(),
                    };
                }
                at
            }
            None => {
                self.attackers
                    .try_reserve(1)
                    .map_err(|_| CarrierError::AllocationFailed)?;
                self.attackers.push(PlayerAttackerEntry {
                    index,
                    generation,
                    authority: PlayerAttackerAuthority::default(),
                });
                self.attackers.len() - 1
            }
        };
        Ok(&mut self.attackers[at].authority)
    }
    test_support_pub! {
        fn bind_attacker_lease(
            &mut self,
            continuity: &NamespaceContinuityGuard,
            actor_ref: ActorRef,
            game_session_id: GameSessionId,
            lease: CharacterLease,
        ) -> Result<(), CarrierError> {
            let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
            match authority.lease {
                None => {
                    authority.lease = Some(lease);
                    Ok(())
                }
                Some(bound) if bound == lease => Ok(()),
                Some(_) => Err(CarrierError::AttackerBindingConflict),
            }
        }
    }
    test_support_pub! {
        fn fence_player_writes(
            &mut self,
            continuity: &NamespaceContinuityGuard,
            actor_ref: ActorRef,
            game_session_id: GameSessionId,
            token: WriteFenceToken,
        ) -> Result<WriteFenceSet, CarrierError> {
            if let WriteFenceToken::ControlLoss(epoch) = token
                && self
                    .player_control_loss(continuity, actor_ref, game_session_id)?
                    .is_none_or(|mark| mark.epoch != epoch)
            {
                return Err(CarrierError::ControlLossConflict);
            }
            let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
            match authority.fence {
                None => {
                    authority.fence = Some(token);
                    Ok(WriteFenceSet::Fenced)
                }
                Some(set) if set == token => Ok(WriteFenceSet::Joined),
                Some(_) => Err(CarrierError::WriteFenceBusy),
            }
        }
    }
    test_support_pub! {
        fn lift_player_fence(
            &mut self,
            continuity: &NamespaceContinuityGuard,
            actor_ref: ActorRef,
            game_session_id: GameSessionId,
            token: WriteFenceToken,
        ) -> Result<bool, CarrierError> {
            let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
            if authority.fence == Some(token) {
                authority.fence = None;
                return Ok(true);
            }
            Ok(false)
        }
    }
    pub fn rebind_player_session(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        terminal: GameSessionId,
        successor: GameSessionId,
        lease: CharacterLease,
    ) -> Result<(), CarrierError> {
        // SPELL-LOCK-2 §1.3: a slot reserved by a pending spell batch keeps its session binding
        // until that batch installs or releases; the rebind refuses retryably before any write.
        let index = self.player_slot_index(continuity, actor_ref, terminal)?;
        self.assert_slot_spell_unreserved(index)?;
        let authority = self.attacker_authority_mut(continuity, actor_ref, terminal)?;
        // Only a newer lease of the same Character, under a different session, is a successor.
        match authority.lease {
            Some(bound)
                if bound.character_id() == lease.character_id()
                    && bound.generation() < lease.generation()
                    && successor != terminal => {}
            _ => return Err(CarrierError::AttackerBindingConflict),
        }
        // One step under the runtime lock: the terminal session's fence goes with its binding.
        *authority = PlayerAttackerAuthority {
            lease: Some(lease),
            fence: None,
        };
        if let Slot::Occupied {
            game_session_id,
            lifecycle,
            ..
        } = &mut self.slots[index]
        {
            *game_session_id = Some(successor);
            lifecycle.control_loss = None;
        }
        Ok(())
    }
    pub fn bind_continuation(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        game_session_id: GameSessionId,
        reconcile: ContinuationReconcile,
    ) -> Result<(), CarrierError> {
        // Terminal: only the post-grace path may attach control. Unprovable: fail closed. Both
        // leave the slot unbound, so no damage write is admitted for the session.
        let ContinuationReconcile::Proven(lease) = reconcile else {
            return Err(CarrierError::ContinuationRefused);
        };
        let authority = self.attacker_authority_mut(continuity, actor_ref, game_session_id)?;
        // A reconstructed slot carries no fence and no other binding.
        if authority.fence.is_some() || authority.lease.is_some_and(|bound| bound != lease) {
            return Err(CarrierError::ContinuationRefused);
        }
        authority.lease = Some(lease);
        Ok(())
    }
}
