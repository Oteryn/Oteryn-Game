//! Material projection from the same admitted item window as IMBUE_V1.
use super::capabilities::OfferedCapability;
use oteryn_gameplay_items::imbuement::{IMBUEMENT_TIER_RULES, imbuement_recipe};
use oteryn_protocol_oteryn::{imbuement, imbuement_details as wire, item_view::ItemHandle};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != wire::CAPABILITY_IMBUEMENT_DETAILS_V1)
        .collect()
}
pub(super) fn project(
    item: ItemHandle,
    window: &imbuement::ImbuementWindow,
) -> Option<wire::Reply> {
    let mut quotes = Vec::with_capacity(window.offers.len());
    for offer in &window.offers {
        let rules = IMBUEMENT_TIER_RULES.get(usize::from(offer.tier.checked_sub(1)?))?;
        let recipe = imbuement_recipe(offer.family, offer.tier);
        quotes.push(wire::Quote {
            family: offer.family,
            tier: offer.tier,
            gold: offer.gold,
            requires_premium: offer.requires_premium,
            recipe_available: recipe.is_some(),
            success_percent: rules.success_percent,
            materials: recipe.map_or_else(Vec::new, |recipe| {
                recipe
                    .materials
                    .iter()
                    .map(|(key, quantity)| wire::Material {
                        definition_key: (*key).into(),
                        quantity: *quantity,
                    })
                    .collect()
            }),
        });
    }
    quotes.sort_by_key(|quote| (quote.family, quote.tier));
    Some(wire::Reply { item, quotes })
}
pub(super) async fn query<A: super::connection::FreshAdmissionAuthority>(
    authority: &A,
    admitted: &super::connection::AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    item_view: Option<&super::item_view::SessionItemView>,
) -> super::imbuement::ImbuementReply {
    use super::imbuement::ImbuementReply as Outcome;
    if command.command_type != wire::COMMAND_TYPE_IMBUEMENT_DETAILS
        || !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_IMBUEMENT_DETAILS_V1)
    {
        return Outcome::Rejected;
    }
    let Ok(item) = wire::decode_query(command.payload) else {
        return Outcome::Rejected;
    };
    let Ok(payload) = imbuement::encode_imbuement_request(&imbuement::ImbuementRequest {
        item,
        action: imbuement::ImbuementAction::Query,
    }) else {
        return Outcome::Rejected;
    };
    match authority
        .imbuement_command(
            *admitted,
            command.command_id,
            imbuement::COMMAND_TYPE_IMBUEMENT,
            &payload,
            item_view,
        )
        .await
    {
        Outcome::Payload(bytes) => {
            let Ok(reply) = imbuement::decode_imbuement_reply(&bytes) else {
                return Outcome::Unavailable;
            };
            let Some(window) = reply.window else {
                return Outcome::Rejected;
            };
            let Some(reply) = project(item, &window) else {
                return Outcome::Unavailable;
            };
            wire::encode_reply(&reply).map_or(Outcome::Unavailable, Outcome::Payload)
        }
        other => other,
    }
}
macro_rules! authority_methods {
    () => {
        fn imbuement_details_available(&self) -> bool {
            false
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn imbuement_details_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
        }
    };
}
pub(super) use composed_methods;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imbuement_details_projection_uses_admitted_offer_and_canonical_recipe() -> Result<(), String>
    {
        let item = ItemHandle::new(7).ok_or("handle")?;
        let window = imbuement::ImbuementWindow {
            slot_count: 2,
            applied: vec![],
            offers: vec![imbuement::ImbuementOffer {
                family: 1,
                tier: 1,
                gold: 7500,
                requires_premium: false,
            }],
            clear_gold: 15000,
        };
        let reply = project(item, &window).ok_or("projection")?;
        assert_eq!(reply.quotes[0].success_percent, 100);
        assert_eq!(reply.quotes[0].gold, window.offers[0].gold);
        let recipe = imbuement_recipe(1, 1).ok_or("recipe")?;
        assert_eq!(
            reply.quotes[0].materials[0].definition_key,
            recipe.materials[0].0
        );
        assert_eq!(reply.quotes[0].materials[0].quantity, recipe.materials[0].1);
        let encoded = wire::encode_reply(&reply).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            wire::decode_reply(&encoded).map_err(|e| format!("{e:?}"))?,
            reply
        );
        Ok(())
    }
}
