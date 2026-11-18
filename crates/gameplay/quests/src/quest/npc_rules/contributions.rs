use super::*;
use std::collections::BTreeSet;

/// Crystal npc/sandomo.lua:53-59 values mortar, frazzlemaws and silencers
/// according to their source weights, treating absent and negative counters as zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionTerm {
    pub track: String,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContributionRedemption {
    pub track: String,
    pub unit_cost: u32,
    pub quantity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityStatus {
    Invalid,
    Insufficient,
    Affordable,
}

pub(super) fn valid_terms(
    terms: &[ContributionTerm],
    mut owns_track: impl FnMut(&String) -> bool,
) -> bool {
    let mut tracks = BTreeSet::new();
    (1..=8).contains(&terms.len())
        && terms
            .iter()
            .all(|term| term.weight > 0 && owns_track(&term.track) && tracks.insert(&term.track))
}

pub(super) fn total(
    terms: &[ContributionTerm],
    mut read: impl FnMut(&str) -> Result<i64, NpcRuleError>,
) -> Result<i64, NpcRuleError> {
    terms.iter().try_fold(0_i64, |sum, term| {
        read(&term.track)?
            .max(0)
            .checked_mul(i64::from(term.weight))
            .and_then(|weighted| sum.checked_add(weighted))
            .ok_or_else(|| NpcRuleError::Arithmetic(term.track.clone()))
    })
}

// Crystal npc/sandomo.lua:151-164 accepts positive integral quantities. The
// native MINT transaction can deliver at most twenty stacks of 100 at once.
pub(super) const MAX_REDEMPTION_QUANTITY: u32 = 2_000;

pub(super) fn quantity(message: &str) -> Option<u32> {
    let value: f64 = message.trim().parse().ok()?;
    (value.is_finite() && value >= 1.0 && value.fract() == 0.0)
        .then(|| value.min(f64::from(u32::MAX)) as u32)
}

pub(super) fn status(quantity: Option<u32>, balance: i64, unit_cost: u32) -> QuantityStatus {
    match quantity {
        None => QuantityStatus::Invalid,
        Some(quantity)
            if quantity <= MAX_REDEMPTION_QUANTITY
                && i64::from(quantity) * i64::from(unit_cost) <= balance =>
        {
            QuantityStatus::Affordable
        }
        Some(_) => QuantityStatus::Insufficient,
    }
}
