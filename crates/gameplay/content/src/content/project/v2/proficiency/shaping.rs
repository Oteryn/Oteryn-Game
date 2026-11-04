//! Data-only import of the Global perk-shaping values (PROFICIENCY-1C amendment of
//! PROFICIENCY-1B §3): one pool of 64 perks per vocation, its rank 0 through rank 10 values, and the
//! dust and orb costs. Values keep the source's display unit (`percent` or `points`) as exact
//! ratios. No Character state and no perk execution.

use super::import::{decimal_ratio, invalid, parse};
use super::*;
use serde_json::Value;
use std::collections::BTreeMap;

const SHAPING_R1: &str =
    include_str!("../../../../../../../../rulesets/progression/weapon-proficiency/shaping.json");
const SHAPING_R2: &str =
    include_str!("../../../../../../../../rulesets/progression/weapon-proficiency/shaping-r2.json");

/// Global ranks: an initial rank 0 and ten refinement steps (PROFICIENCY-1C).
pub const PROFICIENCY_SHAPING_STEPS: usize = 10;

/// One pool entry with its evidenced rank 0 and rank 10 values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportedShapingEntry {
    /// Canonical non-value identity, comparable with [`proficiency_perk_identity`].
    pub identity: String,
    pub unit: ImportedShapingUnit,
    pub rank0: ProjectV2ExactRatio,
    pub rank10: ProjectV2ExactRatio,
    pub rank_values: [ProjectV2ExactRatio; PROFICIENCY_SHAPING_STEPS + 1],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedShapingUnit {
    Percent,
    Points,
}

/// The committed shaping values. Every pool is in entry-index order: the universal perks by
/// identity, then each of the vocation's six spells (source order) with its five augments.
#[derive(Debug, Clone, Serialize)]
pub struct ImportedProficiencyShaping {
    pub revision: String,
    pub modify_dust: [u64; 2],
    /// Dust for the steps 0→1 … 9→10.
    pub rank_step_dust: [u64; PROFICIENCY_SHAPING_STEPS],
    pub reshape_offer_dust: u64,
    pub clear_dust: u64,
    pub orb_count: u8,
    /// `ASSUMPTION: uniform; Global weights unpublished` (PROFICIENCY-1C §3). The one place the
    /// draw weight is decided; a published weight replaces it.
    pub uniform_weight: u32,
    pub pools: BTreeMap<String, Vec<ImportedShapingEntry>>,
    /// Proficiency definition key → vocation pool. Definitions absent here have no shaping.
    pub definition_vocations: BTreeMap<String, String>,
}

/// The draw weight of every pool entry while Global weights are unpublished.
pub const PROFICIENCY_SHAPING_ASSUMED_WEIGHT: u32 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema: String,
    revision: String,
    evidence: Value,
    costs: Costs,
    weights: String,
    ranks: Ranks,
    rank_value_curve: Option<RankValueCurve>,
    universal: Vec<Entry>,
    spell_augments: Vec<Augment>,
    spell_augment_overrides: Vec<Override>,
    vocation_spells: BTreeMap<String, Vec<Spell>>,
    definition_vocations: BTreeMap<String, String>,
    unshaped_definitions: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Costs {
    modify_dust: [u64; 2],
    rank_step_dust: [u64; PROFICIENCY_SHAPING_STEPS],
    reshape_offer_dust: u64,
    clear_dust: u64,
    clear_evidence: String,
    orb_count: u8,
    orb_item: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ranks {
    min: u8,
    max: u8,
    evidenced: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RankValueCurve {
    donor: String,
    curve_definitions: String,
    rank0_weights: [u8; PROFICIENCY_SHAPING_STEPS + 1],
    rank10_weights: [u8; PROFICIENCY_SHAPING_STEPS + 1],
    denominator: u8,
    rank_step_overrides: Vec<RankStepOverride>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RankStepOverride {
    spell_client_id: u16,
    augment: ProjectV2ProficiencySpellAugment,
    rank_step: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    identity: String,
    name: String,
    unit: ImportedShapingUnit,
    rank0: Value,
    rank10: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Augment {
    augment: ProjectV2ProficiencySpellAugment,
    unit: ImportedShapingUnit,
    rank0: Value,
    rank10: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Override {
    spell_client_id: u16,
    augment: ProjectV2ProficiencySpellAugment,
    rank10: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spell {
    spell: String,
    spell_client_id: u16,
}

fn ratio(value: &Value) -> Result<ProjectV2ExactRatio, ProjectError> {
    serde_json::from_value(decimal_ratio(value)?)
        .map_err(|error| ProjectError::InvalidJson(error.to_string()))
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left.max(1)
}

fn rank_values(
    rank0: ProjectV2ExactRatio,
    rank10: ProjectV2ExactRatio,
    curve: &RankValueCurve,
) -> Result<[ProjectV2ExactRatio; PROFICIENCY_SHAPING_STEPS + 1], ProjectError> {
    let mut values = [rank0; PROFICIENCY_SHAPING_STEPS + 1];
    for (rank, value) in values.iter_mut().enumerate() {
        let numerator = i128::from(rank0.numerator)
            * i128::from(curve.rank0_weights[rank])
            * i128::from(rank10.denominator)
            + i128::from(rank10.numerator)
                * i128::from(curve.rank10_weights[rank])
                * i128::from(rank0.denominator);
        let denominator = rank0
            .denominator
            .checked_mul(rank10.denominator)
            .and_then(|product| product.checked_mul(u64::from(curve.denominator)))
            .ok_or_else(|| invalid("proficiency shaping rank curve"))?;
        let numerator =
            i64::try_from(numerator).map_err(|_| invalid("proficiency shaping rank curve"))?;
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        *value = ProjectV2ExactRatio {
            numerator: numerator
                / i64::try_from(divisor).map_err(|_| invalid("proficiency shaping rank curve"))?,
            denominator: denominator / divisor,
        };
    }
    Ok(values)
}

fn endpoint_rank_values(
    rank0: ProjectV2ExactRatio,
    rank10: ProjectV2ExactRatio,
) -> Result<[ProjectV2ExactRatio; PROFICIENCY_SHAPING_STEPS + 1], ProjectError> {
    let mut values = [rank0; PROFICIENCY_SHAPING_STEPS + 1];
    values[PROFICIENCY_SHAPING_STEPS] = rank10;
    Ok(values)
}

fn rank_values_with_step(
    rank0: ProjectV2ExactRatio,
    rank10: ProjectV2ExactRatio,
    step: ProjectV2ExactRatio,
) -> Result<[ProjectV2ExactRatio; PROFICIENCY_SHAPING_STEPS + 1], ProjectError> {
    let mut values = [rank0; PROFICIENCY_SHAPING_STEPS + 1];
    for (rank, value) in values
        .iter_mut()
        .enumerate()
        .take(PROFICIENCY_SHAPING_STEPS)
    {
        let numerator = i128::from(rank0.numerator) * i128::from(step.denominator)
            + i128::from(step.numerator)
                * i128::try_from(rank).map_err(|_| invalid("proficiency shaping rank curve"))?
                * i128::from(rank0.denominator);
        let denominator = rank0
            .denominator
            .checked_mul(step.denominator)
            .ok_or_else(|| invalid("proficiency shaping rank curve"))?;
        let numerator =
            i64::try_from(numerator).map_err(|_| invalid("proficiency shaping rank curve"))?;
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        *value = ProjectV2ExactRatio {
            numerator: numerator
                / i64::try_from(divisor).map_err(|_| invalid("proficiency shaping rank curve"))?,
            denominator: denominator / divisor,
        };
    }
    values[PROFICIENCY_SHAPING_STEPS] = rank10;
    Ok(values)
}

fn augment_key(augment: ProjectV2ProficiencySpellAugment) -> Result<String, ProjectError> {
    serde_json::to_value(augment)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| invalid("shaping augment key"))
}

/// The canonical non-value identity of a perk: its kind, then every other field in name order,
/// joined by `:`. A selected perk and a pool entry are the same perk when these are equal.
pub fn proficiency_perk_identity(perk: &ProjectV2ProficiencyPerk) -> Option<String> {
    let Value::Object(fields) = serde_json::to_value(perk).ok()? else {
        return None;
    };
    let mut identity = fields.get("kind")?.as_str()?.to_owned();
    for (name, value) in &fields {
        if name == "kind" || name == "value" {
            continue;
        }
        identity.push(':');
        match value {
            Value::String(text) => identity.push_str(text),
            other => identity.push_str(&other.to_string()),
        }
    }
    Some(identity)
}

/// Import the repository's committed shaping values, validating the pool shape.
pub fn import_committed_proficiency_shaping() -> Result<ImportedProficiencyShaping, ProjectError> {
    import_shaping(SHAPING_R2)
}

/// Import the retained first revision. It deliberately exposes only the endpoint ranks.
pub fn import_retired_proficiency_shaping_r1() -> Result<ImportedProficiencyShaping, ProjectError> {
    import_shaping(SHAPING_R1)
}

fn import_shaping(text: &str) -> Result<ImportedProficiencyShaping, ProjectError> {
    let file: File = parse(text)?;
    let _ = (
        &file.evidence,
        &file.costs.clear_evidence,
        &file.costs.orb_item,
        file.rank_value_curve.as_ref().map(|curve| &curve.donor),
        file.rank_value_curve
            .as_ref()
            .map(|curve| &curve.curve_definitions),
    );
    if file.schema != "oteryn.weapon-proficiency.shaping/v1"
        || file.weights != "ASSUMPTION_UNIFORM"
        || file.ranks.min != 0
        || usize::from(file.ranks.max) != PROFICIENCY_SHAPING_STEPS
        || file.ranks.evidenced != [0, file.ranks.max]
        || file.costs.orb_count > 1
        || file.rank_value_curve.as_ref().is_some_and(|curve| {
            curve.denominator != PROFICIENCY_SHAPING_STEPS as u8
                || curve.rank0_weights != [10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]
                || curve.rank10_weights != [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        })
    {
        return Err(invalid("proficiency shaping header"));
    }
    let mut universal = Vec::new();
    for entry in &file.universal {
        if entry.name.is_empty() {
            return Err(invalid("proficiency shaping entry name"));
        }
        let rank0 = ratio(&entry.rank0)?;
        let rank10 = ratio(&entry.rank10)?;
        universal.push(ImportedShapingEntry {
            identity: entry.identity.clone(),
            unit: entry.unit,
            rank0,
            rank10,
            rank_values: file.rank_value_curve.as_ref().map_or_else(
                || endpoint_rank_values(rank0, rank10),
                |curve| rank_values(rank0, rank10, curve),
            )?,
        });
    }
    let mut pools = BTreeMap::new();
    for (vocation, spells) in &file.vocation_spells {
        let mut pool = universal.clone();
        for spell in spells {
            if spell.spell.is_empty() {
                return Err(invalid("proficiency shaping spell name"));
            }
            for augment in &file.spell_augments {
                let rank10 = file
                    .spell_augment_overrides
                    .iter()
                    .find(|o| {
                        o.spell_client_id == spell.spell_client_id && o.augment == augment.augment
                    })
                    .map_or(&augment.rank10, |o| &o.rank10);
                let rank0 = ratio(&augment.rank0)?;
                let rank10 = ratio(rank10)?;
                let rank_values = file
                    .rank_value_curve
                    .as_ref()
                    .and_then(|curve| {
                        curve.rank_step_overrides.iter().find(|override_| {
                            override_.spell_client_id == spell.spell_client_id
                                && override_.augment == augment.augment
                        })
                    })
                    .map(|override_| ratio(&override_.rank_step))
                    .transpose()?
                    .map_or_else(
                        || {
                            file.rank_value_curve.as_ref().map_or_else(
                                || endpoint_rank_values(rank0, rank10),
                                |curve| rank_values(rank0, rank10, curve),
                            )
                        },
                        |step| rank_values_with_step(rank0, rank10, step),
                    )?;
                pool.push(ImportedShapingEntry {
                    identity: format!(
                        "spell_augment:{}:{}",
                        augment_key(augment.augment)?,
                        spell.spell_client_id
                    ),
                    unit: augment.unit,
                    rank0,
                    rank10,
                    rank_values,
                });
            }
        }
        let mut identities: Vec<_> = pool.iter().map(|entry| &entry.identity).collect();
        identities.sort_unstable();
        identities.dedup();
        if pool.len() != 64 || identities.len() != pool.len() {
            return Err(invalid("proficiency shaping pool shape"));
        }
        pools.insert(vocation.clone(), pool);
    }
    if file
        .definition_vocations
        .iter()
        .any(|(definition, vocation)| {
            !pools.contains_key(vocation) || file.unshaped_definitions.contains(definition)
        })
    {
        return Err(invalid("proficiency shaping vocation"));
    }
    Ok(ImportedProficiencyShaping {
        revision: file.revision,
        modify_dust: file.costs.modify_dust,
        rank_step_dust: file.costs.rank_step_dust,
        reshape_offer_dust: file.costs.reshape_offer_dust,
        clear_dust: file.costs.clear_dust,
        orb_count: file.costs.orb_count,
        uniform_weight: PROFICIENCY_SHAPING_ASSUMED_WEIGHT,
        pools,
        definition_vocations: file.definition_vocations,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn committed_shaping_has_a_64_perk_pool_per_vocation_and_global_costs() {
        let shaping = import_committed_proficiency_shaping().expect("shaping imports");
        assert_eq!(shaping.modify_dust, [250, 1000]);
        assert_eq!(
            shaping.rank_step_dust,
            [125, 200, 275, 350, 425, 500, 575, 650, 725, 800]
        );
        assert_eq!(shaping.rank_step_dust.iter().sum::<u64>(), 4625);
        assert_eq!((shaping.reshape_offer_dust, shaping.clear_dust), (250, 0));
        assert_eq!(shaping.orb_count, 1);
        assert_eq!(
            shaping.pools.keys().map(String::as_str).collect::<Vec<_>>(),
            ["druid", "knight", "monk", "paladin", "sorcerer"]
        );
        let monk = &shaping.pools["monk"];
        let greater_flurry = monk
            .iter()
            .find(|entry| entry.identity == "spell_augment:base_damage:289")
            .expect("Greater Flurry of Blows base damage");
        assert_eq!(
            (greater_flurry.rank0, greater_flurry.rank10),
            (
                ProjectV2ExactRatio {
                    numerator: 1,
                    denominator: 1
                },
                ProjectV2ExactRatio {
                    numerator: 5,
                    denominator: 1
                }
            )
        );
        // Crystal 201189f, proficiencies.hpp:78-85 and proficiencies.cpp:244-329:
        // value = rank0 + perRank * rank; Greater Flurry uses perRank 0.4.
        assert_eq!(
            greater_flurry.rank_values[1],
            ProjectV2ExactRatio {
                numerator: 7,
                denominator: 5
            }
        );
        assert_eq!(
            greater_flurry.rank_values[9],
            ProjectV2ExactRatio {
                numerator: 23,
                denominator: 5
            }
        );
        assert_eq!(
            greater_flurry.rank_values[10],
            ProjectV2ExactRatio {
                numerator: 5,
                denominator: 1
            }
        );
        let alpha = &shaping.pools["knight"][0];
        assert_eq!(alpha.identity, "alpha_strike_damage");
        assert_eq!(
            alpha.rank_values,
            [
                ProjectV2ExactRatio {
                    numerator: 2,
                    denominator: 1
                },
                ProjectV2ExactRatio {
                    numerator: 14,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 18,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 22,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 26,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 6,
                    denominator: 1
                },
                ProjectV2ExactRatio {
                    numerator: 34,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 38,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 42,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 46,
                    denominator: 5
                },
                ProjectV2ExactRatio {
                    numerator: 10,
                    denominator: 1
                },
            ]
        );
        assert_eq!(
            alpha.rank10,
            ProjectV2ExactRatio {
                numerator: 10,
                denominator: 1
            }
        );
    }

    #[test]
    fn every_bound_definition_but_the_dual_vocation_casters_has_a_pool() {
        let shaping = import_committed_proficiency_shaping().expect("shaping imports");
        let data = import_committed_weapon_proficiency_data().expect("catalogue imports");
        let bound: std::collections::BTreeSet<_> = data
            .bindings()
            .values()
            .map(|binding| binding.profile_binding.key.as_str())
            .collect();
        let unshaped: Vec<_> = bound
            .iter()
            .filter(|key| !shaping.definition_vocations.contains_key(**key))
            .collect();
        assert_eq!(unshaped.len(), 7, "{unshaped:?}");
        assert!(
            shaping
                .definition_vocations
                .keys()
                .all(|key| data.definitions().contains_key(key))
        );
    }

    #[test]
    fn a_selected_perk_and_its_pool_entry_share_an_identity() {
        let shaping = import_committed_proficiency_shaping().expect("shaping imports");
        let data = import_committed_weapon_proficiency_data().expect("catalogue imports");
        let identities: Vec<_> = data
            .definitions()
            .values()
            .flat_map(|definition| &definition.levels)
            .flat_map(|level| &level.perks)
            .filter_map(proficiency_perk_identity)
            .collect();
        for expected in [
            "alpha_strike_damage",
            "bestiary_class_damage:6:Dragon",
            "spell_augment:critical_extra_damage:105",
        ] {
            assert!(
                identities.iter().any(|identity| identity == expected),
                "{expected}"
            );
            assert!(
                shaping
                    .pools
                    .values()
                    .any(|pool| pool.iter().any(|entry| entry.identity == expected)),
                "{expected}"
            );
        }
    }
}
