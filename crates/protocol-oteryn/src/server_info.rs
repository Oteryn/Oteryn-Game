//! Read-only, independently negotiated presentation facts from gameplay owners.
use serde::{Deserialize, Serialize};
pub const CAPABILITY_SERVER_INFO_V1: u32 = 500;
pub const COMMAND_TYPE_SERVER_INFO_QUERY: u32 = 6241;
pub const MAX_REPLY_BYTES: usize = 65_536;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Query {
    Quests,
    Spells(Vec<u32>),
}
impl Query {
    pub fn valid(&self) -> bool {
        match self {
            Self::Spells(ids) => {
                !ids.is_empty()
                    && ids.len() <= 16
                    && ids.iter().all(|id| *id > 0 && *id < (1 << 28))
                    && ids.windows(2).all(|p| p[0] < p[1])
            }
            _ => true,
        }
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        self.valid()
            .then(|| serde_json::to_vec(self).ok())
            .flatten()
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 256 {
            return None;
        }
        let q: Self = serde_json::from_slice(bytes).ok()?;
        q.valid().then_some(q)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellInfo {
    pub cast_index: u32,
    /// Absent only if accepted catalogue content has no library text.
    pub description: Option<String>,
    pub cooldown_ms: u64,
    /// Group ordinal and this spell's base duration. Different spells can share a group.
    pub group_cooldowns_ms: Vec<(u32, u64)>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Reply {
    Quests(Vec<(u32, String)>),
    Spells(Vec<SpellInfo>),
}
impl Reply {
    pub fn valid(&self) -> bool {
        let text = |s: &str, max| !s.is_empty() && s.len() <= max && !s.contains('\0');
        match self {
            Self::Quests(rows) => {
                rows.len() <= 1024
                    && rows.windows(2).all(|p| p[0].0 < p[1].0)
                    && rows.iter().all(|(_, s)| text(s, 128))
            }
            Self::Spells(rows) => {
                rows.len() <= 16
                    && rows.windows(2).all(|p| p[0].cast_index < p[1].cast_index)
                    && rows.iter().all(|s| {
                        s.cast_index > 0
                            && s.cast_index < (1 << 28)
                            && s.description.as_ref().is_none_or(|s| text(s, 8192))
                            && s.group_cooldowns_ms.len() <= 16
                            && s.group_cooldowns_ms.windows(2).all(|p| p[0].0 < p[1].0)
                            && s.group_cooldowns_ms
                                .iter()
                                .all(|(id, _)| (1..=16).contains(id))
                    })
            }
        }
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let b = serde_json::to_vec(self).ok()?;
        (b.len() <= MAX_REPLY_BYTES).then_some(b)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REPLY_BYTES {
            return None;
        }
        let r: Self = serde_json::from_slice(bytes).ok()?;
        r.valid().then_some(r)
    }
    pub fn matches(&self, q: &Query) -> bool {
        match (self, q) {
            (Self::Quests(_), Query::Quests) => true,
            (Self::Spells(rows), Query::Spells(ids)) => {
                rows.iter().map(|r| r.cast_index).eq(ids.iter().copied())
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_info_codec_rejects_unbound_indices_unknown_fields_and_excessive_text()
    -> Result<(), &'static str> {
        for ids in [vec![], vec![0], vec![2, 1], vec![1, 1], (1..=17).collect()] {
            assert!(Query::Spells(ids).encode().is_none());
        }
        assert!(Query::decode(br#"{"Spells":[1],"extra":true}"#).is_none());
        assert!(Reply::decode(br#"{"Spells":[{"cast_index":1,"description":null,"cooldown_ms":1,"group_cooldowns_ms":[],"extra":1}]}"#).is_none());
        assert!(Reply::decode(&[255]).is_none());
        assert!(Reply::Quests(vec![(1, "x".repeat(129))]).encode().is_none());
        let reply = Reply::Spells(vec![SpellInfo {
            cast_index: 1,
            description: Some("Actual catalogue text".into()),
            cooldown_ms: 2000,
            group_cooldowns_ms: vec![(2, 1000)],
        }]);
        let bytes = reply.encode().ok_or("encode")?;
        assert_eq!(Reply::decode(&bytes), Some(reply.clone()));
        assert!(reply.matches(&Query::Spells(vec![1])));
        assert!(!reply.matches(&Query::Spells(vec![2])));
        assert!(!reply.matches(&Query::Quests));
        for end in 0..bytes.len() {
            assert!(Reply::decode(&bytes[..end]).is_none());
        }
        Ok(())
    }
}
