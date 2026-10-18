//! VIP0 §5.1/§9: private consent transitions and persisted list-bucket arithmetic.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentEnd {
    Withdraw,
    Reject,
    Revoke,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentState {
    Pending { expires_at_us: i64 },
    Accepted,
}

/// Expired pending invitations read as absent. Accepted consent does not expire with its
/// original invitation deadline; no function here grants consent or projects presence.
pub fn can_end_consent(action: ConsentEnd, state: ConsentState, now_us: i64) -> bool {
    match state {
        ConsentState::Pending { expires_at_us } => {
            now_us < expires_at_us && action != ConsentEnd::Revoke
        }
        ConsentState::Accepted => action != ConsentEnd::Reject,
    }
}

/// Pure transition decision; callers must independently resolve the exact invitation,
/// database clock and accepted watcher count under the current target's root lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptDecision {
    Accept,
    NotFound,
    ListFull,
}

pub fn accept_consent(state: ConsentState, now_us: i64, accepted_watchers: u64) -> AcceptDecision {
    if !matches!(state, ConsentState::Pending { expires_at_us } if now_us < expires_at_us) {
        return AcceptDecision::NotFound;
    }
    if accepted_watchers >= 100 {
        return AcceptDecision::ListFull;
    }
    AcceptDecision::Accept
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VipRateBucket {
    pub tokens: u8,
    pub at_us: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VipRateCharge {
    pub bucket: VipRateBucket,
    pub exhausted_seconds: Option<u8>,
}

/// VIP0-RL-07: 20 tokens, one per 3 seconds. Caller supplies the database clock and the
/// locked persisted bucket. A future/corrupt clock refuses rather than minting tokens.
pub fn charge_vip_bucket(bucket: VipRateBucket, now_us: i64) -> Option<VipRateCharge> {
    const INTERVAL_US: i64 = 3_000_000;
    if bucket.tokens > 20 || bucket.at_us < 0 || now_us < bucket.at_us {
        return None;
    }
    let elapsed = now_us.checked_sub(bucket.at_us)?;
    let refill = elapsed / INTERVAL_US;
    let available = (i64::from(bucket.tokens) + refill).min(20);
    let at_us = if available == 20 {
        now_us
    } else {
        bucket.at_us.checked_add(refill.checked_mul(INTERVAL_US)?)?
    };
    if available == 0 {
        let until = INTERVAL_US.checked_sub(now_us.checked_sub(at_us)?)?;
        return Some(VipRateCharge {
            bucket: VipRateBucket { tokens: 0, at_us },
            exhausted_seconds: Some(u8::try_from((until + 999_999) / 1_000_000).ok()?),
        });
    }
    Some(VipRateCharge {
        bucket: VipRateBucket {
            tokens: u8::try_from(available - 1).ok()?,
            at_us,
        },
        exhausted_seconds: None,
    })
}

/// VIP0-RL-06. Canonical spelling is retained; uniqueness uses the lowercase key.
pub fn vip_group_name(name: &str) -> Option<(String, String)> {
    if name.chars().any(char::is_control) {
        return None;
    }
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 25 || name.chars().any(char::is_control) {
        return None;
    }
    Some((name.to_owned(), name.to_lowercase()))
}

/// VIP0 §5 / RL-04–05. Fixed groups are 1–3, custom groups are 4–8.
/// Existence of custom groups is resolved independently under the owning list lock.
pub fn vip_edit_description(description: &str, icon: u8, groups: &[u8]) -> Option<String> {
    if description.chars().any(char::is_control) {
        return None;
    }
    let description = description.trim();
    if description.len() > 512
        || description.chars().count() > 128
        || description.chars().any(char::is_control)
        || icon > 10
        || groups.len() > 8
        || groups.iter().any(|group| !(1..=8).contains(group))
        || groups
            .iter()
            .enumerate()
            .any(|(index, group)| groups[..index].contains(group))
    {
        return None;
    }
    Some(description.to_owned())
}
