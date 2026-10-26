//! IMBUE-WIRE-1 shrine window composed from the admitted actor and current Channel owner.
//!
//! The query reads a main backpack direct entry through the durable item owner and its
//! slots and allowances from the qualified native item policy. Apply and clear run the
//! IMBUE-FORGE0 §3 item gates, then shrine access (an imbuing shrine within one tile; the
//! Canary quest unlock is a declared difference) and Premium for tiers 2 and 3, and then the
//! one fenced durable transaction that consumes the recipe materials, burns the gold and writes
//! or removes the imbuement (PARITY-IMBUE-FORGE-WIRE-1, migration 0196).
use super::item_ref_admission::{generation_item_facts, pinned_index};
use super::item_view::{ItemKey, SessionItemView};
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::content::{ReferenceItemField, ReferenceItemImbuement};
use crate::durability::imbuement::StoredImbuement;
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::durability::item_imbuement::{
    ImbuementCommand, ImbuementMaterialNeed, ImbuementWrite, ImbuementWriteError,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{CommandId, RuntimeScopeRefV1};
use crate::map::view;
use oteryn_gameplay_items::imbuement::{
    IMBUEMENT_CATALOGUE_REVISION, IMBUEMENT_CLEAR_GOLD, IMBUEMENT_ETCHER_ITEM_ID,
    IMBUEMENT_TIER_RULES, imbuement_recipe, imbuement_scroll,
};
use oteryn_protocol_oteryn::imbuement as wire;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use sha2::{Digest, Sha256};
use wire::{ImbuementAction, ImbuementResult};

/// The Canary imbuing shrines, sorted (`data/items/items.xml`, `script="imbuement_shrine"`).
pub(super) const SHRINE_KEYS: [&str; 10] = [
    "oteryn:item.tibia.i24964",
    "oteryn:item.tibia.i25060",
    "oteryn:item.tibia.i25061",
    "oteryn:item.tibia.i25103",
    "oteryn:item.tibia.i25104",
    "oteryn:item.tibia.i25174",
    "oteryn:item.tibia.i25175",
    "oteryn:item.tibia.i25182",
    "oteryn:item.tibia.i25183",
    "oteryn:item.tibia.i25202",
];

#[derive(Debug)]
pub enum ImbuementReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

/// The slots and the applicable offers of one item definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Allowance {
    pub(super) slot_count: u8,
    pub(super) offers: Vec<wire::ImbuementOffer>,
}

/// `None` unless the content declares 1-3 slots. Offers need both the allowed families and
/// the excluded families to be known; D1163 admits every tier on an admitted family.
pub(super) fn allowance(
    imbuement: &ReferenceItemField<ReferenceItemImbuement>,
) -> Option<Allowance> {
    let ReferenceItemField::Known(imbuement) = imbuement else {
        return None;
    };
    let ReferenceItemField::Known(slot_count) = imbuement.slot_count else {
        return None;
    };
    if !(1..=wire::MAX_SLOTS).contains(&slot_count) {
        return None;
    }
    let excluded = match &imbuement.excluded_families {
        ReferenceItemField::Known(excluded) => Some(excluded.as_slice()),
        ReferenceItemField::NotApplicable => Some(&[][..]),
        _ => None,
    };
    let mut families: Vec<u8> = match (&imbuement.allowed_family_tiers, excluded) {
        (ReferenceItemField::Known(allowed), Some(excluded)) => allowed
            .iter()
            .filter(|allowance| !excluded.contains(&allowance.family))
            .map(|allowance| allowance.family.wire())
            .collect(),
        _ => Vec::new(),
    };
    families.sort_unstable();
    families.dedup();
    let offers = families
        .into_iter()
        .flat_map(|family| {
            IMBUEMENT_TIER_RULES
                .iter()
                .map(move |rules| wire::ImbuementOffer {
                    family,
                    tier: rules.tier as u8,
                    gold: u32::try_from(rules.apply_gold).unwrap_or(u32::MAX),
                    requires_premium: rules.requires_premium,
                })
        })
        .collect();
    Some(Allowance { slot_count, offers })
}

/// The outcome of the item gates: a reply, or a write that shrine access, Premium, the
/// materials and the gold still decide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Decision {
    Reply(wire::ImbuementReply),
    Apply { slot: u8, offer: wire::ImbuementOffer },
    Clear { slot: u8 },
}

/// Result of qualifying an inventory USE-with before the durable owner consumes its source.
/// In particular, `Etch` describes all rows in one outcome; callers must never loop single-slot
/// shrine clears because that can consume the device after a partial clear.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "qualified before durable USE-with is connected")
)]
pub enum ImbuementItemDecision {
    NotAnImbuementItem,
    Refused(ImbuementResult),
    ApplyScroll { slot: u8, family: u8, tier: u8 },
    Etch { slots: Vec<u8> },
}

/// Global etcher/filled-scroll rules which do not depend on shrine, quest or Premium access.
/// Scrolls select the first empty slot, while normal compatibility and category-conflict gates
/// remain in force. Consumption belongs to the same durable transaction as the returned change.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "qualified before durable USE-with is connected")
)]
pub fn decide_imbuement_item_use(
    source_item_id: u32,
    allowance: Option<&Allowance>,
    applied: &[StoredImbuement],
) -> ImbuementItemDecision {
    let Some(allowance) = allowance else {
        return if source_item_id == IMBUEMENT_ETCHER_ITEM_ID
            || imbuement_scroll(source_item_id).is_some()
        {
            ImbuementItemDecision::Refused(ImbuementResult::NotAllowed)
        } else {
            ImbuementItemDecision::NotAnImbuementItem
        };
    };
    if source_item_id == IMBUEMENT_ETCHER_ITEM_ID {
        let mut slots: Vec<u8> = applied.iter().map(|row| row.slot).collect();
        slots.sort_unstable();
        slots.dedup();
        return if slots.is_empty() {
            ImbuementItemDecision::Refused(ImbuementResult::NotAllowed)
        } else {
            ImbuementItemDecision::Etch { slots }
        };
    }
    let Some(scroll) = imbuement_scroll(source_item_id) else {
        return ImbuementItemDecision::NotAnImbuementItem;
    };
    if !allowance.offers.iter().any(|offer| {
        offer.family == scroll.family && offer.tier == scroll.tier
    }) || applied
        .iter()
        .any(|row| row.category + 1 == scroll.family)
    {
        return ImbuementItemDecision::Refused(ImbuementResult::NotAllowed);
    }
    (0..allowance.slot_count)
        .find(|slot| !applied.iter().any(|row| row.slot == *slot))
        .map_or(
            ImbuementItemDecision::Refused(ImbuementResult::SlotOccupied),
            |slot| ImbuementItemDecision::ApplyScroll {
                slot,
                family: scroll.family,
                tier: scroll.tier,
            },
        )
}

fn reply(result: ImbuementResult) -> wire::ImbuementReply {
    wire::ImbuementReply {
        result,
        window: None,
    }
}

/// IMBUE-FORGE0 §3 item gates in order: the item and its slot, the offer, an empty slot and
/// one imbuement per category. Clear needs an occupied slot.
pub(super) fn decide(
    action: ImbuementAction,
    allowance: Option<&Allowance>,
    applied: &[StoredImbuement],
) -> Decision {
    let Some(allowance) = allowance else {
        return Decision::Reply(reply(ImbuementResult::NotAllowed));
    };
    let occupied = |slot: u8| applied.iter().any(|row| row.slot == slot);
    match action {
        ImbuementAction::Query => Decision::Reply(wire::ImbuementReply {
            result: ImbuementResult::Ok,
            window: Some(wire::ImbuementWindow {
                slot_count: allowance.slot_count,
                applied: applied
                    .iter()
                    .map(|row| wire::AppliedImbuement {
                        slot: row.slot,
                        family: row.category + 1,
                        tier: row.tier,
                        remaining_ms: row.remaining_ms,
                    })
                    .collect(),
                offers: allowance.offers.clone(),
                clear_gold: u32::try_from(IMBUEMENT_CLEAR_GOLD).unwrap_or(u32::MAX),
            }),
        }),
        ImbuementAction::Apply { slot, imbuement } => {
            let Some(offer) = allowance.offers.get(usize::from(imbuement)) else {
                return Decision::Reply(reply(ImbuementResult::NotAllowed));
            };
            if slot >= allowance.slot_count {
                Decision::Reply(reply(ImbuementResult::NotAllowed))
            } else if occupied(slot) {
                Decision::Reply(reply(ImbuementResult::SlotOccupied))
            } else if applied.iter().any(|row| row.category + 1 == offer.family) {
                Decision::Reply(reply(ImbuementResult::NotAllowed))
            } else {
                Decision::Apply {
                    slot,
                    offer: *offer,
                }
            }
        }
        ImbuementAction::Clear { slot } => {
            if slot >= allowance.slot_count || !occupied(slot) {
                Decision::Reply(reply(ImbuementResult::NotAllowed))
            } else {
                Decision::Clear { slot }
            }
        }
    }
}

/// Shrine access: an imbuing shrine on the actor's floor within one tile (Canary
/// `imbuement_shrine` is used from an adjacent tile).
fn shrine_in_reach(at: &super::bundle_map_position::BundleMapPosition<'_>, index: &ItemDefinitionIndex) -> bool {
    let facts = at.map.facts.pinned();
    let Ok(floor) = i8::try_from(at.at.floor) else {
        return false;
    };
    (-1..=1)
        .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
        .any(|(dx, dy)| {
            let (Ok(x), Ok(y)) = (u16::try_from(at.at.x + dx), u16::try_from(at.at.y + dy)) else {
                return false;
            };
            let pos = crate::map::overlay::TilePos { x, y, floor };
            let Ok(Some(tile)) = view::compose_tile(&at.map.overlay, &facts, pos) else {
                return false;
            };
            tile.entries.iter().any(|entry| {
                matches!(entry.facts.definition, MapDefinition::Item(reference)
                    if index.definition_at(reference)
                        .is_some_and(|(key, _)| SHRINE_KEYS.binary_search(&key).is_ok()))
            })
        })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn imbuement_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
        views: Option<&SessionItemView>,
    ) -> ImbuementReply {
        let Ok(request) = wire::decode_imbuement_request(payload) else {
            return ImbuementReply::Rejected;
        };
        let (Some(fence), Ok(_), Some(views)) = (admitted.item_fence, CommandId::new(id), views)
        else {
            return ImbuementReply::Rejected;
        };
        if kind != wire::COMMAND_TYPE_IMBUEMENT {
            return ImbuementReply::Rejected;
        }
        let pin = {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_IMBUE_V1)
                || fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || admitted.world_id != binding.world_id()
                || admitted.channel_id != binding.channel_id()
                || fence.runtime_scope
                    != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !admitted.runtime_actor.is_some_and(|actor| {
                    runtime
                        .player_control_facts(actor, admitted.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                })
            {
                return ImbuementReply::Rejected;
            }
            runtime.content_pin().activation_sequence()
        };
        // An accepted apply or clear replays before any handle or content resolution.
        let binding: [u8; 32] = Sha256::digest(payload).into();
        if request.action != ImbuementAction::Query {
            match self
                .root
                .read_imbuement_command(self.character, admitted.game_session_id, id, binding)
                .await
            {
                Ok(Some(result)) => return encode(&reply(result)),
                Ok(None) => (),
                Err(ImbuementWriteError::Unavailable(_)) => return ImbuementReply::Unavailable,
                Err(_) => return ImbuementReply::Rejected,
            }
        }
        // Only a carried, unequipped instance can be a main backpack direct entry; the owner
        // read below decides that, so every other handle is the same NOT_ALLOWED.
        let (decision, instance) = match views.resolve(request.item) {
            Some(key @ ItemKey::Instance(instance)) if views.equipped_slot(&key).is_none() => {
                let target = match self
                    .root
                    .read_imbuement_target(self.character, fence.character_id, instance)
                    .await
                {
                    Ok(target) => target,
                    Err(_) => return ImbuementReply::Unavailable,
                };
                let runtime = self.runtime.lock().await;
                if runtime.content_pin().activation_sequence() != pin {
                    return ImbuementReply::Unavailable;
                }
                let generation = self.active_generation.filter(|generation| {
                    generation.activation_sequence() == pin
                        && generation.identity().world_id() == runtime.content_pin().world_id()
                });
                let allowance = target.as_ref().and_then(|target| {
                    let definition = &target.definition;
                    let policy = generation?
                        .native_gameplay()?
                        .item_policy(&definition.production_key, &definition.revision_ref)?;
                    let record = policy.record();
                    let actual = &record.production_definition;
                    (record.admission.materializable
                        && actual.family == definition.family
                        && actual.production_key == definition.production_key
                        && actual.revision_ref == definition.revision_ref)
                        .then(|| allowance(&record.semantics.imbuement))
                        .flatten()
                });
                let applied = target.as_ref().map_or(&[][..], |t| t.applied.as_slice());
                (decide(request.action, allowance.as_ref(), applied), instance)
            }
            _ => return encode(&reply(ImbuementResult::NotAllowed)),
        };
        let (write, premium) = match decision {
            Decision::Reply(reply) => return encode(&reply),
            Decision::Apply { slot, offer } => {
                let Some(recipe) = imbuement_recipe(offer.family, offer.tier) else {
                    return encode(&reply(ImbuementResult::NotAllowed));
                };
                let write = ImbuementWrite::Apply {
                    slot,
                    category: offer.family - 1,
                    imbuement_key: recipe.key.into(),
                    imbuement_revision: IMBUEMENT_CATALOGUE_REVISION.into(),
                    tier: offer.tier,
                    gold: recipe.apply_gold,
                    materials: recipe
                        .materials
                        .iter()
                        .map(|&(key, count)| ImbuementMaterialNeed {
                            production_key: key.into(),
                            count,
                        })
                        .collect(),
                };
                (write, recipe.requires_premium)
            }
            Decision::Clear { slot } => (
                ImbuementWrite::Clear {
                    slot,
                    gold: IMBUEMENT_CLEAR_GOLD,
                },
                false,
            ),
        };
        let Some(actor) = admitted.runtime_actor else {
            return ImbuementReply::Rejected;
        };
        let native = {
            let mut runtime = self.runtime.lock().await;
            let Some(index) = pinned_index(&runtime, self.active_generation) else {
                return ImbuementReply::Unavailable;
            };
            let reach = super::bundle_map_position(&mut runtime, self.movement_cells, actor)
                .is_some_and(|at| shrine_in_reach(&at, index));
            if !reach {
                return encode(&reply(ImbuementResult::RequirementNotMet));
            }
            let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
                return ImbuementReply::Unavailable;
            };
            native
        };
        if premium
            && !self
                .session_premium_gate(admitted.game_session_id)
                .await
                .premium_current()
        {
            return encode(&reply(ImbuementResult::RequirementNotMet));
        }
        let Ok(Some(current)) = self.current_quest_fence(admitted.game_session_id).await else {
            return ImbuementReply::Unavailable;
        };
        if current.item_fence() != fence {
            return ImbuementReply::Rejected;
        }
        let Ok(Some(bag)) = self
            .root
            .read_character_backpack(self.character, current.character_id)
            .await
        else {
            return ImbuementReply::Unavailable;
        };
        let derived =
            |tag: &[u8]| super::npc_trade_command::derived(tag, admitted.game_session_id, id);
        let Some(index) = native.item_index() else {
            return ImbuementReply::Unavailable;
        };
        let coin = |key: &str| {
            Some(TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: index.revision(key)?.into(),
            })
        };
        let (Some(gold), Some(platinum), Some(crystal), Ok(backpack)) = (
            coin("oteryn:item.tibia.i3031"),
            coin("oteryn:item.tibia.i3035"),
            coin("oteryn:item.tibia.i3043"),
            generation_item_facts(native, &bag.backpack.definition),
        ) else {
            return ImbuementReply::Unavailable;
        };
        let command = ImbuementCommand {
            transaction_id: derived(b"oteryn:imbuement-transaction:v1"),
            event_id: derived(b"oteryn:imbuement-event:v1"),
            command_id: id,
            binding,
            content_digest: native.source_digest(),
            item: instance,
            write,
            coins: FeeChangeFacts {
                item_instance_ids: [
                    derived(b"oteryn:imbuement-change-0"),
                    derived(b"oteryn:imbuement-change-1"),
                ],
                platinum,
                gold,
                crystal,
                backpack,
            },
            occurred_at_unix_ms: super::npc_trade_command::unix_ms_now(),
        };
        match self
            .root
            .commit_imbuement_command(self.character, self.holder, current, command)
            .await
        {
            Ok(result) => encode(&reply(result)),
            Err(ImbuementWriteError::Unavailable(_)) => ImbuementReply::Unavailable,
            Err(_) => ImbuementReply::Rejected,
        }
    }
}

fn encode(reply: &wire::ImbuementReply) -> ImbuementReply {
    wire::encode_imbuement_reply(reply).map_or(ImbuementReply::Unavailable, ImbuementReply::Payload)
}

macro_rules! authority_methods {
    () => {
        fn imbuement_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
            _views: Option<&$crate::gameplay_transport::item_view::SessionItemView>,
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::imbuement::ImbuementReply>
        {
            async { $crate::gameplay_transport::imbuement::ImbuementReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn imbuement_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
            views: Option<&item_view::SessionItemView>,
        ) -> imbuement::ImbuementReply {
            self.imbuement_wire(admitted, id, kind, payload, views)
                .await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
#[path = "imbuement_tests.rs"]
mod tests;
