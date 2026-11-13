//! Owner-accepted G-4 V2 wire; V1 families and watermark remain closed.
pub mod character_views;
pub mod daily_kills;
pub mod online_sessions;

use super::*;
use serde::{Deserialize, ser::SerializeMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    CharacterDeaths,
    Guild,
    House,
    CharacterSkills,
    CharacterAchievements,
    CharacterEquipment,
    CharacterOutfit,
    CharacterOnlineSessions,
    WorldDailyKills,
}
impl Family {
    pub const ALL: [Self; 9] = [
        Self::CharacterDeaths,
        Self::Guild,
        Self::House,
        Self::CharacterSkills,
        Self::CharacterAchievements,
        Self::CharacterEquipment,
        Self::CharacterOutfit,
        Self::CharacterOnlineSessions,
        Self::WorldDailyKills,
    ];
    pub const fn wire(self) -> &'static str {
        match self {
            Self::CharacterDeaths => "character_deaths",
            Self::Guild => "guild",
            Self::House => "house",
            Self::CharacterSkills => "character_skills",
            Self::CharacterAchievements => "character_achievements",
            Self::CharacterEquipment => "character_equipment",
            Self::CharacterOutfit => "character_outfit",
            Self::CharacterOnlineSessions => "character_online_sessions",
            Self::WorldDailyKills => "world_daily_kills",
        }
    }
    pub const fn operation(self) -> Operation {
        match self {
            Self::CharacterDeaths => Operation::PublishCharacterDeathsV2,
            Self::Guild => Operation::PublishGuildV2,
            Self::House => Operation::PublishHouseV2,
            Self::CharacterSkills => Operation::PublishCharacterSkillsV2,
            Self::CharacterAchievements => Operation::PublishCharacterAchievementsV2,
            Self::CharacterEquipment => Operation::PublishCharacterEquipmentV2,
            Self::CharacterOutfit => Operation::PublishCharacterOutfitV2,
            Self::CharacterOnlineSessions => Operation::PublishCharacterOnlineSessionV2,
            Self::WorldDailyKills => Operation::PublishWorldDailyKillsV2,
        }
    }
}

/// Publication flags are independent of durable source capture and retention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Activation {
    pub character_skills: bool,
    pub character_achievements: bool,
    pub character_equipment: bool,
    pub character_outfit: bool,
    pub character_online_sessions: bool,
    pub world_daily_kills: bool,
}
impl Family {
    /// Wire support alone never authorizes publication. Part B supplies these sources.
    pub const fn produced(self) -> bool {
        match self {
            Self::CharacterDeaths
            | Self::Guild
            | Self::House
            | Self::CharacterOnlineSessions
            | Self::WorldDailyKills => true,
            Self::CharacterSkills
            | Self::CharacterAchievements
            | Self::CharacterEquipment
            | Self::CharacterOutfit => false,
        }
    }
    pub const fn publishable(self, activation: Activation) -> bool {
        self.produced() && self.enabled(activation)
    }
    pub const fn enabled(self, activation: Activation) -> bool {
        match self {
            Self::CharacterDeaths | Self::Guild | Self::House => true,
            Self::CharacterSkills => activation.character_skills,
            Self::CharacterAchievements => activation.character_achievements,
            Self::CharacterEquipment => activation.character_equipment,
            Self::CharacterOutfit => activation.character_outfit,
            Self::CharacterOnlineSessions => activation.character_online_sessions,
            Self::WorldDailyKills => activation.world_daily_kills,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Killer {
    Player { character_id: String, name: String },
    Creature { creature_key: String, name: String },
}
impl Killer {
    fn valid(&self) -> bool {
        match self {
            Self::Player { character_id, name } => id(character_id) && projection_name_valid(name),
            Self::Creature { creature_key, name } => {
                key(creature_key)
                    && !matches!(creature_key.as_str(), "player" | "unknown")
                    && projection_name_valid(name)
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Death {
    pub death_occurrence_id: String,
    pub occurred_at: String,
    pub level: String,
    pub pvp: bool,
    pub killers: Vec<Killer>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rank {
    pub level: u8,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub character_id: String,
    pub name: String,
    pub rank_level: u8,
    pub joined_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GuildState {
    Forming,
    Active,
    Disbanding,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guild {
    pub world_id: String,
    pub name: String,
    pub state: GuildState,
    pub founded_at: String,
    pub ranks: Vec<Rank>,
    pub members: Vec<Member>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OwnerKind {
    Character,
    Guild,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum HouseState {
    Vacant,
    Auction {
        current_bid: String,
        auction_ends_at: String,
    },
    Owned {
        owner_kind: OwnerKind,
        owner_name: String,
        paid_until: String,
    },
    MoveOutPending {
        owner_kind: OwnerKind,
        owner_name: String,
        paid_until: String,
        move_out_at: String,
    },
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseKind {
    PrivateHouse,
    Shop,
    Guildhall,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct House {
    pub name: String,
    pub kind: HouseKind,
    pub town: String,
    pub size_sqm: String,
    pub beds: String,
    pub rent_gold: String,
    #[serde(flatten)]
    pub state: HouseState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Deaths(Vec<Death>),
    Guild(Vec<Guild>),
    House(Vec<House>),
    Skills(Vec<character_views::Skills>),
    Achievements(Vec<character_views::Achievements>),
    Equipment(Vec<character_views::Equipment>),
    Outfit(Vec<character_views::Outfit>),

    OnlineSessions(Vec<online_sessions::Session>),
    DailyKills(Vec<daily_kills::Day>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub subject: String,
    /// House WorldId; empty for Character/Guild subjects.
    pub world_id: String,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub source_observed_at: i64,
    pub body: Body,
}
fn id(value: &str) -> bool {
    canonical_uuid(value, Some(b'7'))
}
fn key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b))
}
fn number(value: &str, zero: bool, max: u64) -> bool {
    value
        .parse::<u64>()
        .is_ok_and(|n| (zero || n > 0) && n <= max && n.to_string() == value)
}
fn time(value: &str) -> bool {
    number(value, false, i64::MAX as u64)
}
impl Snapshot {
    pub const fn family(&self) -> Family {
        match self.body {
            Body::Deaths(_) => Family::CharacterDeaths,
            Body::Guild(_) => Family::Guild,
            Body::House(_) => Family::House,
            Body::Skills(_) => Family::CharacterSkills,
            Body::Achievements(_) => Family::CharacterAchievements,
            Body::Equipment(_) => Family::CharacterEquipment,
            Body::Outfit(_) => Family::CharacterOutfit,
            Body::OnlineSessions(_) => Family::CharacterOnlineSessions,
            Body::DailyKills(_) => Family::WorldDailyKills,
        }
    }
    fn valid(&self) -> bool {
        match &self.body {
            Body::Skills(rows) => {
                id(&self.subject)
                    && self.world_id.is_empty()
                    && rows.len() <= 1
                    && rows.iter().all(character_views::Skills::valid)
            }
            Body::Achievements(rows) => {
                id(&self.subject)
                    && self.world_id.is_empty()
                    && rows.len() <= 1
                    && rows.iter().all(character_views::Achievements::valid)
            }
            Body::Equipment(rows) => {
                id(&self.subject)
                    && self.world_id.is_empty()
                    && rows.len() <= 1
                    && rows.iter().all(character_views::Equipment::valid)
            }
            Body::Outfit(rows) => {
                id(&self.subject)
                    && self.world_id.is_empty()
                    && rows.len() <= 1
                    && rows.iter().all(character_views::Outfit::valid)
            }
            Body::DailyKills(rows) => {
                id(&self.world_id)
                    && daily_kills::valid(&self.subject, self.source_observed_at, rows)
            }
            Body::OnlineSessions(rows) => {
                id(&self.subject)
                    && id(&self.world_id)
                    && online_sessions::valid(rows, self.source_observed_at)
            }
            Body::Deaths(rows) => {
                id(&self.subject)
                    && self.world_id.is_empty()
                    && rows.len() <= 20
                    && rows.iter().all(|d| {
                        id(&d.death_occurrence_id)
                            && time(&d.occurred_at)
                            && number(&d.level, false, u32::MAX.into())
                            && d.killers.len() <= 16
                            && d.killers.iter().all(Killer::valid)
                    })
                    && rows.windows(2).all(|p| {
                        (
                            p[0].occurred_at.parse::<u64>().ok(),
                            &p[0].death_occurrence_id,
                        ) > (
                            p[1].occurred_at.parse::<u64>().ok(),
                            &p[1].death_occurrence_id,
                        )
                    })
                    && rows.iter().enumerate().all(|(i, d)| {
                        !rows[..i]
                            .iter()
                            .any(|v| v.death_occurrence_id == d.death_occurrence_id)
                    })
            }
            Body::Guild(rows) => {
                id(&self.subject)
                    && self.world_id.is_empty()
                    && rows.len() <= 1
                    && rows.iter().all(|g| {
                        id(&g.world_id)
                            && projection_name_valid(&g.name)
                            && time(&g.founded_at)
                            && (3..=20).contains(&g.ranks.len())
                            && g.ranks.iter().all(|r| {
                                (1..=20).contains(&r.level) && projection_name_valid(&r.name)
                            })
                            && g.ranks.windows(2).all(|p| p[0].level < p[1].level)
                            && g.members.len() <= 2000
                            && g.members.iter().all(|m| {
                                id(&m.character_id)
                                    && projection_name_valid(&m.name)
                                    && time(&m.joined_at)
                                    && g.ranks.iter().any(|r| r.level == m.rank_level)
                            })
                            && g.members.windows(2).all(|p| {
                                (p[0].rank_level, &p[0].character_id)
                                    < (p[1].rank_level, &p[1].character_id)
                            })
                            && g.members.iter().enumerate().all(|(i, m)| {
                                !g.members[..i]
                                    .iter()
                                    .any(|v| v.character_id == m.character_id)
                            })
                    })
            }
            Body::House(rows) => {
                id(&self.world_id)
                    && key(&self.subject)
                    && rows.len() <= 1
                    && rows.iter().all(|h| {
                        projection_name_valid(&h.name)
                            && key(&h.town)
                            && number(&h.size_sqm, false, u64::MAX)
                            && number(&h.beds, true, u64::MAX)
                            && number(&h.rent_gold, false, i64::MAX as u64)
                            && match &h.state {
                                HouseState::Vacant | HouseState::Unavailable => true,
                                HouseState::Auction {
                                    current_bid,
                                    auction_ends_at,
                                } => {
                                    number(current_bid, true, i64::MAX as u64)
                                        && time(auction_ends_at)
                                }
                                HouseState::Owned {
                                    owner_name,
                                    paid_until,
                                    ..
                                } => projection_name_valid(owner_name) && time(paid_until),
                                HouseState::MoveOutPending {
                                    owner_name,
                                    paid_until,
                                    move_out_at,
                                    ..
                                } => {
                                    projection_name_valid(owner_name)
                                        && time(paid_until)
                                        && time(move_out_at)
                                }
                            }
                    })
            }
        }
    }
}
struct Wire<'a> {
    authority: &'a str,
    snapshot: &'a Snapshot,
}
impl Serialize for Wire<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = self.snapshot;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("contract_version", &2u8)?;
        let op = match s.family() {
            Family::CharacterDeaths => "PublishCharacterDeathsV2",
            Family::Guild => "PublishGuildV2",
            Family::House => "PublishHouseV2",
            Family::CharacterSkills => "PublishCharacterSkillsV2",
            Family::CharacterAchievements => "PublishCharacterAchievementsV2",
            Family::CharacterEquipment => "PublishCharacterEquipmentV2",
            Family::CharacterOutfit => "PublishCharacterOutfitV2",

            Family::CharacterOnlineSessions => "PublishCharacterOnlineSessionV2",
            Family::WorldDailyKills => "PublishWorldDailyKillsV2",
        };
        map.serialize_entry("operation", op)?;
        map.serialize_entry("source_authority", self.authority)?;
        match s.family() {
            Family::WorldDailyKills => {
                map.serialize_entry("world_id", &s.world_id)?;
                map.serialize_entry("day_start", &s.subject)?;
            }
            Family::CharacterOnlineSessions => {
                map.serialize_entry("character_id", &s.world_id)?;
                map.serialize_entry("statistics_id", &s.subject)?;
            }
            Family::CharacterDeaths
            | Family::CharacterSkills
            | Family::CharacterAchievements
            | Family::CharacterEquipment
            | Family::CharacterOutfit => map.serialize_entry("character_id", &s.subject)?,
            Family::Guild => map.serialize_entry("guild_id", &s.subject)?,
            Family::House => {
                map.serialize_entry("world_id", &s.world_id)?;
                map.serialize_entry("house_key", &s.subject)?;
            }
        }
        map.serialize_entry("projection_epoch", &s.projection_epoch.to_string())?;
        map.serialize_entry("projection_revision", &s.projection_revision.to_string())?;
        map.serialize_entry("source_observed_at", &s.source_observed_at.to_string())?;
        match &s.body {
            Body::Deaths(v) => map.serialize_entry("deaths", v)?,
            Body::Guild(v) => map.serialize_entry("guild", v)?,
            Body::House(v) => map.serialize_entry("house", v)?,
            Body::Skills(v) => map.serialize_entry("skills", v)?,
            Body::Achievements(v) => map.serialize_entry("achievements", v)?,
            Body::Equipment(v) => map.serialize_entry("equipment", v)?,
            Body::Outfit(v) => map.serialize_entry("outfit", v)?,

            Body::OnlineSessions(v) => map.serialize_entry("sessions", v)?,
            Body::DailyKills(v) => map.serialize_entry("days", v)?,
        }
        map.end()
    }
}
pub fn encode(authority: &str, snapshot: &Snapshot) -> Result<String, SourceError> {
    if !envelope_valid(
        authority,
        snapshot.projection_epoch,
        snapshot.projection_revision,
        snapshot.source_observed_at,
    ) || !snapshot.valid()
    {
        return Err(SourceError::InvalidInput);
    }
    finish(
        serde_json::to_string(&Wire {
            authority,
            snapshot,
        }),
        snapshot.family().operation().request_bytes_max(),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ack {
    contract_version: u8,
    result: Delivery,
}
pub fn decode_response(raw: &[u8]) -> Result<Delivery, SourceError> {
    if raw.len() > 256 {
        return Err(SourceError::CapacityExceeded);
    }
    match serde_json::from_slice::<Ack>(raw) {
        Ok(Ack {
            contract_version: 2,
            result,
        }) => Ok(result),
        _ => Err(SourceError::InvalidInput),
    }
}

/// Shared closed watermark vocabulary; WorldDaily's payload remains owned by G-12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkFamily {
    CharacterDeaths,
    Guild,
    House,
    CharacterSkills,
    CharacterAchievements,
    CharacterEquipment,
    CharacterOutfit,
    CharacterOnlineSessions,
    WorldDailyKills,
    WorldDaily,
}
impl From<Family> for WatermarkFamily {
    fn from(value: Family) -> Self {
        match value {
            Family::CharacterDeaths => Self::CharacterDeaths,
            Family::Guild => Self::Guild,
            Family::House => Self::House,
            Family::CharacterSkills => Self::CharacterSkills,
            Family::CharacterAchievements => Self::CharacterAchievements,
            Family::CharacterEquipment => Self::CharacterEquipment,
            Family::CharacterOutfit => Self::CharacterOutfit,
            Family::CharacterOnlineSessions => Self::CharacterOnlineSessions,
            Family::WorldDailyKills => Self::WorldDailyKills,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Facts {
    pub projection_epoch: u64,
    pub oldest_undelivered_ms: Option<i64>,
    pub now_ms: i64,
}
#[derive(Serialize)]
struct Watermark<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    family: WatermarkFamily,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}
pub fn encode_watermark(
    authority: &str,
    family: impl Into<WatermarkFamily>,
    facts: Facts,
    max_transaction: Duration,
) -> Result<String, SourceError> {
    let through = complete_through(
        &WatermarkFacts {
            projection_epoch: facts.projection_epoch,
            oldest_undelivered_ms: facts.oldest_undelivered_ms,
            now_ms: facts.now_ms,
        },
        max_transaction,
    )
    .ok_or(SourceError::InvalidInput)?;
    if !source_authority_valid(authority) || facts.projection_epoch == 0 || through < 0 {
        return Err(SourceError::InvalidInput);
    }
    finish(
        serde_json::to_string(&Watermark {
            contract_version: 2,
            operation: "PublishPublicProjectionWatermarkV2",
            source_authority: authority,
            family: family.into(),
            projection_epoch: facts.projection_epoch.to_string(),
            complete_through: through.to_string(),
            observed_at: (facts.now_ms / 1000).to_string(),
        }),
        512,
    )
}
impl<C: ReportClock, S: PublicStore, K: PublicSink, F: EpochFence> Publisher<C, S, K, F> {
    pub(super) async fn v2_loops(&self) {
        let _ = join_all(
            Family::ALL
                .into_iter()
                .filter(|family| family.publishable(self.v2_activation))
                .map(|family| self.v2_family_loop(family))
                .collect(),
        )
        .await;
    }
    async fn v2_family_loop(&self, family: Family) {
        let mut held: Vec<(String, Duration)> = Vec::new();
        let mut retry = Duration::ZERO;
        let mut backoff = Duration::ZERO;
        let mut watermark_due = Duration::ZERO;
        let mut watermark_retry = Duration::ZERO;
        let mut watermark_backoff = Duration::ZERO;
        loop {
            let now = self.clock.elapsed();
            if now >= watermark_due && now >= watermark_retry {
                watermark_due = now + WATERMARK_PERIOD;
                if let Ok(facts) = self.pass(|| self.store.v2_facts(family)).await
                    && self.admit(facts.projection_epoch).is_ok()
                    && let Ok(body) = encode_watermark(
                        &self.source_authority,
                        family,
                        facts,
                        self.max_transaction,
                    )
                {
                    let result = self
                        .deliver(
                            Operation::PublishPublicProjectionWatermarkV2,
                            &body,
                            EXCHANGE_BOUND,
                        )
                        .await;
                    match result {
                        Ok(_) => {
                            watermark_backoff = Duration::ZERO;
                            watermark_retry = Duration::ZERO;
                        }
                        Err(e) if definite(e) => {
                            self.alarm();
                            watermark_retry = self.clock.elapsed() + DEFINITE_HOLD;
                        }
                        Err(_) => {
                            watermark_backoff = next_backoff(watermark_backoff);
                            watermark_retry = self.clock.elapsed() + watermark_backoff;
                        }
                    }
                }
            }
            held.retain(|(_, until)| *until > self.clock.elapsed());
            if self.clock.elapsed() < retry || held.len() >= MAX_HELD {
                self.clock.sleep(IDLE).await;
                continue;
            }
            let excluded = held.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();
            let snapshot = match self.pass(|| self.store.next_v2(family, &excluded)).await {
                Ok(Some(snapshot)) => snapshot,
                _ => {
                    self.clock.sleep(IDLE).await;
                    continue;
                }
            };
            let result = if self.admit(snapshot.projection_epoch).is_err() {
                Err(NotDelivered::Unavailable)
            } else {
                match encode(&self.source_authority, &snapshot) {
                    Ok(body) => {
                        self.deliver(family.operation(), &body, EXCHANGE_BOUND)
                            .await
                    }
                    Err(_) => Err(NotDelivered::InvalidReport),
                }
            };
            match result {
                Ok(_) => {
                    // A lost ACK transaction resends the same pair, never clears a successor.
                    let _ = self.pass(|| self.store.ack_v2(&snapshot)).await;
                    backoff = Duration::ZERO;
                    retry = Duration::ZERO;
                }
                Err(e) if definite(e) => {
                    self.alarm();
                    held.push((
                        format!("{}:{}", snapshot.world_id, snapshot.subject),
                        self.clock.elapsed() + DEFINITE_HOLD,
                    ));
                }
                Err(_) => {
                    backoff = next_backoff(backoff);
                    retry = self.clock.elapsed() + backoff;
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "fixture assertions in bounded wire tests"
)]
#[path = "v2/tests.rs"]
mod tests;
