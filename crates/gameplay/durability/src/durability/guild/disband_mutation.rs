use super::*;
impl DurabilityRoot {
    /// GUILD0 §3.2 leader-confirmed start. No Premium/Bank/Hall authority is inferred.
    pub async fn commit_guild_disband(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        disband: GuildDisbandRequest,
    ) -> Result<GuildOutcome> {
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-disband-proposal:v1");
        digest.update(binding(&fence, &disband.common)?);
        digest.update(disband.confirmation.as_bytes());
        let binding = digest.finalize().to_vec();
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        lock_fenced_guild_operation(&mut tx, &recovery, &disband.common).await?;
                        let edit = MemberEdit::Disband(&disband);
                        execute(
                            &mut tx,
                            &node,
                            &fence,
                            &disband.common,
                            &binding,
                            None,
                            Some(&edit),
                        )
                        .await
                    }
                    .await;
                    match result {
                        Ok(outcome) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

impl LockedGuild<'_> {
    pub(super) async fn disband(
        self,
        tx: &mut Transaction<'_, Postgres>,
        disband: &GuildDisbandRequest,
    ) -> Result<GuildOutcome> {
        let Self {
            fence,
            request,
            binding,
            previous,
            actor,
            acting,
            guild,
            ..
        } = self;
        if acting.as_ref().is_none_or(|member| member.rank != 1)
            || disband.confirmation != guild.try_get::<String, _>("name")?
        {
            return store(
                tx,
                fence,
                request,
                binding,
                GuildResult::NotAllowed,
                (Some(previous), None),
                None,
            )
            .await;
        }
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = crate::durability::guild_audit::GuildDisbandAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *actor.as_bytes(),
                action: 13,
                previous_revision: previous,
                revision: next,
                rank_before: None,
                rank_after: None,
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            confirmation: disband.confirmation.clone(),
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
                text: None,
                rank: None,
                resign: None,
                edit_ranks: None,
                disband: Some(disband),
            },
        )
        .await?;
        sqlx::query("INSERT INTO game_guild_job_checkpoints(guild_id,step,world_id,reason,previous_revision,revision,occurred_at) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,3,$3::text::numeric,$4::text::numeric,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)")
            .bind(request.guild_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(previous.to_string()).bind(next.to_string()).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_guilds SET state=3,revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        Ok(outcome)
    }
}
