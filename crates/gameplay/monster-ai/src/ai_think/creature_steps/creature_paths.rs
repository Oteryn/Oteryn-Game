//! Bounded path requests retained in the creature's existing table entry, never a second queue.
use super::*;
use crate::ai::{AiProvenance, AiProvenanceInput, PathRequest, RouteStep, build_path_proposal};
use crate::movement::path_search::{CREATURE_COST, measured_route};

impl CreatureAiTable {
    pub(super) fn drain_creature_paths(
        &mut self,
        runtime: &ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        blocking: &dyn CellBlocker,
        now: u64,
        budget: &mut ChannelPathBudget,
        player_waiting: bool,
    ) {
        let mut order: Vec<_> = self
            .entries
            .values()
            .filter_map(|entry| {
                Some((
                    entry.motion.pending?,
                    (
                        entry.state.actor.actor_local_id(),
                        entry.state.actor.actor_local_generation(),
                    ),
                    entry.state.actor,
                ))
            })
            .collect();
        order.sort_by_key(|(deadline, identity, _)| (*deadline, *identity));
        budget.declare_creatures(order.len());
        if order.is_empty() {
            return;
        }
        let mut occupied = std::collections::BTreeMap::<(i32, i32, i16), usize>::new();
        for (_, at) in crate::movement::path_search::standing_positions(runtime) {
            *occupied.entry((at.x, at.y, at.floor)).or_default() += 1;
        }
        // No position/map mutation occurs during this drain. Reuse Movement's admission
        // answers across requests; the next drain and every step commit read fresh facts.
        let mut admissible = std::collections::HashMap::new();
        for (_, _, actor) in order {
            if !budget.admits(now, false, player_waiting) {
                break;
            }
            let Some(entry) = self.entry_mut(actor) else {
                continue;
            };
            let motion = &mut entry.motion;
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                motion.stop();
                continue;
            };
            let start = snapshot.position();
            let (target, exact) = match motion.goal {
                MovementGoal::Chase { at, .. } | MovementGoal::KeepDistance { at, .. } => {
                    (at, false)
                }
                MovementGoal::WalkBack { spawn } => (spawn, true),
                _ => {
                    motion.pending = None;
                    continue;
                }
            };
            // One charge per actual expansion; the admitted maximum is refunded automatically.
            let (route, work) = measured_route(start, target, exact, |at| {
                if (!exact && at == target)
                    || !*admissible.entry((at.x, at.y, at.floor)).or_insert_with(|| {
                        crate::movement::relocate::creature_cell_admissible(cells, blocking, at)
                    })
                {
                    return None;
                }
                let count = occupied.get(&(at.x, at.y, at.floor)).copied().unwrap_or(0);
                Some(if count > usize::from(at == start) {
                    CREATURE_COST
                } else {
                    0
                })
            });
            budget.charge(false, work);
            motion.pending = None;
            match route {
                Some(route) => {
                    let steps: Vec<_> = route
                        .into_iter()
                        .map(|step| {
                            let node = match step {
                                crate::movement::CardinalStep::North => 0,
                                crate::movement::CardinalStep::East => 1,
                                crate::movement::CardinalStep::South => 2,
                                crate::movement::CardinalStep::West => 3,
                            };
                            RouteStep::new(node, 1)
                        })
                        .collect();
                    motion.route = build_path_proposal(
                        PathRequest::new(
                            provenance(runtime, actor, motion),
                            entry.sequence,
                            1,
                            work,
                        ),
                        &steps,
                    )
                    .ok();
                    motion.expected = Some(start);
                    motion.cursor = 0;
                }
                None => {
                    let back = matches!(motion.goal, MovementGoal::WalkBack { .. });
                    motion.stop();
                    entry.state.unreachable = !back;
                    if back {
                        entry.due_us = None;
                    }
                }
            }
        }
    }
}

pub(super) fn provenance(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    motion: &CreatureMotion,
) -> AiProvenance {
    // Exact actor, full content/map revisions and goal remain held alongside this immutable
    // carrier. These compact markers follow the existing AI snapshot convention.
    let marker = |bytes: [u8; 32]| u64::from_be_bytes(bytes[..8].try_into().unwrap_or_default());
    let identity = actor.placement_identity();
    AiProvenance::new(AiProvenanceInput {
        scope_id: u64::from_be_bytes(
            runtime.binding().channel_id().as_bytes()[..8]
                .try_into()
                .unwrap_or_default(),
        ),
        scope_generation: runtime.binding().scope_generation().get(),
        actor_generation: u64::from_be_bytes(identity[..8].try_into().unwrap_or_default()),
        behavior_revision: marker(motion.content),
        content_revision: marker(motion.content),
        navigation_revision: marker(motion.navigation),
        ruleset_revision: 1,
        determinism_profile_revision: 1,
    })
}
