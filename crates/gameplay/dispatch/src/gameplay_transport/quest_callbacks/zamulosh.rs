use super::*;
use crate::durability::quest_state::{QuestTransitionOutcome, zamulosh::QualifiedLovelyStep};
use oteryn_gameplay_quests::quest::zamulosh::{PAIRS, plate_action};

pub(in crate::gameplay_transport) fn capture(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    map: &BundleMap,
    step: crate::map::facts::MonsterStep,
) -> bool {
    if map.overlay.runtime_scope() != runtime.binding().runtime_scope() {
        return false;
    }
    let Some(at) = native(step.to) else {
        return false;
    };
    let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
        return false;
    };
    let mut matches = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (&id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(at, ordinal)?;
            if map.facts.base_hidden(key) {
                return None;
            }
            let native_plate = plate_action(at.x.into(), at.y.into(), at.floor.into())
                .filter(|_| map.facts.quest_callback_base_appearance(at, ordinal, id) == Some(417));
            PAIRS.iter().find_map(|(aid, _, _)| {
                (native_plate == Some(*aid)
                    || map
                        .facts
                        .quest_callback_matches(key, None, Some(*aid), None))
                .then(|| QualifiedLovelyStep::capture(runtime, step.actor, step.to, *aid))
                .flatten()
            })
        });
    let Some((kind, credit)) = matches.next() else {
        return false;
    };
    if matches.next().is_some() {
        return false;
    }
    let Ok(mut pending) = states.zamulosh_steps.lock() else {
        return false;
    };
    if pending.len() + credit.len() > 256 {
        return false;
    }
    let content = runtime.content_pin().server_artifact_digest();
    let Ok(stamp) = runtime.issue_owner_work() else {
        return false;
    };
    if runtime
        .remove_current_native_encounter_subject(stamp, step.actor, kind, content)
        .is_err()
    {
        return false;
    }
    pending.extend(credit);
    true
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn settle_zamulosh_steps(&self) {
        let Some(catalogue) = &self.quest_catalogue else {
            return;
        };
        let count = {
            let states = self.states.spell_states.lock().await;
            states
                .zamulosh_steps
                .lock()
                .map_or(0, |pending| pending.len())
        };
        for _ in 0..count {
            let input = {
                let states = self.states.spell_states.lock().await;
                states
                    .zamulosh_steps
                    .lock()
                    .ok()
                    .and_then(|pending| pending.front().cloned())
            };
            let Some(input) = input else { break };
            let session = input.session();
            let result = match self.current_quest_fence(session).await {
                Ok(Some(fence)) => {
                    let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
                    slot.commit_zamulosh_lovely_step(
                        self.root, self.character, self.holder, fence, input.clone(),
                        std::sync::Arc::clone(catalogue),
                    ).await.map(|outcome| {
                        matches!(outcome, QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_))
                    }).or_else(|error| {
                        if matches!(error, crate::durability::character_progression::CharacterProgressionError::AuthorityRejected) {
                            Ok(false)
                        } else {
                            Err(error)
                        }
                    }).map_err(|_| ())
                }
                Ok(None) => Ok(false),
                Err(()) => Err(()),
            };
            let Ok(committed) = result else { break };
            {
                let states = self.states.spell_states.lock().await;
                if let Ok(mut pending) = states.zamulosh_steps.lock()
                    && pending.front() == Some(&input)
                {
                    pending.pop_front();
                }
            }
            if committed {
                self.refresh_quest_session(session, false).await;
            }
        }
    }
}
