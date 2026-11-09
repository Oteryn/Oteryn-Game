//! Bounded personal progress over a stable, source-qualified target population.
//! This planner grants no authority: its caller must retain the target owner and
//! evaluate the masks under the existing fenced quest transaction.
use super::QuestRefusal;

pub const TARGETS_PER_MASK: usize = 63;
pub const MAX_MASKS: usize = 8;
pub const MAX_TARGETS: u16 = (TARGETS_PER_MASK * MAX_MASKS) as u16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub masks: Vec<i64>,
    pub count: u16,
    pub complete: bool,
}

/// Record one previously unseen target. Each signed storage value uses only
/// its low 63 bits; absent masks and bits outside the population fail closed.
pub fn record(
    masks: &[i64],
    target: u16,
    targets: u16,
    goal: u16,
) -> Result<Progress, QuestRefusal> {
    if targets == 0
        || targets > MAX_TARGETS
        || goal == 0
        || goal > targets
        || target >= targets
        || masks.len() != usize::from(targets).div_ceil(TARGETS_PER_MASK)
    {
        return Err(QuestRefusal::NotSupported);
    }
    let mut count = 0_u32;
    for (group, mask) in masks.iter().enumerate() {
        let bits = (usize::from(targets) - group * TARGETS_PER_MASK).min(TARGETS_PER_MASK);
        let allowed = if bits == TARGETS_PER_MASK {
            i64::MAX
        } else {
            (1_i64 << bits) - 1
        };
        if *mask < 0 || *mask & !allowed != 0 {
            return Err(QuestRefusal::OutOfRange);
        }
        count += mask.count_ones();
    }
    let group = usize::from(target) / TARGETS_PER_MASK;
    let bit = 1_i64 << (usize::from(target) % TARGETS_PER_MASK);
    if count >= u32::from(goal) || masks[group] & bit != 0 {
        return Err(QuestRefusal::StageMismatch);
    }
    let mut next = masks.to_vec();
    next[group] |= bit;
    let count = u16::try_from(count + 1).map_err(|_| QuestRefusal::OutOfRange)?;
    Ok(Progress {
        masks: next,
        count,
        complete: count == goal,
    })
}

/// Three stable target indices fit in one signed quest storage value. Zero
/// denotes an empty slot; stored indices are sorted and encoded as index + 1.
pub fn record_three(packed: i64, target: u16, targets: u16) -> Result<(i64, bool), QuestRefusal> {
    if !(3..=MAX_TARGETS).contains(&targets) || packed < 0 || packed >> 48 != 0 {
        return Err(QuestRefusal::OutOfRange);
    }
    let mut masks = vec![0; usize::from(targets).div_ceil(TARGETS_PER_MASK)];
    let mut previous = 0;
    let mut empty = false;
    for slot in 0..3 {
        let encoded = ((packed >> (slot * 16)) & 0xffff) as u16;
        if encoded == 0 {
            empty = true;
            continue;
        }
        if empty || encoded <= previous || encoded > targets {
            return Err(QuestRefusal::OutOfRange);
        }
        let index = usize::from(encoded - 1);
        masks[index / TARGETS_PER_MASK] |= 1_i64 << (index % TARGETS_PER_MASK);
        previous = encoded;
    }
    let progress = record(&masks, target, targets, 3)?;
    let mut next = 0;
    let mut slot = 0;
    for index in 0..targets {
        let offset = usize::from(index);
        if progress.masks[offset / TARGETS_PER_MASK] & (1_i64 << (offset % TARGETS_PER_MASK)) != 0 {
            next |= i64::from(index + 1) << (slot * 16);
            slot += 1;
        }
    }
    Ok((next, progress.complete))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_operations_require_three_different_targets() -> Result<(), QuestRefusal> {
        let mut masks = vec![0];
        for (i, target) in [2, 0, 1].into_iter().enumerate() {
            let next = record(&masks, target, 3, 3)?;
            assert_eq!(
                next.count,
                u16::try_from(i + 1).map_err(|_| QuestRefusal::OutOfRange)?
            );
            assert_eq!(next.complete, i == 2);
            assert_eq!(
                record(&next.masks, target, 3, 3),
                Err(QuestRefusal::StageMismatch)
            );
            masks = next.masks;
        }
        assert_eq!(masks, vec![7]);
        assert_eq!(record(&masks, 0, 3, 3), Err(QuestRefusal::StageMismatch));
        Ok(())
    }

    #[test]
    fn target_population_can_cross_signed_storage_masks() -> Result<(), QuestRefusal> {
        let mut masks = vec![0; 3];
        for target in [62, 63, 154] {
            masks = record(&masks, target, 155, 3)?.masks;
        }
        assert_eq!(masks, vec![1_i64 << 62, 1, 1_i64 << 28]);
        assert_eq!(
            record(&masks, 153, 155, 3),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }

    #[test]
    fn three_target_receipt_fits_one_track_without_aliasing_indices() -> Result<(), QuestRefusal> {
        let mut packed = 0;
        for (i, target) in [183, 2, 63].into_iter().enumerate() {
            let (next, complete) = record_three(packed, target, 184)?;
            assert_eq!(complete, i == 2);
            assert_eq!(
                record_three(next, target, 184),
                Err(QuestRefusal::StageMismatch)
            );
            packed = next;
        }
        assert_eq!(packed, 3 | (64 << 16) | (184 << 32));
        for corrupt in [-1, 1_i64 << 48, 2 << 16, 2 | (2 << 16), 3 | (2 << 16), 185] {
            assert!(record_three(corrupt, 1, 184).is_err());
        }
        Ok(())
    }

    #[test]
    fn invalid_populations_and_unknown_bits_never_create_credit() {
        for (masks, target, targets, goal) in [
            (vec![0], 0, 0, 1),
            (vec![0], 0, MAX_TARGETS + 1, 1),
            (vec![0], 0, 3, 0),
            (vec![0], 0, 3, 4),
            (vec![0], 3, 3, 3),
            (vec![], 0, 3, 3),
            (vec![0, 0], 0, 3, 3),
            (vec![-1], 0, 3, 3),
            (vec![8], 0, 3, 3),
            (vec![0, 0, 1_i64 << 29], 0, 155, 3),
        ] {
            let before = masks.clone();
            assert!(record(&masks, target, targets, goal).is_err());
            assert_eq!(masks, before);
        }
    }
}
