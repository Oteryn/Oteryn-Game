//! Drome membership lives on the exact actor slot, so reuse cannot inherit an exemption.
use super::*;
impl ChannelRuntimeV1 {
    pub fn set_tibiadrome(
        &mut self,
        actor: ExactActorRef,
        enabled: bool,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let origin = enabled
            .then(|| self.read_actor_position(actor).map(|p| p.position()))
            .transpose()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &mut self.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. } => {
                spell_combat.tibiadrome = enabled;
                spell_combat.tibiadrome_origin = origin;
                Ok(())
            }
            _ => Err(CarrierError::NotCreature),
        }
    }
    pub fn is_tibiadrome(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        let combat = match &self.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } => return spell_combat.tibiadrome,
            Slot::CreatureOccupied {
                spell_combat,
                companion,
                ..
            } => {
                if spell_combat.tibiadrome {
                    return true;
                }
                companion.as_ref().and_then(|c| c.master)
            }
            _ => None,
        };
        let Some(master) = combat else {
            return false;
        };
        let Ok(index) = self.carrier.validate_ref(&self.continuity, master.actor.0) else {
            return false;
        };
        matches!(&self.carrier.slots[index],Slot::Occupied{spell_combat,..} if spell_combat.tibiadrome)
    }
}

impl ChannelRuntimeV1 {
    /// oteryn-choice: +10% native health/damage per Drome level; the current
    /// manual combat.md:297 and pinned donors have no numeric Drome scaling.
    pub fn scale_tibiadrome_opponent(
        &mut self,
        actor: ExactActorRef,
        level: u32,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            companion: Some(companion),
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        let scale = 100u64.saturating_add(u64::from(level).saturating_mul(10));
        let maximum =
            i64::try_from(i128::from(companion.policy.maximum_health) * i128::from(scale) / 100)
                .map_err(|_| CarrierError::DamageOverflow)?;
        let mut policy = (*companion.policy).clone();
        policy.maximum_health = maximum;
        companion.policy = std::sync::Arc::new(policy);
        *health = maximum;
        spell_combat.maximum_health = maximum;
        spell_combat.tibiadrome = true;
        spell_combat.tibiadrome_scale = scale;
        Ok(())
    }
    pub(super) fn tibiadrome_damage(
        &self,
        actor: ExactActorRef,
        raw: u64,
    ) -> Result<Option<u64>, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Ok(None);
        };
        if !spell_combat.tibiadrome {
            return Ok(None);
        }
        u64::try_from(u128::from(raw) * u128::from(spell_combat.tibiadrome_scale) / 100)
            .map(Some)
            .map_err(|_| CarrierError::DamageOverflow)
    }
}
