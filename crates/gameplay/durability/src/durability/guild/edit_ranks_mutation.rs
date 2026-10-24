use super::*;

impl LockedGuild<'_> {
    pub(super) async fn edit_ranks(
        self,
        tx: &mut Transaction<'_, Postgres>,
        edit: &GuildEditRanksRequest,
    ) -> Result<GuildOutcome> {
        let Self {
            fence,
            request,
            binding,
            previous,
            actor,
            acting,
            ..
        } = self;
        // The one sorted member lock query above locks the complete roster for this action.
        // Do not lock the actor first and then acquire smaller CharacterIds out of order.
        let refusal = if acting.as_ref().is_none_or(|m| m.rank != 1) {
            Some(GuildResult::NotAllowed)
        } else if edit.expected_revision != previous {
            Some(GuildResult::StaleRevision)
        } else {
            None
        };
        if let Some(result) = refusal {
            return store(
                tx,
                fence,
                request,
                binding,
                result,
                (Some(previous), None),
                None,
            )
            .await;
        }
        let unique: i64 = sqlx::query_scalar("SELECT count(DISTINCT game_character_name_key(name)) FROM unnest($1::text[]) AS proposed(name)")
            .bind(&edit.names).fetch_one(&mut **tx).await?;
        if usize::try_from(unique).map_err(|_| invalid())? != edit.names.len() {
            return store(
                tx,
                fence,
                request,
                binding,
                GuildResult::NameTaken,
                (Some(previous), None),
                None,
            )
            .await;
        }
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildEditRanksAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *actor.as_bytes(),
                action: 11,
                previous_revision: previous,
                revision: next,
                rank_before: None,
                rank_after: None,
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            expected_revision: edit.expected_revision,
            names: edit.names.clone(),
        }
        .checked_bytes()
        .ok_or(GuildError::InvalidInput)?;
        let outcome = store_operation(
            tx,
            fence,
            request,
            binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(previous), Some(next)),
                audit: Some(&audit),
                found_name: None,
                text: None,
                rank: None,
                resign: None,
                edit_ranks: Some(edit),
                disband: None,
            },
        )
        .await?;
        // Only this action defers the named rank-name constraint; old actions keep immediate checks.
        sqlx::query("SET CONSTRAINTS game_guild_rank_name_unique DEFERRED")
            .execute(&mut **tx)
            .await?;
        for (index, name) in edit.names.iter().enumerate() {
            let level = i16::try_from(index + 1).map_err(|_| invalid())?;
            sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT(guild_id,level) DO UPDATE SET name=EXCLUDED.name")
                .bind(request.guild_id.as_bytes().as_slice()).bind(level).bind(name).execute(&mut **tx).await?;
        }
        let count = i16::try_from(edit.names.len()).map_err(|_| invalid())?;
        // The immediate membership FK remains enabled: move all removed-tail members first.
        sqlx::query("UPDATE game_guild_members SET rank_level=$2 WHERE guild_id=encode($1,'hex')::uuid AND rank_level>$2")
            .bind(request.guild_id.as_bytes().as_slice()).bind(count).execute(&mut **tx).await?;
        sqlx::query(
            "DELETE FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid AND level>$2",
        )
        .bind(request.guild_id.as_bytes().as_slice())
        .bind(count)
        .execute(&mut **tx)
        .await?;
        sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        // Rank definitions and lower-member moves cannot alter positions, vice count or deadlines.
        Ok(outcome)
    }
}
