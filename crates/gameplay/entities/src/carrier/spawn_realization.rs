use super::*;

/// A content-authored spawn source's small stable ordinal within its scope (content input,
/// §4.8; never decoded from a client handle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpawnSourceId(pub u16);

/// §4.3/§4.8: one spawn source's content-declared definition. Code never invents these
/// values; a missing one is a content-validation failure at a higher layer, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnDefinition {
    /// §4.3: "one placement cell per creature" -- exactly `population` cells, never more
    /// (`placement_cells.len() == population <= CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX`).
    /// Placement never searches beyond these.
    pub(super) placement_cells: Vec<LocalPosition>,
    /// `1..=CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX`, and exactly `placement_cells.len()`.
    population: usize,
    /// SPAWN-1b (Codex P1 on #1981): what each point spawns, by point ordinal (same order and
    /// length as `placement_cells`). An authored source may mix creatures and respawn delays
    /// across its points, so these are per point, and the source stays one runtime source.
    points: Vec<SpawnPointDefinition>,
    /// D115: 5,000,000 (5 s).
    occupancy_retry_interval_micros: u64,
}

/// SPAWN-1b: what one spawn point realizes and when it respawns.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnPointDefinition {
    /// Oteryn default, not Reference-qualified: standalone area factor (no event composition).
    /// Assigned only by the admitted map owner.
    cyclopedia_improved: bool,
    /// D115: 60,000,000 (60 s, in the owner clock's microsecond unit).
    respawn_delay_micros: u64,
    creature_target_identity: String,
    /// Always positive.
    creature_initial_health: i64,
    /// CREATURE-AI-0 §6.3: the point's creature profile `blocked_by_nearby_players`. It is per
    /// point because a source may mix blockable and non-blockable creatures (Codex P2 on
    /// #2002). The uniform fixture source is not blockable; only a test definition sets it.
    blockable: bool,
}

/// SPAWN-1b: one point of a source being built: its placement cell and what it spawns.
type SpawnPointBuild = ((i32, i32, i16), SpawnPointDefinition);

impl SpawnDefinition {
    /// A source whose every point spawns the same creature with the same respawn delay.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        placement_cells: Vec<(i32, i32, i16)>,
        population: usize,
        respawn_delay_micros: u64,
        occupancy_retry_interval_micros: u64,
        creature_target_identity: String,
        creature_initial_health: i64,
    ) -> Result<Self, CarrierError> {
        // §4.3: "one declared cell per creature" -- a mismatch either way (fewer cells than
        // the population, or extra unused cells) is invalid, not merely clamped, so
        // `resolve_respawn_timer` can never respawn more than the declared population.
        if population != placement_cells.len() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        // `CREATUREAI0-RL-03` is checked before one point is built.
        if placement_cells.len() > CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        Self::with_points(
            placement_cells
                .into_iter()
                .map(|cell| {
                    (
                        cell,
                        SpawnPointDefinition {
                            cyclopedia_improved: false,
                            respawn_delay_micros,
                            creature_target_identity: creature_target_identity.clone(),
                            creature_initial_health,
                            blockable: false,
                        },
                    )
                })
                .collect(),
            occupancy_retry_interval_micros,
        )
    }

    /// SPAWN-1b: a source from its points in point-ordinal order, each its cell and what it
    /// spawns. `CREATUREAI0-RL-03` bounds the points; every point's health must be positive.
    fn with_points(
        points: Vec<SpawnPointBuild>,
        occupancy_retry_interval_micros: u64,
    ) -> Result<Self, CarrierError> {
        if points.is_empty() || points.len() > CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let population = points.len();
        let mut placement_cells = Vec::with_capacity(population);
        let mut definitions = Vec::with_capacity(population);
        for ((x, y, floor), point) in points {
            if point.creature_initial_health <= 0 {
                return Err(CarrierError::InvalidSpawnDefinition);
            }
            placement_cells.push(LocalPosition { x, y, floor });
            definitions.push(point);
        }
        Ok(Self {
            placement_cells,
            population,
            points: definitions,
            occupancy_retry_interval_micros,
        })
    }

    /// SPAWN-1a: the activated fixture spawn source as a definition. The cells, population,
    /// delays (ms to the owner clock's µs), creature key and authored initial health all come
    /// from the source; nothing is defaulted here.
    pub(super) fn from_activation(facts: ActivationSpawnFacts) -> Result<Self, CarrierError> {
        let (cells, respawn_delay_ms, occupancy_retry_interval_ms, creature, initial_health) =
            facts;
        let population = cells.len();
        Self::new(
            cells,
            population,
            activation_micros(respawn_delay_ms)?,
            activation_micros(occupancy_retry_interval_ms)?,
            creature,
            activation_health(initial_health)?,
        )
    }

    /// SPAWN-1b: one activated source of mixed points as a definition, in its authored point
    /// order; nothing is defaulted here.
    pub(super) fn from_activation_source(
        facts: ActivationSpawnSourceFacts,
    ) -> Result<Self, CarrierError> {
        let (points, occupancy_retry_interval_ms) = facts;
        if points.len() > CREATUREAI0_SPAWN_POINTS_PER_SOURCE_MAX {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let points = points
            .into_iter()
            .map(
                |(cell, respawn_delay_ms, creature, initial_health, blockable)| {
                    Ok((
                        cell,
                        SpawnPointDefinition {
                            cyclopedia_improved: false,
                            respawn_delay_micros: activation_micros(respawn_delay_ms)?,
                            creature_target_identity: creature,
                            creature_initial_health: activation_health(initial_health)?,
                            blockable,
                        },
                    ))
                },
            )
            .collect::<Result<Vec<_>, CarrierError>>()?;
        Self::with_points(points, activation_micros(occupancy_retry_interval_ms)?)
    }

    /// Every point of this source blockable or not.
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_blockable(mut self, blockable: bool) -> Self {
        for point in &mut self.points {
            point.blockable = blockable;
        }
        self
    }

    /// The definition of point `ordinal`.
    fn point(&self, ordinal: usize) -> Result<&SpawnPointDefinition, CarrierError> {
        self.points
            .get(ordinal)
            .ok_or(CarrierError::UnknownSpawnCell)
    }

    /// The respawn timing of point `ordinal`.
    pub(super) fn timing(&self, ordinal: usize) -> Result<spawn::PointTiming, CarrierError> {
        let point = self.point(ordinal)?;
        Ok(spawn::PointTiming {
            respawn_delay_micros: point.respawn_delay_micros
                / if point.cyclopedia_improved { 2 } else { 1 },
            occupancy_retry_interval_micros: self.occupancy_retry_interval_micros,
            blockable: point.blockable,
        })
    }
}

/// An activation delay in ms as the owner clock's µs.
fn activation_micros(ms: u64) -> Result<u64, CarrierError> {
    ms.checked_mul(1_000)
        .ok_or(CarrierError::InvalidSpawnDefinition)
}

/// An activation initial health as the carrier's signed health.
fn activation_health(health: u64) -> Result<i64, CarrierError> {
    i64::try_from(health).map_err(|_| CarrierError::InvalidSpawnDefinition)
}

/// CREATURE-AI-0 §10: checks one activation against the registered spawn ceilings before
/// anything is reserved or admitted, and returns its total point count. More than
/// `CREATUREAI0-RL-02` sources or more than `CREATUREAI0-RL-01` points refuses with
/// `CapacityExceeded`; `CREATUREAI0-RL-03` is already enforced per source by
/// [`SpawnDefinition::new`]. The work is one pass over the sources.
pub(super) fn spawn_points_within_ceilings(
    definitions: &[SpawnDefinition],
) -> Result<usize, CarrierError> {
    if definitions.len() > CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
        return Err(CarrierError::CapacityExceeded);
    }
    let mut total = 0usize;
    for definition in definitions {
        total = total
            .checked_add(definition.placement_cells.len())
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        if total > CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
    }
    Ok(total)
}

/// SPAWN-1a: one activated spawn source as the carrier reads it: placement cells in order,
/// respawn delay and occupancy retry interval (ms), creature key and authored initial health.
/// Content produces it (`NativeEntrySpawnSource::activation_facts`); Foundation does not name
/// the content type.
pub type ActivationSpawnFacts = (Vec<(i32, i32, i16)>, u64, u64, String, u64);

/// SPAWN-1b (Codex P1 on #1981): one activated spawn point as the carrier reads it: its
/// placement cell, respawn delay (ms), creature key, authored initial health and its creature
/// profile's `blocked_by_nearby_players` (CREATURE-AI-0 §6.3; Codex P2 on #2002), which is per
/// point because one source may mix blockable and non-blockable creatures.
pub type ActivationSpawnPointFacts = ((i32, i32, i16), u64, String, u64, bool);

/// SPAWN-1b (Codex P1 on #1981): one activated spawn source whose points may differ in
/// creature, respawn delay and initial health: its points in authored (point-ordinal) order and
/// its occupancy retry interval (ms). It is one runtime source, so it counts once against
/// `CREATUREAI0-RL-02` and its point ordinals run `0..points` without a reset.
pub type ActivationSpawnSourceFacts = (Vec<ActivationSpawnPointFacts>, u64);

/// SPAWN-1a: what one respawn drive did, in run order. The warning is reported, not
/// broadcast: no client effect exists for it yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnDriveReport {
    pub warnings: Vec<MovementLocalPosition>,
    pub admitted: Vec<ExactActorRef>,
    pub blocked: usize,
    pub retried: usize,
    pub skipped: usize,
}

/// One placement cell's live/pending bookkeeping within a realized spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpawnCellState {
    /// The live creature currently occupying this cell, if any.
    pub(super) live: Option<ExactActorRef>,
    /// This cell's current respawn chain's attempt index (1-3), 0 before any attempt.
    pub(super) attempt: u8,
    /// Bumped each time an occurrence chain ends `Skipped` and a fresh one starts (§4.3
    /// "terminal disposition"): keeps every occurrence identity for this cell unique.
    pub(super) successor: u32,
}

/// One realized spawn source and its per-cell state, for this carrier's ownership
/// generation (§4.3: `EphemeralScopeReset` -- a restart/scope-move realizes it again; nothing
/// here is durable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpawnRealization {
    pub(super) source: SpawnSourceId,
    pub(super) definition: SpawnDefinition,
    /// Same order/length as `definition.placement_cells`.
    pub(super) cells: Vec<SpawnCellState>,
}

/// AI-RL for respawn timers (§4.2's table; no separate `RESOURCE_LIMITS_REGISTRY.json` row is
/// named for it distinctly from AI01-PENDING-TIMERS-PER-ACTOR): at most one pending respawn
/// timer per dead creature (`OwnerTimerLane`'s `target` key), which bounds the aggregate
/// pending count for one spawn to its own population (§4.9's derived "at most the spawn's
/// population pending").
pub const AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR: usize = 1;

/// AI-2's one timer family: respawn (§4.2's table: `DEADLINE_STATE`, distinct from AI-1's
/// `SKIP_TO_LATEST` think family -- every due retry/successor must still fire, never collapsed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnFamily {
    Respawn,
}

impl oteryn_gameplay_foundation::foundation::owner_timer::TimerFamily for RespawnFamily {
    fn registered_maximum(self) -> usize {
        match self {
            Self::Respawn => AI01_PENDING_RESPAWN_TIMERS_PER_DEAD_ACTOR,
        }
    }
}

/// A respawn occurrence identity (§4.2: "(spawn source, cell, the dead actor's
/// `ExactActorRef`), and each retry adds its attempt index 1 to 3"; §4.3's terminal
/// disposition adds a successor index for the chain that follows a `Skipped` occurrence).
/// Binding item (a) (bounded replay evidence, carried over from AI-1): `attempt` and
/// `successor` only ever increase for a given `(source, cell_index, dead_actor)` (enforced by
/// `resolve_respawn_timer`, the only place that mints one), so the full tuple can never repeat
/// in a lane's lifetime -- bounded, O(1) state per cell (`SpawnCellState`), never an unbounded
/// log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnOccurrence {
    pub source: SpawnSourceId,
    pub cell_index: u8,
    pub dead_actor: ExactActorRef,
    pub successor: u32,
    pub attempt: u8,
}

/// The result of applying one fired respawn timer (§4.3). This never mutates state itself
/// beyond `ChannelActorCarrier::resolve_respawn_timer`'s own admission/bookkeeping; scheduling
/// the next timer (if any) is the caller's job, exactly as `OwnerTimerLane` requires (binding
/// item (b): the caller's own fence-issued `RuntimeWorkStamp`, never fabricated here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnResolution {
    /// A new creature was admitted at the cell with a fresh actor-local generation (D52: the
    /// dead actor's generation is never reused).
    Admitted(ExactActorRef),
    /// The cell was occupied; the caller schedules one fresh occurrence with `next_attempt`,
    /// due `occupancy_retry_interval_micros` later.
    Postponed { next_attempt: u8 },
    /// The third retry also found the cell occupied: this chain ends `Skipped`, nothing
    /// admitted. The caller schedules one new successor occurrence (`next_successor`, attempt
    /// 1) due one full `respawn_delay_micros` later.
    Skipped { next_successor: u32 },
}

/// SPAWN-CEILINGS-1 (CREATURE-AI-0 §6.2, `CREATUREAI0-RL-14`): an activation in progress: its
/// points in canonical order (source ordinal, point ordinal), split by
/// [`spawn::realization_windows`] into owner windows of at most
/// [`spawn::REALIZATIONS_PER_WINDOW_MAX`], and the cursor of the next window.
#[derive(Debug)]
pub(super) struct SpawnActivation {
    /// The definitions by source ordinal; a source's entry is taken at its point 0.
    definitions: Vec<Option<SpawnDefinition>>,
    /// The realization plan; a realized window is emptied.
    windows: Vec<Vec<spawn::SpawnPoint>>,
    /// The next window to realize: the persisted cursor.
    next_window: usize,
    /// Points realized so far.
    realized_points: usize,
    /// Points still to realize.
    pub(super) remaining_points: usize,
    /// The owner window (`now / OWNER_WINDOW_MICROS`) that realized the last window.
    pub(super) last_window: Option<u64>,
}

impl SpawnActivation {
    fn new(definitions: Vec<SpawnDefinition>, total_points: usize) -> Result<Self, CarrierError> {
        let mut points = Vec::new();
        points
            .try_reserve_exact(total_points)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (source, definition) in definitions.iter().enumerate() {
            let source = u16::try_from(source).map_err(|_| CarrierError::CapacityExceeded)?;
            for ordinal in 0..definition.placement_cells.len() {
                let ordinal = u8::try_from(ordinal).map_err(|_| CarrierError::CapacityExceeded)?;
                points.push(spawn::SpawnPoint { source, ordinal });
            }
        }
        let windows = spawn::realization_windows(&points);
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(definitions.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        pending.extend(definitions.into_iter().map(Some));
        Ok(Self {
            definitions: pending,
            windows,
            next_window: 0,
            realized_points: 0,
            remaining_points: total_points,
            last_window: None,
        })
    }

    pub(super) const fn progress(&self, realized: usize) -> SpawnActivationProgress {
        SpawnActivationProgress {
            realized,
            realized_total: self.realized_points,
            remaining: self.remaining_points,
        }
    }
}

/// SPAWN-CEILINGS-1: what one activation window call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnActivationProgress {
    /// Creatures this call realized: 0 when its owner window had already realized one.
    pub realized: usize,
    /// Creatures the activation has realized so far.
    pub realized_total: usize,
    /// Points still to realize; 0 means the activation is complete.
    pub remaining: usize,
}

/// SPAWN-CEILINGS-1: one owner window of the respawn drive. The first drive of a window runs
/// the window's gone scan; every drive of it then runs the occurrences whose deadlines have
/// passed, from the deadline index, within the window's step budget.
#[derive(Debug)]
pub(super) struct SpawnDriveWindow {
    pub(super) window: u64,
    /// Steps this window may still run ([`SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX`]).
    pub(super) steps_left: usize,
}

impl ChannelRuntimeV1 {
    /// SPAWN-1a (CREATURE-AI-0 §6.2): realizes the activated spawn sources at Channel
    /// activation, before the Channel admits players, in one owner turn. Sources are realized
    /// in canonical order (source ordinal `i`, point ordinal = cell index), each creature a
    /// fresh actor-local generation with the source's authored initial health. A creature
    /// whose definition the active Creature policy table names is bound to that policy;
    /// otherwise it stands unbound. Any refusal realizes nothing. Returns the number of
    /// creatures realized.
    ///
    /// One owner turn realizes at most one owner window, `CREATUREAI0-RL-14`
    /// ([`spawn::REALIZATIONS_PER_WINDOW_MAX`] points): a larger activation refuses here with
    /// `CapacityExceeded` and nothing realized, and runs through
    /// [`Self::begin_activation_spawns`] and [`Self::realize_activation_spawn_window`] instead.
    pub fn realize_activation_spawns(
        &mut self,
        sources: Vec<ActivationSpawnFacts>,
    ) -> Result<usize, CarrierError> {
        let total_points = self.begin_activation_spawns(sources)?;
        self.realize_begun_activation_in_one_turn(total_points)
    }
    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn realize_spawn_definitions(
        &mut self,
        definitions: Vec<SpawnDefinition>,
    ) -> Result<usize, CarrierError> {
        let total_points = self.begin_spawn_definitions(definitions)?;
        self.realize_begun_activation_in_one_turn(total_points)
    }
    /// Completes the activation just begun in this owner turn, which may hold at most one
    /// owner window of points; a larger one is dropped unadmitted and refuses
    /// `CapacityExceeded`. The window is staged under one snapshot: a failure partway (for
    /// example `ActorGenerationExhausted`, `admit_inner`) restores it exactly.
    fn realize_begun_activation_in_one_turn(
        &mut self,
        total_points: usize,
    ) -> Result<usize, CarrierError> {
        if total_points > spawn::REALIZATIONS_PER_WINDOW_MAX {
            self.spawn_activation = None;
            return Err(CarrierError::CapacityExceeded);
        }
        let slots_before = self.carrier.slots.clone();
        let free_before = self.carrier.free_head;
        let occupied_before = self.carrier.occupied.clone();
        match self.realize_activation_spawn_window(0) {
            Ok(progress) => {
                debug_assert_eq!(progress.remaining, 0);
                Ok(progress.realized)
            }
            Err(error) => {
                self.spawn_activation = None;
                self.carrier.slots = slots_before;
                self.carrier.free_head = free_before;
                self.carrier.occupied = occupied_before;
                self.carrier.spawns.clear();
                Err(error)
            }
        }
    }
    pub(super) fn begin_spawn_definitions(
        &mut self,
        definitions: Vec<SpawnDefinition>,
    ) -> Result<usize, CarrierError> {
        let total_points = self.check_spawn_activation(&definitions)?;
        self.spawn_activation = Some(Box::new(SpawnActivation::new(definitions, total_points)?));
        Ok(total_points)
    }
    /// The checks every activation passes before anything is kept or admitted: the owner
    /// fence, no realized or pending activation, every registered ceiling and the slot
    /// capacity, which keeps [`SPAWN_ACTIVATION_PLAYER_HEADROOM`] slots free for player
    /// admission. Returns the total points.
    fn check_spawn_activation(
        &self,
        definitions: &[SpawnDefinition],
    ) -> Result<usize, CarrierError> {
        self.owner_fence()?;
        if !self.carrier.spawns.is_empty() || self.spawn_activation.is_some() {
            return Err(CarrierError::DuplicateSpawnSource);
        }
        // Every ceiling and the slot capacity are checked once, before anything is admitted,
        // so the work is linear in the points, not in sources × slots.
        let total_points = spawn_points_within_ceilings(definitions)?;
        let context = self.pinned_position_context();
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.carrier.validate_position_context(context)?;
        let headroom = spawn_activation_player_headroom(self.carrier.slots.len());
        if self
            .carrier
            .free_slot_count()
            .checked_sub(total_points)
            .is_none_or(|left| left < headroom)
        {
            return Err(CarrierError::CapacityExceeded);
        }
        Ok(total_points)
    }
    /// Realizes the next owner window of `activation`'s plan: each point in canonical order
    /// admits its creature (a source is recorded at its point 0, so a source may span two
    /// windows). The window's creatures are then bound to their Creature policies. Returns the
    /// creatures realized; on a refusal the caller undoes the activation.
    pub(super) fn realize_activation_window(
        &mut self,
        activation: &mut SpawnActivation,
    ) -> Result<usize, CarrierError> {
        let context = self.pinned_position_context();
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.carrier.validate_position_context(context)?;
        let Some(window) = activation.windows.get_mut(activation.next_window) else {
            return Ok(0);
        };
        let points = std::mem::take(window);
        debug_assert!(points.len() <= spawn::REALIZATIONS_PER_WINDOW_MAX);
        for point in &points {
            let definition = if point.ordinal == 0 {
                activation
                    .definitions
                    .get_mut(usize::from(point.source))
                    .and_then(Option::take)
            } else {
                None
            };
            self.carrier
                .admit_spawn_point(&self.continuity, *point, definition, context)?;
        }
        activation.next_window += 1;
        activation.realized_points += points.len();
        activation.remaining_points -= points.len();
        for point in &points {
            if let Ok(state) = self
                .carrier
                .spawn_cell_state(SpawnSourceId(point.source), usize::from(point.ordinal))
                && let Some(actor) = state.live
            {
                self.bind_spawned_creature_policy(actor);
            }
        }
        Ok(points.len())
    }
    /// Binds a realized creature to its active Creature policy when the table names its
    /// definition and agrees on its health. Content that names no policy leaves it unbound.
    pub(super) fn bind_spawned_creature_policy(&mut self, actor: ExactActorRef) {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return;
        };
        let Slot::CreatureOccupied {
            target_identity, ..
        } = &self.carrier.slots[index]
        else {
            return;
        };
        let Ok(key) = std::str::from_utf8(target_identity).map(str::to_owned) else {
            return;
        };
        if self.companion_policy(&key).is_ok() {
            let _ = self.install_creature_policy(actor, &key);
        }
    }
    /// Kills a live creature in place, as a lethal hit would, for owner-turn spawn tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn kill_creature_for_test(&mut self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        let Slot::CreatureOccupied { health, .. } = &mut self.carrier.slots[index] else {
            return false;
        };
        *health = 0;
        true
    }
    /// The live creature of each realized spawn point, in canonical point order (the realized
    /// sources are kept ascending and their cells in ordinal order, so no sort is needed).
    pub fn spawn_point_creatures(
        &self,
    ) -> Vec<(
        spawn::SpawnPoint,
        MovementLocalPosition,
        Option<ExactActorRef>,
    )> {
        let mut points = Vec::new();
        for spawn in &self.carrier.spawns {
            for (index, (cell, state)) in spawn
                .definition
                .placement_cells
                .iter()
                .zip(&spawn.cells)
                .enumerate()
            {
                let Ok(ordinal) = u8::try_from(index) else {
                    continue;
                };
                let live = state
                    .live
                    .filter(|actor| self.carrier.creature_alive(&self.continuity, *actor));
                points.push((
                    spawn::SpawnPoint {
                        source: spawn.source.0,
                        ordinal,
                    },
                    MovementLocalPosition {
                        x: cell.x,
                        y: cell.y,
                        floor: cell.floor,
                    },
                    live,
                ));
            }
        }
        debug_assert!(points.windows(2).all(|pair| pair[0].0 < pair[1].0));
        points
    }
    /// The pending respawn occurrence of `point`, if any.
    pub fn pending_respawn(&self, point: spawn::SpawnPoint) -> Option<spawn::PendingRespawn> {
        self.spawn_schedule.pending(point)
    }
    /// Test only: the spawn records read by the drives since the last call (gone-scan points
    /// and deadline entries), and whether the deadline index still equals the schedule.
    #[cfg(any(test, feature = "test-support"))]
    pub fn take_spawn_drive_visits(&mut self) -> (usize, bool) {
        let consistent = self.spawn_deadlines.len() == self.spawn_schedule.len()
            && self.spawn_deadlines.iter().all(|&(due_micros, point)| {
                self.spawn_schedule
                    .pending(point)
                    .is_some_and(|pending| pending.due_micros == due_micros)
            });
        (std::mem::take(&mut self.spawn_drive_visits), consistent)
    }
    /// Gone detection, once per owner window: reads at most
    /// [`SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX`] realized points in place, from the cursor in
    /// canonical order (sources ascending, cells by ordinal, wrapping) and never one twice, with
    /// no allocation or sort and one generation-exact liveness read per point. A gone creature's
    /// point without an occurrence gets one, one full delay after `now_micros`, and enters the
    /// deadline index. The cursor resumes at the first point at or after where it stopped, so a
    /// source added or removed meanwhile neither repeats nor skips the others.
    pub(super) fn scan_spawn_points(&mut self, now_micros: u64) -> Result<(), CarrierError> {
        let spawns = &self.carrier.spawns;
        let Some(last_source) = spawns.len().checked_sub(1) else {
            return Ok(());
        };
        let cursor = self.spawn_scan_cursor;
        let mut source = spawns.partition_point(|spawn| spawn.source.0 < cursor.source);
        let mut cell = match spawns.get(source) {
            Some(spawn) if spawn.source.0 == cursor.source => usize::from(cursor.ordinal),
            _ => 0,
        };
        if source > last_source {
            (source, cell) = (0, 0);
        }
        if cell >= spawns[source].cells.len() {
            (source, cell) = (if source == last_source { 0 } else { source + 1 }, 0);
        }
        let start = (source, cell);
        let mut read = 0_usize;
        while read < SPAWN_GONE_SCAN_POINTS_PER_WINDOW_MAX {
            let spawn = &spawns[source];
            read += 1;
            #[cfg(any(test, feature = "test-support"))]
            {
                self.spawn_drive_visits += 1;
            }
            let gone = !spawn.cells[cell]
                .live
                .is_some_and(|actor| self.carrier.creature_alive(&self.continuity, actor));
            let point = spawn::SpawnPoint {
                source: spawn.source.0,
                ordinal: u8::try_from(cell).map_err(|_| CarrierError::CapacityExceeded)?,
            };
            if gone && self.spawn_schedule.pending(point).is_none() {
                let due_micros = self
                    .spawn_schedule
                    .creature_gone(point, spawn.definition.timing(cell)?, now_micros)
                    .map_err(|_| CarrierError::CapacityExceeded)?;
                self.spawn_deadlines.insert((due_micros, point));
            }
            cell += 1;
            if cell == spawn.cells.len() {
                (source, cell) = (if source == last_source { 0 } else { source + 1 }, 0);
            }
            if (source, cell) == start {
                break;
            }
        }
        let next = &spawns[source];
        self.spawn_scan_cursor = spawn::SpawnPoint {
            source: next.source.0,
            ordinal: u8::try_from(cell).map_err(|_| CarrierError::CapacityExceeded)?,
        };
        Ok(())
    }
    fn spawn_realization(
        &self,
        point: spawn::SpawnPoint,
    ) -> Result<&SpawnRealization, CarrierError> {
        self.carrier
            .spawn_index(SpawnSourceId(point.source))
            .map(|index| &self.carrier.spawns[index])
            .ok_or(CarrierError::UnknownSpawnSource)
    }
    /// Apply the current World epoch's area membership to home-spawn timing only.
    /// Reassignment is idempotent; raids, summons and other non-home actors have no point.
    pub fn apply_cyclopedia_respawn(
        &mut self,
        improved: impl Fn(LocalPosition, &str) -> bool,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        for source in &mut self.carrier.spawns {
            for (cell, point) in source
                .definition
                .placement_cells
                .iter()
                .zip(&mut source.definition.points)
            {
                point.cyclopedia_improved = improved(*cell, &point.creature_target_identity);
            }
        }
        Ok(())
    }
    pub(super) fn spawn_timing(
        &self,
        point: spawn::SpawnPoint,
    ) -> Result<spawn::PointTiming, CarrierError> {
        self.spawn_realization(point)?
            .definition
            .timing(usize::from(point.ordinal))
    }
    pub(super) fn spawn_cell(
        &self,
        point: spawn::SpawnPoint,
    ) -> Result<MovementLocalPosition, CarrierError> {
        let cell = self
            .spawn_realization(point)?
            .definition
            .placement_cells
            .get(usize::from(point.ordinal))
            .ok_or(CarrierError::UnknownSpawnCell)?;
        Ok(MovementLocalPosition {
            x: cell.x,
            y: cell.y,
            floor: cell.floor,
        })
    }
    /// §6.3 Blocked reads the present players of [`Self::positioned_actor_census`] (committed,
    /// positioned and in control); their interest area (MOVE-RL-11) is the caller's view
    /// around these positions. SPAWN-CEILINGS-1 (Codex 4242270322): they are read from the
    /// occupied index without building the census of every creature; a player under a foreign
    /// position context refuses, as in the census.
    pub(super) fn present_player_positions(
        &self,
    ) -> Result<Vec<MovementLocalPosition>, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        self.owner_fence()?;
        let context = self.pinned_position_context();
        let mut players = Vec::new();
        for index in &self.carrier.occupied {
            let Some(Slot::Occupied {
                game_session_id: Some(_),
                committed: true,
                lifecycle,
                position: Some(version),
                ..
            }) = usize::try_from(*index)
                .ok()
                .and_then(|index| self.carrier.slots.get(index))
            else {
                continue;
            };
            if lifecycle.control_loss.is_some() {
                continue;
            }
            if version.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            players.push(MovementLocalPosition {
                x: version.position.x,
                y: version.position.y,
                floor: version.position.floor,
            });
        }
        Ok(players)
    }
}

impl ChannelActorCarrier {
    // AI-2 (§4.3, §4.9, D116): spawn realization and respawn.

    /// The index of `source` in `self.spawns`, which is kept in strictly ascending source
    /// order: a binary search, not a scan of every realized source.
    fn spawn_index(&self, source: SpawnSourceId) -> Option<usize> {
        self.spawns
            .binary_search_by_key(&source.0, |spawn| spawn.source.0)
            .ok()
    }
    /// Reusable vacant slots: the most creatures this carrier can still admit.
    pub(super) fn free_slot_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, Slot::VacantReusable { .. }))
            .count()
    }
    /// §4.3: realizes one spawn source outside an activation batch: admits one creature per
    /// declared placement cell, each with a fresh actor-local generation. Every fallible check
    /// that does not itself mutate `self` (continuity, the position context, duplicate source,
    /// the registered `CREATUREAI0-RL-02` source and `CREATUREAI0-RL-01` point ceilings,
    /// general capacity) precedes any admission. Admission itself is staged: `self.slots`/
    /// `self.free_head`/`self.occupied` are snapshotted first, and a failure partway through
    /// the population (for example `ActorGenerationExhausted`, the one documented exception
    /// that still mutates a single slot on failure, `admit_inner`) rolls the complete
    /// realization back to that snapshot before returning the error, so a retry never
    /// accumulates untracked, unpositioned creatures (GAME-AI-01 §6: over budget means zero
    /// mutation, applied here to any per-creature failure, not only capacity).
    ///
    /// Activation realizes its points through `admit_spawn_point` instead; this per-source
    /// snapshot is for the single-source paths only.
    pub(super) fn realize_spawn(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        definition: SpawnDefinition,
        position_context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(position_context)?;
        if self.spawn_index(source).is_some() {
            return Err(CarrierError::DuplicateSpawnSource);
        }
        if self.spawns.len() >= CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let realized_points = self
            .spawns
            .iter()
            .try_fold(0usize, |sum, spawn| sum.checked_add(spawn.cells.len()))
            .and_then(|sum| sum.checked_add(definition.placement_cells.len()))
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        if realized_points > CREATUREAI0_SPAWN_POINTS_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        if self.free_slot_count() < definition.population {
            return Err(CarrierError::CapacityExceeded);
        }
        let slots_before = self.slots.clone();
        let free_head_before = self.free_head;
        let occupied_before = self.occupied.clone();
        if let Err(error) =
            self.admit_spawn_source(continuity, source, definition, position_context)
        {
            self.slots = slots_before;
            self.free_head = free_head_before;
            self.occupied = occupied_before;
            return Err(error);
        }
        Ok(())
    }
    /// Admits and positions one creature per declared cell of `definition` and records the
    /// source at its ordered place in `self.spawns`. It is not staged: the caller has already
    /// checked continuity, the position context, the registered ceilings and slot capacity,
    /// holds the snapshot, and restores it on any error. A duplicate source or a source past
    /// `CREATUREAI0-RL-02` still refuses before any admission.
    fn admit_spawn_source(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        definition: SpawnDefinition,
        position_context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        let insert_at = match self
            .spawns
            .binary_search_by_key(&source.0, |spawn| spawn.source.0)
        {
            Ok(_) => return Err(CarrierError::DuplicateSpawnSource),
            Err(insert_at) => insert_at,
        };
        if self.spawns.len() >= CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(definition.placement_cells.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.spawns
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for (cell, point) in definition.placement_cells.iter().zip(&definition.points) {
            let actor = self.admit_creature(
                continuity,
                ActorState(0),
                &point.creature_target_identity,
                point.creature_initial_health,
            )?;
            self.initialize_position(continuity, actor, position_context, *cell)?;
            cells.push(SpawnCellState {
                live: Some(ExactActorRef(actor)),
                attempt: 0,
                successor: 0,
            });
        }
        self.spawns.insert(
            insert_at,
            SpawnRealization {
                source,
                definition,
                cells,
            },
        );
        Ok(())
    }
    /// SPAWN-CEILINGS-1 (CREATURE-AI-0 §6.2): admits and positions the creature of one
    /// activation point, in canonical order. Point 0 records its source (with `definition`) at
    /// the source's ordered place in `self.spawns`; every point appends its cell state, so a
    /// source realized across two owner windows stays recorded in between. The caller has
    /// already checked the ceilings and slot capacity. A refusal leaves every creature it
    /// admitted recorded in `self.spawns` for the caller's undo.
    fn admit_spawn_point(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        point: spawn::SpawnPoint,
        definition: Option<SpawnDefinition>,
        position_context: PreProductionPositionContext,
    ) -> Result<(), CarrierError> {
        let source = SpawnSourceId(point.source);
        if point.ordinal == 0 {
            let definition = definition.ok_or(CarrierError::PlanConflict)?;
            let insert_at = match self
                .spawns
                .binary_search_by_key(&source.0, |spawn| spawn.source.0)
            {
                Ok(_) => return Err(CarrierError::DuplicateSpawnSource),
                Err(insert_at) => insert_at,
            };
            if self.spawns.len() >= CREATUREAI0_SPAWN_SOURCES_PER_CHANNEL_MAX {
                return Err(CarrierError::CapacityExceeded);
            }
            let mut cells = Vec::new();
            cells
                .try_reserve_exact(definition.placement_cells.len())
                .map_err(|_| CarrierError::AllocationFailed)?;
            self.spawns
                .try_reserve(1)
                .map_err(|_| CarrierError::AllocationFailed)?;
            self.spawns.insert(
                insert_at,
                SpawnRealization {
                    source,
                    definition,
                    cells,
                },
            );
        }
        let index = self
            .spawn_index(source)
            .ok_or(CarrierError::UnknownSpawnSource)?;
        let realization = &self.spawns[index];
        let ordinal = usize::from(point.ordinal);
        if realization.cells.len() != ordinal {
            return Err(CarrierError::PlanConflict);
        }
        let cell = *realization
            .definition
            .placement_cells
            .get(ordinal)
            .ok_or(CarrierError::PlanConflict)?;
        let spawned = realization.definition.point(ordinal)?;
        let health = spawned.creature_initial_health;
        let identity = spawned.creature_target_identity.clone();
        let actor = self.admit_creature(continuity, ActorState(0), &identity, health)?;
        // Reserved exactly at point 0: the push never reallocates.
        self.spawns[index].cells.push(SpawnCellState {
            live: Some(ExactActorRef(actor)),
            attempt: 0,
            successor: 0,
        });
        self.initialize_position(continuity, actor, position_context, cell)?;
        Ok(())
    }
    /// True when some other *live* actor (player or creature, `health > 0`) currently occupies
    /// `cell` under `position_context` (§4.3: "if the cell is occupied when the timer is
    /// due"). A dead creature's own corpse (`health == 0`) never counts.
    pub(super) fn cell_occupied(
        &self,
        position_context: PreProductionPositionContext,
        cell: LocalPosition,
    ) -> bool {
        self.slots.iter().any(|slot| match slot {
            Slot::Occupied {
                committed: true,
                position: Some(version),
                ..
            }
            | Slot::CreatureOccupied {
                position: Some(version),
                health: 1..,
                ..
            } => version.context == position_context && version.position == cell,
            Slot::CreatureReserved { planned, .. } => matches!(planned.as_ref(),
                Slot::CreatureOccupied { position: Some(version), .. }
                    if version.context == position_context && version.position == cell),
            Slot::NpcOccupied {
                position: version, ..
            } => version.context == position_context && version.position == cell,
            _ => false,
        })
    }
    /// True when `actor`, at its exact actor-local generation, is still this carrier's live
    /// creature (`health > 0`). A spawn cell may still name a removed creature (removal does
    /// not scan the spawn cells); the generation check keeps that stale reference from reading
    /// as the slot's later occupant.
    pub(super) fn creature_alive(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor: ExactActorRef,
    ) -> bool {
        self.validate_ref(continuity, actor.0).is_ok_and(|index| {
            matches!(
                self.slots[index],
                Slot::CreatureOccupied { generation, health: 1.., .. }
                    if generation == actor.0.actor_local_generation.0
            )
        })
    }
    /// The spawn source and cell index `actor` is (or, if now dead, was) admitted at, if any.
    /// The caller uses this right after a death commits, to schedule that cell's first respawn
    /// occurrence (attempt 1).
    pub(super) fn locate_spawn_cell(&self, actor: ExactActorRef) -> Option<(SpawnSourceId, usize)> {
        self.spawns.iter().find_map(|spawn| {
            spawn
                .cells
                .iter()
                .position(|cell| cell.live == Some(actor))
                .map(|cell_index| (spawn.source, cell_index))
        })
    }
    /// This cell's current respawn-chain bookkeeping (attempt/successor and the actor, if any,
    /// it currently names) -- read-only, for the caller to build the next `RespawnOccurrence`.
    pub(super) fn spawn_cell_state(
        &self,
        source: SpawnSourceId,
        cell_index: usize,
    ) -> Result<SpawnCellState, CarrierError> {
        let spawn = self
            .spawn_index(source)
            .map(|index| &self.spawns[index])
            .ok_or(CarrierError::UnknownSpawnSource)?;
        spawn
            .cells
            .get(cell_index)
            .copied()
            .ok_or(CarrierError::UnknownSpawnCell)
    }
    /// This spawn source's content-declared definition (respawn delay, occupancy retry
    /// interval), for the caller to compute the next occurrence's `due` deadline.
    pub(super) fn spawn_definition(
        &self,
        source: SpawnSourceId,
    ) -> Result<&SpawnDefinition, CarrierError> {
        self.spawn_index(source)
            .map(|index| &self.spawns[index].definition)
            .ok_or(CarrierError::UnknownSpawnSource)
    }
    /// §4.3: applies one fired respawn timer for `(source, cell_index)`. If the cell is free,
    /// admits a fresh creature there (D52: a new actor-local generation; the dead actor's
    /// generation, if its slot still lingers, is retired first). If occupied, advances the
    /// cell's retry/successor bookkeeping and reports what the caller should schedule next
    /// (`RespawnResolution`); this method itself never calls `OwnerTimerLane::schedule` (that
    /// needs the caller's own fence-issued `RuntimeWorkStamp`, binding item (b)).
    /// The context is validated before any lookup or mutation. On the free-cell path, the
    /// dead actor's removal, the replacement's admission, its positioning and the cell's
    /// `live` update are staged and applied atomically: `self.slots`/`self.free_head` are
    /// snapshotted before the removal, and any failure from admission or positioning rolls
    /// the whole step back to that snapshot (so the cell keeps naming the original dead actor
    /// and no unpositioned replacement is left admitted) before returning the error.
    pub(super) fn resolve_respawn_timer(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        cell_index: usize,
        position_context: PreProductionPositionContext,
    ) -> Result<RespawnResolution, CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(position_context)?;
        let spawn_index = self
            .spawn_index(source)
            .ok_or(CarrierError::UnknownSpawnSource)?;
        let cell = *self.spawns[spawn_index]
            .definition
            .placement_cells
            .get(cell_index)
            .ok_or(CarrierError::UnknownSpawnCell)?;
        if self.cell_occupied(position_context, cell) {
            let state = &mut self.spawns[spawn_index].cells[cell_index];
            return Ok(if state.attempt >= AI01_SPAWN_OCCUPANCY_RETRIES_MAX {
                // Terminal disposition (§4.3): this chain ends `Skipped`; a fresh successor
                // chain starts at attempt 0/1 for the same cell, one full respawn delay later.
                state.attempt = 0;
                state.successor = state.successor.saturating_add(1);
                RespawnResolution::Skipped {
                    next_successor: state.successor,
                }
            } else {
                state.attempt = state.attempt.saturating_add(1);
                RespawnResolution::Postponed {
                    next_attempt: state.attempt,
                }
            });
        }

        let point = self.spawns[spawn_index].definition.point(cell_index)?;
        let target_identity = point.creature_target_identity.clone();
        let initial_health = point.creature_initial_health;
        let slots_before = self.slots.clone();
        let free_head_before = self.free_head;
        let occupied_before = self.occupied.clone();
        if let Some(dead_actor) = self.spawns[spawn_index].cells[cell_index].live {
            // Best effort: an already-removed actor (e.g. a repeated call) is not an error here.
            let _ = self.remove(continuity, dead_actor.0);
        }
        let actor = match self.admit_creature(
            continuity,
            ActorState(0),
            &target_identity,
            initial_health,
        ) {
            Ok(actor) => actor,
            Err(error) => {
                self.slots = slots_before;
                self.free_head = free_head_before;
                self.occupied = occupied_before;
                return Err(error);
            }
        };
        if let Err(error) = self.initialize_position(continuity, actor, position_context, cell) {
            self.slots = slots_before;
            self.free_head = free_head_before;
            self.occupied = occupied_before;
            return Err(error);
        }
        let state = &mut self.spawns[spawn_index].cells[cell_index];
        state.live = Some(ExactActorRef(actor));
        state.attempt = 0;
        Ok(RespawnResolution::Admitted(ExactActorRef(actor)))
    }
    /// SPAWN-CEILINGS-1 (Codex 4242270322): the respawn drive's admission at point
    /// `cell_index` of `source`, whose cell the caller has just found free under
    /// `position_context`. Unlike [`Self::resolve_respawn_timer`] it neither checks the cell
    /// again nor snapshots the slots: it costs the dead creature's removal (one pass over the
    /// occupied index) and one admission. Every refusal it can foresee (continuity, context,
    /// source, cell, the creature's health and identity, the identity's allocation) precedes the
    /// first mutation. The dead creature's removal is best effort, as in the timer resolution;
    /// after it only the creature ceiling `CREATUREAI0-RL-04` and the protected generation
    /// exhaustion of the freed slot (#541) can refuse, and that slot is retired either way. A
    /// refused position initialization removes the new creature again.
    pub(super) fn respawn_into_free_cell(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: SpawnSourceId,
        cell_index: usize,
        position_context: PreProductionPositionContext,
    ) -> Result<ExactActorRef, CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(position_context)?;
        let spawn_index = self
            .spawn_index(source)
            .ok_or(CarrierError::UnknownSpawnSource)?;
        let realization = &self.spawns[spawn_index];
        let cell = *realization
            .definition
            .placement_cells
            .get(cell_index)
            .ok_or(CarrierError::UnknownSpawnCell)?;
        let dead_actor = realization
            .cells
            .get(cell_index)
            .ok_or(CarrierError::UnknownSpawnCell)?
            .live;
        let point = realization.definition.point(cell_index)?;
        let initial_health = point.creature_initial_health;
        if initial_health <= 0 {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        let target_identity = creature_target_binding(&point.creature_target_identity)?;
        if let Some(dead_actor) = dead_actor {
            // Best effort: an already-removed actor (e.g. a repeated call) is not an error here.
            let _ = self.remove(continuity, dead_actor.0);
        }
        // `CREATUREAI0-RL-04` (Codex 4242715014), as `admit_creature` checks it, counted after
        // the dead creature has left.
        if self.occupied.len() >= CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX {
            return Err(CarrierError::CapacityExceeded);
        }
        let actor = self.admit_inner(
            continuity,
            ActorState(0),
            None,
            true,
            Some((initial_health, target_identity)),
            false,
        )?;
        if let Err(error) = self.initialize_position(continuity, actor, position_context, cell) {
            let _ = self.remove(continuity, actor);
            return Err(error);
        }
        let state = &mut self.spawns[spawn_index].cells[cell_index];
        state.live = Some(ExactActorRef(actor));
        state.attempt = 0;
        Ok(ExactActorRef(actor))
    }
}
