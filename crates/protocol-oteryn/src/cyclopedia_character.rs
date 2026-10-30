//! PAR-P12b/c Cyclopedia character window tabs (Crystal `protocolgame.cpp`
//! `sendCyclopediaCharacter*`, `:4341-5265`). The server owns every fact; the request only
//! names the tab. Payload schemas: `docs/contracts/protocol-oteryn/v1/cyclopedia-character-v1.md`.

pub const CAPABILITY_CYCLOPEDIA_CHARACTER_V1: u32 = 199;
pub const COMMAND_TYPE_CYCLOPEDIA_BADGES: u32 = 387;
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
}
