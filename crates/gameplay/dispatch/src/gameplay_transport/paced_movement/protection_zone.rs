//! Only a transition into a protection zone needs a PvP-lock observation.
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::foundation::{ExactActorRef, GameSessionId, MovementPositionSnapshot};
use crate::gameplay_transport::{actor_spell::ChannelSpellStates, pvp_live};
use crate::map::floor::CellBlocker;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub(super) struct Probe {
    needs_refresh: AtomicBool,
    blocked_entry: AtomicBool,
}

impl Probe {
    pub(super) fn needs_refresh(&self) -> bool {
        self.needs_refresh.load(Ordering::Relaxed)
    }

    pub(super) fn blocked_entry(&self) -> bool {
        self.blocked_entry.load(Ordering::Relaxed)
    }
}

enum Admission {
    Open,
    Locked,
    Unobserved,
}

pub(super) struct StepBlocker<'a> {
    inner: &'a dyn CellBlocker,
    cells: &'a NativeEntryMovementCells,
    admission: Admission,
    probe: &'a Probe,
}

impl<'a> StepBlocker<'a> {
    pub(super) fn new(
        inner: &'a dyn CellBlocker,
        cells: &'a NativeEntryMovementCells,
        origin: Option<MovementPositionSnapshot>,
        states: &ChannelSpellStates,
        (actor, session): (ExactActorRef, GameSessionId),
        now: u64,
        probe: &'a Probe,
    ) -> Self {
        let already_inside = origin.is_some_and(|at| {
            let at = at.position();
            protection_zone(
                cells,
                LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            )
        });
        let admission = if already_inside {
            Admission::Open
        } else if pvp_live::pz_blocked(states, actor, session, SemanticTimeMicros::from_micros(now))
        {
            Admission::Locked
        } else if pvp_live::tracked(states, actor, session) {
            Admission::Open
        } else {
            Admission::Unobserved
        };
        Self {
            inner,
            cells,
            admission,
            probe,
        }
    }
}

impl CellBlocker for StepBlocker<'_> {
    fn blocks(&self, cell: LogicalCell) -> bool {
        if self.inner.blocks(cell) {
            return true;
        }
        if !protection_zone(self.cells, cell) {
            return false;
        }
        match self.admission {
            Admission::Open => false,
            Admission::Locked => {
                self.probe.blocked_entry.store(true, Ordering::Relaxed);
                true
            }
            Admission::Unobserved => {
                self.probe.needs_refresh.store(true, Ordering::Relaxed);
                true
            }
        }
    }
}

fn protection_zone(cells: &NativeEntryMovementCells, cell: LogicalCell) -> bool {
    cells
        .spell_tiles()
        .lookup(cells.scope(), cell)
        .is_ok_and(|tile| tile.flags().protection_zone)
}
