//! VIP0 §5.1/§9: private consent transitions and persisted list-bucket arithmetic.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsentEnd {
    Withdraw,
    Reject,
    Revoke,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsentState {
    Pending { expires_at_us: i64 },
    Accepted,
}

/// Expired pending invitations read as absent. Accepted consent does not expire with its
/// original invitation deadline; no function here grants consent or projects presence.
pub(crate) fn can_end_consent(action: ConsentEnd, state: ConsentState, now_us: i64) -> bool {
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
pub(crate) enum AcceptDecision {
    Accept,
    NotFound,
    ListFull,
}

pub(crate) fn accept_consent(
    state: ConsentState,
    now_us: i64,
    accepted_watchers: u64,
) -> AcceptDecision {
    if !matches!(state, ConsentState::Pending { expires_at_us } if now_us < expires_at_us) {
        return AcceptDecision::NotFound;
    }
    if accepted_watchers >= 100 {
        return AcceptDecision::ListFull;
    }
    AcceptDecision::Accept
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VipRateBucket {
    pub(crate) tokens: u8,
    pub(crate) at_us: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VipRateCharge {
    pub(crate) bucket: VipRateBucket,
    pub(crate) exhausted_seconds: Option<u8>,
}

/// VIP0-RL-07: 20 tokens, one per 3 seconds. Caller supplies the database clock and the
/// locked persisted bucket. A future/corrupt clock refuses rather than minting tokens.
pub(crate) fn charge_vip_bucket(bucket: VipRateBucket, now_us: i64) -> Option<VipRateCharge> {
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
