//! Resolve selected-target summon facts from the current Movement/Map owner and shared budget.
use crate::ai_think::{
    ThinkOccurrence, profile_schedule::MonsterSummonFacts, summon_plan::MonsterThinkReader,
};
use crate::content::{ProjectV2DefinitionRef, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeWorkStamp};
use crate::monster_summon::{NativeSummonCatalog, SummonLocationPolicy};
use crate::movement::monster_map::CurrentMonsterMap;
use crate::movement::path_budget::ChannelPathBudget;
use crate::movement::path_search::current_path::{PathTileReader, measured_current_route};
pub struct NativeMonsterThinkRead<'a, 'tx, 'db> {
    pub map: CurrentMonsterMap<'tx, 'db>,
    native: &'a NativeGameplayState,
    pub budget: &'a mut ChannelPathBudget,
    now: u64,
    player_waiting: bool,
    reached: Option<(ExactActorRef, ExactActorRef)>,
}
struct SummonPath<'a, 'tx, 'db> {
    map: &'a mut CurrentMonsterMap<'tx, 'db>,
    unknown: bool,
    runtime: &'a ChannelRuntimeV1,
    occupancy: crate::movement::path_search::PathOccupancy,
    start: MovementLocalPosition,
    target: MovementLocalPosition,
}
impl PathTileReader for SummonPath<'_, '_, '_> {
    async fn cost(&mut self, at: MovementLocalPosition) -> Option<u32> {
        if at == self.target {
            return None;
        }
        match self.map.read(self.runtime, at).await {
            Some(true) => {}
            Some(false) => return None,
            None => {
                self.unknown = true;
                return None;
            }
        }
        Some(self.occupancy.extra_cost(at, self.start))
    }
}
impl MonsterThinkReader for NativeMonsterThinkRead<'_, '_, '_> {
    async fn summon_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        occurrence: ThinkOccurrence,
        target: Option<ExactActorRef>,
        route: Option<&[MovementLocalPosition]>,
    ) -> Option<MonsterSummonFacts> {
        self.reached = None;
        let catalog =
            NativeSummonCatalog::for_actor(runtime, self.native, occurrence.actor).ok()?;
        let Some(target) = target else {
            return Some(catalog.facts(runtime, occurrence, None));
        };
        let from = runtime.read_actor_position(occurrence.actor).ok()?;
        let to = runtime.read_actor_position(target).ok()?;
        if from.context() != runtime.pinned_movement_context() || to.context() != from.context() {
            return None;
        }
        if self.map.read(runtime, from.position()).await != Some(true) {
            return None;
        }
        let reachable = if let Some(route) = route {
            let mut clear = true;
            for &at in route {
                clear &= self.map.read(runtime, at).await?;
            }
            clear
        } else {
            if !self.budget.admits(self.now, false, self.player_waiting) {
                return None;
            }
            let mut reader = SummonPath {
                map: &mut self.map,
                unknown: false,
                runtime,
                occupancy: crate::movement::path_search::PathOccupancy::current(runtime),
                start: from.position(),
                target: to.position(),
            };
            let (route, _) = measured_current_route(
                from.position(),
                to.position(),
                false,
                &mut reader,
                self.budget,
            )
            .await;
            if route.is_none() && reader.unknown {
                return None;
            }
            route.is_some()
        };
        if reachable {
            self.reached = Some((occurrence.actor, target));
        }
        // Read the complete bounded physical candidate set BEFORE the schedule commits.
        let at = from.position();
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let (Some(x), Some(y)) = (at.x.checked_add(dx), at.y.checked_add(dy)) {
                    self.map
                        .read(runtime, MovementLocalPosition { x, y, ..at })
                        .await;
                }
            }
        }
        if !self.map.validate_transaction(runtime).await {
            return None;
        }
        Some(catalog.facts(runtime, occurrence, reachable.then_some(target)))
    }
}
impl SummonLocationPolicy for NativeMonsterThinkRead<'_, '_, '_> {
    fn current_target_reachable(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: ExactActorRef,
        target: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.map
            .view()
            .matches(runtime)
            .then_some(self.reached == Some((parent, target)))
    }
    fn current_cell_admits(
        &mut self,
        runtime: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: &ProjectV2DefinitionRef,
        at: MovementLocalPosition,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        (self.map.view().matches(runtime) && self.map.view().known(at))
            .then(|| self.map.view().admits(at))
    }
}

impl<'a, 'tx, 'db> NativeMonsterThinkRead<'a, 'tx, 'db> {
    pub fn new(
        map: CurrentMonsterMap<'tx, 'db>,
        native: &'a NativeGameplayState,
        budget: &'a mut ChannelPathBudget,
        now: u64,
        player_waiting: bool,
    ) -> Self {
        Self {
            map,
            native,
            budget,
            now,
            player_waiting,
            reached: None,
        }
    }
}
