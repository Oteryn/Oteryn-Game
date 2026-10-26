//! CHARINFO wire-independent read model and title qualification rules.
//!
//! The transport intentionally deals in bounded pages.  Storage owners populate
//! these models; this module does not become a second owner of deaths or items.

pub const CAPABILITY_CHARACTER_INFO_V1: u32 = 97;
pub const COMMAND_TYPE_CHARACTER_INFO: u32 = 113;
pub const MAX_PAGE_SIZE: u8 = 50;
pub const HISTORY_WINDOW_MS: i64 = 30 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum CharacterTitle {
    AdeptArmorer = 1,
    AdeptEnhancer = 2,
    Cartographer = 3,
    RotationChampion = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleLifetime {
    Permanent,
    Conditional,
    Until(i64),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TitleFacts {
    pub weapon_mastery: bool,
    pub all_grade_three_mods: bool,
    pub all_map_areas_discovered: bool,
    pub rotation_winner: bool,
    pub next_rotation_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Qualification {
    pub eligible: bool,
    pub lifetime: TitleLifetime,
}

pub fn qualify(title: CharacterTitle, facts: TitleFacts) -> Qualification {
    match title {
        CharacterTitle::AdeptArmorer => Qualification {
            eligible: facts.weapon_mastery,
            lifetime: TitleLifetime::Permanent,
        },
        CharacterTitle::AdeptEnhancer => Qualification {
            eligible: facts.all_grade_three_mods,
            lifetime: TitleLifetime::Conditional,
        },
        CharacterTitle::Cartographer => Qualification {
            eligible: facts.all_map_areas_discovered,
            lifetime: TitleLifetime::Permanent,
        },
        CharacterTitle::RotationChampion => Qualification {
            eligible: facts.rotation_winner && facts.next_rotation_unix_ms.is_some(),
            lifetime: facts
                .next_rotation_unix_ms
                .map_or(TitleLifetime::Until(0), TitleLifetime::Until),
        },
    }
}

pub fn valid_at(qualification: Qualification, now_unix_ms: i64) -> bool {
    qualification.eligible
        && match qualification.lifetime {
            TitleLifetime::Permanent | TitleLifetime::Conditional => true,
            TitleLifetime::Until(expiry) => now_unix_ms < expiry,
        }
}

/// Requalifies a selected title for every read/look. Invalid conditional and
/// expired rotating selections are cleared by the durable owner afterwards.
pub fn qualified_selection(
    selected: Option<CharacterTitle>,
    facts: TitleFacts,
    now_unix_ms: i64,
) -> Option<CharacterTitle> {
    selected.filter(|title| valid_at(qualify(*title, facts), now_unix_ms))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    pub offset: u16,
    pub limit: u8,
}

impl PageRequest {
    pub fn new(offset: u16, limit: u8) -> Option<Self> {
        (1..=MAX_PAGE_SIZE)
            .contains(&limit)
            .then_some(Self { offset, limit })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSummary {
    pub inventory: u32,
    pub depot: u32,
    pub stash: u32,
    pub inboxes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSummary {
    pub level: u32,
    pub experience: u64,
    pub achievement_points: u32,
    pub appearance_count: u16,
    pub items: ItemSummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_has_distinct_permanent_conditional_and_rotation_rules() {
        let facts = TitleFacts {
            weapon_mastery: true,
            all_grade_three_mods: false,
            rotation_winner: true,
            next_rotation_unix_ms: Some(2_000),
            ..TitleFacts::default()
        };
        assert!(valid_at(
            qualify(CharacterTitle::AdeptArmorer, facts),
            3_000
        ));
        assert!(!valid_at(
            qualify(CharacterTitle::AdeptEnhancer, facts),
            1_000
        ));
        assert!(valid_at(
            qualify(CharacterTitle::RotationChampion, facts),
            1_999
        ));
        assert!(!valid_at(
            qualify(CharacterTitle::RotationChampion, facts),
            2_000
        ));
    }

    #[test]
    fn invalid_current_selection_is_cleared() {
        assert_eq!(
            qualified_selection(
                Some(CharacterTitle::AdeptEnhancer),
                TitleFacts::default(),
                1
            ),
            None
        );
    }

    #[test]
    fn pages_are_nonempty_and_bounded() {
        assert!(PageRequest::new(0, 1).is_some());
        assert!(PageRequest::new(0, MAX_PAGE_SIZE).is_some());
        assert!(PageRequest::new(0, 0).is_none());
        assert!(PageRequest::new(0, MAX_PAGE_SIZE + 1).is_none());
    }
}
