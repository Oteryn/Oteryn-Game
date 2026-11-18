//! Crystal's 30-fragment geode closes for 30 minutes after opening an eight-second portal.
use super::{DurabilityError, SpellLanePermit};
use crate::durability::{
    DurabilityRoot,
    db::{begin_semantic_transaction, commit_semantic_transaction},
};
use crate::foundation::{ChannelId, CommandRef, WorldId};
use sqlx::{Postgres, Transaction};

pub(super) async fn contribute(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    channel: ChannelId,
    epoch: u64,
    geode: &str,
    command: CommandRef,
) -> Result<bool, DurabilityError> {
    let changed = sqlx::query(
        "INSERT INTO game_warzone_geodes AS g (world_id,channel_id,geode_key,reset_epoch,fragments,last_session,last_command)
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::text::numeric,1,encode($5,'hex')::uuid,$6::text::numeric)
         ON CONFLICT (world_id,channel_id,geode_key) DO UPDATE SET
           reset_epoch=EXCLUDED.reset_epoch,
           fragments=CASE WHEN g.reset_epoch=EXCLUDED.reset_epoch AND g.fragments<30 THEN g.fragments+1 ELSE 1 END,
           opened_at=CASE WHEN g.reset_epoch=EXCLUDED.reset_epoch AND g.fragments=29 THEN statement_timestamp() END,
           last_session=EXCLUDED.last_session, last_command=EXCLUDED.last_command,
           contribution_xact_id=EXCLUDED.contribution_xact_id
         WHERE g.reset_epoch<EXCLUDED.reset_epoch OR (g.reset_epoch=EXCLUDED.reset_epoch AND
           (g.fragments<30 OR g.opened_at+interval '30 minutes'<=statement_timestamp()))",
    )
    .bind(world.as_bytes().as_slice())
    .bind(channel.as_bytes().as_slice())
    .bind(geode)
    .bind(epoch.to_string())
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .execute(&mut **tx)
    .await?;
    Ok(changed.rows_affected() == 1)
}

impl DurabilityRoot {
    /// Remaining time for the exact committed opening command in the current Channel.
    pub async fn warzone_geode_opening(
        &self,
        permit: &SpellLanePermit,
        world: WorldId,
        channel: ChannelId,
        epoch: u64,
        geode: String,
        command: CommandRef,
    ) -> Result<Option<u64>, DurabilityError> {
        permit.check_channel(world, channel)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let remaining = sqlx::query_scalar::<_, i64>(
                        "SELECT floor(extract(epoch FROM (opened_at+interval '8 seconds'-statement_timestamp())) * 1000000)::bigint
                         FROM game_warzone_geodes
                         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
                           AND geode_key=$3 AND reset_epoch=$4::text::numeric
                           AND last_session=encode($5,'hex')::uuid AND last_command=$6::text::numeric
                           AND opened_at+interval '8 seconds'>statement_timestamp()",
                    )
                    .bind(world.as_bytes().as_slice())
                    .bind(channel.as_bytes().as_slice())
                    .bind(geode)
                    .bind(epoch.to_string())
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(remaining.and_then(|remaining| u64::try_from(remaining).ok()))
                })
            })
            .await
    }
}
