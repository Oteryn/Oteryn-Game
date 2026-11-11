//! Gem Atelier v2 (capability 483): the v1 Atelier writer's replies without the gem domain,
//! plus the ordered Gem Vessel slots and Replace Vessel through the durable Wheel writer.
use super::AtelierWireReply;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::wheel_allocation::FencedWheelChange;
use oteryn_gameplay_progression::atelier::vessels;
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
        let (v1_kind, replace) = match kind {
            wire::COMMAND_TYPE_ATELIER_V2_QUERY if wire::decode_query(payload).is_ok() => {
                (v1::COMMAND_TYPE_ATELIER_QUERY, None)
            }
            wire::COMMAND_TYPE_ATELIER_V2_SET_LOCK if wire::decode_set_lock(payload).is_ok() => {
                (v1::COMMAND_TYPE_ATELIER_SET_LOCK, None)
            }
            wire::COMMAND_TYPE_ATELIER_V2_MUTATE if wire::decode_mutate(payload).is_ok() => {
                (v1::COMMAND_TYPE_ATELIER_MUTATE, None)
            }
            wire::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL => {
                let Ok(request) = wire::decode_replace_vessel(payload) else {
                    return AtelierWireReply::Rejected;
                };
                (v1::COMMAND_TYPE_ATELIER_QUERY, Some(request))
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
        let reply = match self.atelier_v1_wire(admitted, v1_kind, v1_payload).await {
            AtelierWireReply::Payload(bytes) => bytes,
            other => return other,
        };
        let Ok(reply) = v1::decode_reply(&reply) else {
            return AtelierWireReply::Unavailable;
        };
        match self
            .atelier_v2_reply(admitted, reply, replace_refused)
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
        let Some(read) = self.atelier_wheel_read(item_fence).await else {
            return FencedWheelChange::Unavailable;
        };
        let Some(vessels) = vessels::place(&read.vessels, usize::from(request.slot), request.gem)
        else {
            return FencedWheelChange::Rejected;
        };
        let outcome = self
            .wheel_fenced_change(
                admitted,
                item_fence,
                crate::durability::character_wheel::WheelChangeRequest {
                    occurrence,
                    expected_wheel_revision: request.expected_wheel_revision,
                    slots: read.allocation.slots,
                    vessels: Some(vessels),
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
                .map(|quote| wire::QualityQuote {
                    quality: quote.quality,
                    reveal_gold: quote.reveal_gold,
                    fragment_kind: quote.fragment_kind,
                    fragment_min: quote.fragment_min,
                    fragment_max: quote.fragment_max,
                })
                .collect(),
            grade_quotes: reply.grade_quotes,
            vessels: slots,
            vessel_nodes: vessels::node_slots(rules, &read.vocation),
            vessel_node_bonus: vessels::GEM_VESSEL_FLAT_DAMAGE_HEALING,
        })
        .ok()
    }
}

fn quality_code(quality: oteryn_gameplay_progression::atelier::GemQuality) -> u8 {
    match quality {
        oteryn_gameplay_progression::atelier::GemQuality::Lesser => 0,
        oteryn_gameplay_progression::atelier::GemQuality::Regular => 1,
        oteryn_gameplay_progression::atelier::GemQuality::Greater => 2,
    }
}
