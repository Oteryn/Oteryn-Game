//! Bounded scripted scene movement through the existing actor/position owner.
use super::*;

pub const MAX_SOURCE_SCENE_ACTORS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneActorKind {
    Player(GameSessionId),
    Creature,
    Npc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneActorSnapshot {
    pub actor: ExactActorRef,
    pub position: MovementPositionSnapshot,
    pub kind: SceneActorKind,
}

impl ChannelRuntimeV1 {
    pub fn scene_actor_census(&self) -> Result<Vec<SceneActorSnapshot>, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        let mut result = Vec::new();
        for &index in &self.carrier.occupied {
            let index =
                usize::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            let (generation, kind) = match &self.carrier.slots[index] {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(session),
                    committed: true,
                    position: Some(_),
                    ..
                } => (*generation, SceneActorKind::Player(*session)),
                Slot::CreatureOccupied {
                    generation,
                    health,
                    position: Some(_),
                    ..
                } if *health > 0 => (*generation, SceneActorKind::Creature),
                Slot::NpcOccupied {
                    generation,
                    hireling: None,
                    ..
                } => (*generation, SceneActorKind::Npc),
                _ => continue,
            };
            let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
                self.binding.runtime_scope,
                self.binding.scope_generation,
                ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(generation),
            ));
            let position = self
                .carrier
                .read_movement_position(&self.continuity, actor.0)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            result.push(SceneActorSnapshot {
                actor,
                position,
                kind,
            });
        }
        result.sort_by_key(|entry| entry.actor.placement_identity());
        Ok(result)
    }

    /// Scripted stacking never overrides a relocation already reserved by another owner.
    pub fn scene_destination_reserved(
        &self,
        destination: MovementLocalPosition,
    ) -> Result<bool, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        Ok(self.carrier.slots.iter().any(|slot| {
            let combat = match slot {
                Slot::Occupied { spell_combat, .. }
                | Slot::CreatureOccupied { spell_combat, .. } => Some(spell_combat),
                _ => None,
            };
            combat
                .and_then(|c| c.pending_owner.as_ref())
                .and_then(|r| r.destination())
                .is_some_and(|(_, at)| at == destination)
        }))
    }

    /// Current movement snapshot also covers native NPCs; this conveys no write authority.
    pub fn read_scene_actor_position(
        &self,
        actor: ExactActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        self.carrier
            .read_movement_position(&self.continuity, actor.0)
    }
}
