//! Source-bound conditional contacts; availability comes from the reset owner's frozen calendar.
use super::{NpcBoot, NpcPlacementError};
use crate::foundation::{ChannelRuntimeV1, MovementFacing, NpcPlacement};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::ConditionalNpcActor;

const HOAXETTE: &str = "oteryn:npc.hoaxette";
const HOAXETTE_CELL: (u16, u16, i8) = (32380, 32215, 7);

#[derive(Debug, Default)]
pub(super) struct ConditionalNpcSpawns {
    hoaxette: Option<ConditionalNpcActor>,
    calendar: Option<(u8, u8)>,
}

impl ConditionalNpcSpawns {
    pub(super) fn admit(boot: Option<&NpcBoot>, enterable: impl Fn(TilePos) -> bool) -> Self {
        let (x, y, z) = HOAXETTE_CELL;
        let admitted = boot.is_some_and(|boot| boot.admits(HOAXETTE))
            && enterable(TilePos { x, y, floor: -z });
        Self {
            hoaxette: admitted
                .then(|| {
                    ConditionalNpcActor::new(
                        HOAXETTE.into(),
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
            calendar: None,
        }
    }

    pub(super) fn set_calendar(&mut self, calendar: Option<(u8, u8)>) {
        self.calendar = calendar;
    }

    pub(super) fn is_empty(&self) -> bool {
        self.hoaxette.is_none()
    }

    pub(super) fn tick(&mut self, runtime: &mut ChannelRuntimeV1) -> Result<(), NpcPlacementError> {
        // Retained Hoaxette source metadata: https://tibiopedia.pl/npcs/Hoaxette,
        // bulk.source_metadata/wiki_facts/description says the whole of April.
        // Cell and calendar fact: content/npcs/definitions/npcs-00500-00999.json,
        // record107, bulk.source_metadata/wiki_facts (BR revision357473).
        let present = self
            .calendar
            .is_some_and(|(month, day)| month == 4 && (1..=30).contains(&day));
        if let Some(hoaxette) = &mut self.hoaxette {
            hoaxette.reconcile_or_defer(runtime, present)?;
        }
        Ok(())
    }
}
