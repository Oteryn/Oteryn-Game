//! S16c: Dawnport vocation trial tiles on the committed step-in.
//!
//! Crystal 201189f8 `data-global/scripts/movements/others/dawnport_vocation_trial.lua`:
//! the trial action ids are the keys of `vocationTrials` (lines 26-172: 25005 sorcerer,
//! 25006 druid, 25007 paladin, 25008 knight, 25000 monk); `onStepIn` (lines 323-355) refuses
//! the current vocation and level 20 or more (line 334), then calls `changeVocation`
//! (`data-global/lib/others/dawnport.lua:26-120`), which re-levels magic level and skills and
//! refills the pools above level 8. Stepping on another trial tile switches back.
//!
//! Only the vocation conversion is admitted here; the trial's items, outfit and spell list
//! (lines 205-321) have no audited Game owner yet and are not granted.
use super::ComposedFreshAdmission;
use super::actor_spell::TrainingSave;
use super::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use super::vocation_choice::{RetainedNpcVocation, commit_retained_npc_vocation, same_authority};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildFormula, BuildOccurrence, convert_vocation,
};
use crate::foundation::{ExactActorRef, GameSessionId, MovementPositionSnapshot};
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};

/// Trial tile action id and the vocation key it switches to.
const TRIALS: [(u16, &str); 5] = [
    (25000, "monk"),
    (25005, "sorcerer"),
    (25006, "druid"),
    (25007, "paladin"),
    (25008, "knight"),
];

/// Crystal `dawnport_vocation_trial.lua:334`: level 20 and above keeps the vocation.
const TRIAL_LEVEL_LIMIT: u32 = 20;

fn trial_vocation(action: u16) -> Option<&'static str> {
    TRIALS
        .iter()
        .find(|(id, _)| *id == action)
        .map(|(_, vocation)| *vocation)
}

/// Releases the last-position hold however the trial ends.
struct TrialHold<'a>(
    &'a std::sync::Mutex<std::collections::HashSet<GameSessionId>>,
    GameSessionId,
);

impl Drop for TrialHold<'_> {
    fn drop(&mut self) {
        if let Ok(mut held) = self.0.lock() {
            held.remove(&self.1);
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The ordinary step already committed onto `entered`; a refusal leaves the actor there.
    /// `true` once the converted build is installed on the live actor.
    pub(super) async fn dawnport_trial_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> bool {
        let Some(vocation) = self.entered_trial(entered) else {
            return false;
        };
        let Some(formula) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
        else {
            return false;
        };
        if !matches!(
            self.save_spell_training(actor, session, true).await,
            TrainingSave::Saved | TrainingSave::NotApplicable
        ) {
            return false;
        }
        let Some(fence) = self.npc_vocation_fence(session).await else {
            return false;
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(before) = self
            .root
            .read_character_build_state(self.character, fence.character_id)
            .await
        else {
            return false;
        };
        let Ok(Some(progress)) = self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await
        else {
            return false;
        };
        let Some(current) = self.npc_vocation_fence(session).await else {
            return false;
        };
        if !same_authority(&fence, &current)
            || current.expected_character_revision != fence.expected_character_revision
            || progress.character_revision > current.expected_character_revision
            || progress.level >= TRIAL_LEVEL_LIMIT
            || before.vocation() == vocation
        {
            return false;
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered)
                || states.is_dead(actor)
                || states.has_pending_spell_commit(actor, session)
                || states.get(&runtime, actor, session).is_none_or(|state| {
                    state.has_training_to_flush()
                        || Some(state.character_facts().vocation)
                            != crate::spell::Vocation::from_key(before.vocation())
                })
            {
                return false;
            }
        }
        let Some(occurrence) = super::SecureIdentifiers::random::<16>().and_then(|mut bytes| {
            bytes[6] = (bytes[6] & 15) | 0x70;
            bytes[8] = (bytes[8] & 63) | 0x80;
            BuildOccurrence::from_bytes(bytes).ok()
        }) else {
            return false;
        };
        let Ok(after) = convert_vocation(formula, &before, vocation) else {
            return false;
        };
        let retained = RetainedNpcVocation {
            fence: current,
            request: BuildChangeRequest {
                occurrence,
                cause: BuildCause::VocationTrial,
                before,
                after,
                pruned_stance: None,
            },
            formula_digest: formula.digest(),
        };
        if !self
            .states
            .dawnport_trials
            .lock()
            .is_ok_and(|mut held| held.insert(session))
        {
            return false;
        }
        let _hold = TrialHold(&self.states.dawnport_trials, session);
        if !self
            .save_first_vocation_vitals(actor, session, current, &retained.request.before)
            .await
        {
            return false;
        }
        if commit_retained_npc_vocation(
            &mut slot,
            self.root,
            self.character,
            self.holder,
            &retained,
            formula,
        )
        .await
        .is_err()
        {
            return false;
        }
        let CastFactsLoad::Ready {
            facts,
            character_revision,
        } = load_character_cast_facts(self.root, self.character, current.character_id).await
        else {
            return false;
        };
        let (Ok(build), Ok(stance)) = (
            self.root
                .read_character_build_state(self.character, current.character_id)
                .await,
            self.root
                .read_character_stance(self.character, current.character_id)
                .await,
        ) else {
            return false;
        };
        let Some(latest) = self.npc_vocation_fence(session).await else {
            return false;
        };
        if !same_authority(&current, &latest)
            || latest.expected_character_revision != character_revision
            || build != retained.request.after
        {
            return false;
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session) {
            return false;
        }
        let Some(next) = states.get(&runtime, actor, session).and_then(|state| {
            state.after_vocation_trial(
                &retained.request.before,
                &build,
                facts,
                formula,
                self.owner_now(),
            )
        }) else {
            return false;
        };
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return false;
        };
        *state = next;
        // A refused install leaves the committed build for the next admission to load.
        states.load_owned_build(&runtime, actor, session, &build)
            && states.load_owned_stance(&runtime, actor, session, &stance, self.spells)
            && states
                .get_mut(&runtime, actor, session)
                .is_some_and(|state| state.enable_owned_training(&build, formula).is_ok())
    }

    /// The one trial action id among the entered tile's visible ground placements.
    fn entered_trial(&self, entered: MovementPositionSnapshot) -> Option<&'static str> {
        let map = self.bundle_map()?;
        let p = entered.position();
        let at = TilePos {
            x: p.x.try_into().ok()?,
            y: p.y.try_into().ok()?,
            floor: i8::try_from(p.floor).ok()?.checked_neg()?,
        };
        let base = map.overlay.base().tile(at.x, at.y, at.floor)?;
        let mut trials = (0..base.depths().iter().filter(|d| **d == 0).count()).filter_map(|o| {
            let key = placement_key(at, u8::try_from(o).ok()?)?;
            (!map.facts.base_hidden(key))
                .then(|| map.facts.action_at(key))
                .flatten()
                .and_then(trial_vocation)
        });
        let vocation = trials.next()?;
        trials.next().is_none().then_some(vocation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_trial_tile_names_its_vocation_and_no_other_action_does() {
        for (action, vocation) in [
            (25000, "monk"),
            (25005, "sorcerer"),
            (25006, "druid"),
            (25007, "paladin"),
            (25008, "knight"),
        ] {
            assert_eq!(trial_vocation(action), Some(vocation));
            assert!(crate::spell::Vocation::from_key(vocation).is_some());
        }
        for action in [24999, 25001, 25004, 25009] {
            assert_eq!(trial_vocation(action), None);
        }
    }
}
