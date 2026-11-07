use super::*;
pub(super) struct LockedGuild<'a> {
    pub(super) fence: &'a CurrentCharacterItemFence,
    pub(super) request: &'a GuildRequest,
    pub(super) binding: &'a [u8],
    pub(super) guild: sqlx::postgres::PgRow,
    pub(super) previous: u64,
    pub(super) actor: CharacterId,
    pub(super) target: CharacterId,
    pub(super) acting: Option<Member>,
    pub(super) subject: Option<Member>,
    pub(super) state: GuildState,
    pub(super) account: String,
    pub(super) already_member: bool,
    pub(super) target_exists: bool,
}
#[derive(Clone)]
pub(super) struct Member {
    pub(super) guild: [u8; 16],
    pub(super) rank: u8,
}

pub(super) fn member_in_guild<'a>(
    member: &'a Option<Member>,
    guild: &GuildId,
) -> Option<&'a Member> {
    member.as_ref().filter(|m| m.guild == *guild.as_bytes())
}

/// The single native transaction prelude: recovery remains before historical receipt or
/// current actor checks. Factoring it also permits negative DATA against the real predicate;
/// no test constructs or weakens the sealed Root capability.
pub async fn execute_fenced_operation(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    text: Option<&GuildTextRequest>,
) -> Result<GuildOutcome> {
    lock_fenced_guild_operation(tx, recovery, request).await?;
    execute(tx, node, fence, request, binding, text, None).await
}

pub async fn lock_fenced_guild_operation(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    request: &GuildRequest,
) -> Result<()> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    lock_operation(tx, request).await
}

pub async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    text: Option<&GuildTextRequest>,
    edit: Option<&MemberEdit<'_, '_>>,
) -> Result<GuildOutcome> {
    if let Some(outcome) = replay(tx, request, binding).await? {
        return Ok(outcome);
    }
    let account = match assert_session_fence(tx, fence, node).await? {
        Ok(account) => account,
        Err(_) => return Err(GuildError::NotCurrent),
    };
    let actor = fence.character_id;
    let target = request.operation.target(actor);
    let mut roots = vec![actor];
    if matches!(request.operation, GuildCommand::Invite(_)) && target != actor {
        roots.push(target);
    }
    roots.sort();
    let mut target_exists = true;
    for character in roots {
        let root = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let valid = root.as_ref().is_some_and(|r| {
            r.try_get::<i16, _>("lifecycle").ok() == Some(1)
                && r.try_get::<String, _>("world_id")
                    .ok()
                    .and_then(|s| uuid_text(&s).ok())
                    == Some(*fence.runtime_scope.world_id().as_bytes())
        });
        if character == actor {
            if !valid
                || root
                    .as_ref()
                    .and_then(|r| r.try_get::<String, _>("account_id").ok())
                    .as_ref()
                    != Some(&account)
            {
                return Err(GuildError::NotCurrent);
            }
        } else {
            target_exists = valid;
        }
    }
    let guild = sqlx::query("SELECT name,world_id::text,state,revision::text,formation_deadline,vice_deficit_since FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.guild_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(guild) = guild else {
        return store(
            tx,
            fence,
            request,
            binding,
            GuildResult::NotAllowed,
            (None, None),
            None,
        )
        .await;
    };
    let previous = numeric_u64(&guild, "revision")?;
    let state = match guild.try_get::<i16, _>("state")? {
        1 => GuildState::Forming,
        2 => GuildState::Active,
        3 => GuildState::Disbanding,
        4 => GuildState::Disbanded,
        _ => return Err(invalid()),
    };
    if uuid_text(&guild.try_get::<String, _>("world_id")?)?
        != *fence.runtime_scope.world_id().as_bytes()
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
    if state.ensure_command_allowed().is_err() {
        return store(
            tx,
            fence,
            request,
            binding,
            GuildResult::GuildDisbanding,
            (Some(previous), None),
            None,
        )
        .await;
    }
    // Other-Guild membership never authorizes this operation. Accept reads that exclusion
    // predicate under the actor root/admission fence; only this locked Guild's rows are locked.
    let already_member = if request.operation == GuildCommand::Accept {
        sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid)")
            .bind(actor.as_bytes().as_slice()).fetch_one(&mut **tx).await?
    } else {
        false
    };
    let member_target = if matches!(request.operation, GuildCommand::Revoke(_)) {
        actor
    } else {
        target
    };
    let members = sqlx::query("SELECT character_id::text,guild_id::text,rank_level FROM game_guild_members WHERE guild_id=encode($3,'hex')::uuid AND ($4 OR character_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid)) ORDER BY character_id FOR UPDATE")
        .bind(actor.as_bytes().as_slice()).bind(member_target.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(request.operation == GuildCommand::EditRanks).fetch_all(&mut **tx).await?;
    let member = |id: CharacterId| -> Result<Option<Member>> {
        for row in &members {
            if uuid_text(&row.try_get::<String, _>("character_id")?)? == *id.as_bytes() {
                let rank =
                    u8::try_from(row.try_get::<i16, _>("rank_level")?).map_err(|_| invalid())?;
                GuildRank::new(rank).map_err(|_| invalid())?;
                return Ok(Some(Member {
                    guild: uuid_text(&row.try_get::<String, _>("guild_id")?)?,
                    rank,
                }));
            }
        }
        Ok(None)
    };
    let acting = member(actor)?;
    let subject = member(target)?;
    let locked = LockedGuild {
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
        account,
        already_member,
        target_exists,
    };
    match edit {
        Some(MemberEdit::Disband(value)) => locked.disband(tx, value).await,
        Some(MemberEdit::EditRanks(value)) => locked.edit_ranks(tx, value).await,
        Some(MemberEdit::Resign(value)) => locked.resign(tx, value).await,
        Some(MemberEdit::Rank(value)) => locked.rank(tx, value).await,
        None => match text {
            Some(value) => locked.text(tx, value).await,
            None => locked.membership(tx).await,
        },
    }
}
