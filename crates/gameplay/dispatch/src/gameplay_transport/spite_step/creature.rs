//! Detached consumption of the actual MonsterMap commit's original33949 cohort.
//! The world queue survives every SQL cancellation and owns the same reserved
//! Creature slot; no Player session or fresh movement census participates.
use super::*;
use crate::movement::monster_map::{SourceCreatureStepParent, SourceCreatureStepQueue};

enum Selection {
    Through(u64),
    Parent(SourceCreatureStepParent),
}
impl Selection {
    fn accepts(
        &self,
        queue: &SourceCreatureStepQueue,
        runtime: &ChannelRuntimeV1,
    ) -> Result<bool, ()> {
        match self {
            Self::Through(through) => queue
                .next_due_us(runtime)
                .map_err(|_| ())
                .map(|due| due.is_some_and(|due| due <= *through)),
            Self::Parent(parent) => queue.continues_same_pass(runtime, parent).map_err(|_| ()),
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn drain_source_creature_ground_steps(
        &self,
        permit: &SpellLanePermit,
        through_us: u64,
    ) -> Result<(), ()> {
        self.drain_selected_source_creature_ground_steps(permit, Selection::Through(through_us))
            .await
    }

    pub(in crate::gameplay_transport) async fn drain_source_creature_ground_steps_for_pass<
        P: std::any::Any + Send + Sync,
    >(
        &self,
        permit: &SpellLanePermit,
        parent: &Arc<P>,
    ) -> Result<(), ()> {
        let parent: SourceCreatureStepParent = parent.clone();
        self.drain_selected_source_creature_ground_steps(permit, Selection::Parent(parent))
            .await
    }

    async fn drain_selected_source_creature_ground_steps(
        &self,
        permit: &SpellLanePermit,
        selection: Selection,
    ) -> Result<(), ()> {
        let queue = Arc::clone(&self.states.spell_states.lock().await.spite_steps.creatures);
        let bound = queue.pending_items().map_err(|_| ())?;
        for _ in 0..bound {
            let selected = {
                let runtime = self.runtime.lock().await;
                selection.accepts(&queue, &runtime)?
            };
            if !selected {
                return Ok(());
            }
            let pending = queue.front().map_err(|_| ())?.ok_or(())?;
            let input = pending.input().ok_or(())?;
            let outcome = self
                .root
                .commit_source_ground_step(permit, self.character, self.holder, input)
                .await
                .map_err(|_| ())?;
            let mut runtime = self.runtime.lock().await;
            queue
                .publish_known_item_outcome(&mut runtime, &pending, outcome.removed)
                .map_err(|_| ())?;
            if outcome.removed {
                self.states.loop_ground.invalidate();
            }
        }
        let runtime = self.runtime.lock().await;
        if selection.accepts(&queue, &runtime)? {
            return Err(());
        }
        Ok(())
    }
}
