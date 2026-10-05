//! A source-only condition transaction, composing the two existing native owners.
use super::*;
use crate::ability::player_lethal::{SourceAppearanceReceipt, SourceAppearanceTarget};
impl ChannelSpellStates {
    pub(in crate::gameplay_transport::actor_spell) fn native_source_appearance_batch(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        source: ExactActorRef,
        definitions: &[ConditionDefinition],
        targets: &[SourceAppearanceTarget<'_>],
    ) -> Option<Vec<SourceAppearanceReceipt>> {
        if !current_owner(runtime, fence, stamp)
            || !creature_current(runtime, source)
            || definitions.len() != 1
            || targets.is_empty()
            || targets.len() > 64
            || !matches!(
                definitions[0].values(),
                crate::foundation::ConditionValues::Outfit { .. }
                    | crate::foundation::ConditionValues::ItemOutfit { .. }
            )
        {
            return None;
        }
        match definitions[0].values() {
            crate::foundation::ConditionValues::Outfit { .. } => {
                definitions[0].source_appearance_selection()?;
            }
            crate::foundation::ConditionValues::ItemOutfit { .. } => {
                if definitions[0].source_item_appearance()?.artifact_digest()
                    != runtime.content_pin().server_artifact_digest()
                {
                    return None;
                }
            }
            _ => return None,
        }
        let now = targets.first()?.facts.now;
        let mut players = Vec::new();
        let mut plans = Vec::new();
        let mut unique = Vec::new();
        unique.try_reserve(targets.len()).ok()?;
        let mut receipts = Vec::new();
        receipts.try_reserve(targets.len()).ok()?;
        players.try_reserve(targets.len()).ok()?;
        plans.try_reserve(targets.len()).ok()?;
        let label = format!(
            "creature:{}",
            super::super::hex(&source.placement_identity())
        );
        for target in targets {
            if unique.contains(&target.actor)
                || target.facts.now != now
                || target.facts.target_is_player != target.session.is_some()
                || !self.condition_actor_current(runtime, target.actor, target.session)
            {
                return None;
            }
            unique.push(target.actor);
            let position = runtime.read_actor_position(target.actor).ok()?;
            if position.context() != runtime.pinned_movement_context() {
                return None;
            }
            if let Some(session) = target.session {
                let index = self.index(target.actor, session)?;
                let before = self.get(runtime, target.actor, session)?.clone();
                let next = before.stage_source_player_conditions(
                    label.clone(),
                    definitions,
                    &target.immunities,
                    &target.facts,
                )?;
                // Compare exactly the owned appearance conflict, not revision/clock or
                // other expired conditions. KeptCurrent is an explicit no-effect receipt.
                let applied = before
                    .owned_conditions()
                    .get(crate::ability::condition::ConflictKey::Outfit)
                    != next
                        .owned_conditions()
                        .get(crate::ability::condition::ConflictKey::Outfit);
                receipts.push(SourceAppearanceReceipt {
                    actor: target.actor,
                    applied,
                });
                players.push((index, target.actor, session, before, next));
            } else {
                let plan = runtime
                    .prepare_actor_condition(
                        target.actor,
                        None,
                        crate::foundation::ActorConditionTransition::ApplyBatch {
                            definitions,
                            immunities: &target.immunities,
                            facts: target.facts,
                            source: crate::foundation::ConditionSource {
                                actor: source,
                                session: None,
                                kind: crate::foundation::ConditionSourceKind::Creature,
                            },
                        },
                        SemanticTimeMicros::from_micros(now),
                    )
                    .ok()?;
                receipts.push(SourceAppearanceReceipt {
                    actor: target.actor,
                    applied: plan.applications().iter().any(Result::is_ok),
                });
                plans.push(plan);
            }
        }
        runtime
            .commit_actor_condition_batch_with_vitals(
                &plans,
                SemanticTimeMicros::from_micros(now),
                |current| {
                    if !current_owner(current, fence, stamp) || !creature_current(current, source) {
                        return None;
                    }
                    for (index, actor, session, before, _) in &players {
                        if self.actors.get(*index).is_none_or(|(a, s, state)| {
                            a != actor || s != session || state != before
                        }) || !self.condition_player_current(current, *actor, *session)
                        {
                            return None;
                        }
                    }
                    // Receipt allocation and per-target effects were staged before publication.
                    // No fallible check/allocation remains after the first publication.
                    for (index, _, _, _, next) in players {
                        self.actors[index].2 = next;
                    }
                    Some(receipts)
                },
            )
            .ok()?
    }
}
