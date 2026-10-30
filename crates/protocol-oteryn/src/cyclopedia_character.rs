//! PAR-P12b/c Cyclopedia character window tabs (Crystal `protocolgame.cpp`
//! `sendCyclopediaCharacter*`, `:4341-5265`). The server owns every fact; the request only
//! names the tab. Payload schemas: `docs/contracts/protocol-oteryn/v1/cyclopedia-character-v1.md`.

pub const CAPABILITY_CYCLOPEDIA_CHARACTER_V1: u32 = 199;
pub const COMMAND_TYPE_CYCLOPEDIA_BADGES: u32 = 387;
pub const COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB: u32 = 388;
/// `tab:u8, offset:u16, limit:u8` and an optional 16-byte target Character.
pub const MAX_TAB_REQUEST_BYTES: usize = 20;
/// The largest page: 50 entries of `u64 + u8 + u8 + 255` bytes and the headers.
pub const MAX_TAB_RESULT_BYTES: usize = 16384;
pub const MAX_BADGES_REQUEST_BYTES: usize = 0;
/// One count byte and one byte per admitted badge.
pub const MAX_BADGES_RESULT_BYTES: usize = 1 + BADGE_COUNT;
pub const BADGE_COUNT: usize = 20;

/// What a badge rule reads: native account facts summed over the account's Characters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BadgeFacts {
    /// Whole years since the account was created (its UUIDv7 account id time).
    pub account_years: u32,
    /// Loyalty points from the Platform loyalty projection.
    pub loyalty_points: i64,
    /// The sum of every Character's level on the account.
    pub level_sum: i64,
    /// The highest Character level per base vocation, in `BaseVocation` order.
    pub vocation_levels: [u32; 5],
}

/// The five base vocations, promoted keys folded in (`progression::wheel::base_vocation`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseVocation {
    Druid = 0,
    Knight = 1,
    Monk = 2,
    Paladin = 3,
    Sorcerer = 4,
}

impl BaseVocation {
    pub const KEYS: [&'static str; 5] = ["druid", "knight", "monk", "paladin", "sorcerer"];
}

/// Global badge requirements (TibiaWiki "Social", revision 1111449, Badges table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeRule {
    /// "Your account is older than N years."
    AccountOlderThan(u32),
    /// "You earned more than N loyalty points."
    LoyaltyAbove(i64),
    /// "Summing up the level of all your characters amounts to more than N."
    LevelSumAbove(i64),
    /// "At least level N with all four vocations" (Druid, Knight, Paladin, Sorcerer).
    FourVocations(u32),
    /// "At least level N with all five vocations."
    FiveVocations(u32),
    /// "The account has reached at least level N with the monk."
    Monk(u32),
}

macro_rules! badge_catalogue {
    ($($variant:ident = $id:literal, $name:literal, $rule:expr, $description:literal;)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        pub enum Badge { $($variant = $id,)* }

        impl Badge {
            pub const ALL: [Self; BADGE_COUNT] = [$(Self::$variant,)*];

            pub const fn name(self) -> &'static str {
                match self { $(Self::$variant => $name,)* }
            }

            pub const fn description(self) -> &'static str {
                match self { $(Self::$variant => $description,)* }
            }

            pub const fn rule(self) -> BadgeRule {
                use BadgeRule::*;
                match self { $(Self::$variant => $rule,)* }
            }
        }
    };
}

// Ids 1-14 are Crystal's (`game.cpp:293-306`); the Global-only Monk update grades are appended
// as 22-27 after Crystal's sunset tournament ids 15-21. Global wins over Crystal where they
// differ: "Fledgeling" (Crystal "Fledegeling"), Master Class needs the four original
// vocations (Crystal `player_badge.cpp` counts five) and every "more than" is strict
// (Crystal compares with `>=`).
badge_catalogue! {
    FledgelingHero = 1, "Fledgeling Hero", AccountOlderThan(1),
        "Neat! Your account is older than 1 years.";
    VeteranHero = 2, "Veteran Hero", AccountOlderThan(5),
        "Neat! Your account is older than 5 years.";
    SeniorHero = 3, "Senior Hero", AccountOlderThan(10),
        "Neat! Your account is older than 10 years.";
    AncientHero = 4, "Ancient Hero", AccountOlderThan(15),
        "Neat! Your account is older than 15 years.";
    ExaltedHero = 5, "Exalted Hero", AccountOlderThan(20),
        "Neat! Your account is older than 20 years.";
    TibiaLoyalistGrade1 = 6, "Tibia Loyalist (Grade 1)", LoyaltyAbove(100),
        "Terrific! You earned more than 100 loyalty points.";
    TibiaLoyalistGrade2 = 7, "Tibia Loyalist (Grade 2)", LoyaltyAbove(1000),
        "Terrific! You earned more than 1000 loyalty points.";
    TibiaLoyalistGrade3 = 8, "Tibia Loyalist (Grade 3)", LoyaltyAbove(5000),
        "Terrific! You earned more than 5000 loyalty points.";
    GlobalPlayerGrade1 = 9, "Global Player (Grade 1)", LevelSumAbove(500),
        "Stunning! Summing up the level of all your characters amounts to more than 500.";
    GlobalPlayerGrade2 = 10, "Global Player (Grade 2)", LevelSumAbove(1000),
        "Stunning! Summing up the level of all your characters amounts to more than 1000.";
    GlobalPlayerGrade3 = 11, "Global Player (Grade 3)", LevelSumAbove(2000),
        "Stunning! Summing up the level of all your characters amounts to more than 2000.";
    MasterClassGrade1 = 12, "Master Class (Grade 1)", FourVocations(100),
        "Brilliant! You have reached at least level 100 with all four vocations.";
    MasterClassGrade2 = 13, "Master Class (Grade 2)", FourVocations(250),
        "Brilliant! You have reached at least level 250 with all four vocations.";
    MasterClassGrade3 = 14, "Master Class (Grade 3)", FourVocations(500),
        "Brilliant! You have reached at least level 500 with all four vocations.";
    ExtendedMasterClassGrade1 = 22, "Extended Master Class (Grade 1)", FiveVocations(100),
        "Brilliant! You have reached at least level 100 with all five vocations.";
    ExtendedMasterClassGrade2 = 23, "Extended Master Class (Grade 2)", FiveVocations(250),
        "Brilliant! You have reached at least level 250 with all five vocations.";
    ExtendedMasterClassGrade3 = 24, "Extended Master Class (Grade 3)", FiveVocations(500),
        "Brilliant! You have reached at least level 500 with all five vocations.";
    EnlightenedStudentGrade1 = 25, "Enlightened Student (Grade 1)", Monk(100),
        "The account has reached at least level 100 with the monk.";
    EnlightenedStudentGrade2 = 26, "Enlightened Student (Grade 2)", Monk(250),
        "The account has reached at least level 250 with the monk.";
    EnlightenedStudentGrade3 = 27, "Enlightened Student (Grade 3)", Monk(500),
        "The account has reached at least level 500 with the monk.";
}

/// Global badges with no native owner: Tournaments were sunset (CipSoft, 2021-06-14) and Oteryn
/// has no tournament participation or rating.
pub const NOT_ADMITTED_BADGES: [&str; 7] = [
    "Freshman of the Tournament",
    "Regular of the Tournament",
    "Hero of the Tournament",
    "Tournament Competitor",
    "Tournament Challenger",
    "Tournament Master",
    "Tournament Champion",
];

impl Badge {
    pub const fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|badge| badge.id() == id)
    }

    /// Whether `facts` meet the badge's Global requirement.
    pub fn qualifies(self, facts: BadgeFacts) -> bool {
        let level = |vocation: BaseVocation| facts.vocation_levels[vocation as usize];
        let four = [
            BaseVocation::Druid,
            BaseVocation::Knight,
            BaseVocation::Paladin,
            BaseVocation::Sorcerer,
        ];
        match self.rule() {
            BadgeRule::AccountOlderThan(years) => facts.account_years >= years,
            BadgeRule::LoyaltyAbove(points) => facts.loyalty_points > points,
            BadgeRule::LevelSumAbove(sum) => facts.level_sum > sum,
            BadgeRule::FourVocations(min) => four.into_iter().all(|v| level(v) >= min),
            BadgeRule::FiveVocations(min) => facts.vocation_levels.iter().all(|l| *l >= min),
            BadgeRule::Monk(min) => level(BaseVocation::Monk) >= min,
        }
    }
}

/// Badges are permanent (Global): only badges not yet earned are granted, never revoked.
pub fn newly_earned(facts: BadgeFacts, earned: &[Badge]) -> Vec<Badge> {
    Badge::ALL
        .into_iter()
        .filter(|badge| !earned.contains(badge) && badge.qualifies(facts))
        .collect()
}

/// The badges request is empty: the server shows the requesting account's badges.
pub fn decode_badges_request(payload: &[u8]) -> Result<(), BadgesWireError> {
    if payload.is_empty() {
        Ok(())
    } else {
        Err(BadgesWireError)
    }
}

/// `count:u8` then each earned badge id in ascending order.
pub fn encode_badges(earned: &[Badge]) -> Result<Vec<u8>, BadgesWireError> {
    let count = u8::try_from(earned.len()).map_err(|_| BadgesWireError)?;
    if earned.len() > BADGE_COUNT {
        return Err(BadgesWireError);
    }
    let mut ids: Vec<u8> = earned.iter().map(|badge| badge.id()).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() != earned.len() {
        return Err(BadgesWireError);
    }
    let mut bytes = Vec::with_capacity(1 + ids.len());
    bytes.push(count);
    bytes.extend(ids);
    Ok(bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadgesWireError;

/// The character window tabs with a native owner (Crystal `CYCLOPEDIA_CHARACTERINFO_*`).
/// Offence, Defence and Misc (Crystal `protocolgame.cpp:5016`, `:5197`, `:5265`) are live
/// combat aggregates with no durable owner and are not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CharacterTab {
    GeneralStats = 1,
    RecentDeaths = 2,
    RecentPvpKills = 3,
    Achievements = 4,
    Appearances = 5,
    StoreSummary = 6,
    Inspection = 7,
}

impl CharacterTab {
    pub fn from_id(id: u8) -> Option<Self> {
        [
            Self::GeneralStats,
            Self::RecentDeaths,
            Self::RecentPvpKills,
            Self::Achievements,
            Self::Appearances,
            Self::StoreSummary,
            Self::Inspection,
        ]
        .into_iter()
        .find(|tab| *tab as u8 == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabRequest {
    pub tab: CharacterTab,
    pub page: crate::character_info::PageRequest,
    /// `None` reads the requesting Character.
    pub target: Option<[u8; 16]>,
}

pub fn decode_tab_request(payload: &[u8]) -> Result<TabRequest, TabWireError> {
    let (head, target) = match payload.len() {
        4 => (payload, None),
        20 => {
            let mut target = [0; 16];
            target.copy_from_slice(&payload[4..]);
            (&payload[..4], Some(target))
        }
        _ => return Err(TabWireError),
    };
    let [tab, a, b, limit] = head else {
        return Err(TabWireError);
    };
    Ok(TabRequest {
        tab: CharacterTab::from_id(*tab).ok_or(TabWireError)?,
        page: crate::character_info::PageRequest::new(u16::from_le_bytes([*a, *b]), *limit)
            .ok_or(TabWireError)?,
        target,
    })
}

/// Crystal `player_cyclopedia.cpp:72-80`: "Died at Level {level}" and " by {killer}" for each
/// known killer, joined with " and ". Only Character killers have a durable owner.
pub fn death_cause(level_before: u64, killers: &[String]) -> String {
    let mut cause = format!("Died at Level {level_before}");
    if !killers.is_empty() {
        cause.push_str(" by ");
        cause.push_str(&killers.join(" and "));
    }
    cause
}

/// Crystal `player_cyclopedia.cpp:134`.
pub fn pvp_kill_description(victim: &str) -> String {
    format!("Killed {victim}.")
}

/// Crystal `game_definitions.hpp:107-111`. Justified, assisted and arena kills have no durable
/// owner and are not reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KillStatus {
    Unjustified = 1,
    GuildWar = 2,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GeneralStats {
    pub experience: u64,
    pub level: u32,
    pub magic_level: u16,
    /// Fist, club, sword, axe, distance, shielding, fishing.
    pub skills: [u16; 7],
    pub stamina_minutes: u16,
    pub offline_training_minutes: u16,
    /// The last persisted health and mana, when a session wrote them.
    pub last_vitals: Option<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSummary {
    pub blessings: Vec<String>,
    pub permanent_prey_slots: u8,
    pub prey_wildcards: u8,
    pub hirelings: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquippedItem {
    pub slot: u8,
    pub definition: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabView {
    GeneralStats(GeneralStats),
    RecentDeaths(Vec<(i64, String)>),
    RecentPvpKills(Vec<(i64, KillStatus, String)>),
    Achievements { total: u16, keys: Vec<String> },
    Appearances { total: u16, keys: Vec<String> },
    StoreSummary(StoreSummary),
    Inspection(Vec<EquippedItem>),
}

fn push_text(bytes: &mut Vec<u8>, text: &str) -> Result<(), TabWireError> {
    bytes.push(u8::try_from(text.len()).map_err(|_| TabWireError)?);
    bytes.extend_from_slice(text.as_bytes());
    Ok(())
}

fn push_count(bytes: &mut Vec<u8>, count: usize) -> Result<(), TabWireError> {
    let count = u16::try_from(count).map_err(|_| TabWireError)?;
    bytes.extend_from_slice(&count.to_le_bytes());
    Ok(())
}

impl TabView {
    pub fn tab(&self) -> CharacterTab {
        match self {
            Self::GeneralStats(_) => CharacterTab::GeneralStats,
            Self::RecentDeaths(_) => CharacterTab::RecentDeaths,
            Self::RecentPvpKills(_) => CharacterTab::RecentPvpKills,
            Self::Achievements { .. } => CharacterTab::Achievements,
            Self::Appearances { .. } => CharacterTab::Appearances,
            Self::StoreSummary(_) => CharacterTab::StoreSummary,
            Self::Inspection(_) => CharacterTab::Inspection,
        }
    }

    /// `tab:u8`, then the tab body; integers little-endian, text `len:u8` + UTF-8.
    pub fn encode(&self) -> Result<Vec<u8>, TabWireError> {
        let mut bytes = vec![self.tab() as u8];
        match self {
            Self::GeneralStats(stats) => {
                bytes.extend_from_slice(&stats.experience.to_le_bytes());
                bytes.extend_from_slice(&stats.level.to_le_bytes());
                bytes.extend_from_slice(&stats.magic_level.to_le_bytes());
                for skill in stats.skills {
                    bytes.extend_from_slice(&skill.to_le_bytes());
                }
                bytes.extend_from_slice(&stats.stamina_minutes.to_le_bytes());
                bytes.extend_from_slice(&stats.offline_training_minutes.to_le_bytes());
                let (known, (health, mana)) = match stats.last_vitals {
                    Some(vitals) => (1, vitals),
                    None => (0, (0, 0)),
                };
                bytes.push(known);
                bytes.extend_from_slice(&health.to_le_bytes());
                bytes.extend_from_slice(&mana.to_le_bytes());
            }
            Self::RecentDeaths(entries) => {
                push_count(&mut bytes, entries.len())?;
                for (at, cause) in entries {
                    bytes.extend_from_slice(&at.to_le_bytes());
                    push_text(&mut bytes, cause)?;
                }
            }
            Self::RecentPvpKills(entries) => {
                push_count(&mut bytes, entries.len())?;
                for (at, status, description) in entries {
                    bytes.extend_from_slice(&at.to_le_bytes());
                    bytes.push(*status as u8);
                    push_text(&mut bytes, description)?;
                }
            }
            Self::Achievements { total, keys } | Self::Appearances { total, keys } => {
                bytes.extend_from_slice(&total.to_le_bytes());
                push_count(&mut bytes, keys.len())?;
                for key in keys {
                    push_text(&mut bytes, key)?;
                }
            }
            Self::StoreSummary(summary) => {
                bytes.push(u8::try_from(summary.blessings.len()).map_err(|_| TabWireError)?);
                for blessing in &summary.blessings {
                    push_text(&mut bytes, blessing)?;
                }
                bytes.extend([
                    summary.permanent_prey_slots,
                    summary.prey_wildcards,
                    summary.hirelings,
                ]);
            }
            Self::Inspection(items) => {
                bytes.push(u8::try_from(items.len()).map_err(|_| TabWireError)?);
                for item in items {
                    bytes.push(item.slot);
                    // A production key holds up to 512 bytes, so its length is a `u16`.
                    let len = u16::try_from(item.definition.len()).map_err(|_| TabWireError)?;
                    bytes.extend_from_slice(&len.to_le_bytes());
                    bytes.extend_from_slice(item.definition.as_bytes());
                    bytes.extend_from_slice(&item.quantity.to_le_bytes());
                }
            }
        }
        if bytes.len() > MAX_TAB_RESULT_BYTES {
            return Err(TabWireError);
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabWireError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p12b_catalogue_matches_global_and_partitions_tournaments() {
        let mut names: Vec<&str> = Badge::ALL.iter().map(|badge| badge.name()).collect();
        names.extend(NOT_ADMITTED_BADGES);
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 27, "Global lists 27 badges");
        for badge in Badge::ALL {
            assert_eq!(Badge::from_id(badge.id()), Some(badge));
        }
        assert_eq!(Badge::from_id(15), None, "tournament ids are not admitted");
        assert_eq!(Badge::FledgelingHero.name(), "Fledgeling Hero");
    }

    #[test]
    fn p12b_rules_use_global_thresholds() {
        let none = BadgeFacts::default();
        assert!(newly_earned(none, &[]).is_empty());
        let facts = BadgeFacts {
            account_years: 5,
            loyalty_points: 100,
            level_sum: 501,
            vocation_levels: [100, 100, 99, 100, 100],
        };
        assert_eq!(
            newly_earned(facts, &[Badge::VeteranHero]),
            [
                Badge::FledgelingHero,
                Badge::GlobalPlayerGrade1,
                Badge::MasterClassGrade1
            ],
            "100 loyalty points is not more than 100; a level-99 monk misses five vocations"
        );
        let monk = BadgeFacts {
            vocation_levels: [0, 0, 250, 0, 0],
            ..BadgeFacts::default()
        };
        assert_eq!(
            newly_earned(monk, &[]),
            [
                Badge::EnlightenedStudentGrade1,
                Badge::EnlightenedStudentGrade2
            ]
        );
    }

    #[test]
    fn p12b_badges_wire_is_bounded() {
        assert_eq!(decode_badges_request(&[]), Ok(()));
        assert_eq!(decode_badges_request(&[0]), Err(BadgesWireError));
        assert_eq!(
            encode_badges(&[Badge::MasterClassGrade1, Badge::FledgelingHero]),
            Ok(vec![2, 1, 12])
        );
        assert_eq!(
            encode_badges(&[Badge::FledgelingHero, Badge::FledgelingHero]),
            Err(BadgesWireError)
        );
        let all = encode_badges(&Badge::ALL).unwrap_or_default();
        assert_eq!(all.len(), MAX_BADGES_RESULT_BYTES);
    }

    #[test]
    fn p12c_tab_requests_are_bounded() {
        let request = decode_tab_request(&[2, 0, 0, 10]);
        assert!(matches!(
            request,
            Ok(TabRequest {
                tab: CharacterTab::RecentDeaths,
                target: None,
                ..
            })
        ));
        let mut visible = vec![7, 0, 0, 1];
        visible.extend_from_slice(&[9; 16]);
        assert!(matches!(
            decode_tab_request(&visible),
            Ok(TabRequest {
                tab: CharacterTab::Inspection,
                target: Some(target),
                ..
            }) if target == [9; 16]
        ));
        assert_eq!(visible.len(), MAX_TAB_REQUEST_BYTES);
        for invalid in [
            &[][..],
            &[0, 0, 0, 1],
            &[8, 0, 0, 1],
            &[2, 0, 0, 0],
            &[2, 0, 0, 51],
        ] {
            assert_eq!(decode_tab_request(invalid), Err(TabWireError));
        }
    }

    #[test]
    fn p12c_cause_text_follows_crystal() {
        assert_eq!(death_cause(100, &[]), "Died at Level 100");
        assert_eq!(
            death_cause(100, &["Alice".into(), "Bob".into()]),
            "Died at Level 100 by Alice and Bob"
        );
        assert_eq!(pvp_kill_description("Bob"), "Killed Bob.");
    }

    #[test]
    fn p12c_tabs_encode_with_their_bodies() {
        let deaths = TabView::RecentDeaths(vec![(5, "Died at Level 8".into())]);
        let mut expected = vec![2, 1, 0, 5, 0, 0, 0, 0, 0, 0, 0, 15];
        expected.extend_from_slice(b"Died at Level 8");
        assert_eq!(deaths.encode(), Ok(expected));
        let kills = TabView::RecentPvpKills(vec![(1, KillStatus::GuildWar, "Killed A.".into())]);
        assert_eq!(kills.encode().map(|bytes| bytes[11]), Ok(2));
        let stats = TabView::GeneralStats(GeneralStats::default()).encode();
        assert_eq!(
            stats.map(|bytes| bytes.len()),
            Ok(1 + 8 + 4 + 2 + 14 + 4 + 1 + 8)
        );
        let worst = TabView::RecentPvpKills(
            (0..50)
                .map(|at| (at, KillStatus::Unjustified, "x".repeat(255)))
                .collect(),
        );
        assert!(
            worst.encode().is_ok(),
            "a full page fits the registered bound"
        );
        assert_eq!(
            TabView::Achievements {
                total: 1,
                keys: vec!["k".repeat(256)]
            }
            .encode(),
            Err(TabWireError)
        );
    }
}
