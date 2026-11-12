//! Closed progress payload wire objects (accepted revision 3).
use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bestiary {
    pub race_key: String,
    pub race_definition_revision: String,
    pub kill_count: u32,
    pub final_kill_threshold: u32,
    pub charm_points_awarded: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossCooldown {
    pub boss_key: String,
    pub definition_revision: String,
    pub kill_count: String,
    pub available_at_unix_ms: Option<String>,
    pub last_death_occurrence_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quest {
    pub quest_key: String,
    pub pinned_content_revision: String,
    pub definition_hash: String,
    pub completed: bool,
    pub tracks: Vec<Track>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub track_key: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Achievement {
    pub achievement_key: String,
    pub definition_revision: String,
    pub progress: Option<u32>,
    pub target: Option<u32>,
    pub granted_at_unix_ms: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillHistory {
    pub build_occurrence_id: String,
    pub cause: String,
    pub policy_revision: String,
    pub skills: Vec<SkillChange>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillChange {
    pub skill: String,
    pub level_before: u32,
    pub progress_before: String,
    pub level_after: u32,
    pub progress_after: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeathRecap {
    pub death_occurrence_id: String,
    pub level_before: u32,
    pub level_after: u32,
    pub experience_lost: String,
    pub blessings_before: Vec<String>,
    pub blessings_after: Vec<String>,
    pub lost_items: Vec<Loss>,
    pub recap_state: String,
    pub window_ms: u32,
    pub actions: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loss {
    pub loss_index: u32,
    pub item_key: String,
    pub quantity: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pvp {
    pub death_occurrence_id: String,
    pub role: String,
    pub counterpart_character_id: Option<String>,
    pub justification: String,
    pub points_milli: Option<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skull {
    pub skull: String,
    pub skull_until_unix_ms: Option<String>,
    pub kill_block_until_unix_ms: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stamina {
    pub stamina_minutes: u32,
    pub content_revision: String,
    pub build_occurrence_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PremiumEnd {
    pub observed_state: String,
    pub effective_until_unix_us: Option<String>,
    pub authority_revision: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameLogin {
    pub login_event_id: String,
    pub entered_at: String,
    pub left_at: Option<String>,
    pub end_reason: Option<String>,
}
fn natural(v: &str) -> bool {
    v == "0" || counter(v).is_some()
}
fn time(v: &str) -> bool {
    natural(v) && v.parse::<u64>().is_ok_and(|n| n <= 253402300799999)
}
fn valid_fields(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(o) => o.iter().all(|(k, v)| {
            if let Some(s) = v.as_str() {
                if k.ends_with("_unix_ms") || k == "entered_at" || k == "left_at" {
                    return time(s);
                }
                if k == "effective_until_unix_us" {
                    return natural(s) && s.parse::<u64>().is_ok_and(|n| n <= 253402300799999999);
                }
                if k.ends_with("_id") {
                    return uuid_v7(s);
                }
                if k == "value" {
                    return s.parse::<i64>().is_ok_and(|n| n.to_string() == s);
                }
                if k == "kill_count" {
                    return natural(s);
                }
                if matches!(
                    k.as_str(),
                    "progress_before" | "progress_after" | "experience_lost"
                ) {
                    return natural(s) && s.parse::<u64>().is_ok_and(|n| n <= i64::MAX as u64);
                }
                if matches!(k.as_str(), "content_revision" | "authority_revision") {
                    return counter(s).is_some();
                }
                if k == "definition_hash" {
                    return s.len() == 64
                        && s.bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
                }
                if k.ends_with("_key") || k.ends_with("_revision") {
                    let max = if k.ends_with("_revision") { 128 } else { 512 };
                    return s.len() <= max
                        && s.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
                        && s.bytes().all(|b| {
                            b.is_ascii_alphanumeric()
                                || (b"._:/-".contains(&b)
                                    || (k.ends_with("_revision") && b == b'+'))
                        });
                }
            }
            valid_fields(v)
        }),
        serde_json::Value::Array(a) => a.iter().all(valid_fields),
        serde_json::Value::String(s) => s
            .bytes()
            .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\'),
        _ => true,
    }
}
/// A family-selected progress payload. Unknown families, including house_rent, deny.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressPayload {
    Bestiary(Bestiary),
    BossCooldown(BossCooldown),
    Quest(Quest),
    Achievement(Achievement),
    SkillHistory(SkillHistory),
    DeathRecap(DeathRecap),
    Pvp(Pvp),
    Skull(Skull),
    Stamina(Stamina),
    PremiumEnd(PremiumEnd),
    GameLogin(GameLogin),
}
impl ProgressPayload {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        let bytes = serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| SourceError::InvalidInput)?;
        if !valid_fields(&value) || !self.valid() {
            return Err(SourceError::InvalidInput);
        }
        if bytes.len() > 262144 {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(bytes)
    }
    pub fn decode(family: &str, bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 262144 {
            return Err(SourceError::CapacityExceeded);
        }
        let value = match family {
            "bestiary" => Self::Bestiary(
                serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?,
            ),
            "boss_cooldown" => Self::BossCooldown(
                serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?,
            ),
            "quest" => {
                Self::Quest(serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?)
            }
            "achievement" => Self::Achievement(
                serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?,
            ),
            "skill_history" => Self::SkillHistory(
                serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?,
            ),
            "death_recap" => Self::DeathRecap(
                serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?,
            ),
            "pvp" => {
                Self::Pvp(serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?)
            }
            "skull" => {
                Self::Skull(serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?)
            }
            "stamina" => {
                Self::Stamina(serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?)
            }
            "premium_end" => Self::PremiumEnd(
                serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?,
            ),
            "game_login" => Self::GameLogin(
                serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?,
            ),
            _ => return Err(SourceError::InvalidInput),
        };
        if value.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(value)
    }
    fn valid(&self) -> bool {
        match self {
            Self::Bestiary(p) => p.final_kill_threshold > 0,
            Self::BossCooldown(_) => true,
            Self::Quest(p) => {
                p.tracks.len() <= 256
                    && p.tracks.windows(2).all(|v| v[0].track_key < v[1].track_key)
            }
            Self::Achievement(p) => {
                p.achievement_key.len() <= 160
                    && p.achievement_key
                        .strip_prefix("oteryn:achievement/")
                        .is_some_and(|v| {
                            !v.is_empty()
                                && v.split('_').all(|p| {
                                    !p.is_empty()
                                        && p.bytes()
                                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                                })
                        })
                    && match (p.progress, p.target, p.granted_at_unix_ms.as_deref()) {
                        (Some(a), Some(b), None) => b > 0 && b <= 100000 && a <= b,
                        (None, None, Some(t)) => time(t),
                        _ => false,
                    }
            }
            Self::SkillHistory(p) => {
                matches!(
                    p.cause.as_str(),
                    "training"
                        | "vocation_choice"
                        | "promotion"
                        | "offline_settlement"
                        | "logout"
                        | "death"
                ) && !p.skills.is_empty()
                    && p.skills.len() <= 8
                    && p.skills.windows(2).all(|v| v[0].skill < v[1].skill)
                    && p.skills.iter().all(|v| {
                        matches!(
                            v.skill.as_str(),
                            "magic"
                                | "fist"
                                | "club"
                                | "sword"
                                | "axe"
                                | "distance"
                                | "shielding"
                                | "fishing"
                        ) && (if v.skill == "magic" { 0 } else { 10 }..=1000)
                            .contains(&v.level_before)
                            && (if v.skill == "magic" { 0 } else { 10 }..=1000)
                                .contains(&v.level_after)
                    })
            }
            Self::DeathRecap(p) => {
                p.level_after > 0
                    && p.level_after <= p.level_before
                    && p.blessings_before.len() <= 32
                    && p.blessings_before.windows(2).all(|v| v[0] < v[1])
                    && p.blessings_before.iter().all(|v| {
                        v.len() <= 128
                            && v.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
                            && v.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
                    })
                    && p.blessings_after.windows(2).all(|v| v[0] < v[1])
                    && p.blessings_after
                        .iter()
                        .all(|v| p.blessings_before.contains(v))
                    && p.lost_items.len() <= 256
                    && p.lost_items
                        .windows(2)
                        .all(|v| v[0].loss_index < v[1].loss_index)
                    && p.lost_items
                        .iter()
                        .all(|v| v.loss_index <= 255 && (1..=65535).contains(&v.quantity))
                    && p.recap_state == "UNAVAILABLE"
                    && p.window_ms == 0
                    && p.actions.is_empty()
            }
            Self::Pvp(p) => {
                matches!(p.role.as_str(), "KILL" | "ASSIST" | "VICTIM")
                    && match p.justification.as_str() {
                        "UNKNOWN" => p.points_milli.is_none(),
                        "JUSTIFIED" => p.points_milli == Some(0),
                        "UNJUSTIFIED" => p.points_milli.is_some(),
                        _ => false,
                    }
            }
            Self::Skull(p) => match p.skull.as_str() {
                "NONE" => p.skull_until_unix_ms.is_none(),
                "RED" | "BLACK" => p.skull_until_unix_ms.is_some(),
                _ => false,
            },
            Self::Stamina(p) => p.stamina_minutes <= 2520,
            Self::PremiumEnd(p) => matches!(
                p.observed_state.as_str(),
                "ACTIVE" | "NOT_YET_EFFECTIVE" | "EXPIRED" | "REVOKED" | "NONE"
            ),
            Self::GameLogin(p) => match (p.left_at.as_deref(), p.end_reason.as_deref()) {
                (None, None) => true,
                (Some(t), Some(r)) => {
                    t.parse::<u64>().ok() >= p.entered_at.parse::<u64>().ok()
                        && matches!(r, "LOGOUT" | "DISCONNECT" | "REVOKED" | "UNKNOWN")
                }
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mutations(bytes: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let text = std::str::from_utf8(bytes)?;
        let first = text.find(',').ok_or("first member")?;
        let second = first + 1 + text[first + 1..].find(',').ok_or("second member")?;
        Ok(vec![
            text.replacen("{", "{\"extra\":0,", 1).into_bytes(),
            format!("{{{}", &text[first + 1..]).into_bytes(),
            format!(
                "{{{},{},{}",
                &text[first + 1..second],
                &text[1..first],
                &text[second + 1..]
            )
            .into_bytes(),
            vec![b' '; 262145],
        ])
    }
    #[test]
    fn journal_progress_payload_goldens() -> Result<(), Box<dyn std::error::Error>> {
        let bytes=br#"{"race_key":"rat","race_definition_revision":"r1","kill_count":100,"final_kill_threshold":100,"charm_points_awarded":5}"#;
        assert_eq!(ProgressPayload::decode("bestiary", bytes)?.encode()?, bytes);
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("bestiary", &invalid).is_err());
        }
        let bytes=br#"{"boss_key":"boss/example","definition_revision":"r1","kill_count":"2","available_at_unix_ms":"1790086400000","last_death_occurrence_id":"01934f10-7c04-7001-805b-3b1122334410"}"#;
        assert_eq!(
            ProgressPayload::decode("boss_cooldown", bytes)?.encode()?,
            bytes
        );
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("boss_cooldown", &invalid).is_err());
        }
        let bytes=br#"{"quest_key":"quest/example","pinned_content_revision":"r1","definition_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","completed":false,"tracks":[{"track_key":"quest/example/progress","value":"2"}]}"#;
        assert_eq!(ProgressPayload::decode("quest", bytes)?.encode()?, bytes);
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("quest", &invalid).is_err());
        }
        let bytes=br#"{"achievement_key":"oteryn:achievement/example","definition_revision":"r1","progress":2,"target":10,"granted_at_unix_ms":null}"#;
        assert_eq!(
            ProgressPayload::decode("achievement", bytes)?.encode()?,
            bytes
        );
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("achievement", &invalid).is_err());
        }
        let bytes=br#"{"achievement_key":"oteryn:achievement/example","definition_revision":"r1","progress":null,"target":null,"granted_at_unix_ms":"1790000000000"}"#;
        assert_eq!(
            ProgressPayload::decode("achievement", bytes)?.encode()?,
            bytes
        );
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("achievement", &invalid).is_err());
        }
        let bytes=br#"{"build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411","cause":"training","policy_revision":"r1","skills":[{"skill":"sword","level_before":10,"progress_before":"9","level_after":11,"progress_after":"0"}]}"#;
        assert_eq!(
            ProgressPayload::decode("skill_history", bytes)?.encode()?,
            bytes
        );
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("skill_history", &invalid).is_err());
        }
        let bytes=br#"{"death_occurrence_id":"01934f10-7c04-7001-805b-3b1122334412","level_before":20,"level_after":19,"experience_lost":"1000","blessings_before":["blessing/example"],"blessings_after":[],"lost_items":[{"loss_index":0,"item_key":"item/example","quantity":1}],"recap_state":"UNAVAILABLE","window_ms":0,"actions":[]}"#;
        assert_eq!(
            ProgressPayload::decode("death_recap", bytes)?.encode()?,
            bytes
        );
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("death_recap", &invalid).is_err());
        }
        let bytes=br#"{"death_occurrence_id":"01934f10-7c04-7001-805b-3b1122334412","role":"VICTIM","counterpart_character_id":null,"justification":"UNKNOWN","points_milli":null}"#;
        assert_eq!(ProgressPayload::decode("pvp", bytes)?.encode()?, bytes);
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("pvp", &invalid).is_err());
        }
        let bytes=br#"{"skull":"RED","skull_until_unix_ms":"1790086400000","kill_block_until_unix_ms":null}"#;
        assert_eq!(ProgressPayload::decode("skull", bytes)?.encode()?, bytes);
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("skull", &invalid).is_err());
        }
        let bytes=br#"{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}"#;
        assert_eq!(ProgressPayload::decode("stamina", bytes)?.encode()?, bytes);
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("stamina", &invalid).is_err());
        }
        let bytes=br#"{"observed_state":"ACTIVE","effective_until_unix_us":"1790086400000000","authority_revision":"4"}"#;
        assert_eq!(
            ProgressPayload::decode("premium_end", bytes)?.encode()?,
            bytes
        );
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("premium_end", &invalid).is_err());
        }
        let bytes=br#"{"login_event_id":"01934f10-7c04-7001-805b-3b1122334413","entered_at":"1790000000000","left_at":null,"end_reason":null}"#;
        assert_eq!(
            ProgressPayload::decode("game_login", bytes)?.encode()?,
            bytes
        );
        for invalid in mutations(bytes)? {
            assert!(ProgressPayload::decode("game_login", &invalid).is_err());
        }
        let bytes=br#"{"stamina_minutes":2521,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}"#;
        assert!(ProgressPayload::decode("stamina", bytes).is_err());
        let bytes =
            br#"{"skull":"BLACK","skull_until_unix_ms":null,"kill_block_until_unix_ms":null}"#;
        assert!(ProgressPayload::decode("skull", bytes).is_err());
        let bytes=br#"{"login_event_id":"01934f10-7c04-7001-805b-3b1122334413","entered_at":"1790000000000","left_at":null,"end_reason":null,"ip_address":"192.0.2.1"}"#;
        assert!(ProgressPayload::decode("game_login", bytes).is_err());
        let bytes=br#"{"quest_key":"quest/example","pinned_content_revision":"r1","definition_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","completed":false,"tracks":[{"track_key":"quest/example/progress","value":"02"}]}"#;
        assert!(ProgressPayload::decode("quest", bytes).is_err());
        Ok(())
    }
}

fn family(v: &str) -> bool {
    matches!(
        v,
        "bestiary"
            | "boss_cooldown"
            | "quest"
            | "achievement"
            | "skill_history"
            | "death_recap"
            | "pvp"
            | "skull"
            | "stamina"
            | "premium_end"
            | "game_login"
    )
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishGameJournalProgressV1 {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub character_id: Option<String>,
    pub world_id: Option<String>,
    pub projection_revision: Option<String>,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub family: String,
    pub source_revision: String,
    pub record_id: String,
    pub entity_key: String,
    pub character_revision: Option<String>,
    pub occurred_at_unix_ms: String,
    pub record_kind: String,
    pub payload: Option<ProgressPayload>,
}
impl PublishGameJournalProgressV1 {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        if self.contract_version != 1
            || !(self.operation == "PublishGameJournalProgressV1"
                && source_authority_valid(&self.source_authority)
                && uuid_v7(&self.account_id)
                && counter(&self.projection_epoch).is_some()
                && counter(&self.recovery_generation).is_some()
                && counter(&self.source_revision).is_some()
                && uuid_v7(&self.record_id)
                && family(&self.family)
                && time(&self.occurred_at_unix_ms)
                && !self.entity_key.is_empty()
                && self.entity_key.len() <= 512
                && self
                    .entity_key
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && self
                    .entity_key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
                && self.scope_valid()
                && match (self.record_kind.as_str(), &self.payload) {
                    ("REMOVE", None) => true,
                    ("UPSERT", Some(p)) => {
                        p.encode()
                            .is_ok_and(|b| ProgressPayload::decode(&self.family, &b).is_ok())
                            && self.entity_matches(p)
                    }
                    _ => false,
                })
        {
            return Err(SourceError::InvalidInput);
        }
        let b = serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)?;
        if b.len() > 262144 {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 262144 {
            return Err(SourceError::CapacityExceeded);
        }
        let v: Self = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
        if v.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(v)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishGameJournalProgressV1Response {
    pub contract_version: u8,
    pub result: String,
    pub record_id: String,
    pub accepted_source_revision: String,
}
impl PublishGameJournalProgressV1Response {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        if self.contract_version != 1
            || !(self.result == "accepted"
                && uuid_v7(&self.record_id)
                && counter(&self.accepted_source_revision).is_some())
        {
            return Err(SourceError::InvalidInput);
        }
        let b = serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)?;
        if b.len() > 256 {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 256 {
            return Err(SourceError::CapacityExceeded);
        }
        let v: Self = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
        if v.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(v)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishGameJournalProgressWatermarkV1 {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub family: String,
    pub complete_through: String,
    pub observed_at_unix_ms: String,
    pub baseline_complete: bool,
}
impl PublishGameJournalProgressWatermarkV1 {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        if self.contract_version != 1
            || !(self.operation == "PublishGameJournalProgressWatermarkV1"
                && source_authority_valid(&self.source_authority)
                && uuid_v7(&self.account_id)
                && counter(&self.projection_epoch).is_some()
                && counter(&self.recovery_generation).is_some()
                && family(&self.family)
                && natural(&self.complete_through)
                && time(&self.observed_at_unix_ms)
                && (self.complete_through != "0" || self.baseline_complete))
        {
            return Err(SourceError::InvalidInput);
        }
        let b = serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)?;
        if b.len() > 1024 {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 1024 {
            return Err(SourceError::CapacityExceeded);
        }
        let v: Self = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
        if v.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(v)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishGameJournalProgressWatermarkV1Response {
    pub contract_version: u8,
    pub result: String,
    pub complete_through: String,
}
impl PublishGameJournalProgressWatermarkV1Response {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        if self.contract_version != 1
            || !(self.result == "accepted" && natural(&self.complete_through))
        {
            return Err(SourceError::InvalidInput);
        }
        let b = serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)?;
        if b.len() > 256 {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 256 {
            return Err(SourceError::CapacityExceeded);
        }
        let v: Self = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
        if v.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(v)
    }
}
impl PublishGameJournalProgressV1 {
    fn scope_valid(&self) -> bool {
        let character = self.character_id.as_deref().is_some_and(uuid_v7)
            && self.world_id.as_deref().is_some_and(uuid_v7)
            && self
                .character_revision
                .as_deref()
                .is_some_and(|v| counter(v).is_some())
            && self
                .projection_revision
                .as_deref()
                .is_some_and(|v| counter(v).is_some());
        let account = self.character_id.is_none()
            && self.world_id.is_none()
            && self.character_revision.is_none();
        match self.family.as_str() {
            "premium_end" => {
                account
                    && self
                        .projection_revision
                        .as_deref()
                        .is_some_and(|v| counter(v).is_some())
            }
            "achievement" | "game_login" => {
                character || (account && self.projection_revision.is_none())
            }
            _ => character,
        }
    }
    fn entity_matches(&self, p: &ProgressPayload) -> bool {
        match p {
            ProgressPayload::Bestiary(p) => self.entity_key == p.race_key,
            ProgressPayload::BossCooldown(p) => self.entity_key == p.boss_key,
            ProgressPayload::Quest(p) => self.entity_key == p.quest_key,
            ProgressPayload::Achievement(p) => {
                self.entity_key == p.achievement_key
                    && (self.character_id.is_some() == p.progress.is_some())
            }
            ProgressPayload::SkillHistory(p) => self.entity_key == p.build_occurrence_id,
            ProgressPayload::DeathRecap(p) => self.entity_key == p.death_occurrence_id,
            ProgressPayload::Pvp(p) => self.entity_key == p.death_occurrence_id,
            ProgressPayload::Skull(_) => self.entity_key == "skull",
            ProgressPayload::Stamina(_) => self.entity_key == "stamina",
            ProgressPayload::PremiumEnd(p) => {
                self.entity_key == "premium"
                    && self.projection_revision.as_deref() == Some(p.authority_revision.as_str())
            }
            ProgressPayload::GameLogin(p) => self.entity_key == p.login_event_id,
        }
    }
}

#[cfg(test)]
mod envelope_tests {
    use super::*;
    #[test]
    fn journal_progress_envelope_goldens() -> Result<(), Box<dyn std::error::Error>> {
        let bytes=br#"{"contract_version":1,"operation":"PublishGameJournalProgressV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c04-7001-805b-3b1122334402","projection_revision":"12","projection_epoch":"1","recovery_generation":"3","family":"stamina","source_revision":"1","record_id":"01934f10-7c04-7001-805b-3b1122334403","entity_key":"stamina","character_revision":"9","occurred_at_unix_ms":"1790000000000","record_kind":"UPSERT","payload":{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}}"#;
        assert_eq!(
            PublishGameJournalProgressV1::decode(bytes)?.encode()?,
            bytes
        );
        let text = std::str::from_utf8(bytes)?;
        assert!(
            PublishGameJournalProgressV1::decode(text.replacen("{", "{\"extra\":0,", 1).as_bytes())
                .is_err()
        );
        assert!(
            PublishGameJournalProgressV1::decode(
                text.replace("\"contract_version\":1,", "").as_bytes()
            )
            .is_err()
        );
        assert!(PublishGameJournalProgressV1::decode(&[bytes.as_slice(), b"\n"].concat()).is_err());
        let bytes=br#"{"contract_version":1,"result":"accepted","record_id":"01934f10-7c04-7001-805b-3b1122334403","accepted_source_revision":"1"}"#;
        assert_eq!(
            PublishGameJournalProgressV1Response::decode(bytes)?.encode()?,
            bytes
        );
        let text = std::str::from_utf8(bytes)?;
        assert!(
            PublishGameJournalProgressV1Response::decode(
                text.replacen("{", "{\"extra\":0,", 1).as_bytes()
            )
            .is_err()
        );
        assert!(
            PublishGameJournalProgressV1Response::decode(
                text.replace("\"contract_version\":1,", "").as_bytes()
            )
            .is_err()
        );
        assert!(
            PublishGameJournalProgressV1Response::decode(&[bytes.as_slice(), b"\n"].concat())
                .is_err()
        );
        let bytes=br#"{"contract_version":1,"operation":"PublishGameJournalProgressWatermarkV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","projection_epoch":"1","recovery_generation":"3","family":"stamina","complete_through":"1","observed_at_unix_ms":"1790000000000","baseline_complete":true}"#;
        assert_eq!(
            PublishGameJournalProgressWatermarkV1::decode(bytes)?.encode()?,
            bytes
        );
        let text = std::str::from_utf8(bytes)?;
        assert!(
            PublishGameJournalProgressWatermarkV1::decode(
                text.replacen("{", "{\"extra\":0,", 1).as_bytes()
            )
            .is_err()
        );
        assert!(
            PublishGameJournalProgressWatermarkV1::decode(
                text.replace("\"contract_version\":1,", "").as_bytes()
            )
            .is_err()
        );
        assert!(
            PublishGameJournalProgressWatermarkV1::decode(&[bytes.as_slice(), b"\n"].concat())
                .is_err()
        );
        let bytes=br#"{"contract_version":1,"operation":"PublishGameJournalProgressV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c04-7001-805b-3b1122334402","projection_revision":"12","projection_epoch":"1","recovery_generation":"3","family":"stamina","source_revision":1,"record_id":"01934f10-7c04-7001-805b-3b1122334403","entity_key":"stamina","character_revision":"9","occurred_at_unix_ms":"1790000000000","record_kind":"UPSERT","payload":{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}}"#;
        assert!(PublishGameJournalProgressV1::decode(bytes).is_err());
        let bytes=br#"{"contract_version":1,"operation":"PublishGameJournalProgressWatermarkV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","projection_epoch":"1","recovery_generation":"3","family":"stamina","complete_through":"2","observed_at_unix_ms":"1790000000000","baseline_complete":true}"#;
        assert_eq!(
            PublishGameJournalProgressWatermarkV1::decode(bytes)?.encode()?,
            bytes
        );
        let text = std::str::from_utf8(bytes)?;
        assert!(
            PublishGameJournalProgressWatermarkV1::decode(
                text.replacen("{", "{\"extra\":0,", 1).as_bytes()
            )
            .is_err()
        );
        assert!(
            PublishGameJournalProgressWatermarkV1::decode(
                text.replace("\"contract_version\":1,", "").as_bytes()
            )
            .is_err()
        );
        assert!(
            PublishGameJournalProgressWatermarkV1::decode(&[bytes.as_slice(), b"\n"].concat())
                .is_err()
        );
        assert!(PublishGameJournalProgressV1::decode(&vec![b' '; 262145]).is_err());
        Ok(())
    }
}
