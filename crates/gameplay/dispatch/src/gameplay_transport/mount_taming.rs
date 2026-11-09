//! MOUNTTAME: existing USE -> carried item -> current creature -> fenced durable attempt.
mod quest_objects;

use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::content::native_gameplay::mount_taming::{PROVENANCE, rules};
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
use crate::durability::account_achievement::{AchievementGrantRequest, AchievementSourceEvent};
use crate::durability::item_use::mount_taming::MountTamingRequest;
use crate::durability::item_use::{ItemUseError, ItemUseFlaskPlan, ItemUseKind, ItemUseRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

// Once a DB commit can be ambiguous, cancellation must not leave a possibly removed creature
// available to combat or another tamer. Retire this owner on uncertainty; normal Channel
// recovery rereads the durable grants. Known rollback refusals disarm this guard.
struct CommitGuard<'a> {
    runtime: &'a mut ChannelRuntimeV1,
    uncertain: bool,
}
impl Drop for CommitGuard<'_> {
    fn drop(&mut self) {
        if self.uncertain {
            self.runtime.retire_owner_cycle();
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn tame_mount_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        item_target: Option<ItemKey>,
        visible: bool,
    ) -> Result<Option<UseOutcome>, ()> {
        if let Some(result) = self
            .tame_quest_object(actor, command, &intent, source)
            .await?
        {
            return Ok(Some(result));
        }
        match self
            .tame(actor, command, intent, source, item_target, visible)
            .await
        {
            Ok(value) => Ok(value.map(outcome)),
            Err(Some(disposition)) => Ok(Some(outcome(disposition))),
            Err(None) => Err(()),
        }
    }

    /// Registered carried World tools reuse the fenced World receipt owner.
    async fn tame_quest_object(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(target)) =
            &intent.use_with_map_item
        else {
            return Ok(None);
        };
        let source = match (&intent.target, source) {
            (UseTarget::Item(_), Some(ItemKey::Instance(item))) => Some(item),
            (UseTarget::ItemByDefinition(_), None) => None,
            _ => return Ok(None),
        };
        let Some(map) = self.bundle_map() else {
            return Ok(None);
        };
        let view = super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: self
                .active_generation
                .ok_or(())?
                .identity()
                .client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let Ok(resolved) = super::world_map::resolve_map_target(&view, target, |_| None::<()>)
        else {
            return Ok(None);
        };
        let callbacks = self.callback_content();
        let Some(callback) = callbacks.iter().find(|callback| {
            quest_objects::accepts(callback)
                && super::quest_callbacks::matches(map, callback, resolved.placement_key)
        }) else {
            return Ok(None);
        };
        let Some(required) = &callback.item_on_target else {
            return Err(());
        };
        let Some(fence) = command.item_fence else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let selected = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .any(|entry| {
                entry.item.definition.production_key == required.item
                    && quest_objects::matches_source(
                        &intent.target,
                        source,
                        entry.item.item_instance_id,
                        self.active_generation
                            .and_then(crate::content::ActiveGeneration::native_gameplay)
                            .and_then(|native| native.item_index())
                            .and_then(|index| {
                                index.definition_ref(
                                    &entry.item.definition.family,
                                    &entry.item.definition.production_key,
                                    &entry.item.definition.revision_ref,
                                )
                            }),
                    )
            });
        if !selected {
            return Ok(Some(outcome(UseDisposition::NothingToUse)));
        }
        Ok(self.use_quest_callback(actor, command, map, target).await)
    }

    async fn tame(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        item_target: Option<ItemKey>,
        visible: bool,
    ) -> Result<Option<UseDisposition>, Option<UseDisposition>> {
        let reject = Some(UseDisposition::Rejected);
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let index = {
            let runtime = self.runtime.lock().await;
            super::item_ref_admission::pinned_index(&runtime, self.active_generation)
        };
        let Some(index) = index else {
            return Ok(None);
        };
        // A definition-targeted USE names its source item directly. Reject an item which
        // has no applicable taming rule before taking the spell permit or reading any durable fence.
        if let UseTarget::ItemByDefinition(wanted) = intent.target {
            let (key, _) = index.definition_at(wanted).ok_or(reject)?;
            let self_use = intent.use_with.is_none()
                && intent.use_with_map_item.is_none()
                && intent.use_with_position.is_none();
            if !rules().iter().any(|rule| {
                rule.item_key == key && (!self_use || rule.target_item_key.as_deref() == Some(key))
            }) {
                return Ok(None);
            }
        }
        let fence = command.item_fence.ok_or(reject)?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| reject)?,
        );
        let binding: [u8; 32] =
            Sha256::digest(encode_item_use_intent(&intent).map_err(|_| reject)?).into();
        let _permit = self.spell_lane_permit().await.ok_or(reject)?;
        match self
            .root
            .read_mount_taming_receipt(self.character, fence, command_ref, binding)
            .await
        {
            Ok(Some(_)) => return Ok(Some(UseDisposition::Committed)),
            Ok(None) => {}
            Err(ItemUseError::Unavailable(_)) => return Err(None),
            Err(_) => return Err(reject),
        }
        let inventory = match self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        {
            Ok(inventory) => inventory,
            // An unresolved carried source may belong to another owner. Its
            // historical receipt must remain reachable after a scope change.
            // Fresh taming writes still require the current inventory fence.
            Err(ItemUseError::AuthorityRejected)
                if matches!(intent.target, UseTarget::Item(_))
                    && intent.use_with_map_item.is_some() =>
            {
                return Ok(None);
            }
            Err(_) => return Err(reject),
        };
        let bag = inventory
            .backpack
            .as_ref()
            .ok_or(Some(UseDisposition::StaleState))?;
        let item_id = match intent.target {
            UseTarget::ItemByDefinition(wanted) => {
                let (key, revision) = index.definition_at(wanted).ok_or(reject)?;
                if !rules().iter().any(|r| r.item_key == key) {
                    return Ok(None);
                }
                inventory
                    .first_unreserved(
                        &crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: key.into(),
                            revision_ref: revision.into(),
                        },
                        &[],
                    )
                    .map_err(|_| Some(UseDisposition::NothingToUse))?
            }
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => id,
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let Some(entry) = bag
            .entries
            .iter()
            .chain(inventory.nested.iter().map(|(_, e)| e))
            .find(|e| e.item.item_instance_id == item_id)
        else {
            return Ok(None);
        };
        if index
            .definition_ref(
                &entry.item.definition.family,
                &entry.item.definition.production_key,
                &entry.item.definition.revision_ref,
            )
            .is_none()
        {
            return Err(Some(UseDisposition::StaleState));
        }
        if !rules()
            .iter()
            .any(|r| r.item_key == entry.item.definition.production_key)
        {
            return Ok(None);
        }
        let item_target = item_target
            .and_then(|key| match key {
                ItemKey::Instance(id) => bag
                    .entries
                    .iter()
                    .chain(inventory.nested.iter().map(|(_, entry)| entry))
                    .find(|entry| entry.item.item_instance_id == id)
                    .map(|entry| (id, entry.item.definition.production_key.as_str())),
                _ => None,
            })
            .or_else(|| {
                // Crystal mounts.lua constructs these mounts by using the carried
                // object itself. An explicit target must never fall back to self-USE.
                (intent.use_with.is_none()
                    && intent.use_with_map_item.is_none()
                    && intent.use_with_position.is_none()
                    && rules().iter().any(|rule| {
                        rule.item_key == entry.item.definition.production_key
                            && rule.target_item_key.as_deref() == Some(rule.item_key.as_str())
                    }))
                .then_some((item_id, entry.item.definition.production_key.as_str()))
            });
        if intent.use_with.is_some() && !visible {
            return Err(Some(UseDisposition::NoTarget));
        }
        let mut runtime = self.runtime.lock().await;
        let creature_target = intent.use_with.and_then(|wanted| {
            runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| {
                    c.actor.placement_identity() == wanted.identity
                        && c.generation == wanted.generation
                })
                .map(|c| (wanted, c.actor))
        });
        let rule = rules()
            .iter()
            .find(|r| {
                r.item_key == entry.item.definition.production_key
                    && match (creature_target, item_target) {
                        (Some((_, target)), None) => runtime
                            .matches_live_creature_identity(target, r.creature_key.as_bytes()),
                        (None, Some((_, key))) => r.target_item_key.as_deref() == Some(key),
                        _ => false,
                    }
            })
            .ok_or(Some(UseDisposition::NothingToUse))?;
        let cosmetics = native.cosmetics().ok_or(reject)?;
        if !cosmetics.mount_exists(&rule.mount_key)
            || cosmetics.earned_unlock(
                &rule.grant_key,
                PROVENANCE,
                EarnedCosmetic::Mount(&rule.mount_key),
            ) != UnlockFact::Compatible
        {
            return Err(Some(UseDisposition::NothingToUse));
        }
        if rule.required_equipment.as_ref().is_some_and(|key| {
            !inventory
                .equipment
                .iter()
                .any(|item| item.definition.production_key == *key)
        }) {
            return Err(Some(UseDisposition::RequirementNotMet));
        }
        if !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|f| f.control_loss.is_none())
        {
            return Err(reject);
        }
        let states = self.states.spell_states.lock().await;
        if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id) {
            return Err(reject);
        }
        drop(states);
        if let Some((_, target)) = creature_target {
            runtime
                .check_mount_taming_subject(target, &rule.creature_key)
                .map_err(|_| Some(UseDisposition::NothingToUse))?;
            let from = runtime
                .read_actor_position(actor)
                .map_err(|_| reject)?
                .position();
            let to = runtime
                .read_actor_position(target)
                .map_err(|_| reject)?
                .position();
            if from.floor != to.floor || from.x.abs_diff(to.x) > 1 || from.y.abs_diff(to.y) > 1 {
                return Err(Some(UseDisposition::TooFar));
            }
        }
        let transaction_id = crate::operator_files::uuid_v7().map_err(|_| reject)?;
        let digest = Sha256::new()
            .chain_update(b"oteryn.mount.taming.v1")
            .chain_update(transaction_id)
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&digest[..16]);
        let draw = draw(
            deterministic_decision_u64(
                &GameplayDecisionRoot::from_bytes(native.source_digest()),
                DecisionOccurrenceId::from_bytes(occurrence),
                "mount.taming",
                0,
            )
            .map_err(|_| reject)?,
        );
        let event = AchievementSourceEvent {
            kind: "mount_taming".into(),
            event_id: transaction_id.to_vec(),
        };
        let achievements = rule
            .achievement_key
            .iter()
            .map(String::as_str)
            .chain(std::iter::once("oteryn:achievement/natural_born_cowboy"))
            .map(|key| AchievementGrantRequest {
                achievement_key: key.into(),
                catalogue: self.achievements.lookup(key),
                source: event.clone(),
            })
            .collect();
        let request = MountTamingRequest {
            item: ItemUseRequest {
                kind: ItemUseKind::MountTaming,
                command_id: command.command_id,
                content_revision: inventory.content_revision,
                ruleset_revision: inventory.ruleset_revision,
                sim_revision: PROVENANCE.into(),
                transaction_id,
                event_id: crate::operator_files::uuid_v7().map_err(|_| reject)?,
                occurred_at_unix_ms: i64::try_from(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|_| reject)?
                        .as_millis(),
                )
                .map_err(|_| reject)?,
                server_build_id: crate::durability::character_authority::SERVER_BUILD_ID.into(),
                source_item_instance_id: item_id,
                source_definition: entry.item.definition.clone(),
                source_quantity_before: entry.item.quantity,
                flask: ItemUseFlaskPlan::None,
            },
            binding,
            target_identity: creature_target
                .map_or_else(|| item_target.map_or([0; 16], |v| v.0), |v| v.0.identity),
            target_generation: creature_target.map_or(1, |v| v.0.generation),
            creature_key: rule
                .target_item_key
                .clone()
                .unwrap_or_else(|| rule.creature_key.clone()),
            cosmetics: cosmetics.clone(),
            draw,
            achievements,
        };
        let mut guard = CommitGuard {
            runtime: &mut runtime,
            uncertain: true,
        };
        match self
            .root
            .commit_mount_taming(self.character, self.holder, fence, request)
            .await
        {
            Ok(Some(receipt)) => {
                let (_, remove, text) = rule.effects(receipt.outcome).ok_or(reject)?;
                if remove && let Some((_, target)) = creature_target {
                    guard
                        .runtime
                        .remove_mount_taming_subject(target, &rule.creature_key)
                        .map_err(|_| None)?;
                }
                guard.uncertain = false;
                self.states
                    .chat
                    .push_party_spell_feedback(command.game_session_id, text);
                Ok(Some(UseDisposition::Committed))
            }
            Ok(None) => {
                guard.uncertain = false;
                Err(Some(UseDisposition::RequirementNotMet))
            }
            Err(ItemUseError::Unavailable(_)) => Err(None),
            Err(error) => {
                guard.uncertain = false;
                Err(Some(if matches!(error, ItemUseError::StaleState) {
                    UseDisposition::StaleState
                } else {
                    UseDisposition::Rejected
                }))
            }
        }
    }
}

fn draw(draw: u64) -> u64 {
    #[cfg(test)]
    if let Some(injected) = test_draw::take() {
        return injected;
    }
    draw
}
#[cfg(test)]
pub(super) mod test_draw {
    use std::cell::Cell;
    thread_local! { static NEXT:Cell<Option<u64>>=const {Cell::new(None)}; }
    pub(in crate::gameplay_transport) fn set(draw: u64) {
        NEXT.set(Some(draw));
    }
    pub(super) fn take() -> Option<u64> {
        NEXT.take()
    }
}
