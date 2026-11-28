//! Session-independent windows for the existing Channel spawn owner.
use crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS;
use crate::foundation::{ChannelRuntimeV1, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_world::world_clock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnExit {
    Shutdown,
    StaleOwner,
    ClockExhausted,
}

pub struct MonsterRespawnJob {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    last_window: Option<u64>,
}

impl MonsterRespawnJob {
    pub fn new(runtime: &ChannelRuntimeV1) -> Result<Self, RespawnExit> {
        runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
        let binding = runtime.binding();
        Ok(Self {
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            last_window: None,
        })
    }

    /// No session, combat content or player cadence participates in respawn ownership.
    /// Regressing clocks and carrier refusals preserve the last accepted window.
    pub fn pass(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        world_utc_ms: Option<u64>,
    ) -> Result<(), RespawnExit> {
        let fence = runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
        if !fence.is_current_for_scope(self.scope, self.generation) {
            return Err(RespawnExit::StaleOwner);
        }
        let window = now / CREATUREAI0_THINK_WINDOW_MICROS;
        if self.last_window.is_some_and(|last| window <= last) {
            return Ok(());
        }
        if drive_owner_spawns_at_world_time(runtime, now, world_utc_ms) {
            self.last_window = Some(window);
        }
        Ok(())
    }

    pub fn next_owner_window(&self, now: u64) -> Option<u64> {
        let window = self
            .last_window
            .unwrap_or(0)
            .max(now / CREATUREAI0_THINK_WINDOW_MICROS);
        window
            .checked_add(1)?
            .checked_mul(CREATUREAI0_THINK_WINDOW_MICROS)
    }
}

/// Shared by the session cadence and independent wake loop; the carrier owns the clock.
pub fn drive_owner_spawns(runtime: &mut ChannelRuntimeV1, now: u64) -> bool {
    drive_owner_spawns_at_world_time(runtime, now, world_clock::now_utc_ms())
}

fn drive_owner_spawns_at_world_time(
    runtime: &mut ChannelRuntimeV1,
    now: u64,
    world_utc_ms: Option<u64>,
) -> bool {
    runtime
        .drive_spawn_respawns_at_world_time(
            now,
            world_clock::is_spawn_day_at_utc(world_utc_ms),
            |observer, cell| {
                use crate::movement::interest::{VisibilityPosition, VisibilitySettings};
                match (
                    VisibilityPosition::new(observer.x, observer.y, observer.floor),
                    VisibilityPosition::new(cell.x, cell.y, cell.floor),
                ) {
                    (Ok(observer), Ok(cell)) => {
                        VisibilitySettings::REFERENCE.can_see(observer, cell)
                    }
                    _ => false,
                }
            },
        )
        .is_ok()
}
