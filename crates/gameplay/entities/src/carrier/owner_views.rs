use super::*;

impl CurrentOwnerExactActorCommit<'_> {
    /// Test builds only: read the borrowed carrier. No mutable carrier crosses this boundary.
    #[cfg(any(test, feature = "test-support"))]
    pub fn test_carrier(&self) -> &ChannelActorCarrier {
        self.carrier
    }

    pub fn commit_damage(
        &mut self,
        actor: ExactActorRef,
        command: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier.commit_creature_damage_with_pin_inner(
            self.continuity,
            actor.0,
            command,
            None,
            false,
            self.content,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            self.now_ms,
        )
    }

    /// D132/D3-3 + D141: the same owner-authoritative damage commit, additionally attributing the
    /// applied HP loss to `attacker`'s running per-creature damage total
    /// (`COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`, capped at
    /// [`COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX`]). The replay identity of an attributed
    /// commit is derived by the carrier from `attacker` alone: `command.occurrence` is not read,
    /// so a caller can never present one command's identity under another's bytes. An idempotent
    /// replay never double-attributes: it never reaches the mutation boundary a second time.
    /// Test only: the attacker is supplied, not read from a bound slot. Production writes go
    /// through [`Self::commit_damage_for_bound_attacker`] (A2, D295 item 4).
    #[cfg(any(test, feature = "test-support"))]
    pub fn commit_damage_for_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: AttackerCommand,
        command: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.carrier.commit_creature_damage_with_pin_inner(
            self.continuity,
            actor.0,
            command,
            Some(attacker),
            false,
            self.content,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            self.now_ms,
        )
    }
}

impl CurrentOwnerExactActorCommit<'_> {
    /// A2 (D295 item 4): the current lease of the attacker's bound player slot. The slot must hold
    /// the command's session, carry no write fence and have a bound lease, checked in that order
    /// (CHARM-DESC-FENCE-LEASE §3 item 2); anything else is `SupersededAttackerSession`.
    pub fn bound_attacker_lease(
        &self,
        attacker: ExactActorRef,
        command: CommandRef,
    ) -> Result<CharacterLease, CarrierError> {
        self.carrier
            .bound_attacker_lease(self.continuity, attacker.0, command.game_session_id())
    }

    /// The attributed damage commit whose attacker authority is the bound slot of `attacker`,
    /// read in the same runtime-lock critical section as the write.
    pub fn commit_damage_for_bound_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: ExactActorRef,
        command: CommandRef,
        sub_ordinal: u16,
        damage: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let lease = self.bound_attacker_lease(attacker, command)?;
        self.carrier.commit_creature_damage_with_pin_inner(
            self.continuity,
            actor.0,
            damage,
            Some(AttackerCommand::new(
                lease.character_id(),
                lease.generation(),
                command,
                sub_ordinal,
            )),
            false,
            self.content,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            self.now_ms,
        )
    }

    /// ATTACK-1b swing identity: every swing of one attack lineage reuses the lineage's
    /// `CommandRef` (the `AttackTarget` command), and `swing_ordinal` is the swing's index within
    /// that lineage. The bound-attacker session fence is unchanged. Within one lineage the
    /// ordinal is strictly increasing (a replay returns the retained receipt; a lower or equal
    /// unretained ordinal is `StaleAttackerSequence`); against other commands of the same session
    /// the admission is deferred like a SPELL-BATCH completion, so the high-water mark never
    /// lowers.
    pub fn commit_swing_damage_for_bound_attacker(
        &mut self,
        actor: ExactActorRef,
        attacker: ExactActorRef,
        lineage: CommandRef,
        swing_ordinal: u16,
        damage: OwnerDamageCommand<'_>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        let lease = self.bound_attacker_lease(attacker, lineage)?;
        let command = AttackerCommand::new(
            lease.character_id(),
            lease.generation(),
            lineage,
            swing_ordinal,
        );
        self.carrier
            .swing_lineage_admission(self.continuity, actor.0, command)?;
        // Every u16 swing ordinal is admissible: the lineage's last ordinal (`u16::MAX`) still
        // swings, and attack.rs ends the lineage after it.
        self.carrier.commit_creature_damage_inner_limited(
            self.continuity,
            actor.0,
            damage,
            Some(command),
            false,
            None,
            true,
            self.now_ms,
        )
    }
}

impl CurrentOwnerCombatDeath<'_> {
    pub fn committed_lethal_receipt(
        &self,
        actor: ExactActorRef,
    ) -> Result<CommittedLethalReceipt, CarrierError> {
        self.carrier
            .committed_lethal_receipt_inner(self.continuity, actor.0)
    }

    pub fn project_committed_lethal(
        &mut self,
        receipt: CommittedLethalReceipt,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.carrier
            .project_committed_lethal_inner(self.continuity, receipt, false, false)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn project_committed_lethal_with_failures(
        &mut self,
        receipt: CommittedLethalReceipt,
        fail_before_write: bool,
        fail_after_write: bool,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        self.carrier.project_committed_lethal_inner(
            self.continuity,
            receipt,
            fail_before_write,
            fail_after_write,
        )
    }

    /// D2b: the death key and corpse position of this generation's projected
    /// committed death of `actor`, read from the owner's own projection.
    pub fn projected_death(
        &self,
        actor: ExactActorRef,
    ) -> Result<(CreatureDeathOccurrenceKey, MovementLocalPosition), CarrierError> {
        self.carrier.validate_ref(self.continuity, actor.0)?;
        if self.carrier.is_native_summon(actor) {
            return Err(CarrierError::SummonHasNoRewards);
        }
        self.carrier
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == actor)
            .map(|projection| (projection.occurrence.death_key(), projection.position()))
            .ok_or(CarrierError::CommittedLethalUnavailable)
    }

    /// P2 4180039074: the owner clock of the turn that committed `actor`'s lethal receipt, read
    /// from the retained projection, so a replayed projection keeps its original death time.
    /// `Ok(None)` when that turn supplied no clock.
    pub fn projected_death_at_ms(&self, actor: ExactActorRef) -> Result<Option<u64>, CarrierError> {
        self.carrier.validate_ref(self.continuity, actor.0)?;
        self.carrier
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == actor)
            .map(|projection| projection.death_at_ms)
            .ok_or(CarrierError::CommittedLethalUnavailable)
    }

    /// §1.3: the top-damage reward principal of `actor`'s still-live slot, under the D132
    /// tie-break of [`Self::top_damage_character`], with the winner's session identity, slot and
    /// last damage time. See [`ChannelActorCarrier::top_damage_contributor_inner`].
    pub fn top_damage_contributor(
        &self,
        actor: ExactActorRef,
    ) -> Result<TopDamagePrincipal, CarrierError> {
        self.carrier
            .top_damage_contributor_inner(self.continuity, actor.0)
    }

    /// D2b: the memoized XP `ExperienceRewardOccurrence` bytes of this
    /// generation's committed death for `character`, minting them on first
    /// call. See [`ChannelActorCarrier::reward_occurrence_inner`].
    pub fn reward_occurrence(
        &mut self,
        actor: ExactActorRef,
        character: [u8; 16],
    ) -> Result<([u8; 16], bool), CarrierError> {
        self.carrier
            .reward_occurrence_inner(self.continuity, actor.0, character)
    }

    /// D132/D3-3: the deterministic top-damage `CharacterId` for `actor`'s still-live slot in
    /// this generation, read from the owner's own bounded, in-memory `DamageContributors`
    /// accumulation. `Ok(None)` when the creature died (or is still alive) with no tracked
    /// contributor ever recorded against it (a damage-free death, or every hit came from an
    /// untracked 17th-plus attacker). This reads only live slot state, never
    /// `corpse_projections`: like the rest of D132's accumulation, the answer does not outlive
    /// this generation (D52). D3-2 is the composition point that reads this at the same moment
    /// it extracts `(death, corpse)`, before the corpse's own MINT.
    #[allow(
        dead_code,
        reason = "no production caller yet; D3-2 wires this into settle_creature_death_rewards"
    )]
    pub fn top_damage_character(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<CharacterId>, CarrierError> {
        self.carrier
            .top_damage_character_inner(self.continuity, actor.0)
    }
}

impl CurrentOwnerExactActorLookup<'_> {
    /// One direct slot/generation lookup. Invalid, vacant and misrouted refs
    /// share one failure; no actor payload or carrier authority is returned.
    pub fn contains(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(self.continuity, actor.0) else {
            return false;
        };
        match &self.carrier.slots[index] {
            Slot::Occupied {
                generation,
                committed,
                ..
            } => *committed && *generation == actor.0.actor_local_generation.0,
            Slot::CreatureOccupied {
                generation, health, ..
            } => *generation == actor.0.actor_local_generation.0 && *health > 0,
            Slot::VacantReusable { .. }
            | Slot::Exhausted { .. }
            | Slot::CreatureReserved { .. }
            | Slot::NpcOccupied { .. } => false,
        }
    }
}

impl ChannelRuntimeV1 {
    /// VIS-3 (MOVE-RL-11 §4.2, D524): every entity this Channel shows in
    /// `WORLD_SPATIAL_VISIBILITY` under the pinned Movement context, read in one owner work item:
    /// committed present players, live creatures and placed NPCs. Corpses wait on their item binding (D3-7).
    /// Read-only: a value snapshot of the existing `slots`, never an authority token.
    pub fn visible_entities(&self) -> VisibleRuntimeEntities {
        let context = self.pinned_position_context();
        let carrier = &self.carrier;
        let actor = |version: &VersionedPosition| VisibleRuntimeActor {
            actor: ExactActorRef(ActorRef::from_owner_parts(
                carrier.world_id,
                carrier.channel_id,
                carrier.scope_generation,
                version.actor_local_id,
                version.actor_local_generation,
            )),
            generation: version.actor_local_generation.0,
            position: MovementLocalPosition {
                x: version.position.x,
                y: version.position.y,
                floor: version.position.floor,
            },
            revision: version.revision,
        };
        let mut visible = VisibleRuntimeEntities::default();
        // Only the occupied slots (Codex 4178196302): never a scan of the whole capacity.
        let occupied = carrier
            .occupied
            .iter()
            .filter_map(|index| carrier.slots.get(usize::try_from(*index).ok()?));
        for slot in occupied {
            match slot {
                Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    position: Some(version),
                    ..
                } if version.context == context => visible.players.push(actor(version)),
                Slot::CreatureOccupied {
                    health,
                    position: Some(version),
                    ..
                } if *health > 0 && version.context == context => {
                    visible.creatures.push(actor(version));
                }
                Slot::NpcOccupied {
                    position: version, ..
                } if version.context == context => visible.npcs.push(actor(version)),
                _ => {}
            }
        }
        visible
    }
}

impl ChannelActorCarrier {
    test_support_pub! {
        fn current_owner_combat_death<'a>(
            &'a mut self,
            continuity: &'a NamespaceContinuityGuard,
        ) -> CurrentOwnerCombatDeath<'a> {
            CurrentOwnerCombatDeath {
                carrier: self,
                continuity,
            }
        }
    }
    test_support_pub! {
        fn current_owner_exact_commit<'a>(
            &'a mut self,
            continuity: &'a NamespaceContinuityGuard,
        ) -> CurrentOwnerExactActorCommit<'a> {
            self.current_owner_exact_commit_at(continuity, None)
        }
    }
    test_support_pub! {
        fn current_owner_exact_commit_at<'a>(
            &'a mut self,
            continuity: &'a NamespaceContinuityGuard,
            now_ms: Option<u64>,
        ) -> CurrentOwnerExactActorCommit<'a> {
            CurrentOwnerExactActorCommit {
                content: None,
                carrier: self,
                continuity,
                now_ms,
            }
        }
    }
    test_support_pub! {
        fn current_owner_exact_lookup<'a>(
            &'a self,
            continuity: &'a NamespaceContinuityGuard,
        ) -> CurrentOwnerExactActorLookup<'a> {
            CurrentOwnerExactActorLookup {
                carrier: self,
                continuity,
            }
        }
    }
}

impl ChannelRuntimeV1 {
    /// Read the existing current carrier, not timer enrollment. The explicit candidate budget
    /// refuses a partial census; geometry/hostility qualification still belongs to perception.
    pub fn current_live_creature_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<ExactActorRef>, CarrierError> {
        let context = self.pinned_position_context();
        let mut out = Vec::new();
        for (i, slot) in self.carrier.slots.iter().enumerate() {
            let Slot::CreatureOccupied {
                generation,
                health,
                position: Some(position),
                ..
            } = slot
            else {
                continue;
            };
            if *health <= 0 {
                continue;
            }
            if position.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            if out.len() >= limit {
                return Err(CarrierError::AllocationFailed);
            }
            out.try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            out.push(ExactActorRef(ActorRef::from_owner_parts(
                self.carrier.world_id,
                self.carrier.channel_id,
                self.carrier.scope_generation,
                ActorLocalId((i + 1) as u32),
                ActorLocalGeneration(*generation),
            )));
        }
        Ok(out)
    }
    /// Position Combat targets every creature on the tile. This bounded self subset is legal
    /// only when the actual carrier proves no OTHER positioned actor occupies that cell.
    /// Unqualified/dead/temporarily uncontrolled other actors conservatively veto, never vanish.
    pub fn is_sole_current_actor_on_cell(
        &self,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let snapshot = self.read_actor_position(actor)?;
        if snapshot.context() != self.pinned_movement_context() {
            return Err(CarrierError::PositionContextMismatch);
        }
        let cell = snapshot.position();
        for (i, slot) in self.carrier.slots.iter().enumerate() {
            if i == index {
                continue;
            }
            let p = match slot {
                Slot::Occupied {
                    position: Some(p), ..
                }
                | Slot::CreatureOccupied {
                    position: Some(p), ..
                } => p,
                _ => continue,
            };
            if p.position.x == cell.x && p.position.y == cell.y && p.position.floor == cell.floor {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
