//! Source-bound compressed damage schedules. No actor health or world mutation occurs here.
use super::{ApplicationFacts, COND_DOT_TOTAL_DRAW, ConditionRefusal, draw_in_range};
const MAX_SEGMENTS: usize = 128;
const MIN_INTERVAL_MS: u32 = 1_000;
const MAX_DECREASING_AMOUNT: u32 = 2_500;
const COUNT_DRAW: &str = "oteryn.condition.dot_count_draw.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DamageSegment {
    pub(crate) count: u32,
    pub(crate) amount: u32,
    pub(crate) interval_ms: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DamageSchedule {
    Fixed {
        delayed: bool,
        segments: Vec<DamageSegment>,
    },
    Decreasing {
        delayed: bool,
        minimum: u32,
        maximum: u32,
        initial: Option<u32>,
        interval_ms: u32,
    },
    Geometric {
        delayed: bool,
        minimum: u32,
        maximum: u32,
        numerator: u32,
        denominator: u32,
        counts: Vec<u32>,
        interval_ms: u32,
    },
}
impl DamageSchedule {
    pub(crate) fn delayed(&self) -> bool {
        match self {
            Self::Fixed { delayed, .. }
            | Self::Decreasing { delayed, .. }
            | Self::Geometric { delayed, .. } => *delayed,
        }
    }
    pub(crate) fn validate(&self) -> bool {
        match self {
            Self::Fixed { segments, .. } => {
                !segments.is_empty()
                    && segments.len() <= MAX_SEGMENTS
                    && segments
                        .iter()
                        .all(|s| s.count > 0 && s.amount > 0 && s.interval_ms >= MIN_INTERVAL_MS)
                    && total(segments).is_some()
            }
            Self::Decreasing {
                minimum,
                maximum,
                initial,
                interval_ms,
                ..
            } => {
                *maximum > 0
                    && minimum <= maximum
                    && *maximum <= MAX_DECREASING_AMOUNT
                    && *interval_ms >= MIN_INTERVAL_MS
                    && initial.is_none_or(|i| i > 0)
                    && initial.unwrap_or(maximum.div_ceil(20)).min(*maximum) <= MAX_SEGMENTS as u32
            }
            Self::Geometric {
                minimum,
                maximum,
                numerator,
                denominator,
                counts,
                interval_ms,
                ..
            } => {
                *minimum > 0
                    && minimum <= maximum
                    && *denominator > 0
                    && *numerator > 0
                    && *interval_ms >= MIN_INTERVAL_MS
                    && !counts.is_empty()
                    && counts.len() <= MAX_SEGMENTS
                    && counts.iter().all(|&n| n > 0 && n <= MAX_SEGMENTS as u32)
                    && counts.windows(2).all(|w| w[0] < w[1])
                    && counts.last().is_some_and(|&count| {
                        geometric(*maximum, count, *numerator, *denominator, *interval_ms).is_some()
                    })
            }
        }
    }
    pub(crate) fn compile(
        &self,
        facts: &ApplicationFacts<'_>,
    ) -> Result<DamageCursor, ConditionRefusal> {
        if !self.validate() {
            return Err(ConditionRefusal::DrawFailed);
        }
        let segments = match self {
            Self::Fixed { segments, .. } => segments.clone(),
            Self::Decreasing {
                minimum,
                maximum,
                initial,
                interval_ms,
                ..
            } => {
                let amount = draw_in_range(*minimum, *maximum, facts, COND_DOT_TOTAL_DRAW)?;
                if amount == 0 {
                    return Err(ConditionRefusal::ZeroDamage);
                }
                // Canary init clamps explicit start to maxDamage, not to the drawn amount.
                let start = initial.unwrap_or(amount.div_ceil(20).max(1)).min(*maximum);
                decreasing(amount, start, *interval_ms).ok_or(ConditionRefusal::DrawFailed)?
            }
            Self::Geometric {
                minimum,
                maximum,
                numerator,
                denominator,
                counts,
                interval_ms,
                ..
            } => {
                let base = draw_in_range(*minimum, *maximum, facts, COND_DOT_TOTAL_DRAW)?;
                let index = draw_in_range(0, counts.len() as u32 - 1, facts, COUNT_DRAW)?;
                geometric(
                    base,
                    counts[index as usize],
                    *numerator,
                    *denominator,
                    *interval_ms,
                )
                .ok_or(ConditionRefusal::DrawFailed)?
            }
        };
        let remaining = total(&segments).ok_or(ConditionRefusal::DrawFailed)?;
        Ok(DamageCursor {
            left: segments[0].count,
            segments,
            at: 0,
            remaining,
        })
    }
    /// Existing COND-1 stronger-remaining policy. Generated random schedules compare their
    /// nominal midpoint, as Canary getTotalDamage does before init constructs the damage list.
    pub(crate) fn comparison_total(&self, compiled: &DamageCursor) -> u32 {
        match self {
            Self::Decreasing {
                minimum, maximum, ..
            } => ((u64::from(*minimum) + u64::from(*maximum)) / 2) as u32,
            Self::Fixed { .. } | Self::Geometric { .. } => compiled.remaining,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DamageCursor {
    segments: Vec<DamageSegment>,
    at: usize,
    left: u32,
    pub(crate) remaining: u32,
}
impl DamageCursor {
    pub(crate) fn current(&self) -> Option<DamageSegment> {
        self.segments.get(self.at).copied()
    }
    pub(crate) fn consume(&mut self) {
        if let Some(s) = self.current() {
            self.remaining -= s.amount;
            self.left -= 1;
            if self.left == 0 {
                self.at += 1;
                if let Some(next) = self.current() {
                    self.left = next.count;
                }
            }
        }
    }
}
fn total(segments: &[DamageSegment]) -> Option<u32> {
    segments.iter().try_fold(0_u32, |sum, s| {
        sum.checked_add(s.count.checked_mul(s.amount)?)
    })
}
fn decreasing(amount: u32, start: u32, interval_ms: u32) -> Option<Vec<DamageSegment>> {
    if start == 0 || start > MAX_SEGMENTS as u32 {
        return None;
    }
    let mut segments = Vec::new();
    let mut sum = 0_u32;
    // Verbatim arithmetic semantics of Canary ConditionDamage::generateDamageList:
    // integer median, f32 division, then f64 fabs. Always emit at least one tick per i.
    for i in (1..=start).rev() {
        let med = u64::from(start + 1 - i) * u64::from(amount) / u64::from(start);
        let mut count = 0_u32;
        loop {
            sum = sum.checked_add(i)?;
            count = count.checked_add(1)?;
            let x1 = (1.0_f64 - f64::from((sum as f32 + i as f32) / med as f32)).abs();
            let x2 = (1.0_f64 - f64::from(sum as f32 / med as f32)).abs();
            if x1.partial_cmp(&x2) != Some(std::cmp::Ordering::Less) {
                break;
            }
        }
        segments.push(DamageSegment {
            count,
            amount: i,
            interval_ms,
        });
    }
    Some(segments)
}
fn geometric(
    base: u32,
    count: u32,
    numerator: u32,
    denominator: u32,
    interval_ms: u32,
) -> Option<Vec<DamageSegment>> {
    if denominator == 0 || count == 0 || count > MAX_SEGMENTS as u32 {
        return None;
    }
    let factor = f64::from(numerator) / f64::from(denominator);
    let mut current = f64::from(base);
    let mut result = Vec::new();
    for _ in 0..count {
        if !current.is_finite() || current < 1.0 || current > f64::from(i32::MAX) {
            return None;
        }
        // Source Lua multiplies the untruncated value; addDamage truncates each emitted tick.
        result.push(DamageSegment {
            count: 1,
            amount: current as u32,
            interval_ms,
        });
        current *= factor;
    }
    total(&result)?;
    Some(result)
}
