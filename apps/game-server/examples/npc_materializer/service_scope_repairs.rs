//! Retain qualified prices while excluding offers whose eligibility/variant is unresolved.
use super::*;
use oteryn_game_server::content::ProjectV2ServiceOfferDirection;

const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r12/native-repairs.json"
);
const DIGEST: &str = "46845cf6d978c622a8bd4af7c2b0dc01242742bd2eaea9f3f8820d03c19715b4";
const FROM: &str = "g4-npc-d10-fandom-variants-r12";
const TO: &str = "g4-npc-service-scope-r13";
const SCOPE: &[(&str, &[&str])] = &[
    (
        "oteryn:service.trade.alaistar",
        &[
            "oteryn:item.tibia.i21103",
            "oteryn:item.tibia.i21182",
            "oteryn:item.tibia.i21193",
            "oteryn:item.tibia.i21194",
            "oteryn:item.tibia.i21195",
            "oteryn:item.tibia.i21196",
            "oteryn:item.tibia.i21197",
            "oteryn:item.tibia.i21198",
            "oteryn:item.tibia.i21199",
            "oteryn:item.tibia.i21200",
            "oteryn:item.tibia.i21201",
            "oteryn:item.tibia.i21202",
            "oteryn:item.tibia.i21204",
            "oteryn:item.tibia.i21747",
            "oteryn:item.tibia.i21800",
            "oteryn:item.tibia.i21801",
            "oteryn:item.tibia.i23373",
            "oteryn:item.tibia.i23374",
            "oteryn:item.tibia.i23375",
            "oteryn:item.tibia.i236",
            "oteryn:item.tibia.i237",
            "oteryn:item.tibia.i238",
            "oteryn:item.tibia.i239",
            "oteryn:item.tibia.i266",
            "oteryn:item.tibia.i268",
            "oteryn:item.tibia.i283",
            "oteryn:item.tibia.i284",
            "oteryn:item.tibia.i285",
            "oteryn:item.tibia.i2874",
            "oteryn:item.tibia.i7642",
            "oteryn:item.tibia.i7643",
        ],
    ),
    ("oteryn:service.trade.lyonel", &["oteryn:item.tibia.i5552"]),
    ("oteryn:service.trade.tanaro", &["oteryn:item.tibia.i2877"]),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    from_project_revision: String,
    project_revision: String,
    repairs: Vec<NpcNativeDeclarationRepair>,
}

fn scope_allowed(repair: &NpcNativeDeclarationRepair) -> bool {
    let ProjectV2Declaration::Service {
        identity, offers, ..
    } = &repair.before
    else {
        return false;
    };
    let Some((_, keys)) = SCOPE.iter().find(|(key, _)| *key == identity.key) else {
        return false;
    };
    if identity != &repair.identity || identity.revision != "definition-r1" {
        return false;
    }
    if keys
        .iter()
        .any(|key| offers.iter().filter(|o| o.item.key == *key).count() != 1)
        || offers.iter().any(|o| {
            identity.key != "oteryn:service.trade.alaistar"
                && keys.contains(&o.item.key.as_str())
                && o.direction != ProjectV2ServiceOfferDirection::SellToPlayer
        })
    {
        return false;
    }
    let mut permitted = repair.before.clone();
    let ProjectV2Declaration::Service { offers, .. } = &mut permitted else {
        return false;
    };
    offers.retain(|offer| !keys.contains(&offer.item.key.as_str()));
    permitted == repair.after
}

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<usize, Box<dyn std::error::Error>> {
    if hex_sha256(PACKET) != DIGEST {
        return Err("NPC service-scope packet digest drifted".into());
    }
    let packet: Packet = serde_json::from_slice(PACKET)?;
    if packet.schema != "OTERYN_NPC_SERVICE_SCOPE_REPAIRS/v1"
        || packet.from_project_revision != FROM
        || packet.project_revision != TO
        || draft.core.project_revision != FROM
        || packet.repairs.len() != SCOPE.len()
    {
        return Err("NPC service-scope schema/revision/count drifted".into());
    }
    let mut replacements = Vec::new();
    for (slot, repair) in packet.repairs.into_iter().enumerate() {
        if repair.identity.key != SCOPE[slot].0 || !scope_allowed(&repair) {
            return Err("NPC service-scope repair changes an unqualified field".into());
        }
        let indices = draft.state.declarations.iter().enumerate()
            .filter_map(|(i, value)| {
                matches!(value, ProjectV2Declaration::Service { identity, .. } if identity == &repair.identity)
                    .then_some(i)
            }).collect::<Vec<_>>();
        if indices.len() != 1 || draft.state.declarations[indices[0]] != repair.before {
            return Err("NPC service-scope exact before fence drifted".into());
        }
        replacements.push((indices[0], repair.after));
    }
    // Validate every replacement before changing any declaration.
    for (index, replacement) in replacements {
        draft.state.declarations[index] = replacement;
    }
    draft.core.project_revision = TO.to_owned();
    Ok(33)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ProjectV2Draft {
        let packet: Packet = serde_json::from_slice(PACKET).expect("qualified packet");
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: FROM.to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-09-27".to_owned(),
                records: vec![],
                imports: vec![],
                metadata: vec![],
            },
            state: ProjectV2State {
                declarations: packet.repairs.into_iter().map(|r| r.before).collect(),
                ..ProjectV2State::default()
            },
        }
    }

    #[test]
    fn holds_only_thirty_three_offers_and_preserves_price_evidence_and_other_records() {
        let mut draft = fixture();
        let mut unrelated = draft.state.declarations[0].clone();
        if let ProjectV2Declaration::Service { identity, .. } = &mut unrelated {
            identity.key = "oteryn:service.trade.unrelated".to_owned();
        }
        draft.state.declarations.push(unrelated.clone());
        let before = draft.clone();
        assert_eq!(apply(&mut draft).expect("qualified repair"), 33);
        let packet: Packet = serde_json::from_slice(PACKET).expect("qualified packet");
        for (index, repair) in packet.repairs.iter().enumerate() {
            assert_eq!(draft.state.declarations[index], repair.after);
        }
        assert_eq!(draft.state.declarations[3], unrelated);
        assert_eq!(
            draft.state.declarations.len(),
            before.state.declarations.len()
        );
        assert_eq!(draft.state.sources, before.state.sources);
        assert_eq!(draft.state.placements, before.state.placements);
        assert_eq!(draft.core.records, before.core.records);
        assert_eq!(draft.core.project_revision, TO);
    }

    #[test]
    fn late_before_fence_and_revision_drift_reject_atomically() {
        let mut draft = fixture();
        if let ProjectV2Declaration::Service { offers, .. } = &mut draft.state.declarations[2] {
            offers[0].unit_price += 1;
        }
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
        let mut draft = fixture();
        draft.core.project_revision = "wrong".to_owned();
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
    }

    #[test]
    fn unrelated_offer_price_or_removal_is_outside_the_scope() {
        let mut packet: Packet = serde_json::from_slice(PACKET).expect("qualified packet");
        assert!(scope_allowed(&packet.repairs[1]));
        if let ProjectV2Declaration::Service { offers, .. } = &mut packet.repairs[1].after {
            offers[0].unit_price += 1;
        }
        assert!(!scope_allowed(&packet.repairs[1]));
        let mut packet: Packet = serde_json::from_slice(PACKET).expect("qualified packet");
        if let ProjectV2Declaration::Service { offers, .. } = &mut packet.repairs[1].after {
            offers.remove(0);
        }
        assert!(!scope_allowed(&packet.repairs[1]));
    }

    #[test]
    fn duplicate_existing_service_rejects_without_mutation() {
        let mut draft = fixture();
        draft
            .state
            .declarations
            .push(draft.state.declarations[2].clone());
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
    }
}
