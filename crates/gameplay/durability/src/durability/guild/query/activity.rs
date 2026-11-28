use super::*;
use oteryn_protocol_oteryn::guild::activity::{
    ACTIVITY_PAGE_ENTRIES, ActivityCursor, ActivityEntry, ActivityKind, ActivityPage, ActivityQuery,
};

impl DurabilityRoot {
    pub async fn query_guild_activity(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        query: ActivityQuery,
    ) -> Result<ActivityPage> {
        self.with_guild_query( authority, node, fence, move |tx, member| {
            Box::pin(async move {
                if member.is_none_or(|member| member.guild_id.as_bytes() != &query.guild) {
                    return Err(GuildError::InvalidInput);
                }
                activity(tx, query).await
            })
        }).await
    }
}

async fn activity(tx: &mut Transaction<'_, Postgres>, query: ActivityQuery) -> Result<ActivityPage> {
    if query.before.is_some_and(|before| before.guild != query.guild) {
        return Err(GuildError::InvalidInput);
    }
    let before = query.before.map(|before| i64::try_from(before.occurred_at_ms))
        .transpose().map_err(|_| GuildError::InvalidInput)?;
    let rows = sqlx::query(
        "SELECT e.event_id::text,e.occurred_at,o.action,a.name AS actor,t.name AS target,
                o.previous_rank,o.edit_rank
         FROM game_guild_event_outbox e
         JOIN game_guild_operations o USING(occurrence)
         JOIN game_character_roots a ON a.character_id=o.actor
         JOIN game_character_roots t ON t.character_id=o.target
         WHERE e.guild_id=encode($1,'hex')::uuid AND o.action IN(1,2,3,4,5,6,9,10,12,13)
           AND e.occurred_at >= floor(extract(epoch FROM (transaction_timestamp()-interval '720 hours'))*1000)::bigint
           AND e.occurred_at <= floor(extract(epoch FROM transaction_timestamp())*1000)::bigint
           AND ($2::bigint IS NULL OR (e.occurred_at,e.event_id)<($2,encode($3,'hex')::uuid))
         ORDER BY e.occurred_at DESC,e.event_id DESC LIMIT $4",
    )
    .bind(query.guild.as_slice())
    .bind(before)
    .bind(query.before.map(|before| before.event.to_vec()))
    .bind(ACTIVITY_PAGE_ENTRIES as i64 + 1)
    .fetch_all(&mut **tx)
    .await?;
    let more = rows.len() > ACTIVITY_PAGE_ENTRIES;
    let rank = |value: Option<i16>| value.map(u8::try_from).transpose().map_err(|_| invalid());
    let entries = rows.iter().take(ACTIVITY_PAGE_ENTRIES).map(|row| {
        let action = u64::try_from(row.try_get::<i16, _>("action")?).map_err(|_| invalid())?;
        Ok(ActivityEntry {
            occurred_at_ms: u64::try_from(row.try_get::<i64, _>("occurred_at")?).map_err(|_| invalid())?,
            event: uuid_text(&row.try_get::<String, _>("event_id")?)?,
            kind: ActivityKind::from_code(action).ok_or_else(invalid)?,
            actor: row.try_get("actor")?,
            target: row.try_get("target")?,
            previous_rank: rank(row.try_get("previous_rank")?)?,
            rank: rank(row.try_get("edit_rank")?)?,
        })
    }).collect::<Result<Vec<_>>>()?;
    let next = entries.last().filter(|_| more).map(|entry| ActivityCursor {
        guild: query.guild, occurred_at_ms: entry.occurred_at_ms, event: entry.event,
    });
    Ok(ActivityPage { guild: query.guild, entries, next })
}
