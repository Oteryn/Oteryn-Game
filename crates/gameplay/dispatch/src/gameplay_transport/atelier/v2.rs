//! Gem Atelier v2 (capability 483): the v1 Atelier writer's replies without the gem domain,
//! plus the ordered Gem Vessel slots and Replace Vessel through the durable Wheel writer, and
//! Mod Reshaping through the same Atelier writer.
use super::AtelierWireReply;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::wheel_allocation::FencedWheelChange;
use oteryn_gameplay_progression::atelier::{reshape, vessels};
use oteryn_protocol_oteryn::atelier as v1;
use oteryn_protocol_oteryn::atelier::v2 as wire;

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn atelier_v2_wire(
        &self,
        admitted: AdmittedSession,
        kind: u32,
        payload: &[u8],
    ) -> AtelierWireReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_GEM_ATELIER_V2)
        {
            return AtelierWireReply::Rejected;
        }
        // Lock and paid requests carry the v1 bytes and run through the one Atelier writer.
        let (v1_kind, replace, reshape) = match kind {
            wire::COMMAND_TYPE_ATELIER_V2_QUERY if wire::decode_query(payload).is_ok() => {
                (v1::COMMAND_TYPE_ATELIER_QUERY, None, None)
            }
            wire::COMMAND_TYPE_ATELIER_V2_SET_LOCK if wire::decode_set_lock(payload).is_ok() => {
                (v1::COMMAND_TYPE_ATELIER_SET_LOCK, None, None)
            }
            wire::COMMAND_TYPE_ATELIER_V2_MUTATE if wire::decode_mutate(payload).is_ok() => {
                (v1::COMMAND_TYPE_ATELIER_MUTATE, None, None)
            }
            wire::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL => {
                let Ok(request) = wire::decode_replace_vessel(payload) else {
                    return AtelierWireReply::Rejected;
                };
                (v1::COMMAND_TYPE_ATELIER_QUERY, Some(request), None)
            }
            wire::COMMAND_TYPE_ATELIER_V2_RESHAPE => {
                let Ok(request) = wire::decode_reshape(payload) else {
                    return AtelierWireReply::Rejected;
                };
                (v1::COMMAND_TYPE_ATELIER_MUTATE, None, Some(request))
            }
            _ => return AtelierWireReply::Rejected,
        };
        let replace_refused = match replace {
            None => false,
            Some(request) => match self.replace_vessel(admitted, request).await {
                FencedWheelChange::Done { refused } => refused,
                FencedWheelChange::Rejected => return AtelierWireReply::Rejected,
                FencedWheelChange::Unavailable => return AtelierWireReply::Unavailable,
            },
        };
        let v1_payload: &[u8] = if v1_kind == v1::COMMAND_TYPE_ATELIER_QUERY {
            &[]
        } else {
            payload
        };
        let reply = match reshape {
            Some(request) => self.reshape(admitted, request).await,
            None => self.atelier_v1_wire(admitted, v1_kind, v1_payload).await,
        };
        let reply = match reply {
            AtelierWireReply::Payload(bytes) => bytes,
            other => return other,
        };
        let Ok(reply) = v1::decode_reply(&reply) else {
            return AtelierWireReply::Unavailable;
        };
        // Quotes ride on query replies only (contract); a Replace Vessel reply carries none.
        let quotes = kind == wire::COMMAND_TYPE_ATELIER_V2_QUERY;
        match self
            .atelier_v2_reply(admitted, reply, replace_refused, quotes)
            .await
        {
            Some(bytes) => AtelierWireReply::Payload(bytes),
            // A query has no outcome to reconcile; a change whose reply cannot be built is
            // indeterminate for the client, as on the allocation route.
            None if kind == wire::COMMAND_TYPE_ATELIER_V2_QUERY => AtelierWireReply::Rejected,
            None => AtelierWireReply::Unavailable,
        }
    }

    /// Replace Vessel: one Wheel write that keeps the allocation and changes one slot.
    async fn replace_vessel(
        &self,
        admitted: AdmittedSession,
        request: wire::ReplaceVesselRequest,
    ) -> FencedWheelChange {
        let Ok(occurrence) =
            crate::durability::character_wheel::WheelOccurrence::from_bytes(request.occurrence)
        else {
            return FencedWheelChange::Rejected;
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return FencedWheelChange::Rejected;
        };
        let Some(item_fence) = self.wheel_change_gate(admitted, true).await else {
            return FencedWheelChange::Rejected;
        };
        if usize::from(request.slot) >= vessels::GEM_VESSEL_SLOTS {
            return FencedWheelChange::Rejected;
        }
        // The Wheel writer binds the slot and gem as sent and places them against the stored
        // slots after its receipt lookup, keeping the stored allocation.
        let outcome = self
            .wheel_fenced_change(
                admitted,
                item_fence,
                crate::durability::character_wheel::WheelChangeRequest {
                    occurrence,
                    expected_wheel_revision: request.expected_wheel_revision,
                    slots: crate::durability::character_wheel::WheelSlots::ZERO,
                    vessels: Some(
                        crate::durability::character_wheel::WheelVesselIntent::Place {
                            slot: request.slot,
                            gem: request.gem,
                        },
                    ),
                },
            )
            .await;
        drop(permit);
        if let (FencedWheelChange::Done { .. }, Some(actor)) = (&outcome, admitted.runtime_actor) {
            self.refresh_wheel_vitals(actor, admitted.game_session_id)
                .await;
        }
        outcome
    }

    /// Mod Reshaping: only in a temple and with Premium; a refusal writes nothing. The
    /// modifier draw, gold, fragment BURNs and receipt commit in the one Atelier writer.
    async fn reshape(
        &self,
        admitted: AdmittedSession,
        request: wire::ReshapeRequest,
    ) -> AtelierWireReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(v1::CAPABILITY_WHEEL_ALLOCATION_V1)
        {
            return AtelierWireReply::Rejected;
        }
        let (Some(expected), Some(actor)) = (admitted.item_fence, admitted.runtime_actor) else {
            return AtelierWireReply::Rejected;
        };
        if expected.game_session_id != admitted.game_session_id
            || expected.connection_generation.get() != admitted.continuity.connection_generation
        {
            return AtelierWireReply::Rejected;
        }
        let in_temple = crate::gameplay_transport::wheel_allocation::removal_permitted(
            self,
            actor,
            admitted.game_session_id,
        )
        .await;
        let Some(read) = self.atelier_wheel_read(expected).await else {
            return AtelierWireReply::Unavailable;
        };
        // Admission binds fresh writes only: a refused request still reaches the writer as
        // a replay probe, so a committed occurrence whose reply was lost replays its receipt
        // after Premium lapsed or combat began.
        let refused = reshape::admitted(in_temple, read.premium_permitted).is_err();
        let request = Some(crate::durability::character_atelier::AtelierRequest {
            occurrence: request.occurrence,
            expected_atelier_revision: request.expected_revision,
            mutation: crate::durability::character_atelier::AtelierMutation::Reshape {
                gem: request.gem,
                position: request.position,
            },
            resources: None,
        });
        self.atelier_v1_apply(
            admitted,
            expected,
            v1::COMMAND_TYPE_ATELIER_MUTATE,
            request,
            refused,
        )
        .await
    }

    async fn atelier_wheel_read(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Option<crate::durability::character_wheel::WheelPresentationRead> {
        self.root
            .read_wheel_presentation(
                self.character,
                self.holder,
                fence,
                std::sync::Arc::clone(self.wheel_ruleset.as_ref()?),
                self.premium_coordinator?,
            )
            .await
            .ok()
    }

    async fn atelier_v2_reply(
        &self,
        admitted: AdmittedSession,
        reply: v1::AtelierReply,
        replace_refused: bool,
        quotes: bool,
    ) -> Option<Vec<u8>> {
        let fence = admitted.item_fence?;
        let actor = admitted.runtime_actor?;
        let rules = self.wheel_ruleset.as_ref()?;
        let read = self.atelier_wheel_read(fence).await?;
        let in_temple = crate::gameplay_transport::wheel_allocation::removal_permitted(
            self,
            actor,
            admitted.game_session_id,
        )
        .await;
        let open = if read.allocation.current {
            vessels::open_slots_for(
                rules,
                &read.allocation.slots,
                &read.vocation,
                read.level,
                read.promoted,
            )
        } else {
            0
        };
        let slots = if reply.refused && reply.revision == 0 {
            Vec::new()
        } else {
            read.vessels
                .iter()
                .zip(vessels::GEM_VESSEL_SLOT_QUALITIES)
                .enumerate()
                .map(|(slot, (gem, quality))| wire::VesselSlotView {
                    quality: quality_code(quality),
                    open: slot < open,
                    gem: *gem,
                })
                .collect()
        };
        wire::encode_reply(&wire::AtelierReply {
            refused: reply.refused || replace_refused,
            revision: reply.revision,
            wheel_revision: read.allocation.wheel_revision,
            in_temple,
            premium: read.premium_permitted,
            gems: reply
                .gems
                .into_iter()
                .map(|gem| wire::GemView {
                    slot: read
                        .vessels
                        .iter()
                        .position(|cell| *cell == Some(gem.id))
                        .and_then(|slot| u8::try_from(slot).ok()),
                    id: gem.id,
                    quality: gem.quality,
                    locked: gem.locked,
                    modifiers: gem.modifiers,
                })
                .collect(),
            grades: reply.grades,
            quality_quotes: reply
                .quality_quotes
                .into_iter()
                .filter(|_| quotes)
                .map(|quote| wire::QualityQuote {
                    quality: quote.quality,
                    reveal_gold: quote.reveal_gold,
                    fragment_kind: quote.fragment_kind,
                    fragment_min: quote.fragment_min,
                    fragment_max: quote.fragment_max,
                })
                .collect(),
            grade_quotes: if quotes {
                reply.grade_quotes
            } else {
                Vec::new()
            },
            vessels: slots,
            vessel_nodes: vessels::node_slots(rules, &read.vocation),
            vessel_node_bonus: vessels::GEM_VESSEL_FLAT_DAMAGE_HEALING,
            resistance_capped: resistance_capped(rules, &read),
        })
        .ok()
    }
}

/// The cap warning from the same projection the Wheel applies: gems contribute only to a
/// current, eligible Wheel.
fn resistance_capped(
    rules: &oteryn_gameplay_progression::wheel::WheelRuleset,
    read: &crate::durability::character_wheel::WheelPresentationRead,
) -> bool {
    let Some(atelier) = read.atelier.as_ref() else {
        return false;
    };
    if !read.allocation.current
        || oteryn_gameplay_progression::wheel::eligibility(
            rules,
            &read.vocation,
            read.level,
            read.promoted,
            read.premium_permitted,
        )
        .is_err()
    {
        return false;
    }
    let Ok(data) = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded() else {
        return false;
    };
    let Some(mut effects) = oteryn_gameplay_progression::wheel_effects::derive_runtime_effects(
        data.wheel(),
        &read.allocation,
        &read.vocation,
        read.level,
        read.promoted,
        read.premium_permitted,
    ) else {
        return false;
    };
    effects
        .contribute_gems(atelier, &data, &read.allocation)
        .is_some()
        && effects.gem_resistance_over_cap()
}

fn quality_code(quality: oteryn_gameplay_progression::atelier::GemQuality) -> u8 {
    match quality {
        oteryn_gameplay_progression::atelier::GemQuality::Lesser => 0,
        oteryn_gameplay_progression::atelier::GemQuality::Regular => 1,
        oteryn_gameplay_progression::atelier::GemQuality::Greater => 2,
    }
}
