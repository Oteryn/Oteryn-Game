//! Successor corrections of existing NPC authoring, never runtime or identity allocation.
use super::*;
use oteryn_game_server::content::{
    ProjectV2AuthoringProfileData, ProjectV2Family, ProjectV2ServiceOffer,
    ProjectV2ServiceOfferDirection,
};

const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r7/native-repairs.json"
);
const DIGEST: &str = "89a85ec112b25e898566597f95e1f4bcfb21ea427671e8d1ece44d284d5b550e";
const REVISION: &str = "g4-npc-qualified-source-r11";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    project_revision: String,
    repairs: Vec<NpcNativeDeclarationRepair>,
    profile_repairs: Vec<ProfileRepair>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileRepair {
    before: ProjectV2AuthoringProfile,
    after: ProjectV2AuthoringProfile,
}

fn corrected_item(service: &str, old: &str, new: &str) -> bool {
    old == new
        || matches!(
            (service, old, new),
            (
                "oteryn:service.trade.aurelia" | "oteryn:service.trade.avriel",
                "oteryn:item.tibia.i3174",
                "oteryn:item.tibia.i21352"
            ) | (
                "oteryn:service.trade.zora",
                "oteryn:item.tibia.i2884",
                "oteryn:item.tibia.i2881"
            )
        )
}

fn offer_allowed(service: &str, old: &ProjectV2ServiceOffer, new: &ProjectV2ServiceOffer) -> bool {
    if !old.parity_pending {
        return old == new;
    }
    let mut allowed = old.clone();
    if !corrected_item(service, &old.item.key, &new.item.key)
        || old.item.family != new.item.family
        || old.item.revision != new.item.revision
        || new.parity_pending
    {
        return false;
    }
    allowed.item.clone_from(&new.item);
    allowed.unit_price = new.unit_price;
    allowed.parity_pending = false;
    allowed == *new
}

fn scope_allowed(before: &ProjectV2Declaration, after: &ProjectV2Declaration) -> bool {
    let mut allowed = before.clone();
    match (&mut allowed, after) {
        (
            ProjectV2Declaration::Service {
                identity,
                offers,
                fields,
                ..
            },
            ProjectV2Declaration::Service {
                offers: next,
                fields: next_fields,
                ..
            },
        ) => {
            let mut remaining = offers.iter().collect::<Vec<_>>();
            for new in next {
                if let Some(index) = remaining
                    .iter()
                    .position(|old| offer_allowed(&identity.key, old, new))
                {
                    remaining.remove(index);
                } else if identity.key != "oteryn:service.trade.giri"
                    || new.item.key != "oteryn:item.tibia.i32622"
                    || new.item.family != ProjectV2Family::Item
                    || new.item.revision != "definition-r1"
                    || new.direction != ProjectV2ServiceOfferDirection::BuyFromPlayer
                    || new.unit_price != 60_000
                    || new.currency.is_some()
                    || new.count.is_some()
                    || new.sub_type.is_some()
                    || new.parity_pending
                    || offers.iter().any(|old| old.item == new.item)
                {
                    return false;
                }
            }
            if remaining.iter().any(|old| !old.parity_pending)
                || (!remaining.is_empty()
                    && !matches!(
                        identity.key.as_str(),
                        "oteryn:service.trade.elgar" | "oteryn:service.trade.lily"
                    ))
                || !npc_source_fields_allowed(fields, next_fields)
            {
                return false;
            }
            offers.clone_from(next);
            fields.clone_from(next_fields);
        }
        (
            ProjectV2Declaration::Npc {
                identity,
                presentation,
                fields,
                ..
            },
            ProjectV2Declaration::Npc {
                presentation: next,
                fields: next_fields,
                ..
            },
        ) => {
            let key = match identity.key.as_str() {
                "oteryn:npc.hagor" => "oteryn:presentation.npc.hagor",
                "oteryn:npc.a_sleeping_dragon" => "oteryn:presentation.npc.a_sleeping_dragon",
                _ => return false,
            };
            if presentation.is_some()
                || next.as_ref().is_none_or(|value| {
                    value.family != ProjectV2Family::Presentation
                        || value.key != key
                        || value.revision != "definition-r1"
                })
                || !npc_source_fields_allowed(fields, next_fields)
            {
                return false;
            }
            presentation.clone_from(next);
            fields.clone_from(next_fields);
        }
        _ => return false,
    }
    allowed == *after
}

fn profile_allowed(repair: &ProfileRepair) -> bool {
    if repair.before.target != repair.after.target
        || repair.after.target.family != ProjectV2Family::Presentation
        || repair.after.target.revision != "definition-r1"
    {
        return false;
    }
    let expected = match repair.after.target.key.as_str() {
        "oteryn:presentation.npc.a_sleeping_dragon" => serde_json::json!({
            "kind":"Presentation", "profile":{"asset_binding":"canary.appearance:object/11402", "light_level":0}
        }),
        "oteryn:presentation.npc.hagor" => serde_json::json!({
            "kind":"Presentation", "profile":{"asset_binding":"canary.appearance:outfit/129", "light_level":0,
                "palette_bindings":[
                    {"slot":"Head","asset_binding":"canary.appearance:palette/19"},
                    {"slot":"Body","asset_binding":"canary.appearance:palette/58"},
                    {"slot":"Legs","asset_binding":"canary.appearance:palette/105"},
                    {"slot":"Feet","asset_binding":"canary.appearance:palette/94"}]}
        }),
        _ => return false,
    };
    serde_json::from_value::<ProjectV2AuthoringProfileData>(expected)
        .is_ok_and(|data| data == repair.after.data)
}

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<usize, Box<dyn std::error::Error>> {
    if hex_sha256(PACKET) != DIGEST {
        return Err("qualified NPC correction packet digest drifted".into());
    }
    let packet: Packet = serde_json::from_slice(PACKET)?;
    if packet.schema != "OTERYN_NPC_QUALIFIED_REPAIRS/v1"
        || packet.project_revision != REVISION
        || draft.core.project_revision != NPC_R5_PROJECT_REVISION
        || packet.repairs.len() != 43
        || packet.profile_repairs.len() != 2
    {
        return Err("qualified NPC correction scope/revision drifted".into());
    }
    let mut replacements = Vec::new();
    let mut previous = None;
    for repair in packet.repairs {
        let (kind, identity) =
            npc_repair_identity(&repair.before).ok_or("invalid NPC correction kind")?;
        let key = (kind, identity.key.clone(), identity.revision.clone());
        if identity != &repair.identity
            || npc_repair_identity(&repair.after) != Some((kind, identity))
            || previous.as_ref().is_some_and(|prior| prior >= &key)
            || !scope_allowed(&repair.before, &repair.after)
        {
            return Err("qualified NPC correction identity/order/field drifted".into());
        }
        previous = Some(key);
        let index = draft
            .state
            .declarations
            .iter()
            .position(|value| value == &repair.before)
            .ok_or("qualified NPC original declaration drifted")?;
        replacements.push((index, repair.after));
    }
    let mut profiles = Vec::new();
    let mut profile_keys = BTreeSet::new();
    for repair in packet.profile_repairs {
        if !profile_keys.insert(repair.before.target.key.clone()) || !profile_allowed(&repair) {
            return Err("qualified NPC presentation scope/duplicate drifted".into());
        }
        let index = draft
            .state
            .authoring_profiles
            .iter()
            .position(|value| value == &repair.before)
            .ok_or("qualified NPC original presentation drifted")?;
        profiles.push((index, repair.after));
    }
    // Preflight the complete packet before applying any change.
    let count = replacements.len();
    for (index, value) in replacements {
        draft.state.declarations[index] = value;
    }
    for (index, value) in profiles {
        draft.state.authoring_profiles[index] = value;
    }
    draft.core.project_revision = REVISION.to_owned();
    Ok(count)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn qualified_packet_only_changes_pending_prices_and_two_existing_presentations() {
        let packet: Packet = serde_json::from_slice(PACKET).expect("qualified packet");
        assert!(
            packet
                .repairs
                .iter()
                .all(|r| scope_allowed(&r.before, &r.after))
        );
        assert!(packet.profile_repairs.iter().all(profile_allowed));
    }

    #[test]
    fn price_resolution_cannot_change_currency_direction_quantity_or_identity_revision() {
        let packet: Packet = serde_json::from_slice(PACKET).expect("qualified packet");
        let (service, offer) = packet
            .repairs
            .iter()
            .find_map(|repair| match &repair.before {
                ProjectV2Declaration::Service {
                    identity, offers, ..
                } => offers
                    .iter()
                    .find(|o| o.parity_pending)
                    .map(|o| (identity.key.as_str(), o)),
                _ => None,
            })
            .expect("pending original");
        let mut candidate = offer.clone();
        candidate.parity_pending = false;
        assert!(offer_allowed(service, offer, &candidate));
        candidate.count = Some(5);
        assert!(!offer_allowed(service, offer, &candidate));
        candidate = offer.clone();
        candidate.parity_pending = false;
        candidate.item.revision = "invented-r1".into();
        assert!(!offer_allowed(service, offer, &candidate));
        candidate = offer.clone();
        candidate.parity_pending = false;
        candidate.currency = Some(candidate.item.clone());
        assert!(!offer_allowed(service, offer, &candidate));
        candidate = offer.clone();
        candidate.parity_pending = false;
        candidate.direction = match offer.direction {
            ProjectV2ServiceOfferDirection::BuyFromPlayer => {
                ProjectV2ServiceOfferDirection::SellToPlayer
            }
            ProjectV2ServiceOfferDirection::SellToPlayer => {
                ProjectV2ServiceOfferDirection::BuyFromPlayer
            }
        };
        assert!(!offer_allowed(service, offer, &candidate));
    }
}
