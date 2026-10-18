//! PREM-5 Premium NPC services from the BLESSINGS-CONTENT-1 source (#2237): the ruleset price
//! evidence (`rulesets/character/blessings/reference-evidence.json`) and the staged blessing
//! Service evidence (`content/services/blessings`, staged under
//! `docs/reference/content-prep/blessings-content-1/target-files/` until CP applies it to the
//! World Bundle). Both are embedded and read once.
//!
//! `PROVEN` and `DERIVED` facts are admitted, and `PROVEN_BY_OWNER` facts that carry their dated
//! `owner_ruling` (the owner pricing ruling of 2026-10-18). A `CONFLICT` or `UNKNOWN` fact stays
//! held. The ruling prices the `regular` family (The Wisdom of Solitude) and the `enhanced` one
//! (Blood and Heart of the Mountain), and states that a promotion has no price effect: the
//! source refuses to load unless `promotion_price_effect` is admitted as `none`.
//!
//! An offer also needs the NPC catalogue. A blessing is offered when the speaking NPC's native
//! key is the source NPC (`oteryn:npc.` and its lowercase name) and that NPC declares the
//! blessing's Service under the source key; the promotion is offered when the NPC declares
//! `oteryn:service.promotion.<its key stem>`. Every declared Service must resolve to a Service
//! declaration. The node installs this catalogue over its pinned NPC catalogue
//! (`node::serve`); content without such declarations answers no offer (fail closed).

use crate::content::{
    NpcDataCatalogue, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family,
};
use crate::gameplay_transport::npc_promotion::{
    BlessingNpcOffer, BlessingPriceBand, PremiumBlessingService, PremiumNpcServiceCatalogue,
    PromotionNpcOffer,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

const PRICE_EVIDENCE: &str =
    include_str!("../../../../rulesets/character/blessings/reference-evidence.json");
const SERVICE_EVIDENCE: &str = include_str!(
    "../../../../docs/reference/content-prep/blessings-content-1/target-files/content/services/blessings/evidence-00000-00009.json"
);
const PRICES_KEY: &str = "blessings.prices";
const SERVICE_PREFIX: &str = "oteryn:service.blessing.";
const PROMOTION_PREFIX: &str = "oteryn:service.promotion.";
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

/// The value of an admitted fact; `None` for a held one. An owner-classified fact must carry
/// its dated `owner_ruling`.
fn admitted<'a>(record: &'a Value, fact: &str) -> Result<Option<&'a Value>, BlessingSourceError> {
    let fact = record
        .get("facts")
        .and_then(|facts| facts.get(fact))
        .ok_or(BlessingSourceError("blessing evidence fact missing"))?;
    let value = || {
        fact.get("value")
            .filter(|value| !value.is_null())
            .map(Some)
            .ok_or(BlessingSourceError(
                "admitted blessing fact without a value",
            ))
    };
    match fact.get("classification").and_then(Value::as_str) {
        Some("PROVEN" | "DERIVED") => value(),
        Some("PROVEN_BY_OWNER") => {
            let dated = fact
                .get("owner_ruling")
                .and_then(|ruling| ruling.get("date"))
                .and_then(Value::as_str)
                .is_some_and(|date| !date.is_empty());
            if !dated {
                return Err(BlessingSourceError(
                    "an owner-classified blessing fact without its owner ruling",
                ));
            }
            value()
        }
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
        // The owner ruling restates the regular above-120 segment on its own: it must be the
        // last `regular` band.
        if let Some(above) = admitted(price_record, "regular_above_120")? {
            let last = admitted(price_record, "regular")?
                .and_then(Value::as_array)
                .and_then(|bands| bands.last());
            if above.as_array().map(Vec::as_slice) != last.map(std::slice::from_ref) {
                return Err(BlessingSourceError(
                    "the regular above-120 price disagrees with its family",
                ));
            }
        }
        // Owner ruling 2026-10-18: a promotion has no price effect, so a promoted buyer pays the
        // same band. Any other or a held effect would need its own admission.
        if admitted(price_record, "promotion_price_effect")?.and_then(Value::as_str) != Some("none")
        {
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

/// The Premium Services one NPC declares, as the source admits them.
struct NpcPremiumServices {
    npc: ProjectV2DefinitionRef,
    promotion: Option<ProjectV2DefinitionRef>,
    blessings: Vec<(PremiumBlessingService, ProjectV2DefinitionRef)>,
}

/// The source-backed [`PremiumNpcServiceCatalogue`] of one NPC catalogue: the promotion and the
/// Premium blessings its NPCs declare, indexed once at construction.
pub(crate) struct SourcePremiumNpcCatalogue {
    npcs: Arc<NpcDataCatalogue>,
    content_revision: String,
    sources: BlessingSources,
    offers: BTreeMap<String, NpcPremiumServices>,
}

impl SourcePremiumNpcCatalogue {
    /// The node's catalogue: the embedded source over its pinned NPC catalogue, at the Content
    /// revision the node serves.
    pub(crate) fn embedded(
        npcs: Arc<NpcDataCatalogue>,
        content_revision: String,
    ) -> Result<Self, BlessingSourceError> {
        Ok(Self::new(
            npcs,
            content_revision,
            BlessingSources::embedded()?,
        ))
    }

    fn new(
        npcs: Arc<NpcDataCatalogue>,
        content_revision: String,
        sources: BlessingSources,
    ) -> Self {
        let service = |reference: &ProjectV2DefinitionRef| {
            reference.family == ProjectV2Family::Service
                && matches!(
                    npcs.declaration(reference),
                    Some(ProjectV2Declaration::Service { .. })
                )
        };
        let mut offers = BTreeMap::new();
        for (npc, declaration) in npcs.records() {
            let ProjectV2Declaration::Npc { services, .. } = declaration else {
                continue;
            };
            if npc.family != ProjectV2Family::Npc {
                continue;
            }
            let promotion_key = npc
                .key
                .strip_prefix(NPC_PREFIX)
                .map(|stem| format!("{PROMOTION_PREFIX}{stem}"));
            let promotion = services
                .iter()
                .find(|reference| {
                    Some(&reference.key) == promotion_key.as_ref() && service(reference)
                })
                .cloned();
            let blessings: Vec<_> = SERVICES
                .iter()
                .filter_map(|&blessing| {
                    let source = sources.get(blessing)?;
                    if source.prices.is_empty() || !source.npc_keys.contains(&npc.key) {
                        return None;
                    }
                    services
                        .iter()
                        .find(|reference| reference.key == source.service_key && service(reference))
                        .map(|reference| (blessing, reference.clone()))
                })
                .collect();
            if promotion.is_some() || !blessings.is_empty() {
                offers.insert(
                    npc.key.clone(),
                    NpcPremiumServices {
                        npc: npc.clone(),
                        promotion,
                        blessings,
                    },
                );
            }
        }
        Self {
            npcs,
            content_revision,
            sources,
            offers,
        }
    }

    /// How many NPCs offer the promotion and how many blessing offers there are.
    pub(crate) fn offer_counts(&self) -> (usize, usize) {
        self.offers
            .values()
            .fold((0, 0), |(promotions, blessings), npc| {
                (
                    promotions + usize::from(npc.promotion.is_some()),
                    blessings + npc.blessings.len(),
                )
            })
    }
}

impl PremiumNpcServiceCatalogue for SourcePremiumNpcCatalogue {
    fn content_revision(&self) -> &str {
        &self.content_revision
    }
    fn npcs(&self) -> &NpcDataCatalogue {
        &self.npcs
    }
    fn promotion(&self, native_npc_key: &str) -> Option<PromotionNpcOffer> {
        let npc = self.offers.get(native_npc_key)?;
        Some(PromotionNpcOffer {
            npc: npc.npc.clone(),
            service: npc.promotion.clone()?,
        })
    }
    fn blessing(
        &self,
        native_npc_key: &str,
        service: PremiumBlessingService,
    ) -> Option<BlessingNpcOffer> {
        let npc = self.offers.get(native_npc_key)?;
        let (_, service_ref) = npc
            .blessings
            .iter()
            .find(|(blessing, _)| *blessing == service)?;
        let source = self.sources.get(service)?;
        Some(BlessingNpcOffer {
            source: PromotionNpcOffer {
                npc: npc.npc.clone(),
                service: service_ref.clone(),
            },
            blessing: service,
            storage_key: source.storage_key.clone(),
            prices: source.prices.clone(),
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::content::{
        CanonicalProjectDocuments, ProjectDraft, ProjectV2Draft, ProjectV2Identity, ProjectV2State,
        npc_catalogue_preproduction_limits,
    };
    use crate::gameplay_transport::npc_promotion::{
        admitted_blessing_offer, admitted_promotion_offer,
    };

    fn price(prices: &[BlessingPriceBand], level: u32) -> Option<u64> {
        prices
            .iter()
            .find(|band| (band.minimum_level..=band.maximum_level).contains(&level))
            .map(|band| band.gold_units + band.gold_per_level * u64::from(level - band.base_level))
    }

    fn price_facts() -> Value {
        let prices: Value = serde_json::from_str(PRICE_EVIDENCE).expect("json");
        record(records(&prices).expect("records"), PRICES_KEY)
            .expect("prices")
            .clone()
    }

    /// The embedded price evidence with one price fact replaced.
    fn with_price_fact(fact: &str, value: Value) -> String {
        let mut prices: Value = serde_json::from_str(PRICE_EVIDENCE).expect("json");
        let record = prices["records"]
            .as_array_mut()
            .expect("records")
            .iter_mut()
            .find(|record| record["key"] == PRICES_KEY)
            .expect("prices");
        record["facts"][fact] = value;
        prices.to_string()
    }

    /// A native NPC catalogue of `npcs`, each with its declared Service keys; every key in
    /// `services` is declared as a Service.
    pub(crate) fn npc_catalogue(npcs: &[(&str, &[&str])], services: &[&str]) -> NpcDataCatalogue {
        let identity = |key: &str| ProjectV2Identity {
            key: key.into(),
            revision: "definition-r1".into(),
        };
        let reference = |key: &str| ProjectV2DefinitionRef {
            family: ProjectV2Family::Service,
            key: key.into(),
            revision: "definition-r1".into(),
        };
        let mut declarations: Vec<_> = npcs
            .iter()
            .map(|(npc, declared)| ProjectV2Declaration::Npc {
                identity: identity(npc),
                presentation: None,
                behavior: None,
                dialogue: None,
                services: declared.iter().map(|key| reference(key)).collect(),
                fields: vec![],
            })
            .collect();
        declarations.extend(services.iter().map(|key| ProjectV2Declaration::Service {
            bank: false,
            identity: identity(key),
            offers: vec![],
            recipes: vec![],
            routes: vec![],
            fields: vec![],
        }));
        let draft = ProjectV2Draft {
            core: ProjectDraft {
                project_revision: "premium-npc-r1".into(),
                package_key: "oteryn:content.world-project".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "license:project-owned-v1".into(),
                world_id: "0123456789ab70cd8ef0123456789abc".into(),
                coordinate_frame: "global-target-2026-09-27".into(),
                records: vec![],
                imports: vec![],
                metadata: vec![],
            },
            state: ProjectV2State {
                declarations,
                ..ProjectV2State::default()
            },
        };
        let limits = npc_catalogue_preproduction_limits().project;
        let project = CanonicalProjectDocuments::from_v2_draft(draft, limits)
            .expect("native documents")
            .into_snapshot(limits)
            .expect("snapshot")
            .parse(limits)
            .expect("native project");
        NpcDataCatalogue::from_project(project, limits).expect("native NPC catalogue")
    }

    /// The owner pricing ruling of 2026-10-18 at every band boundary.
    #[test]
    fn the_embedded_source_admits_the_owner_ruled_prices() {
        let sources = BlessingSources::embedded().expect("the embedded blessing source");
        let wisdom = sources
            .get(PremiumBlessingService::WisdomOfSolitude)
            .expect("wisdom");
        assert_eq!(wisdom.npc_keys, ["oteryn:npc.eremo"]);
        assert_eq!(wisdom.storage_key, "oteryn:blessing.the-wisdom-of-solitude");
        // Regular: 2000 to 30, 2000+200*(L-30) from 31 to 119, 20000+75*(L-120) from 120.
        for (level, gold) in [
            (1, 2_000),
            (30, 2_000),
            (31, 2_200),
            (50, 6_000),
            (119, 19_800),
            (120, 20_000),
            (121, 20_075),
            (200, 26_000),
        ] {
            assert_eq!(
                price(&wisdom.prices, level),
                Some(gold),
                "wisdom at {level}"
            );
        }
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
            // 2600 to 30, 2600+260*(L-30) from 31 to 119, 26000+100*(L-120) from 120.
            for (level, gold) in [
                (1, 2_600),
                (30, 2_600),
                (31, 2_860),
                (50, 7_800),
                (119, 25_740),
                (120, 26_000),
                (121, 26_100),
                (200, 34_000),
                (u32::MAX, 26_000 + 100 * (u64::from(u32::MAX) - 120)),
            ] {
                assert_eq!(
                    price(&source.prices, level),
                    Some(gold),
                    "{service:?} at {level}"
                );
            }
        }
    }

    /// Twist of Fate is data only (no NPC purchase here), but its ruled bands parse:
    /// 2000 to 30, 2000+200*(L-30) from 31 to 269, 50000 from 270.
    #[test]
    fn the_twist_of_fate_bands_match_the_owner_ruling() {
        let facts = price_facts();
        let twist = bands(
            admitted(&facts, "twist")
                .expect("fact")
                .expect("an admitted twist price"),
        )
        .expect("twist bands");
        for (level, gold) in [
            (1, 2_000),
            (30, 2_000),
            (31, 2_200),
            (269, 49_800),
            (270, 50_000),
            (271, 50_000),
            (u32::MAX, 50_000),
        ] {
            assert_eq!(price(&twist, level), Some(gold), "twist at {level}");
        }
        // The Store prices are recorded as data only.
        let coins = admitted(&facts, "store_tibia_coins")
            .expect("fact")
            .expect("admitted Store prices");
        for (blessing, tibia_coins) in [
            ("all_seven", 130),
            ("twist_of_fate", 8),
            ("blood_of_the_mountain", 25),
            ("heart_of_the_mountain", 25),
            ("the_spark_of_the_phoenix", 20),
            ("the_spiritual_shielding", 15),
            ("the_embrace_of_tibia", 15),
            ("the_fire_of_the_suns", 15),
            ("the_wisdom_of_solitude", 15),
        ] {
            assert_eq!(coins[blessing].as_u64(), Some(tibia_coins), "{blessing}");
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
        // The first `PROVEN` fact is the enhanced family: held, Blood has no price.
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

    /// An owner class needs its dated ruling, and the source loads only while the ruling's
    /// "no promotion price effect" and its above-120 segment agree with the families.
    #[test]
    fn owner_facts_need_their_ruling_and_agree() {
        let facts = price_facts();
        let mut unruled = facts.clone();
        unruled["facts"]["regular"]
            .as_object_mut()
            .expect("regular")
            .remove("owner_ruling");
        assert!(admitted(&unruled, "regular").is_err());
        assert!(admitted(&facts, "regular").expect("ruled").is_some());
        let effect = &facts["facts"]["promotion_price_effect"];
        for (class, value) in [("PROVEN_BY_OWNER", "half"), ("UNKNOWN", "none")] {
            let mut changed = effect.clone();
            changed["classification"] = class.into();
            changed["value"] = value.into();
            let prices = with_price_fact("promotion_price_effect", changed);
            assert!(
                BlessingSources::parse(&prices, SERVICE_EVIDENCE).is_err(),
                "{class} {value}"
            );
        }
        let mut above = facts["facts"]["regular_above_120"].clone();
        above["value"] = serde_json::json!([{"level_min": 120, "gold": "20000"}]);
        let prices = with_price_fact("regular_above_120", above);
        assert!(BlessingSources::parse(&prices, SERVICE_EVIDENCE).is_err());
    }

    /// Offers come only from declared Services of the source NPCs: the promotion under the
    /// NPC's own stem, a blessing under its source key and only at its source NPC.
    #[test]
    fn the_catalogue_offers_only_declared_source_services() {
        const WISDOM: &str = "oteryn:service.blessing.the_wisdom_of_solitude";
        const BLOOD: &str = "oteryn:service.blessing.blood_of_the_mountain";
        const HEART: &str = "oteryn:service.blessing.heart_of_the_mountain";
        let npcs = npc_catalogue(
            &[
                (
                    "oteryn:npc.eremo",
                    &[WISDOM, "oteryn:service.promotion.eremo"],
                ),
                // Kais declares the Heart too, but Nomad is its source NPC.
                ("oteryn:npc.kais", &[BLOOD, HEART]),
                // Another NPC's promotion stem is not this NPC's promotion.
                (
                    "oteryn:npc.king_tibianus",
                    &["oteryn:service.promotion.eremo"],
                ),
                ("oteryn:npc.nomad", &[HEART]),
            ],
            &[WISDOM, BLOOD, HEART, "oteryn:service.promotion.eremo"],
        );
        let catalogue = SourcePremiumNpcCatalogue::embedded(Arc::new(npcs), "content-1".into())
            .expect("the embedded source");
        assert_eq!(catalogue.offer_counts(), (1, 3));
        let promotion =
            admitted_promotion_offer(&catalogue, "oteryn:npc.eremo").expect("Eremo's promotion");
        assert_eq!(promotion.service.key, "oteryn:service.promotion.eremo");
        assert!(admitted_promotion_offer(&catalogue, "oteryn:npc.king_tibianus").is_none());
        assert!(admitted_promotion_offer(&catalogue, "oteryn:npc.kais").is_none());
        let wisdom = admitted_blessing_offer(
            &catalogue,
            "oteryn:npc.eremo",
            PremiumBlessingService::WisdomOfSolitude,
        )
        .expect("Eremo's Wisdom of Solitude");
        assert_eq!(wisdom.source.service.key, WISDOM);
        assert_eq!(price(&wisdom.prices, 50), Some(6_000));
        let blood = admitted_blessing_offer(
            &catalogue,
            "oteryn:npc.kais",
            PremiumBlessingService::BloodOfTheMountain,
        )
        .expect("Kais' Blood of the Mountain");
        assert_eq!(blood.storage_key, "oteryn:blessing.blood-of-the-mountain");
        let heart = admitted_blessing_offer(
            &catalogue,
            "oteryn:npc.nomad",
            PremiumBlessingService::HeartOfTheMountain,
        )
        .expect("Nomad's Heart of the Mountain");
        assert_eq!(heart.source.service.key, HEART);
        for (npc, service) in [
            (
                "oteryn:npc.kais",
                PremiumBlessingService::HeartOfTheMountain,
            ),
            (
                "oteryn:npc.eremo",
                PremiumBlessingService::BloodOfTheMountain,
            ),
            (
                "oteryn:npc.unknown",
                PremiumBlessingService::WisdomOfSolitude,
            ),
        ] {
            assert!(
                admitted_blessing_offer(&catalogue, npc, service).is_none(),
                "{npc} {service:?}"
            );
        }
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
