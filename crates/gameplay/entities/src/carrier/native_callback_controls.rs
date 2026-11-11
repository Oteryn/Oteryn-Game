//! The literal setMoveLocked flag is creature DATA, not a condition, attack
//! suspension, timer or player input grant.
use super::*;
impl ChannelRuntimeV1 {
    pub fn native_callback_movement_locked(
        &self,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                health,
                spell_combat,
                ..
            } if *health > 0 => Ok(spell_combat.native_callback_move_locked),
            _ => Err(CarrierError::NotCreature),
        }
    }
    pub fn set_native_tenebris_movement_locked(
        &mut self,
        native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
        source: &oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallback,
        actor: ExactActorRef,
        locked: bool,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if source.content() != native.source_digest()
            || native.source_digest() != self.content_pin().server_artifact_digest()
            || source.entry().creature.key != "oteryn:creature.lady_tenebris"
            || source.entry().ability.key != "oteryn:ability.spell.tenebris_ultimate"
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let snapshot = self.companion_snapshot(actor)?;
        if snapshot.health <= 0
            || snapshot.state.policy.definition_key != source.entry().creature.key
            || snapshot.state.policy.definition_revision != source.entry().creature.revision
            || !native
                .policies()
                .policies()
                .iter()
                .any(|policy| policy.as_ref() == snapshot.state.policy.as_ref())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.assert_actor_spell_unreserved(actor)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &mut self.carrier.slots[index] {
            Slot::CreatureOccupied { spell_combat, .. } => {
                spell_combat.native_callback_move_locked = locked;
                Ok(())
            }
            _ => Err(CarrierError::NotCreature),
        }
    }
}
