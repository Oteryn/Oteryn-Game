use super::*;

impl LockedGuild<'_> {
    pub(super) async fn text(
        self,
        tx: &mut Transaction<'_, Postgres>,
        text: &GuildTextRequest,
    ) -> Result<GuildOutcome> {
        let Self {
            fence,
            request,
            binding,
            previous,
            actor,
            target,
            acting,
            subject,
            ..
        } = self;
        let allowed = match request.operation {
            GuildCommand::SetTitle(_) => {
                acting.as_ref().is_some_and(|m| m.rank == 1) && subject.is_some()
            }
            GuildCommand::SetMessage => acting.as_ref().is_some_and(|m| m.rank <= 2),
            _ => return Err(GuildError::InvalidInput),
        };
        let refusal = if !allowed {
            Some(GuildResult::NotAllowed)
        } else if text.expected_revision != previous {
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
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildTextAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *target.as_bytes(),
                action: request.operation.code(),
                previous_revision: previous,
                revision: next,
                rank_before: None,
                rank_after: None,
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            expected_revision: text.expected_revision,
            text: text.text.clone(),
        }
        .checked_bytes()
        .ok_or_else(invalid)?;
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
                text: Some(text),
                rank: None,
                resign: None,
                edit_ranks: None,
                disband: None,
            },
        )
        .await?;
        if matches!(request.operation, GuildCommand::SetTitle(_)) {
            let changed = sqlx::query("UPDATE game_guild_members SET title=$3 WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
                .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).bind(&text.text).execute(&mut **tx).await?.rows_affected();
            if changed != 1 {
                return Err(invalid());
            }
            sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
                .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        } else {
            sqlx::query("UPDATE game_guilds SET motd=$3,revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
                .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).bind(&text.text).execute(&mut **tx).await?;
        }
        Ok(outcome)
    }
}
