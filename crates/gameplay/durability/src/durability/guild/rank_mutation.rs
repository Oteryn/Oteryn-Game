use super::*;

impl LockedGuild<'_> {
    pub(super) async fn rank(
        self,
        tx: &mut Transaction<'_, Postgres>,
        edit: &RankEdit<'_, '_>,
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
            state,
            guild,
            ..
        } = self;
        let desired = edit.request.level.level();
        let target_root = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
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
        if target_account != edit.account {
            return Err(GuildError::NotCurrent);
        }
        let rank_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid AND level=$2)")
            .bind(request.guild_id.as_bytes().as_slice()).bind(i16::from(desired)).fetch_one(&mut **tx).await?;
        let allowed = match (&acting, &subject) {
            (Some(a), Some(t)) if target_account.is_some() && rank_exists => {
                GuildRank::new(a.rank).map_err(|_| invalid())?.may_set_rank(
                    GuildRank::new(t.rank).map_err(|_| invalid())?,
                    edit.request.level,
                )
            }
            _ => false,
        };
        let refusal = if !allowed {
            Some(GuildResult::NotAllowed)
        } else if edit.request.expected_revision != previous {
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
        let before = subject.as_ref().ok_or_else(invalid)?.rank;
        let account = target_account.ok_or_else(invalid)?;
        let guard: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(account.as_slice()).fetch_optional(&mut **tx).await?;
        if guard.is_none() {
            return Err(GuildError::NotCurrent);
        }
        let position = sqlx::query("SELECT guild_id::text,character_id::text FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(account.as_slice()).fetch_optional(&mut **tx).await?;
        if before == 2 {
            let Some(row) = &position else {
                return Err(invalid());
            };
            if uuid_text(&row.try_get::<String, _>("guild_id")?)? != *request.guild_id.as_bytes()
                || uuid_text(&row.try_get::<String, _>("character_id")?)? != *target.as_bytes()
            {
                return Err(invalid());
            }
        }
        let promoting = before != 2 && desired == 2;
        let refusal = if promoting && position.is_some() {
            Some(GuildResult::AccountHasPosition)
        } else if promoting
            && !matches!(
                edit.premium,
                Some(PremiumStatus::Current | PremiumStatus::NotActivated(_))
            )
        {
            Some(GuildResult::NotPremium)
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
        let vices: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND rank_level=2")
            .bind(request.guild_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        let after_vices = vices - i64::from(before == 2) + i64::from(desired == 2);
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
        )
        .fetch_one(&mut **tx)
        .await?;
        if state
            .ensure_vice_count_change_allowed(
                usize::try_from(after_vices).map_err(|_| invalid())?,
                guild.try_get("formation_deadline")?,
                guild.try_get("vice_deficit_since")?,
                now,
            )
            .is_err()
        {
            return store(
                tx,
                fence,
                request,
                binding,
                GuildResult::GuildDeadlinePassed,
                (Some(previous), None),
                None,
            )
            .await;
        }
        // GUILD0 §3.3 counts actual vice memberships, including retained ranks after Premium lapse.
        // Premium leadership counts belong only to the daily job; never write its deficit here.
        let activates = state == GuildState::Forming && after_vices >= 4;
        let next_state = if activates {
            2_i16
        } else {
            guild.try_get::<i16, _>("state")?
        };
        let old_deficit: Option<i64> = guild.try_get("vice_deficit_since")?;
        let deficit = if next_state == 2 {
            if after_vices >= 4 {
                None
            } else {
                old_deficit.or(Some(now))
            }
        } else {
            old_deficit
        };
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildRankAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *target.as_bytes(),
                action: 9,
                previous_revision: previous,
                revision: next,
                rank_before: Some(before),
                rank_after: Some(desired),
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
                rank: Some((edit.request, before)),
                resign: None,
                edit_ranks: None,
                disband: None,
            },
        )
        .await?;
        sqlx::query("UPDATE game_guild_members SET rank_level=$3 WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).bind(i16::from(desired)).execute(&mut **tx).await?;
        if promoting {
            sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                .bind(account.as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
        } else if before == 2 && desired != 2 {
            let changed = sqlx::query("DELETE FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid AND guild_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
                .bind(account.as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
            if changed.rows_affected() != 1 {
                return Err(invalid());
            }
        }
        sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric,state=$3,vice_deficit_since=$4 WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).bind(next_state).bind(deficit).execute(&mut **tx).await?;
        Ok(outcome)
    }
}
