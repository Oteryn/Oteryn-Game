//! PvP-specific blessing disposition at the lethal outcome boundary.

/// The one non-regular blessing admitted by the character blessing catalogue.
pub const TWIST_OF_FATE_KEY: &str = "oteryn:blessing.twist_of_fate";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlessingDisposition {
    /// Adventurer protection keeps every blessing.
    KeepAll,
    /// Ordinary deaths consume regular blessings but never consume Twist.
    ConsumeRegular,
    /// A protected PvP death consumes Twist and keeps every regular blessing.
    ConsumeTwist,
}

/// Selects blessing consumption from facts captured in the death transaction.
/// Red/black skull loss takes precedence over Twist protection.
pub fn blessing_disposition(
    pvp_death: bool,
    effective_red_or_black: bool,
    adventurer_protected: bool,
    regular_blessings: usize,
    has_twist: bool,
) -> BlessingDisposition {
    if adventurer_protected {
        BlessingDisposition::KeepAll
    } else if pvp_death && !effective_red_or_black && regular_blessings > 0 && has_twist {
        BlessingDisposition::ConsumeTwist
    } else {
        BlessingDisposition::ConsumeRegular
    }
}
