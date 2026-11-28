//! Grand Master Oberon's per-incarnation physical phase and debate admission.
use super::*;
use oteryn_gameplay_domain::domain::oberon::OBERON_KEY;
use oteryn_gameplay_foundation::foundation::{
    ThinkOccurrence,
    speech::{ChatPosition, SpeechMode, hears},
};

fn chat_position(at: MovementLocalPosition) -> Option<ChatPosition> {
    Some(ChatPosition {
        x: at.x,
        y: at.y,
        floor: u8::try_from(at.floor).ok()?,
    })
}

impl ChannelRuntimeV1 {
    pub fn oberon_phase_due(&self, occurrence: ThinkOccurrence) -> bool {
        self.carrier.validate_ref(&self.continuity, occurrence.actor.0).is_ok_and(|index| {
            matches!(&self.carrier.slots[index], Slot::CreatureOccupied { health, spell_combat, .. }
                if spell_combat.oberon.is_some_and(|phase|
                    phase.phase_due(occurrence.sequence, *health, spell_combat.maximum_health)))
        })
    }

    /// Stage the parent before admitting the Falcon. A retry after physical
    /// admission sees the committed debate and cannot heal or summon twice.
    pub fn think_oberon(
        &mut self,
        occurrence: ThinkOccurrence,
        content: [u8; 32],
        draw: impl FnMut(u32) -> u32,
        map_admits: impl FnOnce(&Self, MovementLocalPosition) -> Option<bool>,
    ) -> Result<Option<&'static str>, EncounterSpawnError> {
        use EncounterSpawnError as E;
        if content != self.content_pin().server_artifact_digest() {
            return Err(E::StaleOwner);
        }
        self.owner_fence().map_err(E::Native)?;
        let actor = occurrence.actor;
        let index = self
            .carrier
            .validate_ref(&self.continuity, actor.0)
            .map_err(E::Native)?;
        self.carrier
            .assert_slot_spell_unreserved(index)
            .map_err(E::Native)?;
        let mut staged = self.carrier.slots[index].clone();
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            target_identity,
            ..
        } = &mut staged
        else {
            return Err(E::Native(CarrierError::NotCreature));
        };
        let Some(phase) = spell_combat.oberon.as_mut() else {
            return Ok(None);
        };
        if target_identity.as_ref() != OBERON_KEY.as_bytes() {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        let Some(next) = phase
            .think(
                occurrence.sequence,
                *health,
                spell_combat.maximum_health,
                draw,
            )
            .map_err(|_| E::Native(CarrierError::PlanConflict))?
        else {
            return Ok(None);
        };
        let at = self
            .read_actor_position(actor)
            .map_err(E::Native)?
            .position();
        let child = self.companion_policy(next.summon).map_err(E::Native)?;
        match self.admit_native_masterless_with_placement(
            next.summon,
            &child.definition_revision,
            content,
            at,
            true,
            map_admits,
        ) {
            Ok(_) | Err(E::Native(CarrierError::CapacityExceeded)) => {
                *health = spell_combat.maximum_health;
                self.carrier.slots[index] = staged;
                Ok(Some(next.taunt))
            }
            Err(error) => Err(error),
        }
    }

    pub fn reply_to_oberon(
        &mut self,
        speaker: ExactActorRef,
        session: GameSessionId,
        text: &str,
    ) -> Option<(ExactActorRef, &'static str)> {
        self.player_control_facts(speaker, session).ok()?;
        let at = chat_position(self.read_actor_position(speaker).ok()?.position())?;
        for &raw in &self.carrier.occupied {
            let index = raw as usize;
            let Slot::CreatureOccupied {
                generation,
                health,
                spell_combat,
                ..
            } = &self.carrier.slots[index]
            else {
                continue;
            };
            if *health <= 0
                || spell_combat.oberon.is_none()
                || self.carrier.assert_slot_spell_unreserved(index).is_err()
            {
                continue;
            }
            let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
                self.carrier.runtime_scope,
                self.carrier.scope_generation,
                ActorLocalId(raw + 1),
                ActorLocalGeneration(*generation),
            ));
            let Some(position) = self
                .read_actor_position(actor)
                .ok()
                .and_then(|p| chat_position(p.position()))
            else {
                continue;
            };
            if hears(SpeechMode::Say, at, position).is_none() {
                continue;
            }
            if let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index]
                && let Some(line) = spell_combat
                    .oberon
                    .as_mut()
                    .and_then(|phase| phase.reply(text))
            {
                return Some((actor, line));
            }
        }
        None
    }
}
