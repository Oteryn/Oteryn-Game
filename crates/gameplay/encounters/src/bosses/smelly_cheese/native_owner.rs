use oteryn_gameplay_entities::carrier::Slot;
use oteryn_gameplay_entities::*;
pub trait SmellyCheeseCarrierExt {
    #[allow(clippy::too_many_arguments)]
    fn smelly_unambiguous_targets(
        &self,
        caster: ExactActorRef,
    ) -> Result<Vec<(ExactActorRef, GameSessionId)>, CarrierError>;
}
impl SmellyCheeseCarrierExt for ChannelRuntimeV1 {
    // Insert inside impl ChannelRuntimeV1. Conservative exact-native source subset:
    // a unique player with no other live creature at its tile is unambiguously topCreature.
    // Ambiguous stack order is deliberately not invented; skipped tiles stay a runtime gap.
    fn smelly_unambiguous_targets(
        &self,
        caster: ExactActorRef,
    ) -> Result<Vec<(ExactActorRef, GameSessionId)>, CarrierError> {
        let caster_index = self.encounter_actor_index(caster)?;
        match self.encounter_slot(caster_index) {
            Slot::CreatureOccupied {
                target_identity,
                health,
                ..
            } if target_identity.as_ref() == b"oteryn:creature.smelly_cheese" && *health > 0 => {}
            _ => return Err(CarrierError::NotCreature),
        }
        let at = self.read_encounter_position(caster)?;
        let mut result = Vec::new();
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                let (Some(x), Some(y)) = (
                    at.version.position.x.checked_add(dx),
                    at.version.position.y.checked_add(dy),
                ) else {
                    continue;
                };
                let mut target = None;
                let mut count = 0;
                for (index, slot) in self.encounter_slots().iter().enumerate() {
                    let (version, generation, player) = match slot {
                        Slot::Occupied {
                            generation,
                            game_session_id: Some(session),
                            committed: true,
                            position: Some(version),
                            ..
                        } => (
                            version,
                            *generation,
                            self.encounter_player_controlled(index).then_some(*session),
                        ),
                        Slot::CreatureOccupied {
                            generation,
                            health,
                            position: Some(version),
                            ..
                        } if *health > 0 => (version, *generation, None),
                        _ => continue,
                    };
                    if version.context != at.version.context
                        || version.position.x != x
                        || version.position.y != y
                        || version.position.floor != at.version.position.floor
                    {
                        continue;
                    }
                    count += 1;
                    target = player.map(|session| {
                        (
                            ExactActorRef(ActorRef::from_owner_parts(
                                self.binding().world_id(),
                                self.binding().channel_id(),
                                self.binding().scope_generation(),
                                ActorLocalId((index + 1) as u32),
                                ActorLocalGeneration(generation),
                            )),
                            session,
                        )
                    });
                }
                if count == 1
                    && let Some(target) = target
                {
                    result.push(target);
                }
            }
        }
        Ok(result)
    }
}
