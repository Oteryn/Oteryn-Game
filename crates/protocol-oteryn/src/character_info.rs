//! CHARINFO wire-independent read model and title qualification rules.
//!
//! The transport intentionally deals in bounded pages.  Storage owners populate
//! these models; this module does not become a second owner of deaths or items.

pub const CAPABILITY_CHARACTER_INFO_V1: u32 = 97;
pub const COMMAND_TYPE_CHARACTER_INFO: u32 = 113;
pub const MAX_PAGE_SIZE: u8 = 50;
pub const HISTORY_WINDOW_MS: i64 = 30 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterInfoOperation {
    QuerySelf(PageRequest),
    QueryVisible {
        character: [u8; 16],
        page: PageRequest,
    },
    Unlock(CharacterTitle),
    Select(Option<CharacterTitle>),
    LookVisible {
        character: [u8; 16],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub occurred_at_unix_ms: i64,
    pub counterpart: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterInfoPage {
    pub summary: CharacterSummary,
    pub deaths: Vec<HistoryEntry>,
    pub pvp_kills: Vec<HistoryEntry>,
    pub earned_titles: Vec<CharacterTitle>,
    pub selected_title: Option<CharacterTitle>,
    pub next_offset: Option<u16>,
}

impl CharacterInfoPage {
    pub fn bounded(self, request: PageRequest, now_unix_ms: i64) -> Option<Self> {
        let oldest = now_unix_ms.saturating_sub(HISTORY_WINDOW_MS);
        let valid =
            |entry: &HistoryEntry| (oldest..=now_unix_ms).contains(&entry.occurred_at_unix_ms);
        (self.deaths.len() <= usize::from(request.limit)
            && self.pvp_kills.len() <= usize::from(request.limit)
            && self.deaths.iter().all(valid)
            && self.pvp_kills.iter().all(valid))
        .then_some(self)
    }
}

/// Admitted title catalogue. Names and lifetimes follow Tibia Global (TibiaWiki
/// `Cyclopedia#Titles`, revid 1197515). The wire and stored ids are Oteryn-owned and stable
/// (CHARACTER-INFO-1, migration 0226): Global publishes no title ids, and the reference
/// reimplementations number the same title differently (Globetrotter is 75 in Crystal and 74 in
/// Canary, and neither has the Adept titles), so no external numbering is adopted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum CharacterTitle {
    AdeptArmorer = 1,
    AdeptEnhancer = 2,
    Globetrotter = 3,
    ReigningDromeChampion = 4,
}

impl CharacterTitle {
    pub const ALL: [Self; 4] = [
        Self::AdeptArmorer,
        Self::AdeptEnhancer,
        Self::Globetrotter,
        Self::ReigningDromeChampion,
    ];

    /// The `u8` wire id, which is also the stored `SMALLINT` value.
    pub const fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|title| title.id() == id)
    }

    /// Global display name. None of the admitted titles has a female form in Global.
    pub const fn name(self) -> &'static str {
        match self {
            Self::AdeptArmorer => "Adept Armorer",
            Self::AdeptEnhancer => "Adept Enhancer",
            Self::Globetrotter => "Globetrotter",
            Self::ReigningDromeChampion => "Reigning Drome Champion",
        }
    }
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
    /// Ranked in the top five of the most recent Tibiadrome rota.
    pub drome_rota_top_five: bool,
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
        // Global: not permanent, revoked once not all map areas are explored.
        CharacterTitle::Globetrotter => Qualification {
            eligible: facts.all_map_areas_discovered,
            lifetime: TitleLifetime::Conditional,
        },
        // Global: not permanent, held for the rota after a top-five finish.
        CharacterTitle::ReigningDromeChampion => Qualification {
            eligible: facts.drome_rota_top_five && facts.next_rotation_unix_ms.is_some(),
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
    selected.filter(|title| {
        let qualification = qualify(*title, facts);
        qualification.lifetime == TitleLifetime::Permanent || valid_at(qualification, now_unix_ms)
    })
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
            drome_rota_top_five: true,
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
            qualify(CharacterTitle::ReigningDromeChampion, facts),
            1_999
        ));
        assert!(!valid_at(
            qualify(CharacterTitle::ReigningDromeChampion, facts),
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
    fn earned_permanent_selection_survives_changed_facts() {
        assert_eq!(
            qualified_selection(Some(CharacterTitle::AdeptArmorer), TitleFacts::default(), 1),
            Some(CharacterTitle::AdeptArmorer)
        );
    }

    #[test]
    fn pages_are_nonempty_and_bounded() {
        assert!(PageRequest::new(0, 1).is_some());
        assert!(PageRequest::new(0, MAX_PAGE_SIZE).is_some());
        assert!(PageRequest::new(0, 0).is_none());
        assert!(PageRequest::new(0, MAX_PAGE_SIZE + 1).is_none());
    }

    #[test]
    fn aggregate_rejects_out_of_window_native_results() {
        let request = PageRequest {
            offset: 0,
            limit: 1,
        };
        let mut page = CharacterInfoPage {
            summary: CharacterSummary {
                level: 1,
                experience: 0,
                achievement_points: 0,
                appearance_count: 0,
                items: ItemSummary {
                    inventory: 0,
                    depot: 0,
                    stash: 0,
                    inboxes: 0,
                },
            },
            deaths: vec![HistoryEntry {
                occurred_at_unix_ms: 99,
                counterpart: None,
            }],
            pvp_kills: vec![],
            earned_titles: vec![],
            selected_title: None,
            next_offset: None,
        };
        assert!(page.clone().bounded(request, 100).is_some());
        page.deaths[0].occurred_at_unix_ms = 100 - HISTORY_WINDOW_MS - 1;
        assert!(page.bounded(request, 100).is_none());
    }

    /// PAR-P12 against Tibia Global (TibiaWiki `Cyclopedia#Titles` revid 1197515, 112 titles)
    /// and Crystal `201189f8` `game.cpp:322-428` (95 titles, ids 1-95). The four admitted
    /// titles carry their Global names and permanence; the other 108 Global titles are MISSING.
    /// Adept Armorer and Adept Enhancer are Global-only (Summer Update 2025/2024), so Crystal
    /// has no entry for them; Globetrotter is Crystal id 75 and Reigning Drome Champion id 33,
    /// both registered as not permanent. The Oteryn ids 1-4 stay Oteryn-owned (DIFF by design):
    /// in Crystal they name Gold/Platinum/Crystal Hoarder and Beaststrider (Grade 1).
    #[test]
    fn p12_crystal_title_registry_parity() {
        let crystal_ids = 1..=95_u16;
        let oteryn = CharacterTitle::ALL;
        // Wire ids overlap Crystal ids 1-4, so the same id names a different title: DIFF.
        assert!(
            oteryn
                .iter()
                .all(|title| crystal_ids.contains(&(*title as u16)))
        );
        assert_eq!(oteryn.len(), 4);
        // Global names; none of the four is gendered in Global or Crystal.
        assert_eq!(
            oteryn.map(CharacterTitle::name),
            [
                "Adept Armorer",
                "Adept Enhancer",
                "Globetrotter",
                "Reigning Drome Champion"
            ]
        );
        // Stored and wire ids are stable across the rename.
        assert_eq!(oteryn.map(CharacterTitle::id), [1, 2, 3, 4]);
        for title in oteryn {
            assert_eq!(CharacterTitle::from_id(title.id()), Some(title));
        }
        assert_eq!(CharacterTitle::from_id(0), None);
        assert_eq!(CharacterTitle::from_id(5), None);
        let all = TitleFacts {
            weapon_mastery: true,
            all_grade_three_mods: true,
            all_map_areas_discovered: true,
            drome_rota_top_five: true,
            next_rotation_unix_ms: Some(10),
        };
        // Global permanence: Adept Armorer yes, the other three no.
        assert_eq!(
            oteryn.map(|title| qualify(title, all).lifetime),
            [
                TitleLifetime::Permanent,
                TitleLifetime::Conditional,
                TitleLifetime::Conditional,
                TitleLifetime::Until(10),
            ]
        );
        // Globetrotter is not permanent in Global or Crystal (id 75, `false`): MATCH.
        assert_eq!(
            qualify(CharacterTitle::Globetrotter, all).lifetime,
            TitleLifetime::Conditional
        );
        // Reigning Drome Champion expires at the next rota, like the Drome highscore title.
        assert_eq!(
            qualify(CharacterTitle::ReigningDromeChampion, all).lifetime,
            TitleLifetime::Until(10)
        );
    }

    /// PAR-P12: a Globetrotter selection is not kept once the map predicate no longer holds,
    /// while it stays displayable while every map area is explored.
    #[test]
    fn p12_globetrotter_is_not_permanent() {
        let explored = TitleFacts {
            all_map_areas_discovered: true,
            ..TitleFacts::default()
        };
        assert!(valid_at(qualify(CharacterTitle::Globetrotter, explored), 1));
        assert_eq!(
            qualified_selection(Some(CharacterTitle::Globetrotter), explored, 1),
            Some(CharacterTitle::Globetrotter)
        );
        assert!(!valid_at(
            qualify(CharacterTitle::Globetrotter, TitleFacts::default()),
            1
        ));
        assert_eq!(
            qualified_selection(Some(CharacterTitle::Globetrotter), TitleFacts::default(), 1),
            None
        );
    }
}
