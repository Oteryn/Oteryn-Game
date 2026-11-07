//! Protocol-, persistence- and UI-neutral deterministic simulation primitives.
//!
//! This crate defines implementation profile revision 1 for the semantics implemented and tested
//! here. It does not define gameplay formulas, Reference values, security randomness, protocol IDs
//! or durable storage representations.

use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

const DECISION_DOMAIN_V1: &[u8] = b"oteryn.sim.decision.v1\0";
const STATE_HASH_DOMAIN_V1: &[u8] = b"oteryn.sim.state-hash.v1\0";

pub const MAX_DECISION_PURPOSE_BYTES: usize = 96;
pub const MAX_CANONICAL_STATE_ENTRIES: usize = 4_096;
pub const MAX_CANONICAL_STATE_KEY_BYTES: usize = 128;
pub const MAX_CANONICAL_STATE_VALUE_BYTES: usize = 16_384;
pub const MAX_FIXED_SCALE_DECIMALS: u8 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimulationDeterminismProfileRevision(u32);

impl SimulationDeterminismProfileRevision {
    pub const V1: Self = Self(1);

    pub fn new(value: u32) -> Result<Self, ProfileRevisionError> {
        if value == 0 {
            return Err(ProfileRevisionError::Zero);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileRevisionError {
    Zero,
}

impl Display for ProfileRevisionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => {
                formatter.write_str("simulation determinism profile revision must be non-zero")
            }
        }
    }
}

impl Error for ProfileRevisionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationDeterminismProfile {
    revision: SimulationDeterminismProfileRevision,
    profile_id: &'static str,
    decision_derivation_id: &'static str,
    state_hash_id: &'static str,
}

impl SimulationDeterminismProfile {
    #[must_use]
    pub const fn revision(self) -> SimulationDeterminismProfileRevision {
        self.revision
    }

    #[must_use]
    pub const fn profile_id(self) -> &'static str {
        self.profile_id
    }

    #[must_use]
    pub const fn decision_derivation_id(self) -> &'static str {
        self.decision_derivation_id
    }

    #[must_use]
    pub const fn state_hash_id(self) -> &'static str {
        self.state_hash_id
    }
}

pub const ACTIVE_PROFILE: SimulationDeterminismProfile = SimulationDeterminismProfile {
    revision: SimulationDeterminismProfileRevision::V1,
    profile_id: "oteryn.simulation-determinism.v1",
    decision_derivation_id: "sha256-domain-separated-decision-v1",
    state_hash_id: "sha256-canonical-kv-state-v1",
};

#[must_use]
pub const fn active_profile() -> SimulationDeterminismProfile {
    ACTIVE_PROFILE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactI64(i64);

impl ExactI64 {
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, NumericError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(NumericError::Overflow)
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, NumericError> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(NumericError::Overflow)
    }

    pub fn checked_mul(self, rhs: Self) -> Result<Self, NumericError> {
        self.0
            .checked_mul(rhs.0)
            .map(Self)
            .ok_or(NumericError::Overflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedScale {
    raw: i64,
    decimals: u8,
}

impl FixedScale {
    pub fn new(raw: i64, decimals: u8) -> Result<Self, NumericError> {
        if decimals > MAX_FIXED_SCALE_DECIMALS {
            return Err(NumericError::InvalidScale);
        }
        Ok(Self { raw, decimals })
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.raw
    }

    #[must_use]
    pub const fn decimals(self) -> u8 {
        self.decimals
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, NumericError> {
        if self.decimals != rhs.decimals {
            return Err(NumericError::ScaleMismatch);
        }
        let raw = self
            .raw
            .checked_add(rhs.raw)
            .ok_or(NumericError::Overflow)?;
        Ok(Self {
            raw,
            decimals: self.decimals,
        })
    }

    pub fn checked_mul_ratio(
        self,
        numerator: i64,
        denominator: i64,
        rounding: RoundingMode,
    ) -> Result<Self, NumericError> {
        if denominator == 0 {
            return Err(NumericError::DivisionByZero);
        }
        let product = i128::from(self.raw) * i128::from(numerator);
        let rounded = rounded_division(product, i128::from(denominator), rounding)?;
        let raw = i64::try_from(rounded).map_err(|_| NumericError::OutOfRange)?;
        Ok(Self {
            raw,
            decimals: self.decimals,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    TowardZero,
    Floor,
    Ceiling,
    NearestTiesToEven,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericError {
    Overflow,
    DivisionByZero,
    InvalidScale,
    ScaleMismatch,
    OutOfRange,
}

impl Display for NumericError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => formatter.write_str("checked numeric operation overflowed"),
            Self::DivisionByZero => formatter.write_str("division by zero is invalid"),
            Self::InvalidScale => {
                formatter.write_str("fixed-scale decimals exceed supported bound")
            }
            Self::ScaleMismatch => {
                formatter.write_str("fixed-scale operands have different scales")
            }
            Self::OutOfRange => formatter.write_str("numeric result is outside the target range"),
        }
    }
}

impl Error for NumericError {}

fn rounded_division(
    numerator: i128,
    denominator: i128,
    mode: RoundingMode,
) -> Result<i128, NumericError> {
    if denominator == 0 {
        return Err(NumericError::DivisionByZero);
    }

    let (numerator, denominator) = if denominator < 0 {
        (-numerator, -denominator)
    } else {
        (numerator, denominator)
    };
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return Ok(quotient);
    }

    let direction = if numerator < 0 { -1_i128 } else { 1_i128 };
    match mode {
        RoundingMode::TowardZero => Ok(quotient),
        RoundingMode::Floor => {
            if numerator < 0 {
                quotient.checked_sub(1).ok_or(NumericError::Overflow)
            } else {
                Ok(quotient)
            }
        }
        RoundingMode::Ceiling => {
            if numerator > 0 {
                quotient.checked_add(1).ok_or(NumericError::Overflow)
            } else {
                Ok(quotient)
            }
        }
        RoundingMode::NearestTiesToEven => {
            let twice_remainder = remainder
                .abs()
                .checked_mul(2)
                .ok_or(NumericError::Overflow)?;
            match twice_remainder.cmp(&denominator) {
                Ordering::Less => Ok(quotient),
                Ordering::Greater => quotient
                    .checked_add(direction)
                    .ok_or(NumericError::Overflow),
                Ordering::Equal => {
                    if quotient % 2 == 0 {
                        Ok(quotient)
                    } else {
                        quotient
                            .checked_add(direction)
                            .ok_or(NumericError::Overflow)
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GameplayDecisionRoot([u8; 32]);

impl GameplayDecisionRoot {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionOccurrenceId([u8; 16]);

impl DecisionOccurrenceId {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionError {
    EmptyPurpose,
    PurposeTooLong,
}

impl Display for DecisionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPurpose => {
                formatter.write_str("deterministic decision purpose must not be empty")
            }
            Self::PurposeTooLong => {
                formatter.write_str("deterministic decision purpose exceeds bound")
            }
        }
    }
}

impl Error for DecisionError {}

pub fn deterministic_decision_u64(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    purpose: &str,
    draw_index: u64,
) -> Result<u64, DecisionError> {
    if purpose.is_empty() {
        return Err(DecisionError::EmptyPurpose);
    }
    if purpose.len() > MAX_DECISION_PURPOSE_BYTES {
        return Err(DecisionError::PurposeTooLong);
    }

    let purpose_len = u16::try_from(purpose.len()).map_err(|_| DecisionError::PurposeTooLong)?;
    let mut hasher = Sha256::new();
    hasher.update(DECISION_DOMAIN_V1);
    hasher.update(ACTIVE_PROFILE.revision().get().to_be_bytes());
    hasher.update(root.as_bytes());
    hasher.update(occurrence.as_bytes());
    hasher.update(purpose_len.to_be_bytes());
    hasher.update(purpose.as_bytes());
    hasher.update(draw_index.to_be_bytes());
    let digest = hasher.finalize();
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest[..8]);
    Ok(u64::from_be_bytes(prefix))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticTimeMicros(u64);

impl SemanticTimeMicros {
    #[must_use]
    pub const fn from_micros(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, delta_micros: u64) -> Result<Self, SemanticTimeError> {
        self.0
            .checked_add(delta_micros)
            .map(Self)
            .ok_or(SemanticTimeError::Overflow)
    }

    pub fn elapsed_since(self, earlier: Self) -> Result<u64, SemanticTimeError> {
        self.0
            .checked_sub(earlier.0)
            .ok_or(SemanticTimeError::BeforeOrigin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTimeError {
    Overflow,
    BeforeOrigin,
}

impl Display for SemanticTimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => formatter.write_str("semantic time overflowed"),
            Self::BeforeOrigin => formatter.write_str("semantic time precedes comparison origin"),
        }
    }
}

impl Error for SemanticTimeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalStateEntry<'a> {
    key: &'a [u8],
    value: &'a [u8],
}

impl<'a> CanonicalStateEntry<'a> {
    #[must_use]
    pub const fn new(key: &'a [u8], value: &'a [u8]) -> Self {
        Self { key, value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalStateHash([u8; 32]);

impl CanonicalStateHash {
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalStateError {
    TooManyEntries,
    EmptyKey,
    KeyTooLong,
    ValueTooLong,
    DuplicateKey,
}

impl Display for CanonicalStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyEntries => {
                formatter.write_str("canonical state entry count exceeds bound")
            }
            Self::EmptyKey => formatter.write_str("canonical state key must not be empty"),
            Self::KeyTooLong => formatter.write_str("canonical state key exceeds bound"),
            Self::ValueTooLong => formatter.write_str("canonical state value exceeds bound"),
            Self::DuplicateKey => formatter.write_str("canonical state contains duplicate key"),
        }
    }
}

impl Error for CanonicalStateError {}

pub fn canonical_state_hash(
    entries: &[CanonicalStateEntry<'_>],
) -> Result<CanonicalStateHash, CanonicalStateError> {
    if entries.len() > MAX_CANONICAL_STATE_ENTRIES {
        return Err(CanonicalStateError::TooManyEntries);
    }
    for entry in entries {
        if entry.key.is_empty() {
            return Err(CanonicalStateError::EmptyKey);
        }
        if entry.key.len() > MAX_CANONICAL_STATE_KEY_BYTES {
            return Err(CanonicalStateError::KeyTooLong);
        }
        if entry.value.len() > MAX_CANONICAL_STATE_VALUE_BYTES {
            return Err(CanonicalStateError::ValueTooLong);
        }
    }

    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_unstable_by(|left, right| left.key.cmp(right.key));
    if ordered.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err(CanonicalStateError::DuplicateKey);
    }

    let entry_count =
        u32::try_from(ordered.len()).map_err(|_| CanonicalStateError::TooManyEntries)?;
    let mut hasher = Sha256::new();
    hasher.update(STATE_HASH_DOMAIN_V1);
    hasher.update(ACTIVE_PROFILE.revision().get().to_be_bytes());
    hasher.update(entry_count.to_be_bytes());
    for entry in ordered {
        let key_len =
            u32::try_from(entry.key.len()).map_err(|_| CanonicalStateError::KeyTooLong)?;
        let value_len =
            u32::try_from(entry.value.len()).map_err(|_| CanonicalStateError::ValueTooLong)?;
        hasher.update(key_len.to_be_bytes());
        hasher.update(entry.key);
        hasher.update(value_len.to_be_bytes());
        hasher.update(entry.value);
    }
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(&digest);
    Ok(CanonicalStateHash(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECISION_GOLDEN: u64 = 6_434_222_789_762_404_336;
    const STATE_HASH_GOLDEN: [u8; 32] = [
        212, 203, 199, 227, 26, 211, 106, 78, 162, 232, 70, 94, 79, 126, 101, 233, 173, 117, 89,
        161, 214, 5, 204, 82, 233, 94, 31, 144, 78, 12, 172, 2,
    ];

    #[test]
    fn active_profile_is_explicit_revision_one() {
        let profile = active_profile();
        assert_eq!(profile.revision(), SimulationDeterminismProfileRevision::V1);
        assert_eq!(profile.profile_id(), "oteryn.simulation-determinism.v1");
        assert_eq!(
            profile.decision_derivation_id(),
            "sha256-domain-separated-decision-v1"
        );
        assert_eq!(profile.state_hash_id(), "sha256-canonical-kv-state-v1");
    }

    #[test]
    fn exact_integer_overflow_fails_closed() {
        let max = ExactI64::new(i64::MAX);
        assert_eq!(
            max.checked_add(ExactI64::new(1)),
            Err(NumericError::Overflow)
        );
        assert_eq!(
            ExactI64::new(i64::MIN).checked_sub(ExactI64::new(1)),
            Err(NumericError::Overflow)
        );
    }

    #[test]
    fn fixed_scale_rounding_is_named_and_deterministic() -> Result<(), NumericError> {
        let positive = FixedScale::new(5, 0)?;
        let negative = FixedScale::new(-5, 0)?;
        assert_eq!(
            positive
                .checked_mul_ratio(1, 2, RoundingMode::TowardZero)?
                .raw(),
            2
        );
        assert_eq!(
            positive
                .checked_mul_ratio(1, 2, RoundingMode::Ceiling)?
                .raw(),
            3
        );
        assert_eq!(
            positive
                .checked_mul_ratio(1, 2, RoundingMode::NearestTiesToEven)?
                .raw(),
            2
        );
        assert_eq!(
            FixedScale::new(7, 0)?
                .checked_mul_ratio(1, 2, RoundingMode::NearestTiesToEven)?
                .raw(),
            4
        );
        assert_eq!(
            negative.checked_mul_ratio(1, 2, RoundingMode::Floor)?.raw(),
            -3
        );
        assert_eq!(
            negative
                .checked_mul_ratio(1, 2, RoundingMode::Ceiling)?
                .raw(),
            -2
        );
        assert_eq!(
            positive.checked_mul_ratio(1, 0, RoundingMode::TowardZero),
            Err(NumericError::DivisionByZero)
        );
        Ok(())
    }

    #[test]
    fn deterministic_decision_is_retry_stable_and_cross_target_golden() -> Result<(), DecisionError>
    {
        let root = GameplayDecisionRoot::from_bytes([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        let occurrence = DecisionOccurrenceId::from_bytes([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ]);
        let first = deterministic_decision_u64(&root, occurrence, "loot.primary", 7)?;
        let retry = deterministic_decision_u64(&root, occurrence, "loot.primary", 7)?;
        assert_eq!(first, DECISION_GOLDEN);
        assert_eq!(retry, first);
        assert_ne!(
            deterministic_decision_u64(&root, occurrence, "ai.target", 7)?,
            first
        );
        assert_ne!(
            deterministic_decision_u64(&root, occurrence, "loot.primary", 8)?,
            first
        );
        Ok(())
    }

    #[test]
    fn semantic_time_is_explicit_and_checked() -> Result<(), SemanticTimeError> {
        let start = SemanticTimeMicros::from_micros(10);
        let end = start.checked_add(25)?;
        assert_eq!(end.get(), 35);
        assert_eq!(end.elapsed_since(start)?, 25);
        assert_eq!(
            start.elapsed_since(end),
            Err(SemanticTimeError::BeforeOrigin)
        );
        assert_eq!(
            SemanticTimeMicros::from_micros(u64::MAX).checked_add(1),
            Err(SemanticTimeError::Overflow)
        );
        Ok(())
    }

    #[test]
    fn canonical_state_hash_is_order_independent_and_cross_target_golden()
    -> Result<(), CanonicalStateError> {
        let forward = [
            CanonicalStateEntry::new(b"z", b"last"),
            CanonicalStateEntry::new(b"a", b"first"),
            CanonicalStateEntry::new(b"m", b"middle"),
        ];
        let reverse = [
            CanonicalStateEntry::new(b"m", b"middle"),
            CanonicalStateEntry::new(b"a", b"first"),
            CanonicalStateEntry::new(b"z", b"last"),
        ];
        let forward_hash = canonical_state_hash(&forward)?;
        let reverse_hash = canonical_state_hash(&reverse)?;
        assert_eq!(forward_hash, reverse_hash);
        assert_eq!(forward_hash.into_bytes(), STATE_HASH_GOLDEN);
        Ok(())
    }

    #[test]
    fn canonical_state_hash_rejects_duplicates_and_bounds() {
        let duplicate = [
            CanonicalStateEntry::new(b"same", b"one"),
            CanonicalStateEntry::new(b"same", b"two"),
        ];
        assert_eq!(
            canonical_state_hash(&duplicate),
            Err(CanonicalStateError::DuplicateKey)
        );

        let oversized_key = [0_u8; MAX_CANONICAL_STATE_KEY_BYTES + 1];
        let entry = [CanonicalStateEntry::new(&oversized_key, b"value")];
        assert_eq!(
            canonical_state_hash(&entry),
            Err(CanonicalStateError::KeyTooLong)
        );
    }
}
