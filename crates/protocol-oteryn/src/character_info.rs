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

/// One catalogue row: the enum, its Global name, female form, permanence, rule and in-game
/// description, all from Tibia Global (TibiaWiki `Cyclopedia#Titles`, revid 1197515).
macro_rules! title_catalogue {
    ($($variant:ident = $id:literal, $male:literal, $female:expr, $permanent:literal, $rule:expr,
        $description:literal;)*) => {
        /// Admitted title catalogue. The wire and stored ids are Oteryn-owned and stable
        /// (CHARACTER-INFO-1, migration 0226): Global publishes no title ids, and the reference
        /// reimplementations number the same title differently (Globetrotter is 75 in Crystal
        /// and 74 in Canary, and neither has the Adept titles), so no external numbering is
        /// adopted. Ids 1-5 keep their meanings; ids 6.. follow the Global table order.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(u16)]
        pub enum CharacterTitle {
            $($variant = $id,)*
        }

        impl CharacterTitle {
            pub const ALL: [Self; TITLE_COUNT] = [$(Self::$variant,)*];

            /// Global display name (the male form where Global has two).
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $male,)*
                }
            }

            /// Global female form, where Global names one.
            pub const fn female_name(self) -> Option<&'static str> {
                match self {
                    $(Self::$variant => $female,)*
                }
            }

            /// Global "Permanent?" column.
            pub const fn permanent(self) -> bool {
                match self {
                    $(Self::$variant => $permanent,)*
                }
            }

            pub const fn rule(self) -> TitleRule {
                use TitleRule::*;
                match self {
                    $(Self::$variant => $rule,)*
                }
            }

            /// Global in-game description.
            pub const fn description(self) -> &'static str {
                match self {
                    $(Self::$variant => $description,)*
                }
            }
        }
    };
}

pub const TITLE_COUNT: usize = 43;

title_catalogue! {
    AdeptArmorer = 1, "Adept Armorer", None, true, WeaponMastery,
        "Reached Mastery with a weapon at level 3 proficiency.";
    AdeptEnhancer = 2, "Adept Enhancer", None, false, AllGradeThreeMods,
        "All mods enhanced to Grade III.";
    Globetrotter = 3, "Globetrotter", None, false, AllMapAreas,
        "Explored all map areas.";
    ReigningDromeChampion = 4, "Reigning Drome Champion", None, false, DromeRota,
        "Finished most recent rota ranked in the highest ranks.";
    PensiveWanderer = 5, "Pensive Wanderer", None, true, MonkPilgrimage,
        "You have honoured the Merudri by visiting all of their shrines and completing the ancient Three-Fold Path.";
    Aeternal = 6, "Aeternal", None, true, CharacterDecade,
        "Awarded exclusively to stalwart heroes keeping the faith under all circumstances.";
    ApexPredator = 7, "Apex Predator", None, false, WorldLead(1),
        "Highest Level on character's world.";
    BossDestroyer = 8, "Boss Destroyer", None, true, BossesUnlocked(BossCategory::Archfoe, 10),
        "Unlocked 10 or more Archfoe bosses.";
    BossDevastator = 9, "Boss Devastator", None, true, BossesUnlocked(BossCategory::Nemesis, 10),
        "Unlocked 10 or more Nemesis bosses.";
    BossHunter = 10, "Boss Hunter", None, true, BossesUnlocked(BossCategory::Bane, 10),
        "Unlocked 10 or more Bane bosses.";
    BossObliterator = 11, "Boss Obliterator", None, true, BossesUnlocked(BossCategory::Nemesis, 40),
        "Unlocked 40 or more Nemesis bosses.";
    BossSmiter = 12, "Boss Smiter", None, true, BossesUnlocked(BossCategory::Archfoe, 40),
        "Unlocked 40 or more Archfoe bosses.";
    BossVeteran = 13, "Boss Veteran", None, true, BossesUnlocked(BossCategory::Bane, 40),
        "Unlocked 40 or more Bane bosses.";
    CreatureOfHabitGrade1 = 14, "Creature of Habit (Grade 1)", None, true, LoginStreak(7),
        "Reward Streak of at least 7 days of consecutive logins.";
    CreatureOfHabitGrade2 = 15, "Creature of Habit (Grade 2)", None, true, LoginStreak(30),
        "Reward Streak of at least 30 days of consecutive logins.";
    CreatureOfHabitGrade3 = 16, "Creature of Habit (Grade 3)", None, true, LoginStreak(90),
        "Reward Streak of at least 90 days of consecutive logins.";
    CreatureOfHabitGrade4 = 17, "Creature of Habit (Grade 4)", None, true, LoginStreak(180),
        "Reward Streak of at least 180 days of consecutive logins.";
    CreatureOfHabitGrade5 = 18, "Creature of Habit (Grade 5)", None, true, LoginStreak(365),
        "Reward Streak of at least 365 days of consecutive logins.";
    CrystalHoarder = 19, "Crystal Hoarder", None, false, BankBalance(100_000_000),
        "Earned at least 100,000,000 gold.";
    Cyclopscamper = 20, "Cyclopscamper", None, false, Level(100),
        "Reached level 100.";
    Demondoom = 21, "Demondoom", None, false, Level(300),
        "Reached level 300.";
    DoomsdayNemesis = 22, "Doomsday Nemesis", None, true, TrackedKills(TRACKED_GAZ_HARAGOTH, 1),
        "Awarded for great help in the battle against Gaz'haragoth.";
    Dragondouser = 23, "Dragondouser", None, false, Level(200),
        "Reached level 200.";
    Drakenbane = 24, "Drakenbane", None, false, Level(400),
        "Reached level 400.";
    Exalted = 25, "Exalted", None, false, Level(1000),
        "Reached level 1000.";
    GoldHoarder = 26, "Gold Hoarder", None, false, BankBalance(1_000_000),
        "Earned at least 1,000,000 gold.";
    GuildLeader = 27, "Guild Leader", None, false, GuildLeader,
        "Leading a Guild.";
    KingOfDemon = 28, "King of Demon", Some("Queen of Demon"), true, TrackedKills(TRACKED_MORSHABAAL, 5),
        "Defeat Morshabaal 5 times.";
    LegendOfFishing = 29, "Legend of Fishing", None, false, WorldLead(9),
        "Highest fishing level on character's world.";
    LegendOfMagic = 30, "Legend of Magic", None, false, WorldLead(2),
        "Highest magic level on character's world.";
    LegendOfMarksmanship = 31, "Legend of Marksmanship", None, false, WorldLead(7),
        "Highest distance fighting level on character's world.";
    LegendOfTheAxe = 32, "Legend of the Axe", None, false, WorldLead(6),
        "Highest axe fighting level on character's world.";
    LegendOfTheClub = 33, "Legend of the Club", None, false, WorldLead(4),
        "Highest club fighting level on character's world.";
    LegendOfTheFist = 34, "Legend of the Fist", None, false, WorldLead(3),
        "Highest fist fighting level on character's world.";
    LegendOfTheShield = 35, "Legend of the Shield", None, false, WorldLead(8),
        "Highest Shielding level on character's world.";
    LegendOfTheSword = 36, "Legend of the Sword", None, false, WorldLead(5),
        "Highest sword fighting level on character's world.";
    MidnightHunter = 37, "Midnight Hunter", None, true, TrackedKills(TRACKED_MIDNIGHT_PANTHER, 25),
        "When the hunter becomes the hunted, perseverance decides the game.";
    PlatinumHoarder = 38, "Platinum Hoarder", None, false, BankBalance(10_000_000),
        "Earned at least 10,000,000 gold.";
    PrinceCharming = 39, "Prince Charming", Some("Princess Charming"), false, WorldLead(11),
        "Highest score of accumulated charm points on character's world.";
    Ratinator = 40, "Ratinator", None, true, TrackedKills(TRACKED_CAVE_RAT, 10_000),
        "Killing some snarky cave rats is helpful, killing over ten thousand of them is a statement.";
    Silencer = 41, "Silencer", None, false, Level(500),
        "Reached level 500.";
    Trolltrasher = 42, "Trolltrasher", None, false, Level(50),
        "Reached level 50.";
    TrulyBoss = 43, "Truly Boss", None, true, BossPoints(15_000),
        "Challenging leagues of foes and prevailing against all odds is truly boss.";
}

impl CharacterTitle {
    /// The `u8` wire id, which is also the stored `SMALLINT` value.
    pub const fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|title| title.id() == id)
    }

    /// The name a character of this sex displays (Crystal `PlayerTitle::getNameBySex`,
    /// `player_title.cpp:119`): the female form when one exists.
    pub fn name_for(self, female: bool) -> &'static str {
        match self.female_name() {
            Some(name) if female => name,
            _ => self.name(),
        }
    }
}

/// Bosstiary categories in their `game_character_bosstiary_kill_receipts.category` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossCategory {
    Bane = 0,
    Archfoe = 1,
    Nemesis = 2,
}

/// Creatures whose kill count a title names; `TitleFacts::tracked_kills` follows this order.
pub const TRACKED_CREATURES: [&str; 4] = [
    "oteryn:creature.midnight_panther",
    "oteryn:creature.cave_rat",
    "oteryn:creature.gaz_haragoth",
    "oteryn:creature.morshabaal",
];
const TRACKED_MIDNIGHT_PANTHER: usize = 0;
const TRACKED_CAVE_RAT: usize = 1;
const TRACKED_GAZ_HARAGOTH: usize = 2;
const TRACKED_MORSHABAAL: usize = 3;

/// The unlock predicate of an admitted title, evaluated against `TitleFacts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleRule {
    MonkPilgrimage,
    WeaponMastery,
    AllGradeThreeMods,
    AllMapAreas,
    DromeRota,
    /// Crystal `checkGold`, `player_title.cpp:191`: bank balance on the character's World.
    BankBalance(i64),
    /// Crystal `checkLevel`, `player_title.cpp:209`.
    Level(u32),
    /// Rank 1 of the current World highscore snapshot for this category (0129 categories).
    WorldLead(u8),
    /// Crystal `checkLoginStreak`, `player_title.cpp:270`: the daily reward streak.
    LoginStreak(u32),
    /// Crystal `checkOther("Guild Leader")`, `player_title.cpp:286`.
    GuildLeader,
    /// Kills of one `TRACKED_CREATURES` entry (Bestiary or Bosstiary count).
    TrackedKills(usize, u32),
    /// Accumulated Bosstiary boss points.
    BossPoints(i64),
    /// Bosses of a category with an unlocked Bosstiary stage (Prowess or higher).
    BossesUnlocked(BossCategory, u32),
    /// Global note for Aeternal: the character is at least ten years old.
    CharacterDecade,
}

/// Why a Global title is not admitted: Oteryn has no durable fact for its predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAdmitted {
    /// Appearance unlock keys are opaque grant keys; no durable outfit/mount/addon projection.
    Appearance,
    /// No durable Bestiary class catalogue; the class exists only in creature authoring.
    BestiaryClass,
    /// No durable per-category Bosstiary catalogue size to compare against.
    BosstiaryTotal,
    /// No World highscore category for the value (0129 publishes categories 1..11).
    NoHighscore,
    /// No hunting-task point owner.
    TaskPoints,
    /// Charm points are derived in the domain and not durable.
    CharmPoints,
    /// Map progress stores no explored percentage.
    MapPercent,
    /// No completion fact for the quest the title names.
    Quest,
    /// No Tibiadrome wave owner.
    Tibiadrome,
    /// No fact for the encounter condition (boss difficulty, burdens, attire, ovations).
    Encounter,
    /// No mod grade, weapon proficiency or animus mastery fact.
    Proficiency,
}

/// The 69 Global titles outside the catalogue, with the missing fact. Together with
/// `CharacterTitle::ALL` they are the 112 titles of TibiaWiki revid 1197515.
pub const NOT_ADMITTED_TITLES: [(&str, NotAdmitted); 69] = [
    ("Admirer of the Crown", NotAdmitted::Appearance),
    ("Armor Breaker", NotAdmitted::Encounter),
    ("Aspiring Huntsman", NotAdmitted::TaskPoints),
    ("Beastly", NotAdmitted::CharmPoints),
    ("Beaststrider (Grade 1)", NotAdmitted::Appearance),
    ("Beaststrider (Grade 2)", NotAdmitted::Appearance),
    ("Beaststrider (Grade 3)", NotAdmitted::Appearance),
    ("Beaststrider (Grade 4)", NotAdmitted::Appearance),
    ("Beaststrider (Grade 5)", NotAdmitted::Appearance),
    ("Big Boss", NotAdmitted::NoHighscore),
    ("Big Spender", NotAdmitted::Appearance),
    ("Bipedantic", NotAdmitted::BestiaryClass),
    ("Blood Moon Hunter", NotAdmitted::BestiaryClass),
    ("Boss Annihilator", NotAdmitted::BosstiaryTotal),
    ("Boss Eraser", NotAdmitted::BosstiaryTotal),
    ("Boss Executioner", NotAdmitted::BosstiaryTotal),
    ("Boss Slayer", NotAdmitted::BosstiaryTotal),
    ("Bringer of Rain", NotAdmitted::Tibiadrome),
    ("Challenger of the Iks", NotAdmitted::Encounter),
    ("Chompmeister", NotAdmitted::Quest),
    ("Cleansing Conqueror", NotAdmitted::Quest),
    ("Coldblooded", NotAdmitted::BestiaryClass),
    ("Competent Beastslayer", NotAdmitted::TaskPoints),
    ("Death from Below", NotAdmitted::BestiaryClass),
    ("Dedicated Entrepreneur", NotAdmitted::MapPercent),
    ("Demonator", NotAdmitted::BestiaryClass),
    ("Djinn Dominator", NotAdmitted::Encounter),
    ("Dragonslayer", NotAdmitted::BestiaryClass),
    ("Eclipse Bringer", NotAdmitted::Encounter),
    ("Elementalist", NotAdmitted::BestiaryClass),
    ("Executioner", NotAdmitted::BestiaryClass),
    ("Exterminator", NotAdmitted::BestiaryClass),
    ("Feared Bountyhunter", NotAdmitted::TaskPoints),
    ("Fey Swatter", NotAdmitted::BestiaryClass),
    ("Ghosthunter", NotAdmitted::BestiaryClass),
    ("Handyman", NotAdmitted::BestiaryClass),
    ("Hero of Bounac", NotAdmitted::Quest),
    ("Huntsman", NotAdmitted::BestiaryClass),
    ("Ink Eraser", NotAdmitted::BestiaryClass),
    ("Interdimensional Destroyer", NotAdmitted::BestiaryClass),
    ("Jack of all Taints", NotAdmitted::NoHighscore),
    ("Lamp Breaker", NotAdmitted::Encounter),
    ("Light Reaver", NotAdmitted::Encounter),
    ("Manhunter", NotAdmitted::BestiaryClass),
    ("Master Enhancer", NotAdmitted::Proficiency),
    ("Master of Illusion", NotAdmitted::BestiaryClass),
    ("Master of Souls", NotAdmitted::Proficiency),
    ("Master of the Arsenal", NotAdmitted::Proficiency),
    ("Novice Enhancer", NotAdmitted::Proficiency),
    ("Ooze Blues", NotAdmitted::BestiaryClass),
    ("Pit Fighter", NotAdmitted::Encounter),
    ("Planegazer", NotAdmitted::Quest),
    ("Proconsul of Iksupan", NotAdmitted::Quest),
    ("Robinson Crusoe", NotAdmitted::Quest),
    ("Royal Bounacean Advisor", NotAdmitted::Appearance),
    ("Sea Bane", NotAdmitted::BestiaryClass),
    ("Seasoned Armorer", NotAdmitted::Proficiency),
    ("Snake Charmer", NotAdmitted::BestiaryClass),
    ("Tibia's Topmodel (Grade 1)", NotAdmitted::Appearance),
    ("Tibia's Topmodel (Grade 2)", NotAdmitted::Appearance),
    ("Tibia's Topmodel (Grade 3)", NotAdmitted::Appearance),
    ("Tibia's Topmodel (Grade 4)", NotAdmitted::Appearance),
    ("Tibia's Topmodel (Grade 5)", NotAdmitted::Appearance),
    ("Time Traveller", NotAdmitted::Quest),
    ("Tumbler", NotAdmitted::BestiaryClass),
    ("Unbreakable Purifier", NotAdmitted::Encounter),
    ("Usurper of the Demon Throne", NotAdmitted::Encounter),
    ("Veteran Armorer", NotAdmitted::Proficiency),
    ("Weedkiller", NotAdmitted::BestiaryClass),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleLifetime {
    Permanent,
    Conditional,
    Until(i64),
}

/// Observations from the native owners; never persisted as title truth.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TitleFacts {
    pub monk_pilgrimage: bool,
    pub weapon_mastery: bool,
    pub all_grade_three_mods: bool,
    pub all_map_areas_discovered: bool,
    /// Ranked in the top five of the most recent Tibiadrome rota.
    pub drome_rota_top_five: bool,
    pub next_rotation_unix_ms: Option<i64>,
    pub bank_balance: i64,
    pub level: u32,
    /// Bit `n` is set when the character holds rank 1 of World highscore category `n`.
    pub world_leads: u16,
    pub login_streak: u32,
    pub guild_leader: bool,
    pub boss_points: i64,
    /// Unlocked bosses per `BossCategory`.
    pub bosses_unlocked: [u32; 3],
    /// Kill counts in `TRACKED_CREATURES` order.
    pub tracked_kills: [u32; 4],
    pub character_decade: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Qualification {
    pub eligible: bool,
    pub lifetime: TitleLifetime,
}

pub fn qualify(title: CharacterTitle, facts: TitleFacts) -> Qualification {
    let eligible = match title.rule() {
        TitleRule::MonkPilgrimage => facts.monk_pilgrimage,
        TitleRule::WeaponMastery => facts.weapon_mastery,
        TitleRule::AllGradeThreeMods => facts.all_grade_three_mods,
        TitleRule::AllMapAreas => facts.all_map_areas_discovered,
        TitleRule::DromeRota => facts.drome_rota_top_five && facts.next_rotation_unix_ms.is_some(),
        TitleRule::BankBalance(amount) => facts.bank_balance >= amount,
        TitleRule::Level(level) => facts.level >= level,
        TitleRule::WorldLead(category) => category < 16 && facts.world_leads & (1 << category) != 0,
        TitleRule::LoginStreak(days) => facts.login_streak >= days,
        TitleRule::GuildLeader => facts.guild_leader,
        TitleRule::TrackedKills(index, kills) => facts
            .tracked_kills
            .get(index)
            .is_some_and(|count| *count >= kills),
        TitleRule::BossPoints(points) => facts.boss_points >= points,
        TitleRule::BossesUnlocked(category, count) => {
            facts.bosses_unlocked[category as usize] >= count
        }
        TitleRule::CharacterDecade => facts.character_decade,
    };
    let lifetime = match title.rule() {
        // Global: not permanent, held for the rota after a top-five finish.
        TitleRule::DromeRota => facts
            .next_rotation_unix_ms
            .map_or(TitleLifetime::Until(0), TitleLifetime::Until),
        _ if title.permanent() => TitleLifetime::Permanent,
        // Global "Permanent? no": revoked once the predicate no longer holds.
        _ => TitleLifetime::Conditional,
    };
    Qualification { eligible, lifetime }
}

/// The titles to grant now: eligible and not yet earned.
pub fn newly_eligible(
    facts: TitleFacts,
    earned: &[CharacterTitle],
    now_unix_ms: i64,
) -> Vec<CharacterTitle> {
    CharacterTitle::ALL
        .into_iter()
        .filter(|title| !earned.contains(title) && valid_at(qualify(*title, facts), now_unix_ms))
        .collect()
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
    fn monk_pilgrimage_title_has_an_appended_id_and_requires_the_earned_flag() {
        assert_eq!(CharacterTitle::ALL.len(), TITLE_COUNT);
        assert_eq!(CharacterTitle::PensiveWanderer.id(), 5);
        assert_eq!(CharacterTitle::PensiveWanderer.name(), "Pensive Wanderer");
        assert!(!qualify(CharacterTitle::PensiveWanderer, TitleFacts::default()).eligible);
        let qualified = qualify(
            CharacterTitle::PensiveWanderer,
            TitleFacts {
                monk_pilgrimage: true,
                ..TitleFacts::default()
            },
        );
        assert!(qualified.eligible);
        assert_eq!(qualified.lifetime, TitleLifetime::Permanent);
    }

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
        let oteryn = [
            CharacterTitle::AdeptArmorer,
            CharacterTitle::AdeptEnhancer,
            CharacterTitle::Globetrotter,
            CharacterTitle::ReigningDromeChampion,
        ];
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
        assert_eq!(
            CharacterTitle::from_id(5),
            Some(CharacterTitle::PensiveWanderer)
        );
        assert_eq!(CharacterTitle::from_id(6), Some(CharacterTitle::Aeternal));
        assert_eq!(CharacterTitle::from_id(44), None);
        let all = TitleFacts {
            monk_pilgrimage: true,
            weapon_mastery: true,
            all_grade_three_mods: true,
            all_map_areas_discovered: true,
            drome_rota_top_five: true,
            next_rotation_unix_ms: Some(10),
            ..TitleFacts::default()
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

    /// PAR-P12a: the admitted catalogue and the not-admitted list partition the 112 Global
    /// titles of TibiaWiki `Cyclopedia#Titles` revid 1197515, with no name twice.
    #[test]
    fn p12a_catalogue_partitions_the_global_titles() {
        let mut names: Vec<&str> = CharacterTitle::ALL
            .iter()
            .map(|title| title.name())
            .collect();
        names.extend(NOT_ADMITTED_TITLES.iter().map(|(name, _)| *name));
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
        assert_eq!(total, 112);
        for (index, title) in CharacterTitle::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(title.id()), index + 1, "ids are dense");
        }
        assert_eq!(CharacterTitle::KingOfDemon.name_for(true), "Queen of Demon");
        assert_eq!(
            CharacterTitle::PrinceCharming.name_for(true),
            "Princess Charming"
        );
        assert_eq!(CharacterTitle::Exalted.name_for(true), "Exalted");
    }

    /// PAR-P12a thresholds: Global levels 50..1000 (Crystal `game.cpp:339-345`), gold
    /// 1M/10M/100M (`game.cpp:323-325`), streaks 7..365 (`game.cpp:387-391`), Global notes for
    /// Midnight Hunter 25, Ratinator 10,000, King of Demon 5 and Truly Boss 15,000.
    #[test]
    fn p12a_rules_unlock_at_their_global_thresholds() {
        let at = |facts: TitleFacts, title| qualify(title, facts).eligible;
        let base = TitleFacts::default();
        let level = |level| TitleFacts { level, ..base };
        assert!(!at(level(49), CharacterTitle::Trolltrasher));
        assert!(at(level(50), CharacterTitle::Trolltrasher));
        assert!(!at(level(999), CharacterTitle::Exalted));
        assert!(at(level(1000), CharacterTitle::Exalted));
        let gold = |bank_balance| TitleFacts {
            bank_balance,
            ..base
        };
        assert!(at(gold(1_000_000), CharacterTitle::GoldHoarder));
        assert!(!at(gold(9_999_999), CharacterTitle::PlatinumHoarder));
        assert!(at(gold(100_000_000), CharacterTitle::CrystalHoarder));
        let streak = |login_streak| TitleFacts {
            login_streak,
            ..base
        };
        assert!(!at(streak(6), CharacterTitle::CreatureOfHabitGrade1));
        assert!(at(streak(365), CharacterTitle::CreatureOfHabitGrade5));
        let kills = |tracked_kills| TitleFacts {
            tracked_kills,
            ..base
        };
        assert!(at(kills([25, 0, 0, 0]), CharacterTitle::MidnightHunter));
        assert!(!at(kills([24, 0, 0, 0]), CharacterTitle::MidnightHunter));
        assert!(at(kills([0, 10_000, 0, 0]), CharacterTitle::Ratinator));
        assert!(at(kills([0, 0, 1, 0]), CharacterTitle::DoomsdayNemesis));
        assert!(!at(kills([0, 0, 0, 4]), CharacterTitle::KingOfDemon));
        assert!(at(kills([0, 0, 0, 5]), CharacterTitle::KingOfDemon));
        assert!(at(
            TitleFacts {
                boss_points: 15_000,
                ..base
            },
            CharacterTitle::TrulyBoss
        ));
        let bosses = |bosses_unlocked| TitleFacts {
            bosses_unlocked,
            ..base
        };
        assert!(at(bosses([10, 0, 0]), CharacterTitle::BossHunter));
        assert!(!at(bosses([0, 39, 0]), CharacterTitle::BossSmiter));
        assert!(at(bosses([0, 0, 40]), CharacterTitle::BossObliterator));
        let lead = |category: u8| TitleFacts {
            world_leads: 1 << category,
            ..base
        };
        assert!(at(lead(1), CharacterTitle::ApexPredator));
        assert!(at(lead(9), CharacterTitle::LegendOfFishing));
        assert!(!at(lead(9), CharacterTitle::LegendOfMagic));
        assert!(at(
            TitleFacts {
                guild_leader: true,
                ..base
            },
            CharacterTitle::GuildLeader
        ));
        assert!(at(
            TitleFacts {
                character_decade: true,
                ..base
            },
            CharacterTitle::Aeternal
        ));
        // Global permanence: a level title is revoked, a streak title stays.
        assert_eq!(
            qualify(CharacterTitle::Exalted, base).lifetime,
            TitleLifetime::Conditional
        );
        assert_eq!(
            qualify(CharacterTitle::CreatureOfHabitGrade1, base).lifetime,
            TitleLifetime::Permanent
        );
    }

    /// PAR-P12a: the automatic check grants every newly eligible title once (Crystal
    /// `checkAndUpdateNewTitles`, `player_title.cpp:123`).
    #[test]
    fn p12a_newly_eligible_skips_earned_titles() {
        let facts = TitleFacts {
            level: 100,
            ..TitleFacts::default()
        };
        assert_eq!(
            newly_eligible(facts, &[], 1),
            [CharacterTitle::Cyclopscamper, CharacterTitle::Trolltrasher]
        );
        assert_eq!(
            newly_eligible(facts, &[CharacterTitle::Trolltrasher], 1),
            [CharacterTitle::Cyclopscamper]
        );
        assert!(newly_eligible(TitleFacts::default(), &[], 1).is_empty());
    }
}
