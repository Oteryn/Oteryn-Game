//! Bounded descriptive provenance for death display, never tick authority or
//! combat credit. It lives and expires with one canonical condition instance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionHistoricalSource {
    PlayerSession {
        game_session_id: [u8; 16],
    },
    Creature {
        key: String,
        name: String,
    },
    Unpublishable {
        source_digest: [u8; 32],
        creature_key: Option<String>,
    },
}
impl ConditionHistoricalSource {
    pub(super) fn valid(&self) -> bool {
        match self {
            Self::PlayerSession { game_session_id } => {
                game_session_id[6] >> 4 == 7 && game_session_id[8] & 0xc0 == 0x80
            }
            Self::Creature { key, name } => {
                !matches!(key.as_str(), "player" | "unknown")
                    && !key.is_empty()
                    && key.len() <= 128
                    && key.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b)
                    })
                    && !name.is_empty()
                    && name.len() <= 64
                    && !matches!(name.as_str(), "." | "..")
                    && name
                        .bytes()
                        .all(|b| (32..=126).contains(&b) && !b"\"\\/".contains(&b))
            }
            Self::Unpublishable { creature_key, .. } => creature_key.as_ref().is_none_or(|key| {
                !matches!(key.as_str(), "player" | "unknown")
                    && !key.is_empty()
                    && key.len() <= 255
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b":._/-".contains(&b))
            }),
        }
    }
}
