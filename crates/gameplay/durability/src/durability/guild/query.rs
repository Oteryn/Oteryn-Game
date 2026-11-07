//! Private, bounded Guild reads under the native recovery/session/root/Guild lock order.
use super::*;
use oteryn_protocol_oteryn::guild::{
    GuildInvitation, GuildMember, GuildQuery, GuildQueryResult, GuildRosterCursor, GuildSummary,
    MAX_GUILD_PAGE_ENTRIES,
};

impl DurabilityRoot {
    /// The actor and World come exclusively from current admission. No Account, position or
    /// Channel is projected. Roster selectors/cursors confer no membership authority.
    pub async fn query_guild(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        query: GuildQuery,
    ) -> Result<GuildQueryResult> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        // Validates/locks the root even when it finds no membership. A membership
                        // read also holds its Guild lock before projecting any private fields.
                        let member =
                            read_current_guild_chat_data(&mut tx, &recovery, &node, &fence, None)
                                .await?;
                        answer(&mut tx, fence, member, query).await
                    }
                    .await;
                    match result {
                        Ok(data) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(data))
                        }
                        Err(GuildError::Unavailable(error)) => {
                            tx.rollback().await?;
                            Err(error)
                        }
                        Err(error) => {
                            tx.rollback().await?;
                            Ok(Err(error))
                        }
                    }
                })
            })
            .await?
    }
}

async fn answer(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    member: Option<GuildChatMembership>,
    query: GuildQuery,
) -> Result<GuildQueryResult> {
    if query == GuildQuery::Invitations {
        return invitations(tx, fence).await;
    }
    let Some(member) = member else {
        return if query == GuildQuery::Own {
            Ok(GuildQueryResult::Own(None))
        } else {
            Err(GuildError::InvalidInput)
        };
    };
    let guild = *member.guild_id.as_bytes();
    let row = sqlx::query(
        "SELECT name,state,revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
    )
    .bind(guild.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let revision = numeric_u64(&row, "revision")?;
    match query {
        GuildQuery::Own => {
            // At most 20 rows (rank level PK and CHECK 1..20). Bound each stored name before
            // allocation too; oversized historic names fail the encoder rather than truncate.
            let ranks = sqlx::query_scalar(
                "SELECT left(name,129) FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid ORDER BY level",
            )
            .bind(guild.as_slice())
            .fetch_all(&mut **tx)
            .await?;
            let title = sqlx::query_scalar(
                "SELECT title FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid",
            )
            .bind(guild.as_slice())
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&mut **tx)
            .await?;
            Ok(GuildQueryResult::Own(Some(GuildSummary {
                guild,
                name: row.try_get("name")?,
                state: u8::try_from(row.try_get::<i16, _>("state")?).map_err(|_| invalid())?,
                revision,
                own_rank: member.rank.level(),
                title,
                message: member.motd,
                ranks,
            })))
        }
        GuildQuery::Roster {
            guild: selected,
            after,
        } => {
            if selected != guild
                || after.is_some_and(|c| c.guild != guild || !(1..=20).contains(&c.level))
            {
                return Err(GuildError::InvalidInput);
            }
            let rows = sqlx::query(
                "SELECT m.character_id::text,r.name,m.rank_level,m.title FROM game_guild_members m JOIN game_character_roots r USING(character_id) WHERE m.guild_id=encode($1,'hex')::uuid AND (m.rank_level,m.character_id)>($2,encode($3,'hex')::uuid) ORDER BY m.rank_level,m.character_id LIMIT 101",
            )
            .bind(guild.as_slice())
            .bind(after.map_or(0_i16, |c| i16::from(c.level)))
            .bind(after.map_or([0; 16], |c| c.character).as_slice())
            .fetch_all(&mut **tx)
            .await?;
            let more = rows.len() > MAX_GUILD_PAGE_ENTRIES;
            let members = rows
                .iter()
                .take(MAX_GUILD_PAGE_ENTRIES)
                .map(|r| {
                    Ok(GuildMember {
                        character: uuid_text(&r.try_get::<String, _>("character_id")?)?,
                        name: r.try_get("name")?,
                        level: u8::try_from(r.try_get::<i16, _>("rank_level")?)
                            .map_err(|_| invalid())?,
                        title: r.try_get("title")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let next = members.last().filter(|_| more).map(|m| GuildRosterCursor {
                guild,
                level: m.level,
                character: m.character,
            });
            Ok(GuildQueryResult::Roster {
                guild,
                revision,
                members,
                next,
            })
        }
        GuildQuery::Invitations => Err(GuildError::InvalidInput),
    }
}

async fn invitations(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
) -> Result<GuildQueryResult> {
    // Only the actor's own live invitations; a single statement snapshot, no other Guild locks.
    let rows = sqlx::query(
        "SELECT g.guild_id::text,g.name,i.expires_at FROM game_guild_invitations i JOIN game_guilds g USING(guild_id) WHERE i.character_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid AND g.state IN(1,2) AND i.expires_at>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint ORDER BY g.guild_id LIMIT 50",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    let entries = rows
        .iter()
        .map(|r| {
            Ok(GuildInvitation {
                guild: uuid_text(&r.try_get::<String, _>("guild_id")?)?,
                name: r.try_get("name")?,
                expires_at_unix_ms: u64::try_from(r.try_get::<i64, _>("expires_at")?)
                    .map_err(|_| invalid())?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(GuildQueryResult::Invitations(entries))
}
