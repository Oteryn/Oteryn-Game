//! Accepted HOUSE-OWN0 §10 / GUILD0 §10 data rules. These values grant no live authority.
//! The native durability owner resolves all current property/guild facts inside its transaction.
use crate::domain::CharacterId;
use crate::durability::spell_house_abi::HouseAccess;
use crate::map::overlay::TilePos;
use std::collections::BTreeSet;

pub(crate) const LIST_MAX: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuleError {
    InvalidIdentity,
    InvalidRank,
    InvalidDoor,
    TooManyEntries,
    DuplicateEntry,
}

/// UUIDv7 data only. No native GuildId exists on this base; this is not a membership capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GuildId([u8; 16]);
impl GuildId {
    pub(crate) fn from_bytes(bytes: [u8; 16]) -> Result<Self, RuleError> {
        crate::durability::item_mint_audit::check_uuid_v7(&bytes)
            .map_err(|_| RuleError::InvalidIdentity)?;
        Ok(Self(bytes))
    }
    pub(crate) const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Rank(u8);
impl Rank {
    pub(crate) fn new(level: u8) -> Result<Self, RuleError> {
        (1..=20)
            .contains(&level)
            .then_some(Self(level))
            .ok_or(RuleError::InvalidRank)
    }
    pub(crate) const fn level(self) -> u8 {
        self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Match {
    Exact,
    AtLeast,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Entry {
    Character(CharacterId),
    Exclude(CharacterId),
    Guild {
        guild: GuildId,
        matching: Match,
        level: Rank,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum List {
    Subowners,
    Guests,
    Door(TilePos),
}
impl List {
    pub(crate) fn validate(self) -> Result<(), RuleError> {
        if let Self::Door(pos) = self
            && !(-15..=0).contains(&pos.floor)
        {
            return Err(RuleError::InvalidDoor);
        }
        Ok(())
    }
}

pub(crate) fn checked_entries(entries: &[Entry]) -> Result<BTreeSet<Entry>, RuleError> {
    if entries.len() > LIST_MAX {
        return Err(RuleError::TooManyEntries);
    }
    let unique = entries.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != entries.len() {
        return Err(RuleError::DuplicateEntry);
    }
    Ok(unique)
}

pub(crate) fn may_edit(role: HouseAccess, list: List) -> bool {
    role == HouseAccess::Owner || (role == HouseAccess::Subowner && list == List::Guests)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuildState {
    Forming,
    Active,
    Disbanding,
    Disbanded,
}
/// Data projected from actually locked guild/member/rank rows; never an input to the Root API.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Membership {
    pub(crate) guild: GuildId,
    pub(crate) level: Rank,
    pub(crate) state: GuildState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListMatch {
    Excluded,
    Granted,
    None,
}

pub(crate) fn match_list(
    entries: &BTreeSet<Entry>,
    character: CharacterId,
    member: Option<Membership>,
    ranks: &BTreeSet<Rank>,
) -> ListMatch {
    if entries.contains(&Entry::Exclude(character)) {
        return ListMatch::Excluded;
    }
    let granted = entries.iter().any(|entry| match entry {
        Entry::Character(id) => *id == character,
        Entry::Exclude(_) => false,
        Entry::Guild {
            guild,
            matching,
            level,
        } => member.is_some_and(|m| {
            m.guild == *guild
                && m.state != GuildState::Disbanded
                && match matching {
                    Match::Exact => ranks.contains(level) && m.level == *level,
                    Match::AtLeast => m.level <= *level,
                }
        }),
    });
    if granted {
        ListMatch::Granted
    } else {
        ListMatch::None
    }
}

/// Non-owner exclusions win across role lists; the accepted composed owner check precedes grants.
pub(crate) fn role(subowners: ListMatch, guests: ListMatch) -> HouseAccess {
    if subowners == ListMatch::Excluded || guests == ListMatch::Excluded {
        HouseAccess::NotInvited
    } else if subowners == ListMatch::Granted {
        HouseAccess::Subowner
    } else if guests == ListMatch::Granted {
        HouseAccess::Guest
    } else {
        HouseAccess::NotInvited
    }
}
