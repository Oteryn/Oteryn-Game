//! IMBUE-WIRE-1 shrine window composed from the admitted actor and current Channel owner.
//!
//! The query reads a main backpack direct entry through the durable item owner and its
//! slots and allowances from the qualified native item policy. Apply and clear run the
//! IMBUE-FORGE0 §3 item gates and then the shrine access predicate, which fails closed with
//! REQUIREMENT_NOT_MET until the Temple of the Forgotten Knowledge access content and the
//! materials catalogue land; no imbuement writer is reachable from this owner yet.
use super::item_view::{ItemKey, SessionItemView};
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::{ReferenceItemField, ReferenceItemImbuement};
use crate::durability::imbuement::StoredImbuement;
use crate::foundation::{CommandId, RuntimeScopeRefV1};
use oteryn_gameplay_items::imbuement::{IMBUEMENT_CLEAR_GOLD, IMBUEMENT_TIER_RULES};
use oteryn_protocol_oteryn::imbuement as wire;
use wire::{ImbuementAction, ImbuementResult};

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

/// IMBUE-FORGE0 §3 gates in order: the item and its slot, the offer, an empty slot, one
/// imbuement per category, then shrine access. Clear needs an occupied slot first.
pub(super) fn decide(
    action: ImbuementAction,
    allowance: Option<&Allowance>,
    applied: &[StoredImbuement],
) -> wire::ImbuementReply {
    let reply = |result| wire::ImbuementReply {
        result,
        window: None,
    };
    let Some(allowance) = allowance else {
        return reply(ImbuementResult::NotAllowed);
    };
    let occupied = |slot: u8| applied.iter().any(|row| row.slot == slot);
    match action {
        ImbuementAction::Query => wire::ImbuementReply {
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
        },
        ImbuementAction::Apply { slot, imbuement } => {
            let Some(offer) = allowance.offers.get(usize::from(imbuement)) else {
                return reply(ImbuementResult::NotAllowed);
            };
            if slot >= allowance.slot_count {
                reply(ImbuementResult::NotAllowed)
            } else if occupied(slot) {
                reply(ImbuementResult::SlotOccupied)
            } else if applied.iter().any(|row| row.category + 1 == offer.family) {
                reply(ImbuementResult::NotAllowed)
            } else {
                // Shrine access: quest progress content is not declared yet (§3).
                reply(ImbuementResult::RequirementNotMet)
            }
        }
        ImbuementAction::Clear { slot } => {
            if slot >= allowance.slot_count || !occupied(slot) {
                reply(ImbuementResult::NotAllowed)
            } else {
                reply(ImbuementResult::RequirementNotMet)
            }
        }
    }
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
        // Only a carried, unequipped instance can be a main backpack direct entry; the owner
        // read below decides that, so every other handle is the same NOT_ALLOWED.
        let reply = match views.resolve(request.item) {
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
                decide(request.action, allowance.as_ref(), applied)
            }
            _ => decide(request.action, None, &[]),
        };
        wire::encode_imbuement_reply(&reply)
            .map_or(ImbuementReply::Unavailable, ImbuementReply::Payload)
    }
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
