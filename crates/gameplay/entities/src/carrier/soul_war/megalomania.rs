//! Exact three source forms share their native defense DATA. Sanity publishes
//! only after the existing fused Item/Quest receipt and physical reservation.
use super::*;

impl ChannelRuntimeV1 {
    /// Literal Blue callback destination. A registered HP0 boss remains the
    /// same source object until its already queued copied death task completes.
    pub fn relocate_soul_war_megalomania_home(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.megalomania_slot(expected)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        let before = self.read_actor_position(expected.actor)?;
        self.carrier
            .compare_commit_registered_source_position(
                &self.continuity,
                before.0,
                before.0.version.context,
                LocalPosition {
                    x: 33710,
                    y: 31634,
                    floor: 14,
                },
            )
            .map(MovementPositionSnapshot)
    }

    fn megalomania_slot(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
    ) -> Result<usize, CarrierError> {
        self.require_soul_war_scope()?;
        self.validate_companion_snapshot(expected)?;
        if expected.state.master.is_some()
            || !matches!(
                expected.state.policy.definition_key.as_str(),
                MEGALOMANIA_PURPLE | MEGALOMANIA_GREEN | MEGALOMANIA_BLUE
            )
            || !self.source_creature_is_registered(expected.actor)?
            || !self
                .soul_war_health_callback(expected.actor)?
                .is_some_and(|policy| policy.is_empowered_boss())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.encounter_actor_index(expected.actor)
    }

    pub fn soul_war_megalomania_drain(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
    ) -> Result<Option<u64>, CarrierError> {
        let index = self.megalomania_slot(expected)?;
        match self.encounter_slot(index) {
            Slot::CreatureOccupied { spell_combat, .. } => Ok(spell_combat.soul_war_cruelty_drain),
            _ => Err(CarrierError::NotCreature),
        }
    }

    pub fn increase_soul_war_megalomania(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        expected_drain: Option<u64>,
    ) -> Result<u64, CarrierError> {
        let index = self.megalomania_slot(expected)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        let Slot::CreatureOccupied { spell_combat, .. } = self.encounter_slot(index) else {
            return Err(CarrierError::NotCreature);
        };
        if spell_combat.soul_war_cruelty_drain != expected_drain {
            return Err(CarrierError::PlanConflict);
        }
        let defense = spell_combat
            .soul_war_defense_delta
            .checked_add(2)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let drain = expected_drain
            .unwrap_or(2)
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let (carrier, _) = self.encounter_parts_mut();
        let Slot::CreatureOccupied { spell_combat, .. } = &mut carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.soul_war_defense_delta = defense;
        spell_combat.soul_war_cruelty_drain = Some(drain);
        Ok(drain)
    }

    fn megalomania_sanity_values(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        defense: u32,
    ) -> Result<(usize, u32), CarrierError> {
        let index = self.megalomania_slot(expected)?;
        if self.effective_creature_defense(expected)? != defense {
            return Err(CarrierError::PlanConflict);
        }
        let Slot::CreatureOccupied { spell_combat, .. } = self.encounter_slot(index) else {
            return Err(CarrierError::NotCreature);
        };
        let delta = spell_combat.soul_war_defense_delta;
        Ok((
            index,
            if delta == 0 {
                0
            } else {
                delta.checked_sub(2).ok_or(CarrierError::PlanConflict)?
            },
        ))
    }

    pub fn validate_soul_war_megalomania_sanity(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        defense: u32,
    ) -> Result<(), CarrierError> {
        self.megalomania_sanity_values(expected, defense)
            .map(|_| ())
    }

    fn decrease_soul_war_megalomania_sanity(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        defense: u32,
    ) -> Result<(), CarrierError> {
        let (index, delta) = self.megalomania_sanity_values(expected, defense)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        let (carrier, _) = self.encounter_parts_mut();
        let Slot::CreatureOccupied { spell_combat, .. } = &mut carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.soul_war_defense_delta = delta;
        Ok(())
    }

    #[allow(
        clippy::expect_used,
        reason = "all exact source values are preflighted before reservation release"
    )]
    pub fn complete_soul_war_megalomania_sanity(
        &mut self,
        reservation: super::super::runtime_actor_source_reservation::SourceActorReservation,
        proof: &impl super::super::runtime_actor_source_reservation::SourceActorReservationProof,
        target: Option<(&runtime_actor_companion::CompanionSnapshot, u32)>,
    ) -> Result<(), CarrierError> {
        self.validate_source_actor_reservation(&reservation, proof)?;
        if let Some((expected, defense)) = target {
            self.validate_soul_war_megalomania_sanity(expected, defense)?;
        }
        self.release_source_actors_after_observed_outcome(reservation, proof)?;
        if let Some((expected, defense)) = target {
            self.decrease_soul_war_megalomania_sanity(expected, defense)
                .expect("preflighted source Sanity decrease after exact reservation release");
        }
        Ok(())
    }
}
