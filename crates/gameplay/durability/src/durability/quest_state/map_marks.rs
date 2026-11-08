//! Personal display facts; only the existing live-fenced Quest writer creates them.
use super::*;
use oteryn_gameplay_quests::quest::map_marks::{MAX_PERSONAL_QUEST_MAP_MARKS, QuestMapMark};
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonalQuestMapMark {
    pub key: String,
    pub quest: String,
    pub marker: QuestMapMark,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestMapMarksSnapshot {
    pub character_revision: u64,
    pub marks: Vec<PersonalQuestMapMark>,
}

pub(super) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &RewardCallback,
    committed_revision: u64,
) -> StoredResult<()> {
    let Some(mark) = &callback.map_mark else {
        return Ok(());
    };
    if !mark.valid() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let id = fence.character_id.as_bytes().as_slice();
    let world = fence.runtime_scope.world_id();
    let payload = serde_json::to_value(mark).map_err(|_| DurabilityError::InvalidStoredState)?;
    let existing: Option<bool> = sqlx::query_scalar(
        "SELECT world_id=encode($3,'hex')::uuid AND quest_key=$4 \
            AND game_quest_map_mark_payload(m)=$5 \
         FROM game_character_quest_map_marks m \
         WHERE character_id=encode($1,'hex')::uuid AND source_key=$2 FOR SHARE",
    )
    .bind(id)
    .bind(&callback.key)
    .bind(world.as_bytes().as_slice())
    .bind(&callback.quest)
    .bind(payload)
    .fetch_optional(&mut **tx)
    .await?;
    match existing {
        Some(true) => return Ok(()),
        Some(false) => return Err(DurabilityError::InvalidStoredState),
        None => {}
    }
    sqlx::query("INSERT INTO game_character_quest_map_marks \
        (character_id,source_key,quest_key,world_id,x,y,floor,kind,label,committed_character_revision) \
        VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5,$6,$7,'cross',$8,$9::text::numeric(20,0))")
        .bind(id).bind(&callback.key).bind(&callback.quest).bind(world.as_bytes().as_slice())
        .bind(i32::from(mark.position.x)).bind(i32::from(mark.position.y))
        .bind(i16::from(mark.position.floor)).bind(&mark.text).bind(committed_revision.to_string())
        .execute(&mut **tx).await?;
    Ok(())
}

impl DurabilityRoot {
    /// A bounded current-World projection. Display facts never authorize gameplay writes.
    pub async fn read_character_quest_map_marks(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<QuestMapMarksSnapshot> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let world = fence.runtime_scope.world_id();
                    let rows = sqlx::query(
                        "SELECT source_key,quest_key, \
                    jsonb_build_object('position',jsonb_build_object('x',x,'y',y,'floor',floor), \
                        'kind',kind,'text',label) AS marker \
                    FROM game_character_quest_map_marks \
                    WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
                    ORDER BY source_key LIMIT $3",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(world.as_bytes().as_slice())
                    .bind(
                        i64::try_from(MAX_PERSONAL_QUEST_MAP_MARKS + 1)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                    )
                    .fetch_all(&mut *tx)
                    .await?;
                    if rows.len() > MAX_PERSONAL_QUEST_MAP_MARKS {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let mut marks = Vec::with_capacity(rows.len());
                    for row in rows {
                        let marker: QuestMapMark =
                            serde_json::from_value(row.try_get("marker")?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?;
                        if !marker.valid() {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        marks.push(PersonalQuestMapMark {
                            key: row.try_get("source_key")?,
                            quest: row.try_get("quest_key")?,
                            marker,
                        });
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(QuestMapMarksSnapshot {
                        character_revision: root.revision,
                        marks,
                    }))
                })
            })
            .await?
    }
}
