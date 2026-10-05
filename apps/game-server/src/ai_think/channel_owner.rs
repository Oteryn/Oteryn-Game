//! CREATURE-AI-1 §2.1: the Channel owner's side of the creature think. The think
//! (`CreatureAiState`) decides; these admit, wake, budget and commit its one step through
//! Movement on the Channel runtime, and return each report's Ability proposals to the caller,
//! which issues them through Ability. A refused step or proposal changes only the next think.
//!
//! The table is held beside `ChannelRuntimeV1` by its owner rather than inside `foundation`:
//! standalone test crates path-include `foundation/mod.rs` without `ai_think`/`content`.

use std::collections::BTreeMap;

use super::behaviour_profile::{CreatureBehaviourProfile, ProfileRefusal};
use super::targeting::TargetCandidate;
use super::{CreatureAiState, CreatureAiTable, CreatureThinkFacts, ThinkReport};
use crate::ability::RevisionSet;
use crate::content::{
    ProjectV2AbilityAuthoring, ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef,
};
use crate::foundation::{CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use oteryn_simulation_determinism::GameplayDecisionRoot;

impl CreatureAiTable {
    /// Admits live creature `actor` of `runtime` to thinking with its projected behaviour
    /// profile; its first think is due at `now_us`. A missing or invalid profile is refused and
    /// admits nothing.
    pub(crate) fn admit_creature(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        behavior: Option<&ProjectV2BehaviorAuthoring>,
        abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        let profile = CreatureBehaviourProfile::project(behavior, abilities).map_err(
            |refusal| match refusal {
                ProfileRefusal::Missing => CarrierError::ProfileMissing,
                ProfileRefusal::Invalid => CarrierError::ProfileInvalid,
            },
        )?;
        if !runtime.contains_live_creature(actor) {
            return Err(CarrierError::NotCreature);
        }
        let anchor = runtime.read_actor_position(actor)?.position();
        self.admit(CreatureAiState::new(actor, profile, anchor), now_us);
        Ok(())
    }

    /// §2.1 item 1: `player` moved or was admitted; wakes the idle creatures it sees.
    pub(crate) fn wake_for_player(
        &mut self,
        runtime: &ChannelRuntimeV1,
        player: ExactActorRef,
        now_us: u64,
    ) -> Result<Vec<ExactActorRef>, CarrierError> {
        let at = runtime.read_actor_position(player)?.position();
        Ok(self.wake(
            at,
            |creature| {
                if !runtime.contains_live_creature(creature) {
                    return None;
                }
                runtime
                    .read_actor_position(creature)
                    .ok()
                    .map(|snapshot| snapshot.position())
            },
            now_us,
        ))
    }

    /// Runs the thinks due at `now_us` within `CREATUREAI0-RL-05`. `players` are the Channel's
    /// player candidates with the owner's eligibility facts; `walkable` is the map's admission of
    /// a cell. A dead or despawned creature leaves the table. Each step is committed here.
    pub(crate) fn run_due_thinks(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        root: &GameplayDecisionRoot,
        revisions: &RevisionSet,
        players: &[TargetCandidate],
        walkable: impl Fn(MovementLocalPosition) -> bool,
    ) -> Vec<(ExactActorRef, ThinkReport)> {
        let mut reports = Vec::new();
        for actor in self.take_due(now_us) {
            let Some(health) = runtime.live_creature_health(actor) else {
                self.remove(actor);
                continue;
            };
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                self.remove(actor);
                continue;
            };
            let has_active_condition = runtime
                .actor_conditions(actor, None)
                .is_ok_and(|store| !store.instances().is_empty());
            let facts = CreatureThinkFacts {
                position: snapshot.position(),
                health,
                has_active_condition,
                players,
                now_us,
                root,
                revisions,
            };
            let report = self.think(actor, &facts, |destination| {
                walkable(destination)
                    && runtime.position_occupied_by_other(actor, destination) == Ok(false)
            });
            let Some(Ok(report)) = report else {
                continue;
            };
            if let Some(step) = report.step {
                // A refused step changes only the next think.
                let _ = runtime
                    .borrow_movement_position()
                    .commit_cardinal(snapshot, step.destination);
            }
            reports.push((actor, report));
        }
        reports
    }
}

#[cfg(test)]
#[path = "../foundation/channel_owner_creature_ai_tests.rs"]
mod channel_owner_creature_ai_tests;
