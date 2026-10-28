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
}
