use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Stable validation and routing failures for framework-neutral input contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// Physical key code zero is reserved as invalid.
    ZeroKeyCode,
    /// Physical key code exceeds the contract range.
    KeyCodeOutOfRange,
    /// Mouse button zero is reserved as invalid.
    ZeroMouseButton,
    /// Mouse button exceeds the contract range.
    MouseButtonOutOfRange,
    /// Pointer coordinate exceeds the contract range.
    CoordinateOutOfRange,
    /// Pointer delta exceeds the contract range.
    DeltaOutOfRange,
    /// A wheel event must contain at least one non-zero axis.
    ZeroWheelDelta,
    /// Wheel delta exceeds its contract range.
    WheelDeltaOutOfRange,
    /// Text commit must contain at least one scalar value.
    EmptyText,
    /// Text commit exceeds its UTF-8 byte limit.
    TextTooLong {
        /// Maximum accepted byte length.
        max: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// Semantic identifier must not be empty.
    EmptyIdentifier,
    /// Semantic identifier exceeds its byte limit.
    IdentifierTooLong {
        /// Maximum accepted byte length.
        max: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// Semantic identifier contains a character outside the stable vocabulary.
    InvalidIdentifier,
    /// Modifier bits contain an unknown flag.
    InvalidModifierBits,
    /// A chord must contain at least one non-modifier input.
    EmptyChord,
    /// A chord exceeds the maximum non-modifier input count.
    ChordTooLong {
        /// Maximum accepted input count.
        max: usize,
        /// Actual input count.
        actual: usize,
    },
    /// A chord contains the same input more than once.
    DuplicateChordInput,
    /// A wheel impulse must be the sole non-modifier input in its chord.
    InvalidWheelChord,
    /// Context identifiers must be unique.
    DuplicateContext,
    /// A binding references a context not declared by the map.
    UnknownBindingContext,
    /// Two bindings compete for the same context and chord.
    ConflictingBinding,
    /// A binding uses a caller-reserved chord.
    ReservedBinding,
    /// The requested context is not declared by the map.
    ContextNotFound,
}

impl Display for InputError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroKeyCode => formatter.write_str("physical key code must be non-zero"),
            Self::KeyCodeOutOfRange => {
                formatter.write_str("physical key code exceeds the contract range")
            }
            Self::ZeroMouseButton => formatter.write_str("mouse button must be non-zero"),
            Self::MouseButtonOutOfRange => {
                formatter.write_str("mouse button exceeds the contract range")
            }
            Self::CoordinateOutOfRange => {
                formatter.write_str("pointer coordinate exceeds the contract range")
            }
            Self::DeltaOutOfRange => {
                formatter.write_str("pointer delta exceeds the contract range")
            }
            Self::ZeroWheelDelta => formatter.write_str("wheel delta must contain a non-zero axis"),
            Self::WheelDeltaOutOfRange => {
                formatter.write_str("wheel delta exceeds the contract range")
            }
            Self::EmptyText => formatter.write_str("text commit must not be empty"),
            Self::TextTooLong { max, actual } => {
                write!(formatter, "text commit length {actual} exceeds limit {max}")
            }
            Self::EmptyIdentifier => formatter.write_str("semantic identifier must not be empty"),
            Self::IdentifierTooLong { max, actual } => {
                write!(
                    formatter,
                    "semantic identifier length {actual} exceeds limit {max}"
                )
            }
            Self::InvalidIdentifier => {
                formatter.write_str("semantic identifier contains an invalid character")
            }
            Self::InvalidModifierBits => {
                formatter.write_str("modifier bits contain an unknown flag")
            }
            Self::EmptyChord => formatter.write_str("input chord must not be empty"),
            Self::ChordTooLong { max, actual } => {
                write!(formatter, "input chord length {actual} exceeds limit {max}")
            }
            Self::DuplicateChordInput => {
                formatter.write_str("input chord contains a duplicate input")
            }
            Self::InvalidWheelChord => {
                formatter.write_str("wheel direction must be the sole input in a chord")
            }
            Self::DuplicateContext => formatter.write_str("input context identifier is duplicated"),
            Self::UnknownBindingContext => {
                formatter.write_str("binding references an unknown context")
            }
            Self::ConflictingBinding => formatter.write_str("bindings conflict within one context"),
            Self::ReservedBinding => formatter.write_str("binding uses a reserved chord"),
            Self::ContextNotFound => formatter.write_str("input context is not declared"),
        }
    }
}

impl Error for InputError {}
