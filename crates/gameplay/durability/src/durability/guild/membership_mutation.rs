use super::*;
impl LockedGuild<'_> {
    pub(super) async fn membership(
        self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<GuildOutcome> {
        let Self {
            fence,
            request,
            binding,
            guild,
            previous,
            actor,
            target,
            acting,
            subject,
            state,
            already_member,
            target_exists,
            ..
        } = self;
        let can_invite = member_in_guild(&acting, &request.guild_id).is_some_and(|m| m.rank <= 2);
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
        )
        .fetch_one(&mut **tx)
        .await?;
        let invitations = if matches!(
            request.operation,
            GuildCommand::Invite(_) | GuildCommand::Accept
        ) {
            sqlx::query("SELECT guild_id::text,expires_at FROM game_guild_invitations WHERE character_id=encode($1,'hex')::uuid ORDER BY guild_id LIMIT 51 FOR UPDATE")
            .bind(target.as_bytes().as_slice()).fetch_all(&mut **tx).await?
        } else if matches!(
            request.operation,
            GuildCommand::Revoke(_) | GuildCommand::Decline
        ) {
            sqlx::query("SELECT guild_id::text,expires_at FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_all(&mut **tx).await?
        } else {
            Vec::new()
        };
        if invitations.len() > TARGET_INVITATIONS_MAX {
            return Err(invalid());
        }
        let mut expired = Vec::new();
        let mut own_invitation = None;
        for row in &invitations {
            let id = uuid_text(&row.try_get::<String, _>("guild_id")?)?;
            let expiry: i64 = row.try_get("expires_at")?;
            if expiry <= now {
                expired.push(id);
            }
            if id == *request.guild_id.as_bytes() {
                own_invitation = Some(expiry);
            }
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(request.guild_id.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        let vices: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND rank_level=2")
        .bind(request.guild_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        let mut rank_before = None;
        let mut rank_after = None;
        let mut removes = Vec::new();
        let result = match request.operation {
            GuildCommand::Invite(_) => {
                if !can_invite
                    || !target_exists
                    || member_in_guild(&subject, &request.guild_id).is_some()
                    || own_invitation.is_some_and(|e| e > now)
                {
                    GuildResult::NotAllowed
                } else {
                    let guild_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid")
                    .bind(request.guild_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                    if guild_count - i64::from(own_invitation.is_some())
                        >= GUILD_INVITATIONS_MAX as i64
                        || invitations.len() - expired.len() >= TARGET_INVITATIONS_MAX
                    {
                        GuildResult::InvitationLimit
                    } else {
                        removes = expired;
                        GuildResult::Ok
                    }
                }
            }
            GuildCommand::Revoke(_) => {
                if !can_invite {
                    GuildResult::NotAllowed
                } else if own_invitation.is_none() {
                    GuildResult::NotInvited
                } else {
                    removes.push(*request.guild_id.as_bytes());
                    GuildResult::Ok
                }
            }
            GuildCommand::Decline => {
                // Accepted decline requires an existing row; it does not require unexpired time,
                // membership/leader rights, or a mutation of the actor's other invitations.
                if own_invitation.is_none() {
                    GuildResult::NotInvited
                } else {
                    removes.push(*request.guild_id.as_bytes());
                    GuildResult::Ok
                }
            }
            GuildCommand::Accept => {
                if already_member {
                    GuildResult::AlreadyInGuild
                } else if own_invitation.is_none_or(|e| e <= now) {
                    GuildResult::NotInvited
                } else if count >= MEMBERS_MAX as i64 {
                    GuildResult::GuildFull
                } else {
                    let rank: i16 = sqlx::query_scalar(
                    "SELECT max(level) FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid",
                )
                .bind(request.guild_id.as_bytes().as_slice())
                .fetch_one(&mut **tx)
                .await?;
                    let rank = u8::try_from(rank).map_err(|_| invalid())?;
                    if !(3..=20).contains(&rank) {
                        return Err(invalid());
                    }
                    rank_after = Some(rank);
                    for row in &invitations {
                        removes.push(uuid_text(&row.try_get::<String, _>("guild_id")?)?);
                    }
                    GuildResult::Ok
                }
            }
            GuildCommand::Found
            | GuildCommand::SetTitle(_)
            | GuildCommand::SetMessage
            | GuildCommand::SetRank(_)
            | GuildCommand::Resign(_)
            | GuildCommand::EditRanks
            | GuildCommand::Disband => {
                return Err(GuildError::InvalidInput);
            }
            GuildCommand::Leave => match member_in_guild(&acting, &request.guild_id) {
                Some(m) if m.rank > 1 => {
                    rank_before = Some(m.rank);
                    GuildResult::Ok
                }
                _ => GuildResult::NotAllowed,
            },
            GuildCommand::Exclude(_) => match (
                member_in_guild(&acting, &request.guild_id),
                member_in_guild(&subject, &request.guild_id),
            ) {
                (Some(a), Some(t))
                    if GuildRank::new(a.rank)
                        .map(|r| r.may_exclude(GuildRank::new(t.rank).unwrap_or(GuildRank::LEADER)))
                        .unwrap_or(false) =>
                {
                    rank_before = Some(t.rank);
                    GuildResult::Ok
                }
                _ => GuildResult::NotAllowed,
            },
        };
        if result != GuildResult::Ok {
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
        let proposed_vices = vices - i64::from(rank_before == Some(2));
        if matches!(
            request.operation,
            GuildCommand::Accept | GuildCommand::Leave | GuildCommand::Exclude(_)
        ) && state
            .ensure_vice_count_change_allowed(
                usize::try_from(proposed_vices).map_err(|_| invalid())?,
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
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildAuditProposal {
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
            rank_before,
            rank_after,
            removed_invitations: u8::try_from(removes.len()).map_err(|_| invalid())?,
            occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
        };
        let audit_bytes = if request.operation == GuildCommand::Decline {
            GuildDeclineAuditProposal { operation: audit }.checked_bytes()
        } else {
            audit.checked_bytes()
        }
        .ok_or_else(invalid)?;
        // Insert the immutable operation before its closed row transitions; deferred guards prove
        // both postconditions and one exact event when the transaction actually commits.
        let outcome = store(
            tx,
            fence,
            request,
            binding,
            GuildResult::Ok,
            (Some(previous), Some(next)),
            Some(&audit_bytes),
        )
        .await?;
        // Accept's INSERT guard observes the still-live invitation, then all invitations disappear.
        // Invite removes its expired target rows before inserting the new invitation.
        if request.operation == GuildCommand::Decline {
            let deleted = sqlx::query("DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(actor.as_bytes().as_slice()).execute(&mut **tx).await?.rows_affected();
            if deleted != 1 {
                return Err(invalid());
            }
        } else if request.operation != GuildCommand::Accept {
            remove_invitations(tx, target, &removes).await?;
        }
        match request.operation {
            GuildCommand::Invite(_) => {
                sqlx::query("INSERT INTO game_guild_invitations(guild_id,character_id,inviter,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
                .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).bind(actor.as_bytes().as_slice())
                .bind(now.checked_add(INVITATION_LIFETIME_MS).ok_or_else(invalid)?).execute(&mut **tx).await?;
            }
            GuildCommand::Accept => {
                sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'',$4)")
                .bind(actor.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(i16::from(rank_after.ok_or_else(invalid)?)).bind(now).execute(&mut **tx).await?;
            }
            GuildCommand::Leave | GuildCommand::Exclude(_) => {
                sqlx::query("DELETE FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid AND guild_id=encode($2,'hex')::uuid")
                .bind(target.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).execute(&mut **tx).await?;
                if rank_before == Some(2) {
                    let target_account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(target.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                    let moved=sqlx::query("DELETE FROM game_account_guild_leadership WHERE account_id=$1::uuid AND guild_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
                    .bind(target_account).bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
                    if moved.rows_affected() != 1 {
                        return Err(invalid());
                    }
                }
            }
            GuildCommand::Revoke(_) | GuildCommand::Decline => {}
            GuildCommand::Found
            | GuildCommand::SetTitle(_)
            | GuildCommand::SetMessage
            | GuildCommand::SetRank(_)
            | GuildCommand::Resign(_)
            | GuildCommand::EditRanks
            | GuildCommand::Disband => {
                return Err(GuildError::InvalidInput);
            }
        }
        if request.operation == GuildCommand::Accept {
            remove_invitations(tx, target, &removes).await?;
        }
        if request.operation == GuildCommand::Decline {
            // Declining changes no vice/lifecycle/Premium facts, including an existing first deficit.
            let changed = sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?.rows_affected();
            if changed != 1 {
                return Err(invalid());
            }
            return Ok(outcome);
        }
        let deficit = if state == GuildState::Active && proposed_vices < 4 {
            Some(
                guild
                    .try_get::<Option<i64>, _>("vice_deficit_since")?
                    .unwrap_or(now),
            )
        } else {
            guild.try_get::<Option<i64>, _>("vice_deficit_since")?
        };
        sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric,vice_deficit_since=$3 WHERE guild_id=encode($1,'hex')::uuid")
        .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).bind(deficit).execute(&mut **tx).await?;
        Ok(outcome)
    }
}
