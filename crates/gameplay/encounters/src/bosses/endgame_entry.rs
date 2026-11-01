//! W10/W11 Quest prerequisites consumed by the existing shared boss activity owner.
//!
//! This is not an admission authority. A qualified snapshot creates no Instance, lease,
//! cooldown, spawn or reward. The W02 owner must revalidate current participants and Quest
//! revisions inside its existing admission transaction before using the snapshot.
//!
//! Crystal 201189f: actions/bosses_levers/the_primal_manace.lua:6-12;
//! quests/soul_war/moveevent-soul_war_entrances.lua:18-32,60-82;
//! quests/rotten_blood_quest/movements_blood_entrance.lua:20-32.

use std::future::Future;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestFamily {
    PrimalOrdeal,
    SoulWar,
    RottenBlood,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Encounter {
    boss_key: &'static str,
    family: QuestFamily,
    final_boss: bool,
}

impl Encounter {
    pub fn from_boss_key(key: &str) -> Option<Self> {
        let (boss_key, family, final_boss) = match key {
            "oteryn:creature.the_primal_menace" => (
                "oteryn:creature.the_primal_menace",
                QuestFamily::PrimalOrdeal,
                true,
            ),
            "oteryn:creature.goshnar_s_cruelty" => (
                "oteryn:creature.goshnar_s_cruelty",
                QuestFamily::SoulWar,
                false,
            ),
            "oteryn:creature.goshnar_s_greed" => (
                "oteryn:creature.goshnar_s_greed",
                QuestFamily::SoulWar,
                false,
            ),
            "oteryn:creature.goshnar_s_hatred" => (
                "oteryn:creature.goshnar_s_hatred",
                QuestFamily::SoulWar,
                false,
            ),
            "oteryn:creature.goshnar_s_malice" => (
                "oteryn:creature.goshnar_s_malice",
                QuestFamily::SoulWar,
                false,
            ),
            "oteryn:creature.goshnar_s_spite" => (
                "oteryn:creature.goshnar_s_spite",
                QuestFamily::SoulWar,
                false,
            ),
            // Entry always names the source's initial purple phase. Blue/green
            // remain valid native terminal identities in the separate kill owner.
            "oteryn:creature.goshnar_s_megalomania_blue"
            | "oteryn:creature.goshnar_s_megalomania_green"
            | "oteryn:creature.goshnar_s_megalomania_purple" => (
                "oteryn:creature.goshnar_s_megalomania_purple",
                QuestFamily::SoulWar,
                true,
            ),
            "oteryn:creature.ichgahal" => {
                ("oteryn:creature.ichgahal", QuestFamily::RottenBlood, false)
            }
            "oteryn:creature.murcion" => {
                ("oteryn:creature.murcion", QuestFamily::RottenBlood, false)
            }
            "oteryn:creature.vemiath" => {
                ("oteryn:creature.vemiath", QuestFamily::RottenBlood, false)
            }
            "oteryn:creature.chagorz" => {
                ("oteryn:creature.chagorz", QuestFamily::RottenBlood, false)
            }
            "oteryn:creature.bakragore" => {
                ("oteryn:creature.bakragore", QuestFamily::RottenBlood, true)
            }
            _ => return None,
        };
        Some(Self {
            boss_key,
            family,
            final_boss,
        })
    }

    pub const fn boss_key(self) -> &'static str {
        self.boss_key
    }
    pub const fn family(self) -> QuestFamily {
        self.family
    }
    pub const fn is_final(self) -> bool {
        self.final_boss
    }
}

/// Server-resolved current Quest facts; never accepted from a client packet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntryFacts {
    pub character: [u8; 16],
    pub character_revision: u64,
    pub level: u32,
    pub access: bool,
    pub chain_ready: bool,
}

/// Source-authored configuration forwarded to W02; this is not a local clock,
/// cooldown writer, lever admission proof or second encounter engine. The source
/// floor is the positive Tibia map floor, before native-coordinate conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceEntryPolicy {
    pub lever: (u16, u16, i8),
    pub retry_seconds: u64,
}

fn source_entry_policy(encounter: Encounter) -> Option<SourceEntryPolicy> {
    // Crystal 201189f quests/rotten_blood_quest/rotten_{ichgahal,vemiath}.lua:22
    // inherit boss_lever.lua:64 -> config.lua.dist:658 (20h). Murcion:6,30 and
    // Chagorz:6,31 explicitly use 20h. Bakragore:7,23 is the accepted 68h path
    // (PR2784 ANSWER6547488765); rotten_bakragore1.lua's duplicate 20h lever at
    // (33079,32398,15) has no qualified profile here.
    // Primal: actions/bosses_levers/the_primal_manace.lua:23, same default20h.
    // Soul: lib/quests/soul_war.lua:288-293 lever positions; 330,353,392,415,443
    // select20h (Greed's later392 overrides372); final452,471 starts purple/72h.
    let (lever, retry_seconds) = match encounter.boss_key {
        "oteryn:creature.the_primal_menace" => ((33547, 32752, 14), 72_000),
        "oteryn:creature.goshnar_s_malice" => ((33678, 31599, 14), 72_000),
        "oteryn:creature.goshnar_s_spite" => ((33773, 31634, 14), 72_000),
        "oteryn:creature.goshnar_s_greed" => ((33775, 31665, 14), 72_000),
        "oteryn:creature.goshnar_s_hatred" => ((33772, 31601, 14), 72_000),
        "oteryn:creature.goshnar_s_cruelty" => ((33853, 31854, 6), 72_000),
        "oteryn:creature.goshnar_s_megalomania_purple" => ((33675, 31634, 14), 259_200),
        "oteryn:creature.ichgahal" => ((32979, 32333, 15), 72_000),
        "oteryn:creature.murcion" => ((32979, 32365, 15), 72_000),
        "oteryn:creature.vemiath" => ((33079, 32333, 15), 72_000),
        "oteryn:creature.chagorz" => ((33079, 32367, 15), 72_000),
        "oteryn:creature.bakragore" => ((33015, 32392, 15), 244_800),
        _ => return None,
    };
    Some(SourceEntryPolicy {
        lever,
        retry_seconds,
    })
}

/// Necessary Quest eligibility, not authority to enter an Instance.
#[derive(Debug, Eq, PartialEq)]
pub struct QualifiedEntry {
    encounter: Encounter,
    participants: Vec<EntryFacts>,
    source_policy: Option<SourceEntryPolicy>,
}
impl QualifiedEntry {
    pub const fn encounter(&self) -> Encounter {
        self.encounter
    }
    pub fn participants(&self) -> &[EntryFacts] {
        &self.participants
    }
    pub const fn source_policy(&self) -> Option<SourceEntryPolicy> {
        self.source_policy
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryRefusal {
    InvalidInput,
    Level,
    QuestAccess,
    BossChain,
    ContentUnavailable,
}

/// Delegates to the one shared owner after checking every participant.
/// The current base supplies a ContentUnavailable owner until W02 is composed (PR #2784,
/// answer 6547426887). The callback is the integration seam, not another activity owner.
pub async fn enter<T, F, Fut>(
    encounter: Encounter,
    participants: &[EntryFacts],
    shared_owner: F,
) -> Result<T, EntryRefusal>
where
    F: FnOnce(QualifiedEntry) -> Fut,
    Fut: Future<Output = Result<T, EntryRefusal>>,
{
    // These source configurations have at most five entry slots. W02 owns the actual
    // roster, positions, cooldown, assignment and group-transaction validation.
    if participants.is_empty() || participants.len() > 5 {
        return Err(EntryRefusal::InvalidInput);
    }
    for (index, participant) in participants.iter().enumerate() {
        if participants[..index]
            .iter()
            .any(|other| other.character == participant.character)
        {
            return Err(EntryRefusal::InvalidInput);
        }
        let minimum_level = if encounter.family == QuestFamily::PrimalOrdeal {
            500
        } else {
            250
        };
        if participant.level < minimum_level {
            return Err(EntryRefusal::Level);
        }
        // Primal's lever has a character-level check, not an invented hazard-level gate.
        if encounter.family != QuestFamily::PrimalOrdeal {
            if !participant.access {
                return Err(EntryRefusal::QuestAccess);
            }
            if encounter.final_boss && !participant.chain_ready {
                return Err(EntryRefusal::BossChain);
            }
        }
    }
    shared_owner(QualifiedEntry {
        encounter,
        participants: participants.to_vec(),
        source_policy: source_entry_policy(encounter),
    })
    .await
}

#[cfg(test)]
#[path = "endgame_entry_tests.rs"]
mod tests;
