use super::*;

impl ChannelAttackStates {
    /// `RANGED0-RL-03`: the Channel's one chase search drain of the owner window of `now`. The
    /// first chase check to reach the window runs every waiting search, at most
    /// [`crate::combat_attack::CHASE_SEARCHES_PER_WINDOW`], in the order [`chase_search_order`] gives, each against
    /// the occupancy the earlier ones leave once they take their next step; every chase then
    /// takes its own stored route. Nothing here depends on which check drains: the inputs are
    /// the Channel's runtime, its path tiles and the waiting chases alone.
    pub(in crate::gameplay_transport) fn drain_chase_searches(
        &mut self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        now: SemanticTimeMicros,
        path_tile: &impl Fn(MovementLocalPosition) -> bool,
    ) {
        let Self {
            entries,
            chase_searches,
            path_budget,
            ..
        } = self;
        if !chase_searches.open(now.get()) {
            return;
        }
        let order = chase_search_order(
            now.get(),
            entries.iter().enumerate().filter_map(|(index, entry)| {
                let since = entry.chase.search_waiting_since()?;
                Some((since, (actor_order(entry.actor), index)))
            }),
        );
        if order.is_empty() {
            return;
        }
        // CREATURE-AI-0 §5.3: +40 over a tile another actor stands on (a creature, an NPC or
        // another player; never the chasing player), and the target's own tile is no path tile.
        let standing = crate::movement::path_search::standing_positions(runtime);
        let mut stepping: Vec<(ExactActorRef, MovementLocalPosition)> = entries
            .iter()
            .filter_map(|entry| {
                let at = runtime.read_actor_position(entry.actor).ok()?.position();
                Some((entry.actor, entry.chase.planned_cell(now.get(), at)?))
            })
            .collect();
        let mut searched = 0;
        for (_, index) in order {
            if searched == chase_searches.remaining() {
                break;
            }
            let entry = &mut entries[index];
            let Some(target) = entry
                .state
                .target()
                .filter(|_| entry.modes.chase == ChaseMode::Chase)
            else {
                continue;
            };
            let Some((from, to)) =
                chase_ends(runtime, room, entry.actor, entry.session, target, now)
            else {
                continue;
            };
            let occupied: std::collections::BTreeSet<(i32, i32)> = standing
                .iter()
                .filter(|(other, _)| *other != entry.actor)
                .map(|(other, position)| {
                    stepping
                        .iter()
                        .find(|(stepper, _)| stepper == other)
                        .map_or(*position, |(_, next)| *next)
                })
                .filter(|position| position.floor == from.floor)
                .map(|position| (position.x, position.y))
                .collect();
            if !path_budget.admits(now.get(), true, true) {
                chase_searches.suspend();
                break;
            }
            let ran = entry.chase.measured_search(now.get(), from, to, |cell| {
                if cell == to || !path_tile(cell) {
                    return None;
                }
                Some(if occupied.contains(&(cell.x, cell.y)) {
                    CREATURE_COST
                } else {
                    0
                })
            });
            if let Some(work) = ran {
                searched += 1;
                path_budget.charge(true, work);
            }
            if let Some(next) = entry.chase.next_cell() {
                stepping.push((entry.actor, next));
            }
        }
        for _ in 0..searched {
            chase_searches.record();
        }
    }

    pub(in crate::gameplay_transport) fn players_waiting(&self, now: u64) -> bool {
        !chase_search_order(
            now,
            self.entries.iter().filter_map(|entry| {
                entry
                    .chase
                    .search_waiting_since()
                    .map(|since| (since, actor_order(entry.actor)))
            }),
        )
        .is_empty()
    }
}
