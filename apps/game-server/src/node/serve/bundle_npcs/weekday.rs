//! One weekly NPC follows only placements admitted by the verified bundle.
use super::{ChannelRuntimeV1, NpcBoot, NpcPlacement, NpcPlacementError};
use oteryn_gameplay_entities::rashid_schedule::{RASHID_NPC_KEY, destination_at};

#[derive(Debug, Clone, Copy)]
pub(super) struct WeekdayNpcSpawns {
    key: &'static str,
    placements: [Option<NpcPlacement>; 7],
}

impl WeekdayNpcSpawns {
    pub(super) fn from_boot(boot: Option<&NpcBoot>) -> Option<Self> {
        let NpcBoot::Place {
            table, admitted, ..
        } = boot?
        else {
            return None;
        };
        if !admitted.contains(RASHID_NPC_KEY) {
            return None;
        }
        let source = table.npcs.iter().find(|npc| npc.key == RASHID_NPC_KEY)?;
        let placements = std::array::from_fn(|day| {
            let day = i64::try_from(day).ok()?;
            let expected = destination_at((4 + day) * 86_400_000).placement;
            source.placements.iter().copied().find(|p| *p == expected)
        });
        placements.iter().any(Option::is_some).then_some(Self {
            key: RASHID_NPC_KEY,
            placements,
        })
    }

    pub(super) fn tick(
        self,
        runtime: &mut ChannelRuntimeV1,
        unix_ms: i64,
    ) -> Result<(), NpcPlacementError> {
        // Same UTC day convention as the existing trade-availability owner.
        let day = match (unix_ms.div_euclid(86_400_000) + 3).rem_euclid(7) {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 5,
            _ => 6,
        };
        reconcile(runtime, self.key, self.placements[day], unix_ms)
    }
}

/// Today's admitted wander area retains the exact actor. A changed or absent desired
/// placement retires old identities before trying the new cell. Native occupancy
/// refusal leaves the NPC absent until the next tick; it never creates a duplicate.
fn reconcile(
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    wanted: Option<NpcPlacement>,
    unix_ms: i64,
) -> Result<(), NpcPlacementError> {
    let present: Vec<_> = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .filter(|npc| runtime.npc_key(npc.actor) == Some(key))
        .collect();
    if let (Some(_), [npc]) = (wanted, present.as_slice())
        && oteryn_gameplay_entities::rashid_schedule::trade_available(key, npc.position, unix_ms)
    {
        return Ok(());
    }
    for _ in present {
        runtime.remove_dynamic_npc(key)?;
    }
    if let Some(wanted) = wanted {
        runtime.place_dynamic_npc(key, wanted)?;
    }
    Ok(())
}
