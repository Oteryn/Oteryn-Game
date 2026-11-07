//! PUBPROJ stable historical killer identity. Native identity is captured at the
//! lethal owner write, then copied by the fenced receipt transaction. Display
//! names never become join keys (Game public projections V2, owner 2026-11-07).
use super::super::pvp_death_request::{PvpContributorKind, PvpDeathContributor};
use super::*;

/// Fixed-size native provenance keeps the runtime death receipt Copy and bounded.
/// A GameSessionId is private provenance only; its persisted CharacterId is what
/// is captured. Unknown/environment sources remain None, never guessed creatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicDeathCause {
    /// Known native provenance which the accepted public schema cannot encode.
    /// Its digest remains part of occurrence identity; publication fails closed.
    Unpublishable {
        source_digest: [u8; 32],
    },
    Player {
        character_id: [u8; 16],
    },
    PlayerSession {
        game_session_id: [u8; 16],
    },
    Creature {
        key: [u8; 128],
        key_len: u8,
        name: [u8; 64],
        name_len: u8,
    },
}
impl PublicDeathCause {
    pub fn native_creature(key: &str, name: &str) -> Self {
        Self::creature(key, name).unwrap_or_else(|| {
            let mut hash = Sha256::new();
            hash.update(b"oteryn:unpublishable-native-death-cause:v1");
            hash.update((key.len() as u64).to_be_bytes());
            hash.update(key.as_bytes());
            hash.update((name.len() as u64).to_be_bytes());
            hash.update(name.as_bytes());
            Self::Unpublishable {
                source_digest: hash.finalize().into(),
            }
        })
    }
    pub fn creature(key: &str, name: &str) -> Option<Self> {
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b))
            || !crate::native_admission_source::public_projections::projection_name_valid(name)
        {
            return None;
        }
        let mut stored_key = [0; 128];
        let mut stored_name = [0; 64];
        stored_key[..key.len()].copy_from_slice(key.as_bytes());
        stored_name[..name.len()].copy_from_slice(name.as_bytes());
        Some(Self::Creature {
            key: stored_key,
            key_len: u8::try_from(key.len()).ok()?,
            name: stored_name,
            name_len: u8::try_from(name.len()).ok()?,
        })
    }
    fn creature_parts(&self) -> Option<(&str, &str)> {
        let Self::Creature {
            key,
            key_len,
            name,
            name_len,
        } = self
        else {
            return None;
        };
        let (key, tail) = key.split_at_checked(usize::from(*key_len))?;
        let (name, name_tail) = name.split_at_checked(usize::from(*name_len))?;
        if tail.iter().chain(name_tail).any(|b| *b != 0) {
            return None;
        }
        Some((
            std::str::from_utf8(key).ok()?,
            std::str::from_utf8(name).ok()?,
        ))
    }
    pub(super) fn bind(&self, semantic: &mut Vec<u8>) -> Result<()> {
        semantic.extend_from_slice(b"oteryn:death-public-cause:v1");
        match self {
            Self::Unpublishable { source_digest } => {
                semantic.push(4);
                semantic.extend_from_slice(source_digest);
            }
            Self::Player { character_id } => {
                CharacterId::from_bytes(*character_id)
                    .map_err(|_| CharacterProgressionError::InvalidInput)?;
                semantic.push(1);
                semantic.extend_from_slice(character_id);
            }
            Self::PlayerSession { game_session_id } => {
                crate::foundation::GameSessionId::decode(game_session_id)
                    .map_err(|_| CharacterProgressionError::InvalidInput)?;
                semantic.push(2);
                semantic.extend_from_slice(game_session_id);
            }
            Self::Creature { .. } => {
                let (key, name) = self
                    .creature_parts()
                    .ok_or(CharacterProgressionError::InvalidInput)?;
                if Self::creature(key, name).as_ref() != Some(self) {
                    return Err(CharacterProgressionError::InvalidInput);
                }
                semantic.push(3);
                push_bytes(semantic, key.as_bytes())?;
                push_bytes(semantic, name.as_bytes())?;
            }
        }
        Ok(())
    }
}

pub(super) async fn capture(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: PlayerDeathOccurrence,
    contributors: &[PvpDeathContributor],
    cause: Option<PublicDeathCause>,
) -> std::result::Result<(), DurabilityError> {
    let final_player = match cause {
        Some(PublicDeathCause::Player { character_id }) => Some(character_id),
        Some(PublicDeathCause::PlayerSession { game_session_id }) => {
            let id: Option<String> = sqlx::query_scalar("SELECT character_id::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
                .bind(game_session_id.as_slice()).fetch_optional(&mut **tx).await?;
            id.map(|id| uuid_text(&id).map_err(|_| DurabilityError::InvalidStoredState))
                .transpose()?
        }
        _ => None,
    };
    let mut players = Vec::with_capacity(contributors.len());
    for contributor in contributors {
        // The death already locks the contributor cohort in CharacterId order;
        // this read neither reacquires authority nor resolves a historical name.
        let name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(contributor.character_id.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        players.push((
            if final_player.as_ref() == Some(contributor.character_id.as_bytes()) {
                0
            } else {
                match contributor.kind {
                    PvpContributorKind::Damage => 1,
                    PvpContributorKind::Assist => 2,
                }
            },
            name,
            contributor.character_id,
        ));
    }
    players.sort_by(|a, b| {
        (a.0, a.1.as_bytes(), a.2.as_bytes()).cmp(&(b.0, b.1.as_bytes(), b.2.as_bytes()))
    });
    let mut ordinal: i16 = 0;
    // A native creature final blow is independent of any earlier player damage.
    if let Some(cause @ PublicDeathCause::Creature { .. }) = cause {
        let (key, name) = cause
            .creature_parts()
            .ok_or(DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_public_death_killers(death_occurrence_id,ordinal,kind,creature_key,name) VALUES(encode($1,'hex')::uuid,$2,'creature',$3,$4)")
            .bind(occurrence.as_bytes().as_slice()).bind(ordinal).bind(key).bind(name).execute(&mut **tx).await?;
        ordinal += 1;
    }
    for (_, name, character) in players {
        sqlx::query("INSERT INTO game_public_death_killers(death_occurrence_id,ordinal,kind,character_id,name) VALUES(encode($1,'hex')::uuid,$2,'player',encode($3,'hex')::uuid,$4)")
            .bind(occurrence.as_bytes().as_slice()).bind(ordinal).bind(character.as_bytes().as_slice()).bind(name).execute(&mut **tx).await?;
        ordinal += 1;
    }
    sqlx::query("INSERT INTO game_public_death_classifications(death_occurrence_id,pvp,publishable) VALUES(encode($1,'hex')::uuid,$2,$3)")
        .bind(occurrence.as_bytes().as_slice()).bind(!contributors.is_empty()).bind(!matches!(cause,Some(PublicDeathCause::Unpublishable {..}))).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "synthetic source identity and semantic binding assertions"
)]
mod tests {
    use super::*;
    #[test]
    fn native_creature_cause_preserves_identity_and_historical_display() {
        let cause = PublicDeathCause::creature("oteryn:creature.dragon", "Dragon").unwrap();
        assert_eq!(
            cause.creature_parts(),
            Some(("oteryn:creature.dragon", "Dragon"))
        );
        assert!(PublicDeathCause::creature("Dragon", "Dragon").is_none());
        let unknown = PublicDeathCause::native_creature("Dragon", "Dragon");
        assert!(matches!(unknown, PublicDeathCause::Unpublishable { .. }));
        let mut unknown_binding = Vec::new();
        unknown.bind(&mut unknown_binding).unwrap();
        let mut different_unknown = Vec::new();
        PublicDeathCause::native_creature("Other", "Dragon")
            .bind(&mut different_unknown)
            .unwrap();
        assert_ne!(unknown_binding, different_unknown);

        assert!(PublicDeathCause::creature("oteryn:creature.dragon", "Bad\\Name").is_none());
        let mut before = Vec::new();
        cause.bind(&mut before).unwrap();
        let mut renamed = Vec::new();
        PublicDeathCause::creature("oteryn:creature.dragon", "Old Dragon")
            .unwrap()
            .bind(&mut renamed)
            .unwrap();
        assert_ne!(before, renamed, "replayed source changes must conflict");
        let mut different = Vec::new();
        PublicDeathCause::creature("oteryn:creature.dragon_lord", "Dragon")
            .unwrap()
            .bind(&mut different)
            .unwrap();
        assert_ne!(
            before, different,
            "historical name cannot replace content identity"
        );
    }
}
