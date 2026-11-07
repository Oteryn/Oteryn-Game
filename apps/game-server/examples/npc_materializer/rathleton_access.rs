//! Forward qualification after r13's historical hold: the native trade writer now
//! proves the promoted Commoner rank before Alaistar can buy this source product.
//! The original removal packet stays immutable; unrelated shop offers stay unchanged.
use super::*;
use oteryn_game_server::content::{ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection};

const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r12/native-repairs.json"
);
const DIGEST: &str = "46845cf6d978c622a8bd4af7c2b0dc01242742bd2eaea9f3f8820d03c19715b4";
const SERVICE: &str = "oteryn:service.trade.alaistar";
const ITEM: &str = "oteryn:item.tibia.i21182";
const SUFFIX: &str = "-wcontrib-rank-access";

fn source_offer() -> Result<ProjectV2ServiceOffer, Box<dyn std::error::Error>> {
    if hex_sha256(PACKET) != DIGEST {
        return Err("Rathleton source offer packet digest drift".into());
    }
    let packet: serde_json::Value = serde_json::from_slice(PACKET)?;
    let service = packet["repairs"]
        .as_array()
        .ok_or("repair array")?
        .iter()
        .find(|row| row["identity"]["key"] == SERVICE)
        .ok_or("Alaistar source repair")?;
    let offer = service["before"]["offers"]
        .as_array()
        .ok_or("source offers")?
        .iter()
        .find(|row| row["item"]["key"] == ITEM)
        .ok_or("source glob of glooth offer")?;
    let offer: ProjectV2ServiceOffer = serde_json::from_value(offer.clone())?;
    if offer.item.family != ProjectV2Family::Item
        || offer.item.revision != "definition-r1"
        || offer.direction != ProjectV2ServiceOfferDirection::BuyFromPlayer
        || offer.unit_price != 125
        || offer.count.is_some()
        || offer.currency.is_some()
        || offer.sub_type.is_some()
        || offer.parity_pending
    {
        return Err("Rathleton source offer changed shape".into());
    }
    Ok(offer)
}

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<bool, Box<dyn std::error::Error>> {
    let offer = source_offer()?;
    let indices: Vec<_> = draft
        .state
        .declarations
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            matches!(row, ProjectV2Declaration::Service { identity, .. } if identity.key == SERVICE)
                .then_some(index)
        })
        .collect();
    let [index] = indices.as_slice() else {
        return Err("expected one Rathleton source trade service".into());
    };
    let ProjectV2Declaration::Service { offers, .. } = &mut draft.state.declarations[*index] else {
        return Err("Rathleton source is not a trade service".into());
    };
    if let Some(existing) = offers
        .iter()
        .find(|row| row.item == offer.item && row.direction == offer.direction)
    {
        return if existing == &offer {
            Ok(false)
        } else {
            Err("Rathleton source offer conflicts".into())
        };
    }
    offers.push(offer);
    offers.sort();
    if !draft.core.project_revision.ends_with(SUFFIX) {
        draft.core.project_revision.push_str(SUFFIX);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wcontrib_rathleton_restores_exact_source_commoner_product_price()
    -> Result<(), Box<dyn std::error::Error>> {
        let offer = source_offer()?;
        assert_eq!(offer.item.key, ITEM);
        assert_eq!(offer.unit_price, 125);
        assert_eq!(
            offer.direction,
            ProjectV2ServiceOfferDirection::BuyFromPlayer
        );
        Ok(())
    }
}
