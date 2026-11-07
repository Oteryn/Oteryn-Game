//! Existing D35 semantics shared by ordinary and Source-NPC Quest writers.
use super::{CharacterId, DurabilityError};
use sqlx::Row;

pub(super) struct StoredQuestPin {
    pub content_revision: String,
    pub definition_hash: [u8; 32],
    pub completed: bool,
}
pub(super) struct PreparedQuestPin {
    pub pinned_content_revision: String,
    pub pinned_definition_hash: [u8; 32],
    pub evaluation_definition_hash: [u8; 32],
    pub state_exists: bool,
}
pub(super) fn resolve(
    current_revision: &str,
    current_hash: [u8; 32],
    stored: Option<StoredQuestPin>,
) -> Result<PreparedQuestPin, oteryn_gameplay_quests::quest::QuestRefusal> {
    use oteryn_gameplay_quests::quest::QuestRefusal;
    match stored {
        Some(old) if old.definition_hash != current_hash && !old.completed => {
            Err(QuestRefusal::RevisionMismatch)
        }
        Some(old) => Ok(PreparedQuestPin {
            pinned_content_revision: old.content_revision,
            pinned_definition_hash: old.definition_hash,
            evaluation_definition_hash: current_hash,
            state_exists: true,
        }),
        None => Ok(PreparedQuestPin {
            pinned_content_revision: current_revision.to_owned(),
            pinned_definition_hash: current_hash,
            evaluation_definition_hash: current_hash,
            state_exists: false,
        }),
    }
}
pub(super) async fn read_locked(
    tx: &mut sqlx::PgConnection,
    character: CharacterId,
    quest: &str,
) -> Result<Option<StoredQuestPin>, DurabilityError> {
    let row = sqlx::query("SELECT pinned_content_revision, definition_hash, completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE")
        .bind(character.as_bytes().as_slice()).bind(quest).fetch_optional(&mut *tx).await?;
    row.map(|row| {
        Ok(StoredQuestPin {
            content_revision: row.try_get("pinned_content_revision")?,
            definition_hash: row
                .try_get::<Vec<u8>, _>("definition_hash")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            completed: row.try_get("completed")?,
        })
    })
    .transpose()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_quest_child_keeps_mature_d35_completed_and_pending_semantics() -> Result<(), String> {
        let current = [2; 32];
        let make = |completed| StoredQuestPin {
            content_revision: "old".into(),
            definition_hash: [1; 32],
            completed,
        };
        assert!(matches!(
            resolve("new", current, Some(make(false))),
            Err(oteryn_gameplay_quests::quest::QuestRefusal::RevisionMismatch)
        ));
        let completed = resolve("new", current, Some(make(true)))
            .map_err(|error| format!("completed pin: {error:?}"))?;
        assert_eq!(completed.pinned_content_revision, "old");
        assert_eq!(completed.pinned_definition_hash, [1; 32]);
        assert_eq!(completed.evaluation_definition_hash, current);
        assert!(completed.state_exists);
        let new = resolve("new", current, None).map_err(|error| format!("new pin: {error:?}"))?;
        assert_eq!(new.pinned_content_revision, "new");
        assert_eq!(new.pinned_definition_hash, current);
        assert!(!new.state_exists);
        Ok(())
    }
}
