//! Optional WSP1 trailer: one physical point and cooldown, several weighted identities.
use super::*;

const MAGIC: [u8; 4] = *b"WSP1";
pub const MAX_CHOICES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedChoice {
    pub creature: u32,
    pub weight: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedPoint {
    pub source: u32,
    pub point: u16,
    /// Strict creature-index order; weights are positive integer ratios.
    pub choices: Vec<WeightedChoice>,
}

pub fn validate_weighted(table: &Table) -> Result<(), Error> {
    limit(
        table.weighted.len() <= MAX_POINTS,
        "weighted point count over RL-01",
    )?;
    check(
        table
            .weighted
            .windows(2)
            .all(|p| (p[0].source, p[0].point) < (p[1].source, p[1].point)),
        "weighted points not ascending and unique",
    )?;
    let mut choices = 0usize;
    for selection in &table.weighted {
        let source = table
            .sources
            .get(selection.source as usize)
            .ok_or_else(|| Error::Format("weighted source index out of range".into()))?;
        let point = source
            .points
            .get(usize::from(selection.point))
            .ok_or_else(|| Error::Format("weighted point index out of range".into()))?;
        check(point.boss_policy.is_none(), "weighted boss point")?;
        limit(
            (2..=MAX_CHOICES).contains(&selection.choices.len()),
            "weighted choice count outside2..64",
        )?;
        choices = choices
            .checked_add(selection.choices.len())
            .ok_or_else(|| Error::Limit("weighted choice count overflow".into()))?;
        limit(choices <= MAX_POINTS, "weighted choices over RL-01")?;
        check(
            selection
                .choices
                .windows(2)
                .all(|p| p[0].creature < p[1].creature),
            "weighted creatures not ascending and unique",
        )?;
        let mut total = 0u32;
        for choice in &selection.choices {
            let creature = table
                .creatures
                .get(choice.creature as usize)
                .ok_or_else(|| Error::Format("weighted creature index out of range".into()))?;
            check(
                creature.period == Period::All && choice.weight > 0,
                "invalid weighted period or zero weight",
            )?;
            total = total
                .checked_add(choice.weight)
                .ok_or_else(|| Error::Limit("weighted sum overflow".into()))?;
        }
        check(
            total > 0
                && selection
                    .choices
                    .iter()
                    .any(|c| c.creature == point.creature),
            "weighted timing creature absent from choices",
        )?;
    }
    Ok(())
}

pub(super) fn encode(table: &Table, out: &mut Vec<u8>) {
    if table.weighted.is_empty() {
        return;
    }
    out.extend_from_slice(&MAGIC);
    put(out, table.weighted.len() as u64);
    for selection in &table.weighted {
        put(out, u64::from(selection.source));
        put(out, u64::from(selection.point));
        put(out, selection.choices.len() as u64);
        for choice in &selection.choices {
            put(out, u64::from(choice.creature));
            put(out, u64::from(choice.weight));
        }
    }
}

pub(super) fn decode(r: &mut Reader<'_>) -> Result<Vec<WeightedPoint>, Error> {
    if r.pos == r.buf.len() {
        return Ok(Vec::new());
    }
    for byte in MAGIC {
        check(r.byte()? == byte, "unknown spawn trailer")?;
    }
    let count = r.bounded(MAX_POINTS as u64, "weighted point count")? as usize;
    check(
        count > 0 && count <= (r.buf.len() - r.pos) / 7,
        "empty or truncated weighted trailer",
    )?;
    let mut out = Vec::with_capacity(count);
    let mut total = 0usize;
    for _ in 0..count {
        let source = r.bounded((MAX_SOURCES - 1) as u64, "weighted source")? as u32;
        let point = r.bounded((MAX_POINTS_PER_SOURCE - 1) as u64, "weighted point")? as u16;
        let n = r.bounded(MAX_CHOICES as u64, "weighted choices")? as usize;
        check(
            n >= 2 && n <= (r.buf.len() - r.pos) / 2,
            "truncated weighted choices",
        )?;
        total = total
            .checked_add(n)
            .ok_or_else(|| Error::Limit("weighted choices overflow".into()))?;
        limit(total <= MAX_POINTS, "weighted choices over RL-01")?;
        let mut choices = Vec::with_capacity(n);
        for _ in 0..n {
            choices.push(WeightedChoice {
                creature: r.bounded(u64::from(u32::MAX), "weighted creature")? as u32,
                weight: r.bounded(u64::from(u32::MAX), "weighted weight")? as u32,
            });
        }
        out.push(WeightedPoint {
            source,
            point,
            choices,
        });
    }
    Ok(out)
}
