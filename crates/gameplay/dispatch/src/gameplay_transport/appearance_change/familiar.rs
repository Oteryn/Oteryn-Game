//! Familiar appearance commands reuse the fenced familiar checkpoint owner.
use super::*;
use crate::durability::character_familiar::{
    FamiliarLifecycleWriter, FamiliarStateOccurrence, FamiliarStateOutcome, FamiliarStateRequest,
};
use oteryn_protocol_oteryn::appearance::familiar;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn familiar_appearance_command(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        kind: u32,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        let change = match kind {
            familiar::QUERY if payload.is_empty() => None,
            familiar::CHANGE => Some(familiar::Change::decode(payload)?),
            _ => return None,
        };
        let active = self.active_generation?.native_gameplay()?;
        let (_, content) = self.appearance_context()?;
        let _permit = self.spell_lane_permit().await?;
        let fence =
            super::super::character_gameplay_fence::current(self, command.game_session_id, false)
                .await?;
        if command.item_fence != Some(fence.item_fence()) {
            return None;
        }
        let projection = self
            .root
            .read_character_familiar_state(self.character, fence.character_id)
            .await
            .ok()?;
        let state = projection.state();
        let progression = self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await
            .ok()??;
        if progression.character_revision != fence.expected_character_revision {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !current_runtime(&runtime, actor, command.game_session_id, &fence, &content)
                || states.is_dead(actor)
                || states.has_pending_familiar(actor, command.game_session_id)
            {
                return None;
            }
        }
        let mut choices: Vec<_> = active
            .policies()
            .policies()
            .iter()
            .filter_map(|policy| {
                let look_type = policy.outfit_appearance()?;
                (policy.is_familiar && state.granted_looks.binary_search(&look_type).is_ok()).then(
                    || familiar::Choice {
                        key: policy.definition_key.clone(),
                        look_type,
                    },
                )
            })
            .collect();
        choices.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        // Durability stores the look ID. Expose one canonical key per look so
        // a confirmed key reads back identically after reconnect.
        let mut looks = std::collections::BTreeSet::new();
        choices.retain(|choice| looks.insert(choice.look_type));
        let revision = fence.expected_character_revision.get();
        let selected_index = choices
            .iter()
            .position(|choice| choice.look_type == state.selected_look);
        let (selected_index, root_revision) = match change {
            None => (selected_index, revision),
            Some(change) => {
                let index = choices.iter().position(|choice| choice.key == change.key)?;
                if selected_index == Some(index) {
                    (selected_index, revision)
                } else {
                    if change.root_revision != revision {
                        return None;
                    }
                    let committed = self
                        .select_familiar_appearance(
                            &fence,
                            &projection,
                            progression,
                            active.source_digest(),
                            choices[index].look_type,
                        )
                        .await?;
                    (Some(index), committed)
                }
            }
        };
        let selected = selected_index.map(|i| familiar::Choice {
            key: choices[i].key.clone(),
            look_type: choices[i].look_type,
        });
        familiar::View {
            root_revision,
            choices,
            selected,
        }
        .encode()
    }

    async fn select_familiar_appearance(
        &self,
        fence: &CurrentCharacterGameplayFence,
        projection: &crate::durability::character_familiar::DurableFamiliarProjection,
        progression: crate::durability::character_progression::CharacterProgressionState,
        policy_digest: [u8; 32],
        look_type: u32,
    ) -> Option<u64> {
        let writer = FamiliarLifecycleWriter::from_fence(fence).ok()?;
        let before = projection.state();
        let mut after = before
            .for_next_write(projection.checkpoint_committed_at_ms(), &writer)
            .ok()?;
        after.selected_look = look_type;
        after.lifecycle.as_mut()?.writer = writer;
        let occurrence =
            FamiliarStateOccurrence::from_bytes(crate::operator_files::uuid_v7().ok()?).ok()?;
        let request = FamiliarStateRequest {
            occurrence,
            before: before.clone(),
            after,
            content_revision: progression.context.content,
            policy_revision: progression.policy_revision,
            policy_digest,
        };
        match self
            .root
            .commit_character_familiar_state(self.character, self.holder, *fence, request)
            .await
            .ok()?
        {
            FamiliarStateOutcome::Committed(receipt)
            | FamiliarStateOutcome::AlreadyCommitted(receipt) => {
                Some(receipt.committed_character_revision().get())
            }
        }
    }
}
