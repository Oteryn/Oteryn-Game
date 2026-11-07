use super::*;
use crate::durability::item_mint::uuid_text;
use oteryn_gameplay_encounter_rules::boss::{
    RewardParticipation,
    reward_chest::{FrozenBossContribution, PersonalRewardChest},
};
use sqlx::Row;

#[derive(Clone, Debug)]
pub(super) struct Entitlement {
    pub death: [u8; 16],
    pub boss_key: String,
    pub minted: u64,
    pub expires: u64,
    pub contribution: RewardParticipation,
    pub item: BossRewardPageItem,
}

pub(super) async fn now(tx: &mut Transaction<'_, Postgres>) -> Result<i64, DurabilityError> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?,
    )
}

pub(super) async fn lock_command(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:boss-reward-collect:' || encode($1,'hex') || ':' || $2,0))")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn receipt(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: &BossRewardCollectRequest,
) -> Result<Option<BossRewardCollection>, DurabilityError> {
    let row = sqlx::query("SELECT uuid_send(character_id) AS character,intent,status,moved,remaining FROM game_boss_reward_collection_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        if row.try_get::<Vec<u8>, _>("character")? != fence.character_id.as_bytes()
            || row.try_get::<Vec<u8>, _>("intent")? != request.intent
        {
            return Ok(BossRewardCollection::refused(
                BossRewardCollectionStatus::Rejected,
            ));
        }
        Ok(BossRewardCollection {
            status: decode_status(row.try_get("status")?)?,
            moved: u16::try_from(row.try_get::<i32, _>("moved")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            remaining: u32::try_from(row.try_get::<i64, _>("remaining")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        })
    })
    .transpose()
}

pub(super) fn status_code(status: BossRewardCollectionStatus) -> i16 {
    match status {
        BossRewardCollectionStatus::Collected => 0,
        BossRewardCollectionStatus::Empty => 1,
        BossRewardCollectionStatus::Expired => 2,
        BossRewardCollectionStatus::NoRoom => 3,
        BossRewardCollectionStatus::Rejected => 4,
        BossRewardCollectionStatus::ContentUnavailable => 5,
    }
}

fn decode_status(value: i16) -> Result<BossRewardCollectionStatus, DurabilityError> {
    Ok(match value {
        0 => BossRewardCollectionStatus::Collected,
        1 => BossRewardCollectionStatus::Empty,
        2 => BossRewardCollectionStatus::Expired,
        3 => BossRewardCollectionStatus::NoRoom,
        4 => BossRewardCollectionStatus::Rejected,
        5 => BossRewardCollectionStatus::ContentUnavailable,
        _ => return Err(DurabilityError::InvalidStoredState),
    })
}

pub(super) fn selection(selection: BossRewardSelection) -> (Option<[u8; 16]>, Option<i32>) {
    match selection {
        BossRewardSelection::All => (None, None),
        BossRewardSelection::Group { death } => (Some(death), None),
        BossRewardSelection::Item { death, ordinal } => (Some(death), Some(i32::from(ordinal))),
    }
}

pub(super) async fn selected_group_expired(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    selected: BossRewardSelection,
    now: i64,
) -> Result<bool, DurabilityError> {
    let (death, _) = selection(selected);
    if death.is_none() {
        return Ok(false);
    }
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_boss_reward_chests WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND death_id=encode($3,'hex')::uuid AND expires_at<=$4)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(death.as_ref().map(|d| d.as_slice())).bind(now).fetch_one(&mut **tx).await?)
}

pub(super) async fn pending(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    selected: BossRewardSelection,
    now: i64,
) -> Result<Vec<Entitlement>, DurabilityError> {
    let (death, ordinal) = selection(selected);
    let rows = sqlx::query("SELECT i.death_id::text,p.boss_key,p.minted_at,p.expires_at,c.damage_dealt::text,c.damage_received::text,c.healing_others::text,i.item_ordinal,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity FROM game_boss_reward_items i JOIN game_character_boss_reward_chests p USING(character_id,world_id,death_id) JOIN game_boss_reward_contributors c USING(character_id,world_id,death_id) WHERE i.character_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND ($3::bytea IS NULL OR i.death_id=encode($3,'hex')::uuid) AND ($4::integer IS NULL OR i.item_ordinal=$4) AND p.expires_at>$5 AND NOT i.expired AND i.collected_transaction_id IS NULL ORDER BY p.minted_at,i.death_id,i.item_ordinal LIMIT $6 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(death.as_ref().map(|d| d.as_slice())).bind(ordinal).bind(now).bind((BOSS_REWARD_COLLECTION_MAX + 1) as i64).fetch_all(&mut **tx).await?;
    rows.iter().map(decode).collect()
}

pub(super) fn decode(row: &sqlx::postgres::PgRow) -> Result<Entitlement, DurabilityError> {
    let number = |name| -> Result<u64, DurabilityError> {
        row.try_get::<String, _>(name)?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    Ok(Entitlement {
        death: uuid_text(row.try_get("death_id")?)?,
        boss_key: row.try_get("boss_key")?,
        minted: u64::try_from(row.try_get::<i64, _>("minted_at")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        expires: u64::try_from(row.try_get::<i64, _>("expires_at")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        contribution: RewardParticipation {
            damage_dealt: number("damage_dealt")?,
            damage_received: number("damage_received")?,
            healing_others: number("healing_others")?,
        },
        item: BossRewardPageItem {
            ordinal: u16::try_from(row.try_get::<i32, _>("item_ordinal")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            definition: TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
    })
}

pub(super) fn chest<I>(
    row: &Entitlement,
    character: [u8; 16],
    items: Vec<I>,
) -> Result<PersonalRewardChest<I>, DurabilityError> {
    let chest = PersonalRewardChest::reopen(
        row.boss_key.clone(),
        row.death,
        row.minted,
        FrozenBossContribution {
            character,
            contribution: row.contribution,
        },
        items,
    )
    .ok_or(DurabilityError::InvalidStoredState)?;
    if chest.expires_at_unix_ms != row.expires {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(chest)
}

pub(super) async fn expire(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    now: i64,
) -> Result<(), DurabilityError> {
    let rows = sqlx::query("SELECT p.death_id::text,p.boss_key,p.minted_at,c.damage_dealt::text,c.damage_received::text,c.healing_others::text FROM game_character_boss_reward_chests p JOIN game_boss_reward_contributors c USING(character_id,world_id,death_id) WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid AND p.item_count>0 AND p.expires_at<=$3 ORDER BY p.minted_at,p.death_id LIMIT 64 FOR UPDATE OF p")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(now).fetch_all(&mut **tx).await?;
    for row in rows {
        let death = uuid_text(row.try_get("death_id")?)?;
        let parse = |key| -> Result<u64, DurabilityError> {
            row.try_get::<String, _>(key)?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)
        };
        let mut group = PersonalRewardChest::<()>::reopen(
            row.try_get("boss_key")?,
            death,
            u64::try_from(row.try_get::<i64, _>("minted_at")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            FrozenBossContribution {
                character: *fence.character_id.as_bytes(),
                contribution: RewardParticipation {
                    damage_dealt: parse("damage_dealt")?,
                    damage_received: parse("damage_received")?,
                    healing_others: parse("healing_others")?,
                },
            },
            vec![],
        )
        .ok_or(DurabilityError::InvalidStoredState)?;
        if group.expire(u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?) {
            sqlx::query("UPDATE game_boss_reward_items SET expired=true WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND death_id=encode($3,'hex')::uuid AND NOT expired AND collected_transaction_id IS NULL")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(death.as_slice()).execute(&mut **tx).await?;
            refresh_group(tx, fence, death).await?;
        }
    }
    Ok(())
}

pub(super) async fn refresh_group(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    death: [u8; 16],
) -> Result<(), DurabilityError> {
    sqlx::query("UPDATE game_character_boss_reward_chests p SET item_count=(SELECT count(*) FROM game_boss_reward_items i WHERE (i.character_id,i.world_id,i.death_id)=(p.character_id,p.world_id,p.death_id) AND NOT i.expired AND i.collected_transaction_id IS NULL) WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid AND p.death_id=encode($3,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(death.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn remaining(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    selected: BossRewardSelection,
    now: i64,
) -> Result<u32, DurabilityError> {
    match selected {
        BossRewardSelection::Item { death, ordinal } => {
            let present: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_boss_reward_items i JOIN game_character_boss_reward_chests p USING(character_id,world_id,death_id) WHERE i.character_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.death_id=encode($3,'hex')::uuid AND i.item_ordinal=$4 AND p.expires_at>$5 AND NOT i.expired AND i.collected_transaction_id IS NULL)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(death.as_slice()).bind(i32::from(ordinal)).bind(now).fetch_one(&mut **tx).await?;
            Ok(u32::from(present))
        }
        BossRewardSelection::Group { death } => {
            let count: Option<i32> = sqlx::query_scalar("SELECT item_count FROM game_character_boss_reward_chests WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND death_id=encode($3,'hex')::uuid AND expires_at>$4")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(death.as_slice()).bind(now).fetch_optional(&mut **tx).await?;
            u32::try_from(count.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)
        }
        BossRewardSelection::All => {
            // The existing projection is maintained with each entitlement mutation. Count
            // its groups in bounded keyset pages, inside the caller's semantic deadline.
            // expires_at=minted_at+7d preserves oldest-first order and matches the live index.
            let mut cursor = (now, [0_u8; 16]);
            let mut total = 0_u32;
            loop {
                let rows: Vec<(i64, Vec<u8>, i32)> = sqlx::query_as("SELECT expires_at,uuid_send(death_id),item_count FROM game_character_boss_reward_chests WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND expires_at>$3 AND item_count>0 AND (expires_at,death_id)>($4,encode($5,'hex')::uuid) ORDER BY expires_at,death_id LIMIT $6")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .bind(now).bind(cursor.0).bind(cursor.1.as_slice()).bind(BOSS_REWARD_PAGE_SIZE as i64)
                    .fetch_all(&mut **tx).await?;
                for (expires, death, count) in &rows {
                    let count =
                        u32::try_from(*count).map_err(|_| DurabilityError::InvalidStoredState)?;
                    total = total
                        .checked_add(count)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    cursor = (
                        *expires,
                        death
                            .as_slice()
                            .try_into()
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                    );
                }
                if rows.len() < BOSS_REWARD_PAGE_SIZE {
                    return Ok(total);
                }
            }
        }
    }
}

pub(super) async fn record_receipt(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: &BossRewardCollectRequest,
    result: BossRewardCollection,
    content: [u8; 32],
    now: i64,
) -> Result<(), DurabilityError> {
    let (kind, scope) = match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => (1_i16, *channel_id.as_bytes()),
        RuntimeScopeRefV1::Instance { instance_id, .. } => (2, instance_id),
    };
    sqlx::query("INSERT INTO game_boss_reward_collection_receipts(game_session_id,command_id,character_id,world_id,intent,status,moved,remaining,source_placement,scope_kind,scope_id,scope_generation,lease_generation,connection_generation,content_digest,occurred_at) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10,encode($11,'hex')::uuid,$12::text::numeric,$13::text::numeric,$14::text::numeric,$15,$16)")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&request.intent).bind(status_code(result.status)).bind(i32::from(result.moved)).bind(i64::from(result.remaining))
        .bind(&request.source_placement).bind(kind).bind(scope.as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string()).bind(fence.connection_generation.get().to_string()).bind(content.as_slice()).bind(now).execute(&mut **tx).await?;
    Ok(())
}
