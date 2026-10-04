//! PREMIUM-DELIVERY0 candidate adapter. Platform owns lifecycle; Game only consumes bounded
//! authenticated evidence, fences it durably, and exposes current benefit under the actual owner.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::spell_access_facts::{AccessFactsError, CurrentSpellAccessOwner, owner_registration};
use crate::native_admission_source::{
    SourceError, TransientCapacity, descriptor::ProducerDescriptor,
};
use crate::spell::owned_cast_facts::{AccessProjections, CastFactsBinding, CurrentProjection};
use std::{collections::BTreeSet, sync::Arc};

pub(crate) use crate::durability::spell_premium_abi::{
    AuthenticatedSnapshot, Classification, TrustedTime,
};
use crate::durability::spell_premium_abi::{decode_authenticated, hash, hex, uuid};
#[derive(Debug)]
pub(crate) enum Error {
    Source(SourceError),
    Malformed,
    WrongBinding,
    UnavailableClock,
    Unsupported,
    Durability(crate::durability::DurabilityError),
}
impl From<crate::durability::spell_premium_abi::Error> for Error {
    fn from(error: crate::durability::spell_premium_abi::Error) -> Self {
        use crate::durability::spell_premium_abi::Error as E;
        match error {
            E::Malformed => Self::Malformed,
            E::WrongBinding => Self::WrongBinding,
            E::UnavailableClock => Self::UnavailableClock,
            E::Unsupported => Self::Unsupported,
        }
    }
}
impl From<SourceError> for Error {
    fn from(e: SourceError) -> Self {
        Self::Source(e)
    }
}
impl From<crate::durability::DurabilityError> for Error {
    fn from(e: crate::durability::DurabilityError) -> Self {
        Self::Durability(e)
    }
}
impl AuthenticatedSnapshot {
    pub(crate) fn premium_projection(
        &self,
        binding: &CastFactsBinding,
        time: TrustedTime,
        conflict: bool,
        fresh_pull: bool,
    ) -> Option<CurrentProjection<bool>> {
        let class = self.classification(time, conflict, fresh_pull);
        if !matches!(class, Classification::Current | Classification::Free) {
            return None;
        }
        let end = self.benefit_valid_until();
        let remaining = end.checked_sub(time.upper())?;
        let valid_until_micros = time
            .upper()
            .checked_mul(1_000_000)?
            .checked_add(remaining.checked_mul(1_000_000)?)?;
        Some(CurrentProjection {
            binding: binding.clone(),
            authority_revision: self.authority_revision(),
            valid_until_micros: u64::try_from(valid_until_micros).ok()?,
            value: class == Classification::Current,
        })
    }
}
/// Descriptor supplies genuine bounded TLS trust, client identity and exact producer purpose.
/// Accepted build SHAs are explicit compatibility configuration, never inferred ordering.
pub(crate) struct PremiumSource {
    descriptor: Arc<ProducerDescriptor>,
    capacity: TransientCapacity,
    compatible_builds: BTreeSet<String>,
    in_flight: std::sync::Mutex<BTreeSet<[u8; 16]>>,
}
impl PremiumSource {
    pub(crate) fn new(
        descriptor: Arc<ProducerDescriptor>,
        compatible_builds: BTreeSet<String>,
    ) -> Result<Self, Error> {
        if compatible_builds.is_empty()
            || compatible_builds.len() > 2
            || compatible_builds.iter().any(|s| !hash(s, 40))
        {
            return Err(Error::Unsupported);
        }
        Ok(Self {
            descriptor,
            capacity: TransientCapacity::new(),
            compatible_builds,
            in_flight: std::sync::Mutex::new(BTreeSet::new()),
        })
    }
    pub(crate) async fn pull(&self, account: [u8; 16]) -> Result<AuthenticatedSnapshot, Error> {
        {
            let mut pending = self.in_flight.lock().map_err(|_| Error::WrongBinding)?;
            if pending.len() >= 8 || !pending.insert(account) {
                return Err(Error::Source(SourceError::CapacityExceeded));
            }
        }
        struct Guard<'a> {
            pending: &'a std::sync::Mutex<BTreeSet<[u8; 16]>>,
            account: [u8; 16],
        }
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Ok(mut pending) = self.pending.lock() {
                    pending.remove(&self.account);
                }
            }
        }
        let _guard = Guard {
            pending: &self.in_flight,
            account,
        };
        let nonce = crate::node::operator_files::random_bytes::<16>()
            .map_err(|e| Error::Source(SourceError::Io(e)))?;
        let nonce = hex(&nonce);
        let account_text = uuid(account);
        let body=serde_json::to_string(&serde_json::json!({"schema":"oteryn.premium_snapshot_request.v1","account_id":account_text,"nonce":nonce})).map_err(|_|Error::Malformed)?;
        if body.len() > 256 {
            return Err(Error::Malformed);
        }
        let mut permit = self.capacity.try_queue()?;
        permit.try_activate()?;
        let bytes = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            crate::native_admission_source::read_premium_snapshot(
                &self.descriptor,
                &body,
                &mut permit,
            ),
        )
        .await
        .map_err(|_| Error::Source(SourceError::Unavailable))??;
        decode_authenticated(
            &bytes,
            account,
            &nonce,
            self.descriptor.source_authority(),
            &self.compatible_builds,
        )
        .map_err(Error::from)
    }
}
/// A committed fence read from this process's successful fresh pull. Reconstructing this adapter
/// from cached snapshot bytes after restart is prohibited by its private constructor.
pub(crate) struct PremiumAccessOwner {
    account: [u8; 16],
    current: Option<AuthenticatedSnapshot>,
    conflict: bool,
    fresh: bool,
    time: TrustedTime,
    observed_owner_micros: u64,
    other: AccessProjections,
}
impl PremiumAccessOwner {
    pub(crate) fn from_committed(
        read: crate::durability::spell_entitlements::CommittedPremiumRead,
        time: TrustedTime,
        observed_owner_micros: u64,
        other: AccessProjections,
    ) -> Self {
        Self {
            account: read.account(),
            current: read.snapshot().cloned(),
            conflict: read.conflict(),
            fresh: read.fresh(),
            time,
            observed_owner_micros,
            other,
        }
    }
    pub(crate) fn read_current_at_trusted_time(
        &self,
        binding: &CastFactsBinding,
        now_micros: u64,
        time: TrustedTime,
    ) -> Result<AccessProjections, AccessFactsError> {
        let mut result = self.other.clone();
        result.premium = None;
        if now_micros < self.observed_owner_micros {
            return Ok(result);
        }
        result.premium = self
            .current
            .as_ref()
            .and_then(|snapshot| {
                snapshot.premium_projection(binding, time, self.conflict, self.fresh)
            })
            .and_then(|mut projection| {
                let upper = u64::try_from(time.upper()).ok()?.checked_mul(1_000_000)?;
                let remaining = projection.valid_until_micros.checked_sub(upper)?;
                projection.valid_until_micros = now_micros.checked_add(remaining)?;
                Some(projection)
            });
        Ok(result)
    }
    pub(crate) fn classification(&self) -> Classification {
        self.current.as_ref().map_or(
            if self.conflict {
                Classification::Conflicting
            } else {
                Classification::Unavailable
            },
            |s| s.classification(self.time, self.conflict, self.fresh),
        )
    }
    pub(crate) fn account(&self) -> [u8; 16] {
        self.account
    }
}
impl owner_registration::Registered for PremiumAccessOwner {}
impl CurrentSpellAccessOwner for PremiumAccessOwner {
    fn account_id(&self) -> Option<[u8; 16]> {
        Some(self.account)
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now_micros: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        let mut result = self.other.clone();
        result.premium = None;
        let Some(time) = advance_trusted_time(self.time, self.observed_owner_micros, now_micros)
        else {
            return Ok(result);
        };
        result.premium = self
            .current
            .as_ref()
            .and_then(|s| s.premium_projection(binding, time, self.conflict, self.fresh))
            .and_then(|mut projection| {
                // Evidence uses Unix seconds; Channel expiry uses its monotonic clock.
                // Subtract the conservative upper bound before translating domains.
                let upper_micros = u64::try_from(time.upper()).ok()?.checked_mul(1_000_000)?;
                let remaining = projection.valid_until_micros.checked_sub(upper_micros)?;
                projection.valid_until_micros = now_micros.checked_add(remaining)?;
                Some(projection)
            });
        Ok(result)
    }
}

fn advance_trusted_time(time: TrustedTime, observed: u64, now: u64) -> Option<TrustedTime> {
    let elapsed = now.checked_sub(observed)?;
    let lower_elapsed = i64::try_from(elapsed / 1_000_000).ok()?;
    let upper_elapsed = i64::try_from(elapsed.div_ceil(1_000_000)).ok()?;
    TrustedTime::from_current_clock_owner(
        time.lower().checked_add(lower_elapsed)?,
        time.upper().checked_add(upper_elapsed)?,
    )
    .ok()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod clock_domain_tests {
    use super::*;
    #[test]
    fn elapsed_monotonic_time_advances_a_conservative_unix_window() {
        let time =
            TrustedTime::from_current_clock_owner(1_800_000_000, 1_800_000_002).expect("clock");
        let advanced = advance_trusted_time(time, 10, 2_500_010).expect("advance");
        assert_eq!(
            (advanced.lower(), advanced.upper()),
            (1_800_000_002, 1_800_000_005)
        );
        assert!(advance_trusted_time(time, 10, 9).is_none());
        assert!(advance_trusted_time(time, 10, u64::MAX).is_some());
    }
    #[test]
    fn clock_translation_rejects_regression_and_overflow_and_rounds_subsecond_expiry() {
        let time = TrustedTime::from_current_clock_owner(100, 101).expect("clock");
        assert!(advance_trusted_time(time, 10, 9).is_none());
        let same = advance_trusted_time(time, 10, 10).expect("unchanged");
        assert_eq!((same.lower(), same.upper()), (100, 101));
        let partial = advance_trusted_time(time, 10, 11).expect("partial second");
        assert_eq!((partial.lower(), partial.upper()), (100, 102));
        let exact = advance_trusted_time(time, 10, 1_000_010).expect("exact second");
        assert_eq!((exact.lower(), exact.upper()), (101, 102));
        let last =
            TrustedTime::from_current_clock_owner(i64::MAX - 1, i64::MAX).expect("last window");
        assert!(advance_trusted_time(last, 10, 11).is_none());
    }
}
