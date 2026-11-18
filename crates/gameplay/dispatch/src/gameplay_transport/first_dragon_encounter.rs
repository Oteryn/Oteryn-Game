//! Native First Dragon phase hooks on the shared boss activity.
use super::ComposedFreshAdmission;
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

pub(super) async fn step_in(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    committed: MovementPositionSnapshot,
    now_ms: u64,
) -> MovementPositionSnapshot {
    let Some(map) = owner.bundle_map() else {
        return committed;
    };
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    if !states
        .encounter_phases
        .first_dragon_portal_at(&runtime, committed.position(), now_ms)
    {
        return committed;
    }
    let destination = MovementLocalPosition {
        x: 33621,
        y: 31023,
        floor: committed.position().floor,
    };
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let blocked = super::pvp_live::pz_blocked(&states, actor, session, owner.owner_now());
    let Ok(proof) = crate::map::floor::prepare_scripted_quest_teleport_facing(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        committed,
        destination,
        blocked,
        &closed,
        committed.facing(),
        true,
    ) else {
        return committed;
    };
    runtime.commit_source_step(proof).unwrap_or(committed)
}

const FALLEN: &str = "oteryn:creature.fallen_challenger";
const SPIRIT: &str = "oteryn:creature.spirit_of_fertility";
const UNBEATABLE: &str = "oteryn:creature.unbeatable_dragon";
const ESSENCE: &str = "oteryn:creature.dragon_essence";
const SOMEWHAT: &str = "oteryn:creature.somewhat_beatable";
const BOSS: &str = "oteryn:creature.the_first_dragon";
const TERMINAL: MovementLocalPosition = cell(33617, 31023);

const fn cell(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 14 }
}

fn walkable(cells: &NativeEntryMovementCells, at: MovementLocalPosition) -> bool {
    cells
        .index()
        .lookup(
            cells.scope(),
            LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        )
        .ok()
        == Some(CollisionClass::Walkable)
}

fn spawn_in(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    key: &str,
    count: usize,
    positions: impl IntoIterator<Item = MovementLocalPosition>,
) -> Result<Vec<ExactActorRef>, CarrierError> {
    let health = runtime.companion_policy(key)?.maximum_health;
    let placements = positions
        .into_iter()
        .filter(|at| walkable(cells, *at) && runtime.native_summon_cell_free(*at))
        .take(count)
        .map(|at| (key, health, at))
        .collect::<Vec<_>>();
    if placements.len() != count {
        return Err(CarrierError::InvalidSpawnDefinition);
    }
    runtime.commit_quest_creature_spawn(&placements)
}

fn shuffled_room(
    runtime: &ChannelRuntimeV1,
    seed: [u8; 16],
    label: &str,
    x: std::ops::RangeInclusive<i32>,
    y: std::ops::RangeInclusive<i32>,
) -> std::vec::IntoIter<MovementLocalPosition> {
    let mut positions = y
        .flat_map(|y| x.clone().map(move |x| cell(x, y)))
        .collect::<Vec<_>>();
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let occurrence = DecisionOccurrenceId::from_bytes(seed);
    for ordinal in (1..positions.len()).rev() {
        let Ok(draw) = deterministic_decision_u64(&root, occurrence, label, ordinal as u64) else {
            break;
        };
        let index = (u128::from(draw) * (ordinal as u128 + 1)) >> 64;
        positions.swap(ordinal, index as usize);
    }
    positions.into_iter()
}

/// Crystal actions_lever.lua:4,32–44. The terminal boss is created only after
/// a Dragon Essence dies. All coordinates are in the qualified bundle frame.
pub(super) fn spawn_initial(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
) -> Result<(), CarrierError> {
    spawn_in(
        runtime,
        cells,
        FALLEN,
        5,
        [
            cell(33592, 31013),
            cell(33583, 31022),
            cell(33574, 31013),
            cell(33574, 31031),
            cell(33592, 31031),
        ],
    )?;
    // Crystal force-creates the Spirit at the water tile itself.
    let spirit_at = cell(33625, 31021);
    cells
        .index()
        .lookup(
            cells.scope(),
            LogicalCell {
                x: spirit_at.x,
                y: spirit_at.y,
                z: i32::from(spirit_at.floor),
            },
        )
        .map_err(|_| CarrierError::InvalidSpawnDefinition)?;
    if !runtime.native_summon_cell_free(spirit_at) {
        return Err(CarrierError::InvalidSpawnDefinition);
    }
    let health = runtime.companion_policy(SPIRIT)?.maximum_health;
    let spirit = runtime.commit_quest_creature_spawn(&[(SPIRIT, health, spirit_at)])?[0];
    let positions = shuffled_room(
        runtime,
        spirit.placement_identity(),
        "first_dragon_unbeatable",
        33610..=33622,
        31016..=31030,
    );
    spawn_in(runtime, cells, UNBEATABLE, 5, positions)?;
    Ok(())
}

#[derive(Debug)]
pub(super) struct Phase {
    cells: NativeEntryMovementCells,
    deaths: Vec<ExactActorRef>,
    counts: Vec<(GameSessionId, u8)>,
    pending_waves: usize,
    essences: Vec<ExactActorRef>,
    terminal_pending: bool,
    terminal_spawned: bool,
}

impl Phase {
    pub(super) fn new(cells: NativeEntryMovementCells) -> Self {
        Self {
            cells,
            deaths: Vec::new(),
            counts: Vec::new(),
            pending_waves: 0,
            essences: Vec::new(),
            terminal_pending: false,
            terminal_spawned: false,
        }
    }

    pub(super) fn death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        creature: &str,
    ) {
        if self.terminal_spawned || self.deaths.contains(&actor) {
            return;
        }
        if creature == ESSENCE && self.essences.contains(&actor) {
            self.terminal_pending = true;
        } else if creature == SOMEWHAT {
            let Ok(census) = runtime.positioned_actor_census() else {
                return;
            };
            for (_, position, session) in census {
                let Some(session) = session else {
                    continue;
                };
                let p = position.position();
                if p.floor != 14
                    || !(33603..=33631).contains(&p.x)
                    || !(31009..=31037).contains(&p.y)
                {
                    continue;
                }
                if let Some((_, count)) = self
                    .counts
                    .iter_mut()
                    .find(|(member, _)| *member == session)
                {
                    *count += 1;
                    if *count == 5 {
                        *count = 0;
                        self.pending_waves += 1;
                    }
                } else {
                    self.counts.push((session, 1));
                }
            }
        } else {
            return;
        }
        self.deaths.push(actor);
        self.advance(runtime);
    }

    pub(super) fn advance(&mut self, runtime: &mut ChannelRuntimeV1) {
        if self.terminal_pending {
            let Ok(policy) = runtime.companion_policy(BOSS) else {
                return;
            };
            if runtime
                .replace_quest_encounter_group(
                    &self.essences,
                    (BOSS, policy.maximum_health, TERMINAL),
                )
                .is_ok()
            {
                self.terminal_pending = false;
                self.terminal_spawned = true;
                self.essences.clear();
            }
            return;
        }
        if let Some(death) = self.deaths.last() {
            while self.pending_waves != 0 {
                let positions = shuffled_room(
                    runtime,
                    death.placement_identity(),
                    "first_dragon_essence",
                    33609..=33624,
                    31017..=31028,
                );
                let Ok(actors) = spawn_in(runtime, &self.cells, ESSENCE, 6, positions) else {
                    return;
                };
                self.essences.extend(actors);
                self.pending_waves -= 1;
            }
        }
    }
}
