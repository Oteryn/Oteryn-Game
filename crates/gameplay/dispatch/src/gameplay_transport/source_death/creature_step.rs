//! The real movement cohort keeps its place among retained native callbacks.
use super::*;
use std::sync::Arc;

enum SelectedStep {
    Movement(u64),
    Profile(Arc<super::super::monster_profile_cycle::retained::PendingProfilePass>),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// A movement made inside an older retained pass is that same pass's
    /// unfinished statement. Its actual movement clock remains unchanged.
    pub(in crate::gameplay_transport) async fn drain_selected_creature_source_step(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        now: u64,
    ) -> Result<bool, ()> {
        let selected = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            let queue = &states.spite_steps.creatures;
            if !queue.is_pending() || states.source_death_action.is_some() {
                return Ok(false);
            }
            let native = self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or(())?;
            let Some((family, due)) = next_source_work(&runtime, &states, native)? else {
                return Err(());
            };
            let death_cutoff = runtime
                .next_source_creature_death()
                .map_err(|_| ())?
                .map(|notice| notice.enqueued_at_us().ok_or(()))
                .transpose()?;
            if due > now || death_cutoff.is_some_and(|cutoff| due > cutoff) {
                return Ok(false);
            }
            match family {
                SourceWorkFamily::CreatureStep => SelectedStep::Movement(due),
                SourceWorkFamily::Profile => {
                    let original = states.native_profile_pass.pending().ok_or(())?;
                    if !queue
                        .is_continuation_of(&runtime, &original)
                        .map_err(|_| ())?
                    {
                        return Ok(false);
                    }
                    SelectedStep::Profile(original)
                }
                _ => return Ok(false),
            }
        };
        match selected {
            SelectedStep::Movement(through) => {
                self.drain_source_creature_ground_steps(permit, through)
                    .await?;
            }
            SelectedStep::Profile(original) => {
                self.drain_source_creature_ground_steps_for_pass(permit, &original)
                    .await?;
            }
        }
        Ok(true)
    }
}
