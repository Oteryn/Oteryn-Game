//! Event presence projects the existing committed world-change owner; no calendar is inferred.
use super::{NpcBoot, NpcPlacementError};
use crate::foundation::{ChannelRuntimeV1, MovementFacing, NpcPlacement, WorldId};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::ConditionalNpcActor;

/// Oteryn binding for the event named in Rascalio's retained source metadata.
pub(super) const EVENT: &str = "nimmersatts_lair";

const CONTACTS: [(&str, &str, TilePos); 2] = [
    // NPC BR snapshot Chip revision 416548. W15 binds the corresponding reset draw.
    (
        "lumberjack",
        "oteryn:npc.chip",
        TilePos {
            x: 32418,
            y: 31702,
            floor: -7,
        },
    ),
    (
        EVENT,
        "oteryn:npc.rascalio",
        TilePos {
            x: 32259,
            y: 31643,
            floor: -7,
        },
    ),
];

pub(super) fn owns_presence(npc: &str) -> bool {
    CONTACTS.iter().any(|(_, key, _)| *key == npc)
}

#[derive(Debug)]
struct Contact {
    actor: ConditionalNpcActor,
    event: &'static str,
    active: bool,
}

#[derive(Debug, Default)]
pub(super) struct EventContacts {
    contacts: Vec<Contact>,
}

impl EventContacts {
    pub(super) fn admit(boot: Option<&NpcBoot>, enterable: impl Fn(TilePos) -> bool) -> Self {
        let contacts = CONTACTS
            .into_iter()
            .filter_map(|(event, npc, tile)| {
                if !boot.is_some_and(|boot| boot.admits(npc)) || !enterable(tile) {
                    return None;
                }
                let actor = ConditionalNpcActor::new(
                    npc.into(),
                    NpcPlacement {
                        x: tile.x,
                        y: tile.y,
                        floor: tile.floor,
                        direction: MovementFacing::South,
                    },
                )
                .ok()?;
                Some(Contact {
                    actor,
                    event,
                    active: false,
                })
            })
            .collect();
        Self { contacts }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    pub(super) async fn refresh(
        &mut self,
        root: &crate::durability::DurabilityRoot,
        world: WorldId,
    ) -> Result<(), crate::durability::DurabilityError> {
        for contact in &mut self.contacts {
            contact.active = root.world_change_active(world, contact.event).await?;
        }
        Ok(())
    }

    pub(super) fn tick(&mut self, runtime: &mut ChannelRuntimeV1) -> Result<(), NpcPlacementError> {
        for contact in &mut self.contacts {
            contact.actor.reconcile_or_defer(runtime, contact.active)?;
        }
        Ok(())
    }
}
