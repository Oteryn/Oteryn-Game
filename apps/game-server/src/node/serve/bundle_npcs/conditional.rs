//! Source-bound conditional contacts; availability comes from the reset owner's frozen calendar.
use super::{NpcBoot, NpcPlacementError};
use crate::foundation::{ChannelRuntimeV1, MovementFacing, NpcPlacement};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::ConditionalNpcActor;
use oteryn_gameplay_world_change::owner::SeasonalWindow;

struct Contact {
    npc: &'static str,
    position: (u16, u16, i8),
    window: SeasonalWindow,
}

const STAN: &str = "oteryn:npc.stan";
const VALENTINA: &str = "oteryn:npc.valentina";

const CONTACTS: [Contact; 3] = [
    // BR NPC snapshot revision 357473 and Tibiopedia Hoaxette (all April).
    Contact {
        npc: "oteryn:npc.hoaxette",
        position: (32380, 32215, 7),
        window: SeasonalWindow {
            start: (4, 1),
            end: (5, 1),
        },
    },
    // Crystal 201189f8 world-npc.xml:804-805 and Dia das Máscaras revision 419853.
    Contact {
        npc: STAN,
        position: (32944, 32110, 6),
        window: SeasonalWindow {
            start: (2, 1),
            end: (3, 1),
        },
    },
    // Crystal 201189f8 world-npc.xml:1813-1814 and the February 2026 event calendar.
    Contact {
        npc: VALENTINA,
        position: (32271, 32052, 7),
        window: SeasonalWindow {
            start: (2, 14),
            end: (2, 15),
        },
    },
];

pub(super) fn seasonal_merchant(npc: &str) -> bool {
    matches!(npc, STAN | VALENTINA)
}

#[derive(Debug, Default)]
pub(super) struct ConditionalNpcSpawns {
    actors: [Option<ConditionalNpcActor>; CONTACTS.len()],
    calendar: Option<(u8, u8)>,
}

impl ConditionalNpcSpawns {
    pub(super) fn admit(boot: Option<&NpcBoot>, enterable: impl Fn(TilePos) -> bool) -> Self {
        let actors = std::array::from_fn(|index| {
            let contact = &CONTACTS[index];
            let (x, y, z) = contact.position;
            let floor = -z;
            if !boot.is_some_and(|boot| boot.admits(contact.npc))
                || !enterable(TilePos { x, y, floor })
            {
                return None;
            }
            ConditionalNpcActor::new(
                contact.npc.into(),
                NpcPlacement {
                    x,
                    y,
                    floor,
                    direction: MovementFacing::South,
                },
            )
            .ok()
        });
        Self {
            actors,
            calendar: None,
        }
    }

    pub(super) fn set_calendar(&mut self, calendar: Option<(u8, u8)>) {
        self.calendar = calendar;
    }

    pub(super) fn is_empty(&self) -> bool {
        self.actors.iter().all(Option::is_none)
    }

    pub(super) fn tick(&mut self, runtime: &mut ChannelRuntimeV1) -> Result<(), NpcPlacementError> {
        for (contact, actor) in CONTACTS.iter().zip(&mut self.actors) {
            if let Some(actor) = actor {
                let present = self
                    .calendar
                    .is_some_and(|(month, day)| contact.window.contains(month, day));
                actor.reconcile_or_defer(runtime, present)?;
            }
        }
        Ok(())
    }
}
