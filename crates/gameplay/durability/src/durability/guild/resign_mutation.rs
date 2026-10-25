use super::*;

impl LockedGuild<'_> {
    pub(super) async fn resign(
        self,
        tx: &mut Transaction<'_, Postgres>,
        edit: &ResignEdit<'_, '_>,
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
            account,
            ..
        } = self;
        let target_root=sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(target.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let target_account = match target_root {
            Some(row)
                if row.try_get::<i16, _>("lifecycle")? == 1
                    && uuid_text(&row.try_get::<String, _>("world_id")?)?
                        == *fence.runtime_scope.world_id().as_bytes() =>
            {
                Some(uuid_text(&row.try_get::<String, _>("account_id")?)?)
            }
            _ => None,
        };
        let current_accounts = target_account
            .map(|target| Ok::<_, GuildError>([uuid_text(&account)?, target]))
            .transpose()?;
        if current_accounts != edit.accounts {
            return Err(GuildError::NotCurrent);
        }
        let allowed = actor != target
            && target_account.is_some()
            && acting.as_ref().is_some_and(|a| a.rank == 1)
            && subject.as_ref().is_some_and(|t| t.rank == 2);
        let refusal = if !allowed {
            Some(GuildResult::NotAllowed)
        } else if edit.request.expected_revision != previous {
            Some(GuildResult::StaleRevision)
        } else if crate::durability::bank::fee_payer_is_junior(target) {
            Some(GuildResult::Junior)
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
        let accounts = current_accounts.ok_or_else(invalid)?;
        if accounts[0] == accounts[1] {
            return Err(invalid());
        }
        let mut ordered = accounts;
        ordered.sort();
        // Both positions already exist. Lock/validate them in AccountId order and retain every byte.
        for account_id in ordered {
            let guard:Option<i32>=sqlx::query_scalar("SELECT 1 FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(account_id.as_slice()).fetch_optional(&mut **tx).await?;
            if guard.is_none() {
                return Err(GuildError::NotCurrent);
            }
        }
        let positions=sqlx::query("SELECT account_id::text,guild_id::text,character_id::text FROM game_account_guild_leadership WHERE account_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY account_id FOR UPDATE")
            .bind(accounts[0].as_slice()).bind(accounts[1].as_slice()).fetch_all(&mut **tx).await?;
        if positions.len() != 2 {
            return Err(invalid());
        }
        for row in &positions {
            let account_id = uuid_text(&row.try_get::<String, _>("account_id")?)?;
            let expected = if account_id == accounts[0] {
                actor
            } else if account_id == accounts[1] {
                target
            } else {
                return Err(invalid());
            };
            if uuid_text(&row.try_get::<String, _>("guild_id")?)? != *request.guild_id.as_bytes()
                || uuid_text(&row.try_get::<String, _>("character_id")?)? != *expected.as_bytes()
            {
                return Err(invalid());
            }
        }
        if !edit.premium.is_some_and(|s| {
            s.len() == 2
                && s.iter()
                    .all(|s| matches!(s, PremiumStatus::Current | PremiumStatus::NotActivated(_)))
        }) {
            return store(
                tx,
                fence,
                request,
                binding,
                GuildResult::NotPremium,
                (Some(previous), None),
                None,
            )
            .await;
        }
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildResignAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *target.as_bytes(),
                action: 10,
                previous_revision: previous,
                revision: next,
                rank_before: Some(2),
                rank_after: Some(1),
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            expected_revision: edit.request.expected_revision,
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
                resign: Some(edit.request),
                edit_ranks: None,
                disband: None,
            },
        )
        .await?;
        // Preserve the immediate unique leader index: never create two leaders, even transiently.
        for (character, before, after) in [(actor, 1_i16, 2_i16), (target, 2, 1)] {
            let changed=sqlx::query("UPDATE game_guild_members SET rank_level=$4 WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND rank_level=$3")
                .bind(request.guild_id.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(before).bind(after).execute(&mut **tx).await?;
            if changed.rows_affected() != 1 {
                return Err(invalid());
            }
        }
        // Vice count is unchanged; no lifecycle, deadline or deficit timestamp can be cured here.
        sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        Ok(outcome)
    }
}
