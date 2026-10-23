//! PARTY_PVP0 §5.1: channel-local eligibility and overflow-safe equal shares.
use crate::shared_experience::{SharedExperienceActivityObservation, shared_experience_active};

pub const SHARED_EXPERIENCE_PRINCIPALS_MAX: usize = 50;

#[derive(Debug, Clone, Copy)]
pub struct SharedExperienceMember {
    pub character: [u8; 16],
    pub channel: [u8; 16],
    pub level: u32,
    pub x: i32,
    pub y: i32,
    /// Promoted vocations have already been normalized by the consuming owner.
    pub vocation: u8,
    pub activity: Option<SharedExperienceActivityObservation>,
}

/// The creature's worth decides the bonus threshold; damage accounting supplies the
/// party's own pool. Summon XP has already been attributed by the consuming combat owner.
#[derive(Debug, Clone, Copy)]
pub struct SharedExperiencePool {
    pub creature_xp: i64,
    pub party_xp: i64,
    pub summon_xp: i64,
}

/// Missing/ambiguous owner inputs are errors; a known failed eligibility condition is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSharedExperience;

/// Floor one group's damage entitlement without overflowing an aggregated product.
/// Individual products fit in u128; the bounded quotient/remainder sums preserve the exact
/// result even when `base_xp * sum(group_damage)` itself would exceed u128.
pub fn damage_experience_pool(
    base_xp: i64,
    contributions: &[(u64, bool)],
) -> Result<i64, InvalidSharedExperience> {
    if base_xp < 0 || contributions.len() > SHARED_EXPERIENCE_PRINCIPALS_MAX {
        return Err(InvalidSharedExperience);
    }
    let total = contributions
        .iter()
        .map(|(d, _)| u128::from(*d))
        .sum::<u128>();
    if total == 0 {
        return Ok(base_xp);
    }
    let mut quotient = 0_u128;
    let mut remainder = 0_u128;
    for (damage, own) in contributions {
        if *own {
            let product = base_xp as u128 * u128::from(*damage);
            quotient += product / total;
            remainder += product % total;
        }
    }
    i64::try_from(quotient + remainder / total).map_err(|_| InvalidSharedExperience)
}

pub fn shared_experience_share(
    enabled: bool,
    leader: [u8; 16],
    channel: [u8; 16],
    leader_pz_blocked: bool,
    members: &[SharedExperienceMember],
    pool: SharedExperiencePool,
) -> Result<Option<i64>, InvalidSharedExperience> {
    let SharedExperiencePool {
        creature_xp: base_xp,
        party_xp,
        summon_xp,
    } = pool;
    if members.is_empty()
        || members.len() > SHARED_EXPERIENCE_PRINCIPALS_MAX
        || base_xp < 0
        || summon_xp < 0
        || party_xp < 0
        || party_xp > base_xp
        || summon_xp > party_xp
        || members.iter().any(|m| m.level == 0)
        || members
            .iter()
            .enumerate()
            .any(|(i, m)| members[..i].iter().any(|p| p.character == m.character))
    {
        return Err(InvalidSharedExperience);
    }
    let local: Vec<_> = members.iter().filter(|m| m.channel == channel).collect();
    let Some(leader) = local.iter().find(|m| m.character == leader) else {
        return Ok(None);
    };
    if !enabled || leader_pz_blocked || local.is_empty() {
        return Ok(None);
    }
    let highest = local
        .iter()
        .map(|m| m.level)
        .max()
        .ok_or(InvalidSharedExperience)?;
    if local.iter().any(|m| {
        u64::from(m.level) * 3 < u64::from(highest) * 2
            || m.x.abs_diff(leader.x) > 30
            || m.y.abs_diff(leader.y) > 30
            || !m.activity.is_some_and(shared_experience_active)
    }) {
        return Ok(None);
    }
    let mut vocations = std::collections::BTreeSet::new();
    for member in &local {
        if member.vocation != 0 {
            vocations.insert(member.vocation);
        }
    }
    let bonus = match vocations.len() {
        0 | 1 => {
            if base_xp >= 20 {
                20
            } else {
                0
            }
        }
        2 => 30,
        3 => 60,
        _ => 100,
    };
    // Keep the party bonus rational until the final equal-share rounding. A summon is
    // deducted from the boosted pool before any Character's equal share is formed.
    let pool = i128::from(party_xp) * (100 + bonus) - i128::from(summon_xp) * 100;
    let denominator = 100 * members.iter().filter(|m| m.channel == channel).count() as i128;
    let share = pool / denominator + i128::from(pool % denominator != 0);
    Ok(Some(
        i64::try_from(share).map_err(|_| InvalidSharedExperience)?,
    ))
}

#[cfg(test)]
#[path = "shared_experience_split_tests.rs"]
mod tests;
