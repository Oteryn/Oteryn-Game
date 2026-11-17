//! Event presence projects the existing committed world-change owner; no calendar is inferred.
use super::{NpcBoot, NpcPlacementError};
use crate::foundation::{ChannelRuntimeV1, MovementFacing, NpcPlacement, WorldId};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::ConditionalNpcActor;

/// Oteryn binding for the event named in Rascalio's retained source metadata.
pub(super) const EVENT: &str = "nimmersatts_lair";
const NPC: &str = "oteryn:npc.rascalio";
const POINT: (u16, u16, i8) = (32259, 31643, 7);

#[derive(Debug, Default)]
pub(super) struct EventContacts {
    rascalio: Option<ConditionalNpcActor>,
    active: bool,
}

impl EventContacts {
    pub(super) fn admit(boot: Option<&NpcBoot>, enterable: impl Fn(TilePos) -> bool) -> Self {
        let (x, y, z) = POINT;
        let admitted =
            boot.is_some_and(|boot| boot.admits(NPC)) && enterable(TilePos { x, y, floor: -z });
        Self {
            rascalio: admitted
                .then(|| {
                    ConditionalNpcActor::new(
                        NPC.into(),
                        NpcPlacement {
                            x,
                            y,
                            floor: -z,
                            direction: MovementFacing::South,
                        },
                    )
                    .ok()
                })
                .flatten(),
            active: false,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.rascalio.is_none()
    }

    pub(super) async fn refresh(
        &mut self,
        root: &crate::durability::DurabilityRoot,
        world: WorldId,
    ) -> Result<(), crate::durability::DurabilityError> {
        if !self.is_empty() {
            self.active = root.world_change_active(world, EVENT).await?;
        }
        Ok(())
    }

    pub(super) fn tick(&mut self, runtime: &mut ChannelRuntimeV1) -> Result<(), NpcPlacementError> {
        if let Some(actor) = &mut self.rascalio {
            actor.reconcile_or_defer(runtime, self.active)?;
        }
        Ok(())
    }
}
