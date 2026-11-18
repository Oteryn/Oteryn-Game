//! Crystal 201189f8 data-global/npc/paulie.lua:160-169. The Bank owner holds the
//! acting Character root and commits this stage with the audited conversion.
use super::*;
use oteryn_gameplay_quests::quest::npc_rules::BoundBankDepositBonus;

pub(in crate::durability) struct PreparedBonus<'a> {
    character: CharacterId,
    root_revision: u64,
    request: QuestTransitionRequest,
    pin: String,
    hash: [u8; 32],
    bonus: &'a BoundBankDepositBonus,
}

pub(in crate::durability) async fn prepare<'a>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    command: CommandRef,
    bonus: &'a BoundBankDepositBonus,
) -> StoredResult<Option<PreparedBonus<'a>>> {
    let row = sqlx::query(
        "SELECT q.pinned_content_revision, q.definition_hash, r.character_revision::text \
         FROM game_character_quest_tracks t \
         JOIN game_character_quest_states q USING(character_id,quest_key) \
         JOIN game_character_roots r USING(character_id) \
         JOIN game_character_progression_state p USING(character_id) \
         WHERE t.character_id=encode($1,'hex')::uuid AND t.quest_key=$2 AND t.track_key=$3 \
           AND t.value=$4 AND q.completed_character_revision IS NULL \
           AND r.character_revision=p.character_revision AND p.content_revision=$5 \
           AND NOT EXISTS(SELECT 1 FROM game_character_pending_respawns d WHERE d.character_id=t.character_id) \
         FOR UPDATE OF q,t,p",
    )
    .bind(character.as_bytes().as_slice())
    .bind(&bonus.quest)
    .bind(&bonus.rule.track)
    .bind(bonus.rule.before)
    .bind(&bonus.content_revision)
    .fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None) };
    let hash: [u8; 32] = row.try_get::<Vec<u8>, _>("definition_hash")?
        .try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
    if !bonus.definition_hashes.contains(&hash) {
        return Ok(None);
    }
    Ok(Some(PreparedBonus {
        character,
        root_revision: numeric_u64(&row, "character_revision")?,
        request: QuestTransitionRequest {
            cause: QuestCause::Command(command),
            transition_key: bonus.rule.key.clone(),
        },
        pin: row.try_get("pinned_content_revision")?,
        hash,
        bonus,
    }))
}

impl PreparedBonus<'_> {
    pub(in crate::durability) fn gold(&self) -> u64 {
        self.bonus.rule.gold
    }

    pub(in crate::durability) async fn commit(
        self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> StoredResult<()> {
        let rule = &self.bonus.rule;
        let transition = rule.transition(&self.bonus.quest);
        let change = QuestTrackChange {
            track: rule.track.clone(),
            before: rule.before,
            after: rule.after,
        };
        persist::persist(tx, persist::QuestWrite {
            character: self.character,
            root_revision: self.root_revision,
            request: &self.request,
            transition: &transition,
            pin: &self.pin,
            hash: &self.hash,
            changes: &[change],
            binding: &request_binding(self.character, &self.request),
            state_exists: true,
        }).await?;
        Ok(())
    }
}
