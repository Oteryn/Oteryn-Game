//! Native Dangerous Depths entry at the three Crystal energy tiles.
use super::*;
use crate::durability::quest_state::DepthsFaction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entrance {
    faction: DepthsFaction,
    destination: (i32, i32),
    refusal_offset: (i32, i32),
}

fn entrance(map: &BundleMap, at: MovementLocalPosition) -> Option<Entrance> {
    let (faction, destination, refusal_offset) = match (at.x, at.y, at.floor) {
        (33784, 32205, 14) => (DepthsFaction::Dwarves, (33921, 32401), (-1, 0)),
        (33829, 32187, 14) => (DepthsFaction::Gnomes, (33982, 32236), (0, -1)),
        (33831, 32138, 14) => (DepthsFaction::Scouts, (34023, 32037), (0, 1)),
        _ => return None,
    };
    let pos = native(at)?;
    let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    // The imported entrance is blocked ground. Only the captured lava and
    // energy tile may take the callback path, so a newly placed obstacle is
    // still authoritative.
    if tile.depths() != [0, 0]
        || map.facts.house_tile(pos)
        || !map.facts.quest_step_source_is_unmodified(pos)
    {
        return None;
    }
    let [left, right] = tile.ids() else {
        return None;
    };
    let appearances = [
        map.facts.base_entry(pos, 0, *left)?.appearance_id,
        map.facts.base_entry(pos, 1, *right)?.appearance_id,
    ];
    let energy = matches!(appearances, [22675, 5068] | [5068, 22675]);
    energy.then_some(Entrance {
        faction,
        destination,
        refusal_offset,
    })
}

pub(super) fn owns_step_cell(map: &BundleMap, cell: crate::content::LogicalCell) -> bool {
    i16::try_from(cell.z).ok().is_some_and(|floor| {
        entrance(
            map,
            MovementLocalPosition {
                x: cell.x,
                y: cell.y,
                floor,
            },
        )
        .is_some()
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_depths_faction_entry(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        let map = self.bundle_map()?;
        let entry = entrance(map, entered.position())?;
        let lease = self
            .runtime
            .lock()
            .await
            .bound_attacker_lease(actor, session)
            .ok()?;
        let fence = self
            .current_quest_fence(session)
            .await
            .ok()
            .flatten()
            .filter(|fence| {
                actor.runtime_scope() == fence.runtime_scope
                    && actor.scope_generation() == fence.scope_ownership_generation
                    && lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            });
        let allowed = if let Some(fence) = fence {
            self.root
                .check_depths_faction_access(self.character, self.holder, fence, entry.faction)
                .await
                .unwrap_or(false)
        } else {
            false
        };
        let policy = self.special_movement_policy(permit, actor, session).await;
        let Some((mut runtime, mut states)) = self.lock_quest_teleport_owners().await else {
            return Some(entered);
        };
        if runtime.read_actor_position(actor) != Ok(entered)
            || runtime.bound_attacker_lease(actor, session) != Ok(lease)
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !self.states.player_trades.physical(session, None)
            || entrance(map, entered.position()) != Some(entry)
        {
            return Some(entered);
        }
        let origin = origin.position();
        let destination = if allowed {
            MovementLocalPosition {
                x: entry.destination.0,
                y: entry.destination.1,
                floor: 14,
            }
        } else {
            self.states
                .chat
                .push_quest_feedback(session, "You can not use this portal yet.");
            MovementLocalPosition {
                x: origin.x + entry.refusal_offset.0,
                y: origin.y + entry.refusal_offset.1,
                floor: origin.floor,
            }
        };
        let now = self.owner_now();
        let blocked = super::super::pvp_live::pz_blocked(&states, actor, session, now);
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let prepare = |target| {
            crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                self.movement_cells(),
                actor,
                session,
                entered,
                target,
                blocked,
                &doors,
            )
        };
        let proof = if allowed {
            prepare(destination)
        } else {
            prepare(destination).or_else(|_| prepare(origin))
        };
        let Ok(proof) = proof else {
            return Some(entered);
        };
        let Ok(snapshot) = runtime.commit_source_step(proof) else {
            return Some(entered);
        };
        if let Some(position) = native(snapshot.position()) {
            map.facts.emit_movement_effect(position, 11);
        }
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            now.get(),
            Some(entered.position()),
            snapshot.position(),
            policy,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now.get(),
            underwater(map, entered.position()),
            underwater(map, snapshot.position()),
        );
        Some(snapshot)
    }
}
