//! Explicit native quest predicates qualify retained source trade tuples without rewriting them.
use super::service::Record;
use crate::content::{
    ProjectV2CandidateValue, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family,
    ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

const SOURCE: &str = include_str!("../../../../../../content/quests/native-service-access.json");
// Existing canonical evidence, not a separately authored shop or price table.
const RETAINED: &str =
    include_str!("../../../../../../content/services/trade/trade-00000-00323.json");
const HELD_PREFIX: &str = "oteryn:source.npc.trade.held_tuple_";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessRule {
    pub npc: String,
    pub service: String,
    pub quest: String,
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
    source_callback: String,
    source_predicate: SourcePredicate,
    source_proof: SourceProof,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SourcePredicate {
    operator: String,
    storage: String,
    value: i64,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SourceProof {
    repository: String,
    revision: String,
    path: String,
    sha256: String,
    line: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalogue {
    schema: String,
    rules: Vec<AccessRule>,
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn key(value: &str, prefix: &str) -> bool {
    value.len() <= 160
        && value.strip_prefix(prefix).is_some_and(|tail| {
            !tail.is_empty()
                && tail.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'/' | b'-' | b'.' | b':')
                })
        })
}

fn parse(text: &str) -> Result<Catalogue, &'static str> {
    let catalogue: Catalogue =
        serde_json::from_str(text).map_err(|_| "native service access data")?;
    if catalogue.schema != "OTERYN_NATIVE_SERVICE_ACCESS/v1"
        || catalogue.rules.is_empty()
        || catalogue.rules.len() > 16
        || catalogue
            .rules
            .iter()
            .map(|rule| &rule.npc)
            .collect::<BTreeSet<_>>()
            .len()
            != catalogue.rules.len()
        || catalogue
            .rules
            .iter()
            .map(|rule| &rule.service)
            .collect::<BTreeSet<_>>()
            .len()
            != catalogue.rules.len()
        || catalogue.rules.iter().any(|rule| {
            !key(&rule.npc, "oteryn:npc.")
                || !key(&rule.service, "oteryn:service.")
                || !key(&rule.quest, "oteryn:quest.")
                || !key(&rule.track, "oteryn:quest-progress/")
                || rule.minimum < 0
                || rule.minimum > rule.maximum
                || rule.source_callback != "CALLBACK_ON_TRADE_REQUEST"
                || rule.source_predicate.operator != "=="
                || rule.source_predicate.storage.is_empty()
                || rule.source_predicate.storage.len() > 160
                || !rule.source_predicate.storage.is_ascii()
                || rule.source_proof.repository.is_empty()
                || rule.source_proof.repository.len() > 160
                || !rule.source_proof.repository.is_ascii()
                || !hex(&rule.source_proof.revision, 40)
                || !hex(&rule.source_proof.sha256, 64)
                || rule.source_proof.path.is_empty()
                || rule.source_proof.path.len() > 256
                || !rule.source_proof.path.is_ascii()
                || rule.source_proof.path.split('/').any(|part| part == "..")
                || rule.source_proof.line == 0
        })
    {
        return Err("native service access bounds");
    }
    Ok(catalogue)
}

fn catalogue() -> Result<&'static Catalogue, &'static str> {
    static CATALOGUE: OnceLock<Result<Catalogue, &'static str>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| parse(SOURCE))
        .as_ref()
        .map_err(|error| *error)
}

pub fn rule(npc: &str) -> Result<Option<&'static AccessRule>, &'static str> {
    Ok(catalogue()?.rules.iter().find(|rule| rule.npc == npc))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeldTuple {
    access_fact: AccessFact,
    admission_eligible: bool,
    disposition: String,
    npc: String,
    old_native_offer: ProjectV2ServiceOffer,
    reason: String,
    runtime_qualified: bool,
    service: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessFact {
    authority: String,
    native_predicate: serde_json::Value,
    runtime_qualified: bool,
    source_callback: String,
    source_predicate: SourcePredicate,
    source_proof: SourceProof,
}

fn retained_offers(
    declaration: &ProjectV2Declaration,
    rule: &AccessRule,
) -> Result<Vec<ProjectV2ServiceOffer>, &'static str> {
    let ProjectV2Declaration::Service {
        identity, fields, ..
    } = declaration
    else {
        return Err("native service access declaration");
    };
    if identity.key != rule.service || identity.revision != "definition-r1" {
        return Err("native service access identity");
    }
    let mut offers = BTreeSet::new();
    for field in fields
        .iter()
        .filter(|field| field.field_path.starts_with(HELD_PREFIX))
    {
        if offers.len() >= 512 || !hex(&field.field_path[HELD_PREFIX.len()..], 16) {
            return Err("native service retained tuple bound");
        }
        let ProjectV2CandidateValue::Text(text) = &field.value else {
            return Err("native service retained tuple type");
        };
        let tuple: HeldTuple =
            serde_json::from_str(text).map_err(|_| "native service retained tuple")?;
        let fact = &tuple.access_fact;
        let offer = &tuple.old_native_offer;
        if tuple.npc != rule.npc
            || tuple.service != rule.service
            || tuple.admission_eligible
            || tuple.runtime_qualified
            || tuple.disposition != "SOURCE_FACT_RETAINED_NOT_EXECUTABLE_OFFER"
            || tuple.reason != "SOURCE_TRADE_ACCESS_NATIVE_PREDICATE_UNAVAILABLE"
            || fact.authority != "OTS_HYPOTHESIS_ONLY"
            || !fact.native_predicate.is_null()
            || fact.runtime_qualified
            || fact.source_callback != rule.source_callback
            || fact.source_predicate != rule.source_predicate
            || fact.source_proof != rule.source_proof
            || offer.item.family != ProjectV2Family::Item
            || !key(&offer.item.key, "oteryn:item.")
            || offer.item.revision != "definition-r1"
            || offer.unit_price == 0
            || !(1..=100).contains(&offer.count.unwrap_or(1))
            || offer.currency.is_some()
            || offer.sub_type.is_some()
            || !offers.insert(offer.clone())
        {
            return Err("native service retained tuple does not match source rule");
        }
    }
    if offers.is_empty() {
        return Err("native service retained offers absent");
    }
    Ok(offers.into_iter().collect())
}

#[derive(Deserialize)]
struct RetainedDocument {
    records: Vec<RetainedRecord>,
}
#[derive(Deserialize)]
struct RetainedRecord {
    declaration: ProjectV2Declaration,
}

fn bound_offers() -> Result<&'static BTreeMap<String, Vec<ProjectV2ServiceOffer>>, &'static str> {
    static OFFERS: OnceLock<Result<BTreeMap<String, Vec<ProjectV2ServiceOffer>>, &'static str>> =
        OnceLock::new();
    OFFERS
        .get_or_init(|| {
            let source: RetainedDocument =
                serde_json::from_str(RETAINED).map_err(|_| "native service retained catalogue")?;
            let mut offers = BTreeMap::new();
            for rule in &catalogue()?.rules {
                let declaration = source
                    .records
                    .iter()
                    .find_map(|record| match &record.declaration {
                        ProjectV2Declaration::Service { identity, .. }
                            if identity.key == rule.service =>
                        {
                            Some(&record.declaration)
                        }
                        _ => None,
                    })
                    .ok_or("native service retained declaration absent")?;
                offers.insert(rule.npc.clone(), retained_offers(declaration, rule)?);
            }
            Ok(offers)
        })
        .as_ref()
        .map_err(|error| *error)
}

/// A mapped NPC never bypasses its predicate through a changed Item, price or count.
pub fn required(
    npc: &str,
    item: &ProjectV2DefinitionRef,
    direction: ProjectV2ServiceOfferDirection,
    unit_price: u64,
    count: u32,
) -> Result<Option<&'static AccessRule>, &'static str> {
    let Some(rule) = rule(npc)? else {
        return Ok(None);
    };
    if bound_offers()?.get(npc).is_some_and(|offers| {
        offers.iter().any(|offer| {
            &offer.item == item
                && offer.direction == direction
                && offer.unit_price == unit_price
                && offer.count.unwrap_or(1) == count
                && !offer.parity_pending
        })
    }) {
        Ok(Some(rule))
    } else {
        Err("unknown quest-gated native service offer")
    }
}

/// UI projection only. The durable trade gate also requires the quest completion receipt.
pub fn visible(npc: &str, tracks: &BTreeMap<String, i64>) -> bool {
    match rule(npc) {
        Ok(None) => true,
        Ok(Some(rule)) => tracks
            .get(&rule.track)
            .is_some_and(|value| (rule.minimum..=rule.maximum).contains(value)),
        Err(_) => false,
    }
}

pub fn book_digest(source_tree_digest: &str) -> String {
    Sha256::new()
        .chain_update(b"oteryn:native-service-access:v1\0")
        .chain_update(source_tree_digest.as_bytes())
        .chain_update(SOURCE.as_bytes())
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Clones only the mapped Service. The caller still runs its ordinary physical, count,
/// currency and arbitrage classifiers; all canonical held metadata remains intact.
pub(super) fn declarations(
    records: &[Record<'_>],
) -> Vec<(ProjectV2DefinitionRef, ProjectV2Declaration)> {
    let Ok(catalogue) = catalogue() else {
        return Vec::new();
    };
    let Ok(bound) = bound_offers() else {
        return Vec::new();
    };
    let mut projected = Vec::new();
    for rule in &catalogue.rules {
        let linked = records.iter().any(|(reference, declaration)| {
            reference.family == ProjectV2Family::Npc && reference.key == rule.npc
                && matches!(declaration, ProjectV2Declaration::Npc { services, .. }
                    if services.iter().any(|service| service.family == ProjectV2Family::Service && service.key == rule.service && service.revision == "definition-r1"))
        });
        if !linked {
            continue;
        }
        let Some((reference, declaration)) = records.iter().find(|(reference, _)| {
            reference.family == ProjectV2Family::Service
                && reference.key == rule.service
                && reference.revision == "definition-r1"
        }) else {
            continue;
        };
        let Ok(admitted) = retained_offers(declaration, rule) else {
            continue;
        };
        if bound.get(&rule.npc) != Some(&admitted) {
            continue;
        }
        let mut declaration = (**declaration).clone();
        let ProjectV2Declaration::Service { offers, .. } = &mut declaration else {
            continue;
        };
        // Existing executable tuples retain their own owner; don't replace a conflicting row.
        if offers.iter().any(|existing| {
            admitted.iter().any(|added| {
                existing.item == added.item
                    && existing.direction == added.direction
                    && existing != added
            })
        }) {
            continue;
        }
        offers.extend(admitted);
        offers.sort();
        offers.dedup();
        projected.push(((**reference).clone(), declaration));
    }
    projected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_service_access_uses_retained_tuples_and_exact_source_identity()
    -> Result<(), &'static str> {
        let npc = "oteryn:npc.rashid";
        let offers = bound_offers()?.get(npc).ok_or("Rashid offers")?;
        assert_eq!(offers.len(), 157);
        for offer in offers {
            assert!(
                required(
                    npc,
                    &offer.item,
                    offer.direction,
                    offer.unit_price,
                    offer.count.unwrap_or(1)
                )?
                .is_some()
            );
            assert!(
                required(
                    npc,
                    &offer.item,
                    offer.direction,
                    offer.unit_price + 1,
                    offer.count.unwrap_or(1)
                )
                .is_err()
            );
            assert!(required(npc, &offer.item, offer.direction, offer.unit_price, 101).is_err());
            let mut wrong = offer.item.clone();
            wrong.revision = "definition-r2".into();
            assert!(
                required(
                    npc,
                    &wrong,
                    offer.direction,
                    offer.unit_price,
                    offer.count.unwrap_or(1)
                )
                .is_err()
            );
        }
        assert!(!visible(npc, &BTreeMap::new()));
        for stage in [0, 12, 13, 14] {
            assert_eq!(
                visible(
                    npc,
                    &BTreeMap::from([(
                        "oteryn:quest-progress/chosen/travelling-trader/stage".into(),
                        stage
                    )])
                ),
                stage == 13
            );
        }
        assert!(visible("oteryn:npc.other", &BTreeMap::new()));
        let mut changed: serde_json::Value = serde_json::from_str(SOURCE).map_err(|_| "mapping")?;
        changed["rules"][0]["maximum"] = serde_json::json!(12);
        assert!(parse(&changed.to_string()).is_err());
        assert_ne!(book_digest("original"), book_digest("changed"));
        Ok(())
    }
}
