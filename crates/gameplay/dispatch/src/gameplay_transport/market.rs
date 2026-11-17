//! Depot-admitted MARKET_V1. No client Account, World or custody proof is accepted.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::market::{self as wire, MarketReply, MarketRow};
use std::num::NonZeroU32;
#[path = "market/details.rs"]
mod details;

macro_rules! authority_methods {
    () => {
        fn market_command(
            &self,
            _admitted: AdmittedSession,
            _id: u64,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn market_command(
            &self,
            admitted: AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            crate::gameplay_transport::market::execute(self, admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;

fn refused(code: u8) -> Option<Vec<u8>> {
    wire::encode_reply(&MarketReply {
        action: 0,
        code,
        offer: [0; 16],
        rows: Vec::new(),
    })
    .ok()
}
fn marketable(
    native: &crate::content::native_gameplay::NativeGameplayState,
    key: &str,
    revision: &str,
) -> bool {
    native.marketable_definition(key, revision)
}
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    payload: &[u8],
) -> Option<Vec<u8>> {
    let query = if oteryn_protocol_oteryn::market_details::is_query(payload) {
        match oteryn_protocol_oteryn::market_details::decode_query(payload) {
            Ok(query) => Some(query),
            Err(_) => {
                return oteryn_protocol_oteryn::market_details::encode_reply(
                    &oteryn_protocol_oteryn::market_details::Reply::Refused { code: 9 },
                )
                .ok();
            }
        }
    } else {
        None
    };
    let refusal = |code| {
        if query.is_some() {
            oteryn_protocol_oteryn::market_details::encode_reply(
                &oteryn_protocol_oteryn::market_details::Reply::Refused { code },
            )
            .ok()
        } else {
            refused(code)
        }
    };
    let request = if let Some(query) = &query {
        wire::MarketRequest {
            action: if matches!(
                query,
                oteryn_protocol_oteryn::market_details::Query::Statistics { .. }
                    | oteryn_protocol_oteryn::market_details::Query::Book { .. }
            ) {
                2
            } else {
                1
            },
            definition: match query {
                oteryn_protocol_oteryn::market_details::Query::Statistics { definition }
                | oteryn_protocol_oteryn::market_details::Query::Book { definition, .. } => {
                    *definition
                }
                _ => 0,
            },
            offer: [0; 16],
            amount: 0,
            price: 0,
            anonymous: false,
            page: 0,
        }
    } else {
        match wire::decode_request(payload) {
            Ok(request) => request,
            Err(_) => return refusal(9),
        }
    };
    let Some(fence) = admitted.item_fence else {
        return refusal(9);
    };
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_MARKET_V1)
        || !admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1)
        || fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
        || fence.runtime_scope != RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
    {
        return refusal(9);
    }
    let actor = admitted.runtime_actor?;
    let _permit = owner.spell_lane_permit().await?;
    let (town, mut definition, native) = {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let Some(native) = super::equipment_move::native_for_move(
            &runtime,
            owner.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, admitted.game_session_id),
        ) else {
            return refusal(9);
        };
        let town = super::bundle_map_position(&mut runtime, owner.movement_cells(), actor)
            .as_ref()
            .and_then(super::depot::locker);
        let index = native.item_index()?;
        let definition = NonZeroU32::new(request.definition)
            .and_then(|r| super::item_look::typed_definition(index, r));
        (town, definition, native)
    };
    let Some(town) = town else {
        return refusal(11);
    };
    if matches!(request.action, 2 | 5 | 6)
        && !definition
            .as_ref()
            .is_some_and(|d| marketable(native, &d.production_key, &d.revision_ref))
    {
        return refusal(14);
    }
    if let Some(query) = query.clone() {
        return details::project(owner, fence, native, town, query, definition).await;
    }
    if matches!(request.action, 5 | 6)
        && !owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    {
        return refusal(12);
    }
    if request.action >= 5 && crate::durability::bank::fee_payer_is_junior(fence.character_id) {
        return refusal(13);
    }
    if owner
        .root
        .advance_market(owner.character, owner.holder, admitted.world_id)
        .await
        .is_err()
    {
        // The requested trade has not started; background market work can retry later.
        return refusal(9);
    }
    if request.action == 8 {
        let offers = owner
            .root
            .query_market(owner.character, owner.holder, fence, request, String::new())
            .await
            .ok()?;
        let Some((offer, _)) = offers.first() else {
            return refusal(6);
        };
        let index = native.item_index()?;
        let Some(reference) = index.definition_ref(
            offer.definition.family(),
            offer.definition.key(),
            offer.definition.revision(),
        ) else {
            return refusal(14);
        };
        if !marketable(native, offer.definition.key(), offer.definition.revision()) {
            return refusal(14);
        }
        definition = super::item_look::typed_definition(index, reference);
    }
    if request.action >= 5 {
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(id).ok()?);
        let reply = owner
            .root
            .commit_market(
                owner.character,
                owner.holder,
                fence,
                command,
                request,
                definition,
                town,
            )
            .await
            .ok()?;
        if reply.code == 1 && matches!(request.action, 5 | 6) {
            owner
                .root
                .advance_market(owner.character, owner.holder, admitted.world_id)
                .await
                .ok()?;
        }
        return wire::encode_reply(&reply).ok();
    }
    let offers = owner
        .root
        .query_market(
            owner.character,
            owner.holder,
            fence,
            request,
            definition
                .as_ref()
                .map(|d| d.production_key.clone())
                .unwrap_or_default(),
        )
        .await
        .ok()?;
    let index = native.item_index()?;
    let mut rows = Vec::new();
    if request.action == 1 {
        let mut seen = 0_u32;
        for number in 1..=u32::MAX {
            let reference = NonZeroU32::new(number)?;
            let Some((key, revision)) = index.definition_at(reference) else {
                break;
            };
            if !marketable(native, key, revision) {
                continue;
            }
            if seen >= request.page * 32 {
                rows.push(MarketRow {
                    offer: [0; 16],
                    definition: number,
                    side: 0,
                    amount: 0,
                    price: 0,
                    state: 0,
                    anonymous: false,
                });
            }
            seen += 1;
            if rows.len() == 32 {
                break;
            }
        }
    } else {
        for (offer, filled) in offers {
            let definition = &offer.definition;
            let Some(reference) =
                index.definition_ref(definition.family(), definition.key(), definition.revision())
            else {
                continue;
            };
            rows.push(MarketRow {
                offer: *offer.id.as_bytes(),
                definition: reference.get(),
                side: if offer.side == crate::durability::market::Side::Buy {
                    1
                } else {
                    2
                },
                amount: filled.unwrap_or(offer.remaining),
                price: offer.piece_price,
                state: match offer.state {
                    crate::durability::market::OfferState::Open => 1,
                    crate::durability::market::OfferState::Ended(_) => 2,
                    crate::durability::market::OfferState::CreditHeld(_) => 5,
                },
                anonymous: offer.anonymous,
            });
        }
    }
    wire::encode_reply(&MarketReply {
        action: request.action,
        code: 1,
        offer: [0; 16],
        rows,
    })
    .ok()
}
