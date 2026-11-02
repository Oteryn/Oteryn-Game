//! Session-bound projection of the durable hazard selection, in the actual actor slot.
use super::*;
impl ChannelRuntimeV1 {
    pub fn install_hazard_level(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        level: u16,
    ) -> Result<(), CarrierError> {
        if !(1..=12).contains(&level) {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        self.player_control_facts(actor, session)?;
        self.assert_actor_spell_unreserved(actor)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.hazard = Some((session, level));
        Ok(())
    }
    /// Exact actor/session binding prevents a reused slot inheriting another player's selection.
    pub fn hazard_level(&self, actor: ExactActorRef, session: GameSessionId) -> Option<u16> {
        self.player_control_facts(actor, session).ok()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        let Slot::Occupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return None;
        };
        spell_combat
            .hazard
            .filter(|(bound, _)| *bound == session)
            .map(|(_, level)| level)
    }

    /// The dispatch composition supplies the coordinate frame of its active movement cells.
    /// Only the unshifted world-bundle source frame may interpret Gnomprona's donor geometry.
    pub fn install_hazard_world_frame(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        frame: &str,
    ) -> Result<(), CarrierError> {
        self.player_control_facts(actor, session)?;
        self.assert_actor_spell_unreserved(actor)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.hazard_native_frame =
            (frame == "oteryn:frame/world-bundle-v3").then_some(session);
        Ok(())
    }

    pub fn hazard_world_frame(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        if self.player_control_facts(actor, session).is_err() {
            return false;
        }
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        matches!(&self.carrier.slots[index], Slot::Occupied { spell_combat, .. }
            if spell_combat.hazard_native_frame == Some(session))
    }

    /// Crystal's incoming critical cooldown belongs to the player, across every monster
    /// and every melee/spell owner. It is not an independent per-attack-owner timer.
    pub fn last_hazard_critical_us(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<u64> {
        self.player_control_facts(actor, session).ok()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        let Slot::Occupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return None;
        };
        spell_combat
            .hazard_critical
            .filter(|(bound, _)| *bound == session)
            .map(|(_, at)| at)
    }

    /// Called under the same runtime lock only after an actual incoming HP mutation.
    /// A retired actor/session cannot stamp a replacement occupant's critical window.
    pub fn record_hazard_critical(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        at_us: u64,
    ) -> Result<(), CarrierError> {
        self.player_control_facts(actor, session)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.hazard_critical = Some((session, at_us));
        Ok(())
    }
}
