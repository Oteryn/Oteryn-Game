//! HIGHSCORES0 §5: ranks and vocation lists from one complete eligible World/category cut.
//!
//! The caller supplies committed, already eligible rows (including the category's live-root,
//! vocation and account-representative rules). Values come from their existing source: a scalar
//! or a lexicographic (level, progress) tuple, never a score computed here. These borrowed facts
//! grant no gameplay authority. The separate snapshot owner reads committed sources;
//! scheduling and authenticated wire integration remain CP responsibilities.

use std::collections::BTreeSet;

use crate::durability::character_wheel::base_vocation;
use crate::foundation::CharacterId;

/// HIGHSCORES0-RL-02: each filter has its own limit, applied after filtering.
pub const RANKING_ROWS_MAX: usize = 1_000;

/// One eligible row at the snapshot's cut, not a live Character or an authority token.
pub struct RankingRow<'a, Value> {
    pub character_id: CharacterId,
    /// Exact display text at the cut; no naming-policy or uniqueness-key transformation.
    pub name: &'a str,
    /// The durable build's vocation key, including its promotion when present.
    pub vocation: &'a str,
    pub value: Value,
}

pub struct RankedRow<'a, Value> {
    pub global_rank: usize,
    pub row: &'a RankingRow<'a, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingError {
    InvalidVocationFilter,
    DuplicateCharacter,
}

/// Descending values, competition ranks, then raw Unicode code-point name and CharacterId order.
/// `None` selects all rows; a filter must be a base vocation key, not a promotion. Supply the
/// entire eligible cut, not a previous capped list: ranks and filtered membership need every row.
/// Equal values alone share rank; name/identity only order their positions. No source is mutated.
pub fn ranked_list<'a, Value: Ord>(
    rows: &'a [RankingRow<'a, Value>],
    vocation_filter: Option<&str>,
) -> Result<Vec<RankedRow<'a, Value>>, RankingError> {
    if vocation_filter.is_some_and(|filter| base_vocation(filter) != Some(filter)) {
        return Err(RankingError::InvalidVocationFilter);
    }
    let mut identities = BTreeSet::new();
    for row in rows {
        if !identities.insert(row.character_id) {
            return Err(RankingError::DuplicateCharacter);
        }
    }
    let mut ordered: Vec<_> = rows.iter().collect();
    ordered.sort_unstable_by(|left, right| {
        right
            .value
            .cmp(&left.value)
            .then_with(|| left.name.chars().cmp(right.name.chars()))
            .then_with(|| left.character_id.cmp(&right.character_id))
    });

    let mut result = Vec::new();
    let mut previous_value = None;
    let mut global_rank = 0;
    for (position, row) in ordered.into_iter().enumerate() {
        if previous_value != Some(&row.value) {
            global_rank = position + 1;
        }
        previous_value = Some(&row.value);
        if vocation_filter.is_none_or(|filter| base_vocation(row.vocation) == Some(filter)) {
            result.push(RankedRow { global_rank, row });
            if result.len() == RANKING_ROWS_MAX {
                break;
            }
        }
    }
    Ok(result)
}
