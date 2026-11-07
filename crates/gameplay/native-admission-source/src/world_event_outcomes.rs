//! T94 canonical records. These values are publication data, never source authority.
//! Decoding a hash does not prove a native receipt or authorize settlement.
use crate::{character_commands::source_authority_valid, runtime_status::canonical_uuid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const EVENT_BYTES: usize = 16_384;
pub const SEASON_BYTES: usize = 262_144;
pub const WATERMARK_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Pending,
    Malformed,
    Conflict,
    InvalidSource,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pending => "T94 pending",
            Self::Malformed => "T94 malformed",
            Self::Conflict => "T94 conflict",
            Self::InvalidSource => "T94 invalid source",
        })
    }
}
impl std::error::Error for Failure {}
type Result<T> = std::result::Result<T, Failure>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventRecord {
    pub kind: String,
    pub source_key: String,
    pub occurrence_id: String,
    pub ruleset_revision: String,
    pub content_sha256: String,
    pub scheduled_start: String,
    pub scheduled_end: String,
    pub lock_at: String,
    pub options: Vec<String>,
    pub state: String,
    pub outcome: Vec<String>,
    pub decided_at: Vec<String>,
    pub correction_of: Vec<String>,
    pub reason_code: String,
    pub evidence_sha256: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub challenge_id: String,
    pub source_key: String,
    pub scoring_revision: String,
    pub max_score: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    pub challenge_id: String,
    pub score: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standing {
    pub guild_id: String,
    pub guild_name: String,
    pub scores: Vec<Score>,
    pub total_score: String,
    pub rank: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonRecord {
    pub ruleset_revision: String,
    pub starts_at: String,
    pub ends_at: String,
    pub freeze_at: String,
    pub state: String,
    pub challenges: Vec<Challenge>,
    pub standings: Vec<Standing>,
    pub computed_at: String,
    pub source_complete_through: String,
    pub correction_of: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventSnapshot {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub world_id: String,
    pub event_id: String,
    pub projection_epoch: String,
    pub projection_revision: String,
    pub source_observed_at: String,
    pub event: Vec<EventRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonSnapshot {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub world_id: String,
    pub season_id: String,
    pub projection_epoch: String,
    pub projection_revision: String,
    pub source_observed_at: String,
    pub season: Vec<SeasonRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Snapshot {
    Event(EventSnapshot),
    Season(SeasonSnapshot),
}

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn require(valid: bool) -> Result<()> {
    if valid {
        Ok(())
    } else {
        Err(Failure::Malformed)
    }
}
pub fn decimal(value: &str, zero: bool) -> Result<u64> {
    require(
        !value.is_empty()
            && value.bytes().all(|b| b.is_ascii_digit())
            && (value.len() == 1 || !value.starts_with('0')),
    )?;
    let n = value.parse().map_err(|_| Failure::Malformed)?;
    require(zero || n != 0)?;
    Ok(n)
}
pub fn uuid7_valid(value: &str) -> bool {
    uuid(value)
}
fn uuid(value: &str) -> bool {
    canonical_uuid(value, Some(b'7'))
}
fn key(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:/-".contains(&b))
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn optional_hash(value: &[String]) -> bool {
    value.len() <= 1 && value.iter().all(|v| hash(v))
}
fn json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|_| Failure::Malformed)
}

impl EventRecord {
    fn validate(&self) -> Result<()> {
        require(
            matches!(
                self.kind.as_str(),
                "RAID" | "BOSS" | "DAILY_WORLD_GATE" | "WORLD_CHANGE"
            ) && key(&self.source_key)
                && uuid(&self.occurrence_id)
                && key(&self.ruleset_revision)
                && hash(&self.content_sha256),
        )?;
        let start = decimal(&self.scheduled_start, false)?;
        let end = decimal(&self.scheduled_end, false)?;
        let lock = decimal(&self.lock_at, false)?;
        require(
            lock <= start
                && start < end
                && (2..=16).contains(&self.options.len())
                && self.options.iter().all(|v| key(v))
                && self.options.windows(2).all(|w| w[0] < w[1]),
        )?;
        require(
            optional_hash(&self.correction_of)
                && optional_hash(&self.evidence_sha256)
                && self.outcome.len() <= 1
                && self.decided_at.len() <= 1,
        )?;
        for time in &self.decided_at {
            decimal(time, false)?;
        }
        require(self.outcome.iter().all(|v| self.options.contains(v)))?;
        match self.state.as_str() {
            "SCHEDULED" | "LOCKED" => require(
                self.outcome.is_empty()
                    && self.decided_at.is_empty()
                    && self.correction_of.is_empty()
                    && self.evidence_sha256.is_empty()
                    && self.reason_code == "NONE",
            ),
            "SETTLED" => require(
                self.outcome.len() == 1
                    && self.decided_at.len() == 1
                    && self.evidence_sha256.len() == 1
                    && ((self.reason_code == "NATIVE_RESULT" && self.correction_of.is_empty())
                        || (self.reason_code == "CORRECTION" && self.correction_of.len() == 1)),
            ),
            "VOID" => require(
                self.outcome.is_empty()
                    && self.decided_at.len() == 1
                    && self.evidence_sha256.len() == 1
                    && ((matches!(
                        self.reason_code.as_str(),
                        "CANCELLED" | "SOURCE_UNAVAILABLE" | "INVALID_SOURCE"
                    ) && self.correction_of.is_empty())
                        || (self.reason_code == "CORRECTION" && self.correction_of.len() == 1)),
            ),
            "CLOSED" => require(
                self.decided_at.len() == 1
                    && self.evidence_sha256.len() == 1
                    && self.reason_code == "FINAL_FREEZE",
            ),
            _ => Err(Failure::Malformed),
        }
    }
    /// Validate an update against the immediately preceding retained record.
    /// Native evidence and current recovery fences must additionally be proved by the owner.
    pub fn follows(&self, old: &Self, now: u64) -> Result<()> {
        self.validate()?;
        old.validate()?;
        let mut question = self.clone();
        question.state = old.state.clone();
        question.outcome = old.outcome.clone();
        question.decided_at = old.decided_at.clone();
        question.correction_of = old.correction_of.clone();
        question.reason_code = old.reason_code.clone();
        question.evidence_sha256 = old.evidence_sha256.clone();
        require(question == *old)?;
        for decided in &self.decided_at {
            require(decimal(decided, false)? <= now)?;
        }
        let valid = match (old.state.as_str(), self.state.as_str()) {
            ("SCHEDULED", "LOCKED") => now >= decimal(&self.lock_at, false)?,
            ("LOCKED", "SETTLED") | ("SCHEDULED" | "LOCKED", "VOID") => {
                self.correction_of.is_empty()
            }
            ("SETTLED" | "VOID", "SETTLED" | "VOID") => {
                self.reason_code == "CORRECTION"
                    && self.correction_of == [digest(json(old)?.as_bytes())]
                    && self.evidence_sha256 != old.evidence_sha256
            }
            ("SETTLED" | "VOID", "CLOSED") => {
                self.outcome == old.outcome
                    && self.decided_at == old.decided_at
                    && self.correction_of == old.correction_of
                    && self.evidence_sha256 == old.evidence_sha256
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(Failure::Conflict)
        }
    }
}
impl SeasonRecord {
    fn validate(&self) -> Result<()> {
        let start = decimal(&self.starts_at, false)?;
        let end = decimal(&self.ends_at, false)?;
        let freeze = decimal(&self.freeze_at, false)?;
        let computed = decimal(&self.computed_at, false)?;
        let complete = decimal(&self.source_complete_through, false)?;
        require(
            key(&self.ruleset_revision)
                && start < end
                && end <= freeze
                && complete <= computed
                && matches!(self.state.as_str(), "OPEN" | "FROZEN" | "VOID")
                && optional_hash(&self.correction_of)
                && (1..=32).contains(&self.challenges.len())
                && self.standings.len() <= 1000
                && self
                    .challenges
                    .windows(2)
                    .all(|w| w[0].challenge_id < w[1].challenge_id),
        )?;
        if self.state == "FROZEN" {
            require(computed >= freeze && complete >= end)?;
        }
        for c in &self.challenges {
            require(uuid(&c.challenge_id) && key(&c.source_key) && key(&c.scoring_revision))?;
            decimal(&c.max_score, false)?;
        }
        let mut guilds = BTreeSet::new();
        let mut previous: Option<(u64, Vec<u64>, &str)> = None;
        for (index, g) in self.standings.iter().enumerate() {
            require(
                uuid(&g.guild_id)
                    && guilds.insert(&g.guild_id)
                    && crate::public_projections::projection_name_valid(&g.guild_name)
                    && g.scores.len() == self.challenges.len()
                    && decimal(&g.rank, false)? == index as u64 + 1,
            )?;
            let mut sum = 0u64;
            let mut scores = Vec::with_capacity(g.scores.len());
            for (score, c) in g.scores.iter().zip(&self.challenges) {
                let n = decimal(&score.score, true)?;
                require(
                    score.challenge_id == c.challenge_id && n <= decimal(&c.max_score, false)?,
                )?;
                sum = sum.checked_add(n).ok_or(Failure::Malformed)?;
                scores.push(n);
            }
            require(sum == decimal(&g.total_score, true)?)?;
            if let Some((total, vector, id)) = previous {
                require(
                    total > sum
                        || (total == sum
                            && (vector > scores || (vector == scores && id < g.guild_id.as_str()))),
                )?;
            }
            previous = Some((sum, scores, &g.guild_id));
        }
        Ok(())
    }
    pub fn follows(&self, old: &Self) -> Result<()> {
        self.validate()?;
        old.validate()?;
        require(
            self.ruleset_revision == old.ruleset_revision
                && self.starts_at == old.starts_at
                && self.ends_at == old.ends_at
                && self.freeze_at == old.freeze_at
                && self.challenges == old.challenges,
        )?;
        if old.state != "OPEN" {
            return Err(Failure::Conflict);
        }
        if self.standings != old.standings {
            require(self.state == "OPEN" && self.correction_of == [digest(json(old)?.as_bytes())])?;
        } else {
            require(self.correction_of == old.correction_of)?;
        }
        require(
            decimal(&self.computed_at, false)? >= decimal(&old.computed_at, false)?
                && decimal(&self.source_complete_through, false)?
                    >= decimal(&old.source_complete_through, false)?,
        )
    }
}
impl Snapshot {
    /// Check sequential publication. This is not a resync shortcut: a gap or
    /// missing predecessor requires retained lineage and native-source recovery.
    pub fn follows(&self, previous: Option<&Self>, now: u64) -> Result<()> {
        self.encode()?;
        let (epoch, revision, observed) = self.pair()?;
        require(observed <= now.saturating_add(1))?;
        let Some(old) = previous else {
            require(revision == 1)?;
            return match self {
                Self::Event(s) => {
                    let e = s.event.first().ok_or(Failure::InvalidSource)?;
                    require(
                        (e.state == "SCHEDULED" && now < decimal(&e.lock_at, false)?)
                            || (e.state == "VOID"
                                && e.correction_of.is_empty()
                                && e.decided_at
                                    .iter()
                                    .all(|time| decimal(time, false).is_ok_and(|at| at <= now))),
                    )
                }
                Self::Season(s) => {
                    let season = s.season.first().ok_or(Failure::InvalidSource)?;
                    require(
                        matches!(season.state.as_str(), "OPEN" | "VOID")
                            && season.correction_of.is_empty()
                            && decimal(&season.computed_at, false)? <= now.saturating_add(1),
                    )
                }
            };
        };
        old.encode()?;
        let (old_epoch, old_revision, old_observed) = old.pair()?;
        if epoch != old_epoch {
            return Err(Failure::Conflict);
        }
        if revision == old_revision {
            return if self == old {
                Ok(())
            } else {
                Err(Failure::Conflict)
            };
        }
        if old_revision.checked_add(1) != Some(revision) || observed < old_observed {
            return Err(Failure::Conflict);
        }
        match (self, old) {
            (Self::Event(s), Self::Event(o)) => {
                require(
                    s.source_authority == o.source_authority
                        && s.world_id == o.world_id
                        && s.event_id == o.event_id,
                )?;
                // An empty display snapshot does not erase the final authority
                // record. Reappearance needs that separately retained record.
                match (s.event.first(), o.event.first()) {
                    (Some(e), Some(old)) => e.follows(old, now),
                    (None, _) => Ok(()),
                    _ => Err(Failure::Conflict),
                }
            }
            (Self::Season(s), Self::Season(o)) => {
                require(
                    s.source_authority == o.source_authority
                        && s.world_id == o.world_id
                        && s.season_id == o.season_id,
                )?;
                match (s.season.first(), o.season.first()) {
                    (Some(season), Some(old)) => {
                        require(decimal(&season.computed_at, false)? <= now.saturating_add(1))?;
                        season.follows(old)
                    }
                    (None, _) => Ok(()),
                    _ => Err(Failure::Conflict),
                }
            }
            _ => Err(Failure::Conflict),
        }
    }
    fn pair(&self) -> Result<(u64, u64, u64)> {
        let (e, r, o) = match self {
            Self::Event(s) => (
                &s.projection_epoch,
                &s.projection_revision,
                &s.source_observed_at,
            ),
            Self::Season(s) => (
                &s.projection_epoch,
                &s.projection_revision,
                &s.source_observed_at,
            ),
        };
        Ok((decimal(e, false)?, decimal(r, false)?, decimal(o, false)?))
    }
    pub fn decode(wire: &str) -> Result<Self> {
        require(wire.len() <= SEASON_BYTES && !wire.contains('\\'))?;
        let value = if let Ok(s) = serde_json::from_str::<EventSnapshot>(wire) {
            Self::Event(s)
        } else {
            Self::Season(serde_json::from_str(wire).map_err(|_| Failure::Malformed)?)
        };
        require(value.encode()? == wire)?;
        Ok(value)
    }
    pub fn encode(&self) -> Result<String> {
        let (v, op, authority, world, subject, epoch, revision, observed, bytes, expected) =
            match self {
                Self::Event(s) => {
                    require(s.event.len() <= 1)?;
                    for e in &s.event {
                        e.validate()?;
                    }
                    (
                        s.contract_version,
                        s.operation.as_str(),
                        &s.source_authority,
                        &s.world_id,
                        &s.event_id,
                        &s.projection_epoch,
                        &s.projection_revision,
                        &s.source_observed_at,
                        EVENT_BYTES,
                        "PublishWorldEventOutcomeV1",
                    )
                }
                Self::Season(s) => {
                    require(s.season.len() <= 1)?;
                    for e in &s.season {
                        e.validate()?;
                    }
                    (
                        s.contract_version,
                        s.operation.as_str(),
                        &s.source_authority,
                        &s.world_id,
                        &s.season_id,
                        &s.projection_epoch,
                        &s.projection_revision,
                        &s.source_observed_at,
                        SEASON_BYTES,
                        "PublishGuildSeasonV1",
                    )
                }
            };
        require(
            v == 1
                && op == expected
                && source_authority_valid(authority)
                && uuid(world)
                && uuid(subject),
        )?;
        decimal(epoch, false)?;
        decimal(revision, false)?;
        decimal(observed, false)?;
        let wire = match self {
            Self::Event(s) => json(s)?,
            Self::Season(s) => json(s)?,
        };
        require(wire.len() <= bytes && !wire.contains('\\'))?;
        Ok(wire)
    }
    pub fn record_hash(&self) -> Result<String> {
        let wire = match self {
            Self::Event(s) => json(s.event.first().ok_or(Failure::Malformed)?)?,
            Self::Season(s) => json(s.season.first().ok_or(Failure::Malformed)?)?,
        };
        Ok(digest(wire.as_bytes()))
    }
}

/// Safe defaults contain no approved source/scoring rules. A flag alone cannot
/// approve a hidden draw, a Channel aggregation rule or a guild scoring rule.
#[derive(Debug)]
pub struct Policy {
    pub enabled: bool,
    pub event_rules: BTreeSet<(String, String, String)>,
    pub season_rules: BTreeSet<(String, String, String)>,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            enabled: true,
            event_rules: BTreeSet::new(),
            season_rules: BTreeSet::new(),
        }
    }
}
impl Policy {
    pub fn admit(&self, snapshot: &Snapshot) -> Result<()> {
        if !self.enabled {
            return Err(Failure::Pending);
        }
        snapshot.encode()?;
        let approved = match snapshot {
            Snapshot::Event(s) => s.event.iter().all(|e| {
                self.event_rules.contains(&(
                    e.kind.clone(),
                    e.source_key.clone(),
                    e.ruleset_revision.clone(),
                ))
            }),
            Snapshot::Season(s) => s.season.iter().all(|s| {
                s.challenges.iter().all(|c| {
                    self.season_rules.contains(&(
                        s.ruleset_revision.clone(),
                        c.source_key.clone(),
                        c.scoring_revision.clone(),
                    ))
                })
            }),
        };
        if approved {
            Ok(())
        } else {
            Err(Failure::InvalidSource)
        }
    }
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: String,
    pub projection_epoch: String,
    pub complete_through: String,
    pub observed_at: String,
}
impl Watermark {
    /// The durable owner must also prove complete enumeration and that no
    /// unacknowledged source cut precedes complete_through. A clock is insufficient.
    pub fn encode(&self, now: u64) -> Result<String> {
        require(
            self.contract_version == 1
                && self.operation == "PublishLeagueWatermarkV1"
                && source_authority_valid(&self.source_authority)
                && matches!(self.family.as_str(), "world_event_outcome" | "guild_season"),
        )?;
        decimal(&self.projection_epoch, false)?;
        let complete = decimal(&self.complete_through, false)?;
        let observed = decimal(&self.observed_at, false)?;
        require(complete <= observed && observed <= now.saturating_add(1))?;
        let wire = json(self)?;
        require(wire.len() <= WATERMARK_BYTES)?;
        Ok(wire)
    }
    pub fn fresh(&self, now: u64) -> bool {
        self.encode(now).is_ok()
            && decimal(&self.complete_through, false).is_ok_and(|cut| {
                now.checked_sub(cut)
                    .and_then(|age| age.checked_add(1))
                    .is_some_and(|age| age <= 30)
            })
    }
}

pub(crate) fn is_operation(operation: crate::descriptor::Operation) -> bool {
    use crate::descriptor::Operation;
    matches!(
        operation,
        Operation::PublishWorldEventOutcomeV1
            | Operation::PublishGuildSeasonV1
            | Operation::PublishLeagueWatermarkV1
    )
}

/// Exact durable publication and its independently acknowledged ANL audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub id: String,
    pub delivery_token: Option<String>,
    pub family: String,
    pub epoch: u64,
    pub revision: u64,
    pub wire: String,
    pub sha256: Vec<u8>,
    pub audit: Vec<u8>,
    pub audit_event_id: String,
    pub transaction_id: String,
}

pub mod publisher;

/// Owner 2026-11-06 conservative configurable defaults. There is no implied
/// eligible guild, score, hidden draw disclosure or advance prediction window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProducerDefaults {
    pub fury_gate_realization_available: bool,
    pub season_seconds: u64,
    pub season_advance_seconds: u64,
    pub freeze_delay_seconds: u64,
    pub event_window_seconds: u64,
    pub correction_seconds: u64,
    pub max_score: u64,
    pub eligible_guilds: Vec<String>,
    pub tombstone_name: Option<String>,
}
impl Default for ProducerDefaults {
    fn default() -> Self {
        Self {
            fury_gate_realization_available: false,
            season_seconds: 86400,
            season_advance_seconds: 3,
            freeze_delay_seconds: 1,
            event_window_seconds: 1,
            correction_seconds: 1,
            max_score: 1,
            eligible_guilds: Vec::new(),
            tombstone_name: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WatermarkDelivery {
    pub watermark: Watermark,
    pub token: String,
}

pub mod fence;
