//! MAP-ITEM-REF-1 (ARCH-MAP-TRACK-PACKETS-V1 §1.6): the production fresh admission's capability 4
//! offer and its domain 9 read. Capability 4 is offered only once [`ITEM_VIEW_OFFERED`] is set and
//! only when the active generation pins a non-empty Item key set. Domain 9 names each durable backpack definition by that generation's
//! Item definition index, read only while the Channel content pin is the same generation; any
//! definition the index does not name fails closed.
//!
//! The index is read from the active generation rather than carried on `ChannelContentPin`:
//! `foundation` is also compiled into the durability integration tests without `content`.

use super::ComposedFreshAdmission;
use super::capabilities::{
    ITEM_VIEW_OFFERED_CAPABILITIES, OfferedCapability, PRODUCTION_OFFERED_CAPABILITIES,
};
use super::connection::UseCommand;
use super::item_view::{
    InventoryItems, ItemKey, ItemTargetObservation, UseItemTarget, ViewItem,
    inventory_from_backpack,
};
use super::world_spatial::{ActorPosition, ChannelEntity, VisibleKind};
use super::{FreshAdmissionStore, GameSessionId, GameSessionState};
use crate::combat_pickup::GroundPickupError;
use crate::content::ActiveGeneration;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemStackClass, ItemTransferDestination, ItemTransferError, ItemTransferOutcome,
    ItemTransferRequest, ItemWeightLookup, ItemWeightRefused,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use std::num::NonZeroU32;

pub(super) struct UseProbeRefusal(oteryn_protocol_oteryn::world_object::UseDisposition);

impl From<UseProbeRefusal> for super::connection::UseOutcome {
    fn from(refusal: UseProbeRefusal) -> Self {
        Self {
            disposition: refusal.0,
            ..Self::rejected()
        }
    }
}

/// One advisory definition lookup serves all fresh candidates in the item-action chain.
pub(super) struct UseSourceHint {
    pub(super) key: Option<ItemKey>,
    definition: tokio::sync::OnceCell<Option<TypedDefinitionRef>>,
}

impl UseSourceHint {
    pub(super) fn new(key: Option<ItemKey>) -> Self {
        Self {
            key,
            definition: tokio::sync::OnceCell::new(),
        }
    }

    /// A successful false delegates to the next owner. A failed read is a typed refusal.
    pub(super) async fn candidate(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        command: UseCommand,
        stage: &'static str,
        qualifies: impl FnOnce(&TypedDefinitionRef) -> bool,
    ) -> Result<bool, UseProbeRefusal> {
        Ok(self
            .definition(owner, command, stage)
            .await?
            .is_some_and(qualifies))
    }

    pub(super) async fn definition(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        command: UseCommand,
        stage: &'static str,
    ) -> Result<Option<&TypedDefinitionRef>, UseProbeRefusal> {
        let (Some(ItemKey::Instance(item)), Some(fence)) = (self.key, command.item_fence) else {
            return Ok(None);
        };
        let definition = self
            .definition
            .get_or_try_init(|| async {
                let budget = super::AdmissionRetryBudget::for_command(owner.root);
                budget
                    .retry_command_guard(async || {
                        owner
                            .root
                            .plain_use_item_definition(
                                item,
                                fence.runtime_scope.world_id(),
                                budget.deadline,
                            )
                            .await
                    })
                    .await
            })
            .await
            .map_err(|_| UseProbeRefusal(super::use_receipt_refused(command, stage).disposition))?;
        Ok(definition.as_ref())
    }
}

pub(super) async fn retry_use_read<T>(
    budget: &super::AdmissionRetryBudget<'_>,
    mut read: impl AsyncFnMut() -> Result<T, crate::durability::item_use::ItemUseError>,
) -> Result<T, crate::durability::item_use::ItemUseError> {
    use crate::durability::{DurabilityError, item_use::ItemUseError};

    loop {
        let result = read().await;
        if !matches!(
            result,
            Err(ItemUseError::Unavailable(
                DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded
            ))
        ) || !budget
            .retries_unavailable(super::AdmissionRefusal::Unavailable)
            .await
        {
            return result;
        }
    }
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static USE_INVENTORY_CHECKOUT_PROBE:
        std::cell::Cell<Option<std::sync::Arc<super::composed_admission::HolderCheckoutProbe>>>;
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn probe_plain_use_receipt(
        &self,
        family: crate::durability::PlainUseReceipt,
        command: crate::foundation::CommandRef,
    ) -> Result<bool, crate::durability::DurabilityError> {
        let budget = super::AdmissionRetryBudget::for_command(self.root);
        budget
            .retry_command_guard(async || self.root.plain_use_receipt_exists(family, command).await)
            .await
    }

    pub(super) async fn read_use_backpack(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Result<
        Option<crate::durability::item_transfer::CharacterBackpack>,
        crate::durability::DurabilityError,
    > {
        let budget = super::AdmissionRetryBudget::for_command(self.root);
        budget
            .retry_command_guard(async || {
                self.root
                    .read_character_backpack(self.character, fence.character_id)
                    .await
                    .map_err(|error| match error {
                        ItemTransferError::Unavailable(error) => error,
                        _ => crate::durability::DurabilityError::Unavailable,
                    })
            })
            .await
    }

    pub(super) async fn read_use_inventory(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Result<
        crate::durability::item_use::ItemUseInventory,
        crate::durability::item_use::ItemUseError,
    > {
        #[cfg(test)]
        let probe = USE_INVENTORY_CHECKOUT_PROBE
            .try_with(std::cell::Cell::take)
            .ok()
            .flatten();
        #[cfg(test)]
        if let Some(probe) = &probe {
            probe.pause().await;
        }
        let budget = super::AdmissionRetryBudget::for_command(self.root);
        retry_use_read(&budget, async || {
            let result = self
                .root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await;
            #[cfg(test)]
            if matches!(
                result,
                Err(crate::durability::item_use::ItemUseError::Unavailable(
                    crate::durability::DurabilityError::RootUnavailable
                ))
            ) && let Some(probe) = &probe
            {
                probe.blocked();
            }
            result
        })
        .await
    }

    pub(super) async fn read_house_use_inventory(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Result<crate::durability::item_use::ItemUseInventory, crate::durability::DurabilityError>
    {
        let budget = super::AdmissionRetryBudget::for_command(self.root);
        budget
            .retry_command_guard(async || {
                self.root
                    .read_hireling_use_inventory(
                        self.character,
                        self.holder,
                        fence,
                        *self.channel_id.as_bytes(),
                    )
                    .await
            })
            .await
    }
}

/// The non-empty Item definition index of the active generation, if it pins one.
fn item_index(generation: Option<&ActiveGeneration>) -> Option<&ItemDefinitionIndex> {
    generation?
        .native_gameplay()?
        .item_index()
        .map(|index| &**index)
        .filter(|index| !index.is_empty())
}

/// MAP-ITEM-REF-1 Part B: set atomically with the composed corpse observation
/// ([`visible_corpses`]), [`observe_item_target`] and [`take_corpse_entry`] (CP ruling on Codex
/// 4201178872).
const ITEM_VIEW_OFFERED: bool = true;

pub(super) fn offered_capabilities(
    active_generation: Option<&ActiveGeneration>,
) -> &'static [OfferedCapability] {
    offered_capabilities_when(ITEM_VIEW_OFFERED, active_generation)
}

fn offered_capabilities_when(
    item_view_offered: bool,
    active_generation: Option<&ActiveGeneration>,
) -> &'static [OfferedCapability] {
    if item_view_offered && item_index(active_generation).is_some() {
        ITEM_VIEW_OFFERED_CAPABILITIES
    } else {
        PRODUCTION_OFFERED_CAPABILITIES
    }
}

/// The Item definition index of `admission`'s generation while the Channel content pin is that
/// same generation; `None` otherwise, and then no item is named at all.
pub(super) fn pinned_index<'g>(
    runtime: &ChannelRuntimeV1,
    generation: Option<&'g ActiveGeneration>,
) -> Option<&'g ItemDefinitionIndex> {
    let generation = generation?;
    let index = item_index(Some(generation))?;
    (runtime.content_pin().server_artifact_digest()
        == generation.identity().server_artifact_digest())
    .then_some(index)
}

pub(super) use oteryn_gameplay_items::item_ref_admission::{
    definition_ref, generation_inbox_item_facts, generation_item_facts,
};

fn native_corpse_appearance(
    runtime: &ChannelRuntimeV1,
    generation: Option<&ActiveGeneration>,
    index: &ItemDefinitionIndex,
    reference: std::num::NonZeroU32,
) -> Option<std::num::NonZeroU16> {
    let generation = generation?;
    let native = generation.native_gameplay()?;
    let pin = runtime.content_pin();
    if generation.identity().server_artifact_digest() != pin.server_artifact_digest()
        || generation.identity().client_artifact_digest() != pin.client_artifact_digest()
    {
        return None;
    }
    let (key, revision) = index.definition_at(reference)?;
    native_item_appearance(native, key, revision)
}

pub(super) fn native_item_appearance(
    native: &crate::content::native_gameplay::NativeGameplayState,
    key: &str,
    revision: &str,
) -> Option<std::num::NonZeroU16> {
    let policy = native.item_policy(key, revision)?;
    if policy.source_digest() != native.source_digest() {
        return None;
    }
    // Native policy qualification proves the source binding, including aliases.
    // A generation-local ordinal is never interpreted as an appearance number.
    let id = policy
        .record()
        .production_binding
        .external_id
        .parse::<u16>()
        .ok()?;
    std::num::NonZeroU16::new(id)
}

/// MAP-ITEM-REF-1 Part B: the bound corpses of the Channel as D85 objects (entity generation 0),
/// read under the caller's runtime lock. Positions are in the runtime movement frame.
/// A corpse whose definition the pinned index does not name
/// is left out; with capability 4 not offered, or no pinned index, there are none.
pub(super) fn visible_corpses(
    runtime: &ChannelRuntimeV1,
    generation: Option<&ActiveGeneration>,
) -> Vec<ChannelEntity> {
    if !ITEM_VIEW_OFFERED {
        return Vec::new();
    }
    let Some(index) = pinned_index(runtime, generation) else {
        return Vec::new();
    };
    runtime
        .visible_corpses()
        .into_iter()
        .filter_map(|corpse| {
            let item_definition_ref = index.definition_ref(
                &corpse.item.family,
                &corpse.item.production_key,
                &corpse.item.revision_ref,
            )?;
            Some(ChannelEntity {
                source_icons:
                    oteryn_protocol_oteryn::world_spatial_entities::NativeCreatureIcons::default(),
                motion: None,
                display_name: None,
                health_percent: 100,
                direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                kind: VisibleKind::Corpse {
                    native_appearance_id: native_corpse_appearance(
                        runtime,
                        generation,
                        index,
                        item_definition_ref,
                    ),
                    item_definition_ref,
                },
                identity: corpse.identity,
                generation: 0,
                position: ActorPosition {
                    x: corpse.position.x,
                    y: corpse.position.y,
                    floor: corpse.position.floor,
                },
                revision: corpse.revision,
            })
        })
        .collect()
}

/// MAP-ITEM-REF-1 Part B: `USE` on a domain-1 object (ITEM-MOVE-WIRE-0 §4.3). Only a bound corpse
/// is an item target; an unbound or vanished one is `Gone`, so no corpse Item id is ever made up.
/// Its contents are the durable corpse entries, each named by the pinned index; any it does not
/// name, or a failed read, is `None` (`STALE`).
pub(super) async fn observe_item_target(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    target: ItemKey,
) -> Option<ItemTargetObservation> {
    let ItemKey::Entity(identity) = target else {
        return None;
    };
    let (position, corpse, index) = {
        let mut runtime = admission.runtime.lock().await;
        let index = pinned_index(&runtime, admission.active_generation)?;
        let position = runtime
            .borrow_movement_position()
            .read(actor)
            .ok()?
            .position();
        (position, runtime.bound_corpse(identity), index)
    };
    let actor_position = ActorPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };
    let Some(corpse) = corpse else {
        return Some(ItemTargetObservation {
            actor: actor_position,
            target: UseItemTarget::Gone,
        });
    };
    let Some(entries) = admission
        .root
        .read_corpse_contents(admission.character, corpse.item.item_instance_id)
        .await
        .ok()?
    else {
        return Some(ItemTargetObservation {
            actor: actor_position,
            target: UseItemTarget::Gone,
        });
    };
    let contents = entries
        .iter()
        .map(|entry| {
            Some(ViewItem {
                appearance_id: super::item_view::appearance_of(&entry.item.definition),
                key: ItemKey::Instance(entry.item.item_instance_id),
                item_definition_ref: definition_ref(index, &entry.item.definition)?,
                count: NonZeroU32::new(entry.item.quantity)?,
                sub_type: 0,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ItemTargetObservation {
        actor: actor_position,
        target: UseItemTarget::Corpse {
            position: ActorPosition {
                x: corpse.position.x,
                y: corpse.position.y,
                floor: corpse.position.floor,
            },
            contents,
        },
    })
}

/// D1211: resolve every locked carried definition from this exact admitted native generation.
pub(super) struct NativeItemWeights<'a>(pub(super) &'a NativeGameplayState);

#[path = "item_ref_admission/wheel_capacity.rs"]
mod wheel_capacity;

impl ItemWeightLookup for NativeItemWeights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }

    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        let facts = generation_item_facts(self.0, definition)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?;
        if facts.stack == ItemStackClass::Unknown {
            return Err(ItemWeightRefused::DefinitionMismatch);
        }
        facts.weight.ok_or(ItemWeightRefused::Unknown)
    }
}

/// Inbox custody admits the pinned immovable Hireling lamp without widening
/// ordinary carried-item admission. Existing carried definitions keep their facts.
pub(super) struct NativeInboxItemWeights<'a>(
    pub(super) &'a NativeGameplayState,
    pub(super) Option<&'a super::spell_premium_coordinator::SpellPremiumCoordinator>,
);

impl ItemWeightLookup for NativeInboxItemWeights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }

    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        let facts = generation_inbox_item_facts(self.0, definition)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?;
        if facts.stack == ItemStackClass::Unknown {
            return Err(ItemWeightRefused::DefinitionMismatch);
        }
        facts.weight.ok_or(ItemWeightRefused::Unknown)
    }

    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        self.1
            .is_some_and(|owner| owner.view(account).premium_gate().premium_current())
    }
}

/// The Channel half of a corpse take, decided before any durable call: the corpse is a bound
/// D85 entity and the entry an Item instance, and a dead actor takes nothing. Gives the corpse's
/// bound Item id and the entry id. An unbound corpse is refused as `AuthorityRejected`; no Item
/// id is ever derived for it.
fn corpse_for_take(
    runtime: &ChannelRuntimeV1,
    dead: bool,
    corpse: ItemKey,
    entry: ItemKey,
) -> Result<([u8; 16], [u8; 16]), GroundPickupError> {
    let rejected = || GroundPickupError::Transfer(ItemTransferError::AuthorityRejected);
    let (ItemKey::Entity(identity), ItemKey::Instance(entry_id)) = (corpse, entry) else {
        return Err(rejected());
    };
    if dead {
        return Err(rejected());
    }
    let corpse = runtime.bound_corpse(identity).ok_or_else(rejected)?;
    Ok((corpse.item.item_instance_id, entry_id))
}

/// MAP-ITEM-REF-1 Part B: command 9's corpse-entry TRANSFER into the main backpack, one durable
/// TRANSFER keyed by the command's CommandRef. A dead actor, an unbound corpse, a Channel content
/// pin of another generation or a session without an item fence is refused before any durable
/// write; an entry that is not in the corpse's durable contents is `SourceMismatch`. The facts of
/// the entry and of the equipped backpack are the pinned generation's ([`generation_item_facts`]).
pub(super) async fn take_corpse_entry(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    corpse: ItemKey,
    entry: ItemKey,
) -> Result<ItemTransferOutcome, GroundPickupError> {
    use crate::interaction_chest_use::entry_chest;
    let rejected = || GroundPickupError::Transfer(ItemTransferError::AuthorityRejected);
    // Lock order (SPELL-LOCK-2 §1.2): the lane before any Channel guard; the TRANSFER is a
    // key-33 writer. `None` is retryable with no side effect while the lane is fenced.
    let Some(permit) = admission.spell_lane_permit().await else {
        return Err(GroundPickupError::Transfer(ItemTransferError::Unavailable(
            crate::durability::DurabilityError::Unavailable,
        )));
    };
    if !admission
        .states
        .player_trades
        .physical(command.game_session_id, None)
    {
        return Err(rejected());
    }
    let (corpse, entry_id, native) = {
        let runtime = admission.runtime.lock().await;
        let dead = admission.states.spell_states.lock().await.is_dead(actor);
        let (corpse, entry_id) = corpse_for_take(&runtime, dead, corpse, entry)?;
        pinned_index(&runtime, admission.active_generation).ok_or_else(rejected)?;
        let native = admission
            .active_generation
            .and_then(ActiveGeneration::native_gameplay)
            .ok_or_else(rejected)?;
        (corpse, entry_id, native)
    };
    let fence = command.item_fence.ok_or_else(rejected)?;
    let command_id = crate::foundation::CommandId::new(command.command_id)
        .map_err(|_| GroundPickupError::Transfer(ItemTransferError::InvalidInput))?;
    let contents = admission
        .root
        .read_corpse_contents(admission.character, corpse)
        .await?
        .ok_or(GroundPickupError::SourceMismatch)?;
    let source = contents
        .into_iter()
        .find(|candidate| candidate.item.item_instance_id == entry_id)
        .ok_or(GroundPickupError::SourceMismatch)?;
    let item = generation_item_facts(native, &source.item.definition)
        .map_err(GroundPickupError::Content)?;
    let backpack = match admission
        .root
        .read_character_backpack(admission.character, fence.character_id)
        .await?
    {
        Some(current) => Some(
            generation_item_facts(native, &current.backpack.definition)
                .map_err(GroundPickupError::Content)?,
        ),
        None => None,
    };
    let request = ItemTransferRequest {
        command: crate::foundation::CommandRef::new(command.game_session_id, command_id),
        source_item_instance_id: entry_id,
        weight_generation_digest: NativeItemWeights(native)
            .with_wheel(admission.premium_coordinator)
            .generation_digest(),
        destination: ItemTransferDestination::MainBackpack,
        item,
        backpack,
        content_revision: entry_chest::CONTENT_REVISION.to_owned(),
        ruleset_revision: entry_chest::RULESET_REVISION.to_owned(),
        sim_revision: entry_chest::SIM_REVISION.to_owned(),
    };
    let mut candidate = admission
        .root
        .freeze_item_transfer(
            admission.character,
            admission.holder,
            fence,
            request,
            &NativeItemWeights(native).with_wheel(admission.premium_coordinator),
        )
        .await?;
    Ok(admission
        .root
        .commit_item_transfer(
            &permit,
            admission.character,
            admission.holder,
            fence,
            &mut candidate,
            &NativeItemWeights(native).with_wheel(admission.premium_coordinator),
        )
        .await?)
}

pub(super) async fn observe_character_inventory(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    game_session_id: GameSessionId,
) -> Option<InventoryItems> {
    use super::connection::AdmissionRefusal::{Rejected, Unavailable};
    use crate::durability::{DurabilityError, item_use::ItemUseError};
    // These reads share the root's single holder with the node cadence. A busy
    // holder must not turn a valid join/resume into a missing inventory snapshot.
    let backpack = super::AdmissionRetryBudget::for_join_read(admission.root, "inventory")
        .retry_admission_checkout(async || {
            let current = FreshAdmissionStore::from_root(admission.root.clone())
                .current_session_at(game_session_id)
                .await
                .map_err(|error| match error {
                    DurabilityError::RootUnavailable => Unavailable,
                    _ => Rejected,
                })?
                .0;
            if current.session_state() != GameSessionState::Active
                || current.current_game_session_id() != game_session_id
            {
                return Err(Rejected);
            }
            let fence = super::item_fence_of(current).ok_or(Rejected)?;
            let in_house = admission
                .runtime
                .lock()
                .await
                .house_player_for_session(game_session_id)
                .map_err(|_| Rejected)?
                .is_some();
            if in_house {
                // Character-carried custody uses the existing current House
                // custody fence without adding a service ACL requirement.
                admission
                    .root
                    .read_house_carried_inventory(
                        admission.character,
                        admission.holder,
                        fence,
                        *admission.channel_id.as_bytes(),
                    )
                    .await
                    .map(crate::durability::item_use::ItemUseInventory::into_backpack)
                    .map_err(|error| match error {
                        DurabilityError::RootUnavailable => Unavailable,
                        _ => Rejected,
                    })
            } else {
                // Ordinary Channel and activity snapshots cross the current actor
                // fence before publishing inventory, including closed death bags.
                admission
                    .root
                    .read_item_use_inventory(admission.character, admission.holder, fence)
                    .await
                    .map(crate::durability::item_use::ItemUseInventory::into_backpack)
                    .map_err(|error| match error {
                        ItemUseError::Unavailable(DurabilityError::RootUnavailable) => Unavailable,
                        _ => Rejected,
                    })
            }
        })
        .await
        .ok()?;
    let index = pinned_index(
        &*admission.runtime.lock().await,
        admission.active_generation,
    )?;
    inventory_from_backpack(backpack.as_ref(), |definition| {
        definition_ref(index, definition)
    })
}

#[cfg(test)]
#[path = "item_ref_admission_tests.rs"]
mod tests;
