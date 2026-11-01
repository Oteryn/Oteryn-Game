use super::super::character_progression::assert_gameplay_fence;
use super::*;
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn consume<O: SharedTerminalOwner>(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    owner: &O,
    request: &EndgameFailureRequest,
) -> Result<EndgameFailureReceipt> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:endgame-terminal:' || encode($1,'hex'),0))")
        .bind(request.attempt.as_slice()).execute(&mut **tx).await.map_err(DurabilityError::from)?;
    if let Some(receipt) = replay(tx, request).await? {
        return Ok(receipt);
    }
    let proof = owner.prove(tx, request).await?;
    if !proof.matches(tx, request).await? {
        return Err(EndgameTerminalError::AuthorityRejected);
    }
    let mut common = if request.encounter == FinalEncounter::Megalomania {
        0
    } else {
        request.encounter.mask()
    };
    let mut affected = Vec::new();
    let mut expired = Vec::new();
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await
            .map_err(DurabilityError::from)?;
    for member in &request.participants {
        assert_gameplay_fence(tx, member, node).await??;
        let (flags, is_expired) = completed_flags(tx, request.encounter, member, now).await?;
        match request.encounter {
            FinalEncounter::Megalomania => {
                // ANSWER6547994219: expiry of one admitted participant cannot
                // leave another participant with five taints after the failure.
                if flags == request.encounter.mask() {
                    common = request.encounter.mask();
                    affected.extend_from_slice(member.character_id.as_bytes());
                } else if is_expired {
                    expired.extend_from_slice(member.character_id.as_bytes());
                }
            }
            FinalEncounter::Bakragore => common &= flags,
        }
    }
    let removed_flags = choose_pair(&request.attempt, request.encounter, common)?;
    if request.encounter == FinalEncounter::Bakragore && removed_flags.is_some() {
        affected = request.participant_bytes();
    }
    insert_decision(tx, request, common, removed_flags, &affected, &expired).await?;
    if let Some(removed) = removed_flags {
        match request.encounter {
            FinalEncounter::Megalomania=>super::super::quest_state::soul_war::apply_soul_war_failed_final_in_transaction(
                tx,node,&request.attempt,&request.participants,removed,
            ).await?,
            FinalEncounter::Bakragore=>super::super::quest_state::rotten_blood::apply_rotten_blood_failed_final_in_transaction(
                tx,node,&request.attempt,removed,&request.participants,
            ).await?,
        }
    }
    Ok(EndgameFailureReceipt {
        attempt: request.attempt,
        encounter: request.encounter,
        removed_flags,
        replayed: false,
    })
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    request: &EndgameFailureRequest,
) -> Result<Option<EndgameFailureReceipt>> {
    let row = sqlx::query(
        "SELECT binding,removed_flags FROM game_endgame_terminal_outcomes WHERE attempt=$1",
    )
    .bind(request.attempt.as_slice())
    .fetch_optional(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    let Some(row) = row else { return Ok(None) };
    let binding: Vec<u8> = row.try_get("binding").map_err(DurabilityError::from)?;
    if binding.as_slice() != request.binding()?.as_slice() {
        return Err(EndgameTerminalError::ConflictingOccurrence);
    }
    let removed: Option<i16> = row
        .try_get("removed_flags")
        .map_err(DurabilityError::from)?;
    let removed_flags = removed
        .map(u8::try_from)
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if removed_flags
        .is_some_and(|flags| flags.count_ones() != 2 || flags & !request.encounter.mask() != 0)
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(Some(EndgameFailureReceipt {
        attempt: request.attempt,
        encounter: request.encounter,
        removed_flags,
        replayed: true,
    }))
}

async fn completed_flags(
    tx: &mut Transaction<'_, Postgres>,
    encounter: FinalEncounter,
    member: &CurrentCharacterGameplayFence,
    now: i64,
) -> Result<(u8, bool)> {
    let character = member.character_id.as_bytes().as_slice();
    match encounter {
        FinalEncounter::Megalomania => {
            let row=sqlx::query("SELECT teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed FROM game_character_soul_war WHERE character_id=encode($1,'hex')::uuid")
                .bind(character).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
            let Some(row) = row else {
                return Ok((0, false));
            };
            let state = oteryn_gameplay_quests::quest::soul_war::SoulWarState {
                teleport_access: row
                    .try_get("teleport_access")
                    .map_err(DurabilityError::from)?,
                defeated_bosses: u8::try_from(
                    row.try_get::<i16, _>("defeated_bosses")
                        .map_err(DurabilityError::from)?,
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                first_taint_at: row
                    .try_get("first_taint_at")
                    .map_err(DurabilityError::from)?,
                megalomania_completed: row
                    .try_get("megalomania_completed")
                    .map_err(DurabilityError::from)?,
                final_reward_claimed: row
                    .try_get("final_reward_claimed")
                    .map_err(DurabilityError::from)?,
            };
            state
                .validate()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let effective = state.expired(now);
            Ok((
                effective.defeated_bosses,
                state.first_taint_at.is_some() && effective.first_taint_at.is_none(),
            ))
        }
        FinalEncounter::Bakragore => {
            let flags:Option<i16>=sqlx::query_scalar("SELECT child_flags FROM game_character_rotten_blood WHERE character_id=encode($1,'hex')::uuid")
                .bind(character).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
            u8::try_from(flags.unwrap_or(0))
                .map(|flags| (flags, false))
                .map_err(|_| DurabilityError::InvalidStoredState.into())
        }
    }
}

async fn insert_decision(
    tx: &mut Transaction<'_, Postgres>,
    request: &EndgameFailureRequest,
    common: u8,
    removed: Option<u8>,
    affected: &[u8],
    expired: &[u8],
) -> Result<()> {
    let (scope, generation) = request.scope()?;
    let (kind, id) = scope_parts(scope);
    sqlx::query("INSERT INTO game_endgame_terminal_outcomes(attempt,binding,encounter_kind,world_id,scope_kind,scope_id,scope_ownership_generation,participants,common_completed_flags,removed_flags,affected_participants,expired_participants) VALUES($1,$2,$3,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid,$7::text::numeric,$8,$9,$10,$11,$12)")
        .bind(request.attempt.as_slice()).bind(request.binding()?.as_slice()).bind(request.encounter.code())
        .bind(scope.world_id().as_bytes().as_slice()).bind(kind).bind(id.as_slice()).bind(generation.to_string())
        .bind(request.participant_bytes()).bind(i16::from(common)).bind(removed.map(i16::from)).bind(affected).bind(expired)
        .execute(&mut **tx).await.map_err(DurabilityError::from)?;
    Ok(())
}
