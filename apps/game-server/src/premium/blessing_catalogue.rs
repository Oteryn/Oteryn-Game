//! PREM-5 Premium blessing offers from the BLESSINGS-CONTENT-1 source (#2237): the ruleset
//! price evidence (`rulesets/character/blessings/reference-evidence.json`) and the staged
//! blessing Service evidence (`content/services/blessings`, staged under
//! `docs/reference/content-prep/blessings-content-1/target-files/` until CP applies it to the
//! World Bundle). Both are embedded unchanged and read once.
//!
//! Only `PROVEN` and `DERIVED` facts are admitted. A `CONFLICT` or `UNKNOWN` fact stays held:
//! - the `regular` price family is `CONFLICT` (its above-120 segment), so The Wisdom of Solitude
//!   has no price and is never offered;
//! - `promotion_price_effect` is `UNKNOWN`, so no offer admits a promoted buyer
//!   ([`BlessingNpcOffer::promoted_price_admitted`] stays `false`).
//!
//! An offer also needs the NPC catalogue: the speaking NPC's native key must be the source NPC
//! (`oteryn:npc.` and its lowercase name) and that NPC must declare the blessing's Service under
//! the source key. No such declaration exists in the production content yet, so a production
//! catalogue answers no offer until CP applies the Service source (fail closed).

use crate::content::{NpcDataCatalogue, ProjectV2Declaration, ProjectV2Family};
use crate::gameplay_transport::npc_promotion::{
    BlessingNpcOffer, BlessingPriceBand, PremiumBlessingService, PremiumNpcServiceCatalogue,
    PromotionNpcOffer,
};
use serde_json::Value;
use std::sync::Arc;

const PRICE_EVIDENCE: &str =
    include_str!("../../../../rulesets/character/blessings/reference-evidence.json");
const SERVICE_EVIDENCE: &str = include_str!(
    "../../../../docs/reference/content-prep/blessings-content-1/target-files/content/services/blessings/evidence-00000-00009.json"
);
const PRICES_KEY: &str = "blessings.prices";
const SERVICE_PREFIX: &str = "oteryn:service.blessing.";
const STORAGE_PREFIX: &str = "oteryn:blessing.";
const NPC_PREFIX: &str = "oteryn:npc.";

/// One Premium blessing as the source admits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlessingSource {
    pub(crate) service: PremiumBlessingService,
    /// The Service key the source NPC declares (`oteryn:service.blessing.<name>`).
    pub(crate) service_key: String,
    /// The durable held-blessing key: the Service name under `oteryn:blessing.`, `_` written as
    /// `-` (the 0016 blessing-key alphabet has no `_`).
    pub(crate) storage_key: String,
    /// The native keys of the NPCs that sell it.
    pub(crate) npc_keys: Vec<String>,
    /// Empty when the price family is held: the blessing is never offered.
    pub(crate) prices: Vec<BlessingPriceBand>,
}

/// The three Premium blessings of the embedded source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlessingSources {
    sources: Vec<BlessingSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlessingSourceError(&'static str);

impl std::fmt::Display for BlessingSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

const fn service_name(service: PremiumBlessingService) -> &'static str {
    match service {
        PremiumBlessingService::WisdomOfSolitude => "the_wisdom_of_solitude",
        PremiumBlessingService::BloodOfTheMountain => "blood_of_the_mountain",
        PremiumBlessingService::HeartOfTheMountain => "heart_of_the_mountain",
    }
}

const SERVICES: [PremiumBlessingService; 3] = [
    PremiumBlessingService::WisdomOfSolitude,
    PremiumBlessingService::BloodOfTheMountain,
    PremiumBlessingService::HeartOfTheMountain,
];

fn records(document: &Value) -> Result<&Vec<Value>, BlessingSourceError> {
    if document.get("schema").and_then(Value::as_str) != Some("OTERYN_CONTENT_PREP_EVIDENCE/v1")
        || document.get("task").and_then(Value::as_str) != Some("BLESSINGS-CONTENT-1")
    {
        return Err(BlessingSourceError("unexpected blessing evidence schema"));
    }
    document
        .get("records")
        .and_then(Value::as_array)
        .ok_or(BlessingSourceError("blessing evidence without records"))
}

fn record<'a>(records: &'a [Value], key: &str) -> Result<&'a Value, BlessingSourceError> {
    let mut found = records
        .iter()
        .filter(|record| record.get("key").and_then(Value::as_str) == Some(key));
    match (found.next(), found.next()) {
        (Some(record), None) => Ok(record),
        _ => Err(BlessingSourceError(
            "blessing evidence record missing or repeated",
        )),
    }
}

/// The value of an admitted (`PROVEN` or `DERIVED`) fact; `None` for a held one.
fn admitted<'a>(record: &'a Value, fact: &str) -> Result<Option<&'a Value>, BlessingSourceError> {
    let fact = record
        .get("facts")
        .and_then(|facts| facts.get(fact))
        .ok_or(BlessingSourceError("blessing evidence fact missing"))?;
    match fact.get("classification").and_then(Value::as_str) {
        Some("PROVEN" | "DERIVED") => fact
            .get("value")
            .filter(|value| !value.is_null())
            .map(Some)
            .ok_or(BlessingSourceError(
                "admitted blessing fact without a value",
            )),
        Some("CONFLICT" | "UNKNOWN") => Ok(None),
        _ => Err(BlessingSourceError("unclassified blessing evidence fact")),
    }
}

fn integer<T: TryFrom<u64>>(text: &str) -> Option<T> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) || text.starts_with('0') {
        return None;
    }
    T::try_from(text.parse::<u64>().ok()?).ok()
}

/// The `gold` formula of a band: `N`, `A*(L-B)` or `C+A*(L-B)` as `(C, A, B)`. Anything else
/// is not admitted.
fn formula(text: &str) -> Option<(u64, u64, u32)> {
    let Some((head, tail)) = text.split_once("*(L-") else {
        return Some((integer(text)?, 0, 0));
    };
    let (constant, slope) = match head.split_once('+') {
        Some((constant, slope)) => (integer(constant)?, slope),
        None => (0, head),
    };
    let base = tail.strip_suffix(')')?;
    Some((constant, integer(slope)?, integer(base)?))
}

fn level(band: &serde_json::Map<String, Value>, key: &str) -> Option<Option<u32>> {
    match band.get(key) {
        None => Some(None),
        Some(value) => Some(Some(u32::try_from(value.as_u64()?).ok()?)),
    }
}

/// One price family's bands, contiguous from level 1 to the last level; `None` if any band is
/// not of an admitted shape.
fn bands(value: &Value) -> Option<Vec<BlessingPriceBand>> {
    let mut bands = Vec::new();
    let mut next = 1_u32;
    let mut open = false;
    for band in value.as_array()? {
        let band = band.as_object()?;
        if open
            || band.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "gold"
                        | "level_min"
                        | "level_max"
                        | "level_min_exclusive"
                        | "level_max_exclusive"
                )
            })
        {
            return None;
        }
        let minimum = match (
            level(band, "level_min")?,
            level(band, "level_min_exclusive")?,
        ) {
            (None, None) => 1,
            (Some(minimum), None) => minimum,
            (None, Some(exclusive)) => exclusive.checked_add(1)?,
            (Some(_), Some(_)) => return None,
        };
        let maximum = match (
            level(band, "level_max")?,
            level(band, "level_max_exclusive")?,
        ) {
            (None, None) => {
                open = true;
                u32::MAX
            }
            (Some(maximum), None) => maximum,
            (None, Some(exclusive)) => exclusive.checked_sub(1)?,
            (Some(_), Some(_)) => return None,
        };
        let (constant, slope, base) = formula(band.get("gold")?.as_str()?)?;
        if minimum != next || maximum < minimum || (slope != 0 && base > minimum) {
            return None;
        }
        bands.push(BlessingPriceBand {
            minimum_level: minimum,
            maximum_level: maximum,
            gold_units: constant,
            gold_per_level: slope,
            base_level: base,
        });
        next = maximum.wrapping_add(1);
    }
    (open && !bands.is_empty()).then_some(bands)
}

impl BlessingSources {
    /// The embedded #2237 source.
    pub(crate) fn embedded() -> Result<Self, BlessingSourceError> {
        Self::parse(PRICE_EVIDENCE, SERVICE_EVIDENCE)
    }

    fn parse(prices: &str, services: &str) -> Result<Self, BlessingSourceError> {
        let prices: Value = serde_json::from_str(prices)
            .map_err(|_| BlessingSourceError("blessing price evidence is not JSON"))?;
        let services: Value = serde_json::from_str(services)
            .map_err(|_| BlessingSourceError("blessing Service evidence is not JSON"))?;
        let price_record = record(records(&prices)?, PRICES_KEY)?;
        let services = records(&services)?;
        let mut sources = Vec::with_capacity(SERVICES.len());
        for service in SERVICES {
            let name = service_name(service);
            let service_key = format!("{SERVICE_PREFIX}{name}");
            let record = record(services, &service_key)?;
            // Only the Premium NPC purchase is in scope here.
            if admitted(record, "premium_to_buy_from_named_npc")?.and_then(Value::as_bool)
                != Some(true)
            {
                return Err(BlessingSourceError(
                    "a Premium blessing is not Premium-only",
                ));
            }
            let npc_keys = admitted(record, "npcs")?
                .and_then(Value::as_array)
                .map(|npcs| {
                    npcs.iter()
                        .map(|npc| {
                            npc.as_str()
                                .filter(|name| {
                                    !name.is_empty()
                                        && name.bytes().all(|byte| byte.is_ascii_alphabetic())
                                })
                                .map(|name| format!("{NPC_PREFIX}{}", name.to_ascii_lowercase()))
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .unwrap_or_default()
                .ok_or(BlessingSourceError(
                    "a blessing NPC name is not a plain name",
                ))?;
            let family = admitted(record, "price_family")?.and_then(Value::as_str);
            let prices = match family {
                Some(family @ ("regular" | "enhanced")) => admitted(price_record, family)?
                    .map(|value| {
                        bands(value).ok_or(BlessingSourceError("unadmitted blessing price band"))
                    })
                    .transpose()?
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            sources.push(BlessingSource {
                service,
                storage_key: format!("{STORAGE_PREFIX}{}", name.replace('_', "-")),
                service_key,
                npc_keys,
                prices,
            });
        }
        // No price is admitted for a promoted buyer while the effect is not known.
        if admitted(price_record, "promotion_price_effect")?.is_some() {
            return Err(BlessingSourceError(
                "a promotion price effect needs its own admission",
            ));
        }
        Ok(Self { sources })
    }

    pub(crate) fn get(&self, service: PremiumBlessingService) -> Option<&BlessingSource> {
        self.sources.iter().find(|source| source.service == service)
    }
}

/// The source-backed [`PremiumNpcServiceCatalogue`] of one NPC catalogue. It offers no
/// promotion: that has its own source.
pub(crate) struct SourceBlessingCatalogue {
    npcs: Arc<NpcDataCatalogue>,
    content_revision: String,
    sources: BlessingSources,
}

impl SourceBlessingCatalogue {
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "PREM-5: the node installs it once CP applies the blessing Service source to \
                      the World Bundle NPCs"
        )
    )]
    pub(crate) fn new(
        npcs: Arc<NpcDataCatalogue>,
        content_revision: String,
        sources: BlessingSources,
    ) -> Self {
        Self {
            npcs,
            content_revision,
            sources,
        }
    }
}

impl PremiumNpcServiceCatalogue for SourceBlessingCatalogue {
    fn content_revision(&self) -> &str {
        &self.content_revision
    }
    fn npcs(&self) -> &NpcDataCatalogue {
        &self.npcs
    }
    fn promotion(&self, _native_npc_key: &str) -> Option<PromotionNpcOffer> {
        None
    }
    fn blessing(
        &self,
        native_npc_key: &str,
        service: PremiumBlessingService,
    ) -> Option<BlessingNpcOffer> {
        let source = self.sources.get(service)?;
        if source.prices.is_empty() || !source.npc_keys.iter().any(|key| key == native_npc_key) {
            return None;
        }
        let (npc, declaration) = self.npcs.records().find(|(reference, _)| {
            reference.family == ProjectV2Family::Npc && reference.key == native_npc_key
        })?;
        let ProjectV2Declaration::Npc { services, .. } = declaration else {
            return None;
        };
        let service_ref = services.iter().find(|reference| {
            reference.family == ProjectV2Family::Service && reference.key == source.service_key
        })?;
        Some(BlessingNpcOffer {
            source: PromotionNpcOffer {
                npc: npc.clone(),
                service: service_ref.clone(),
            },
            blessing: service,
            storage_key: source.storage_key.clone(),
            prices: source.prices.clone(),
            promoted_price_admitted: false,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;

    fn price(source: &BlessingSource, level: u32) -> Option<u64> {
        source
            .prices
            .iter()
            .find(|band| (band.minimum_level..=band.maximum_level).contains(&level))
            .map(|band| band.gold_units + band.gold_per_level * u64::from(level - band.base_level))
    }

    #[test]
    fn the_embedded_source_admits_only_the_proven_enhanced_prices() {
        let sources = BlessingSources::embedded().expect("the embedded blessing source");
        let wisdom = sources
            .get(PremiumBlessingService::WisdomOfSolitude)
            .expect("wisdom");
        assert_eq!(wisdom.npc_keys, ["oteryn:npc.eremo"]);
        assert!(wisdom.prices.is_empty(), "the regular family is CONFLICT");
        for (service, npc, storage) in [
            (
                PremiumBlessingService::BloodOfTheMountain,
                "oteryn:npc.kais",
                "oteryn:blessing.blood-of-the-mountain",
            ),
            (
                PremiumBlessingService::HeartOfTheMountain,
                "oteryn:npc.nomad",
                "oteryn:blessing.heart-of-the-mountain",
            ),
        ] {
            let source = sources.get(service).expect("an enhanced blessing");
            assert_eq!(source.npc_keys, [npc]);
            assert_eq!(source.storage_key, storage);
            // 2600 to 30, 260*(L-20) from 31 to 119, 26000+100*(L-120) from 120.
            for (level, gold) in [
                (1, 2_600),
                (30, 2_600),
                (31, 2_860),
                (50, 7_800),
                (119, 25_740),
                (120, 26_000),
                (200, 34_000),
                (u32::MAX, 26_000 + 100 * (u64::from(u32::MAX) - 120)),
            ] {
                assert_eq!(price(source, level), Some(gold), "{service:?} at {level}");
            }
        }
    }

    #[test]
    fn unadmitted_shapes_and_classes_are_held() {
        for text in [
            "",
            "02",
            "2000+",
            "200*(L-)",
            "200*(X-20)",
            "20000+75*(L-120",
            "-5",
        ] {
            assert_eq!(formula(text), None, "{text}");
        }
        assert_eq!(formula("26000+100*(L-120)"), Some((26_000, 100, 120)));
        assert_eq!(formula("260*(L-20)"), Some((0, 260, 20)));
        assert_eq!(formula("2600"), Some((2_600, 0, 0)));
        // A gap, an overlap, a closed last band and a slope below its base are not admitted.
        for value in [
            serde_json::json!([{"level_max": 30, "gold": "1"}, {"level_min": 32, "gold": "2"}]),
            serde_json::json!([{"level_max": 30, "gold": "1"}, {"level_min": 30, "gold": "2"}]),
            serde_json::json!([{"level_max": 30, "gold": "1"}]),
            serde_json::json!([{"level_min": 1, "gold": "5*(L-20)"}]),
            serde_json::json!([{"level_min": 1, "gold": "1", "discount": 1}]),
        ] {
            assert_eq!(bands(&value), None, "{value}");
        }
        let held = PRICE_EVIDENCE.replacen("\"PROVEN\"", "\"CONFLICT\"", 1);
        let enhanced_held = BlessingSources::parse(&held, SERVICE_EVIDENCE).expect("parsed");
        assert!(
            enhanced_held
                .get(PremiumBlessingService::BloodOfTheMountain)
                .expect("blood")
                .prices
                .is_empty()
        );
        let unclassified = PRICE_EVIDENCE.replacen("\"PROVEN\"", "\"LIKELY\"", 1);
        assert!(BlessingSources::parse(&unclassified, SERVICE_EVIDENCE).is_err());
    }

    /// The DEATH writer consumes every held blessing and reduces the experience loss by 8% each
    /// and 30% more when promoted (D58/D59). The source loss table derives the same values.
    #[test]
    fn the_source_loss_table_matches_the_death_calculation() {
        use crate::domain::death::death_experience_loss;
        let prices: Value = serde_json::from_str(PRICE_EVIDENCE).expect("json");
        let records = records(&prices).expect("records");
        // Level 100: `(150/100) × 50 × 9508` = 713100, a multiple of 100.
        let full = death_experience_loss(100, 0, false).expect("loss");
        assert_eq!(full, 713_100);
        for count in 0_u8..=7 {
            let record = record(records, &format!("blessings.loss.count{count}")).expect("row");
            let percent = |fact| {
                u128::from(
                    admitted(record, fact)
                        .expect("fact")
                        .and_then(Value::as_u64)
                        .expect("an admitted percent"),
                )
            };
            for (promoted, fact) in [
                (false, "xp_skill_base_reduction_percent"),
                (true, "promoted_reduction_percent"),
            ] {
                assert_eq!(
                    death_experience_loss(100, count, promoted).expect("loss") * 100,
                    full * (100 - percent(fact)),
                    "{count} blessings, promoted {promoted}"
                );
            }
        }
    }
}
