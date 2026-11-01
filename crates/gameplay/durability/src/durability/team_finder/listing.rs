use super::*;

/// Existing commands 390/391 now change the retained template and its active
/// advertisement together. A historical replay never republishes an old ad.
pub async fn apply_leader_listing_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    listing: Option<LeaderListing>,
) -> Result<bool> {
    let (scope, party) = owner(tx, authority).await?;
    let character = authority.character_id_bytes();
    let command = authority.command();
    let mut hash = Sha256::new();
    hash.update(b"oteryn.team-finder.leader.v1");
    hash.update(character);
    match listing {
        Some(value) => {
            hash.update([1]);
            hash.update(encode_upsert(value).map_err(|_| reject("team finder invalid listing"))?);
        }
        None => hash.update([0]),
    }
    let binding: [u8; 32] = hash.finalize().into();
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).fetch_one(&mut **tx).await?;
    let replay = sqlx::query("SELECT accepted,binding,uuid_send(leader_character_id) AS character FROM game_team_finder_leader_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .fetch_optional(&mut **tx).await?;
    if let Some(row) = replay {
        if uuid(row.try_get("character")?)? != character
            || row
                .try_get::<Option<Vec<u8>>, _>("binding")?
                .is_some_and(|old| old != binding)
        {
            return Err(reject("team finder leader command conflict"));
        }
        return Ok(row.try_get("accepted")?);
    }
    if let Some(listing) = listing {
        let (kind, first, area) = match listing.target {
            TeamTarget::Boss(id) => (1_i16, id, None),
            TeamTarget::Hunt { kind, area } => (2, kind, Some(i32::from(area))),
            TeamTarget::Quest(id) => (3, id, None),
        };
        sqlx::query("INSERT INTO game_team_finder_leader_listings(leader_character_id,min_level,max_level,vocation_mask,team_slots,free_slots,party_members_join,client_timestamp,team_type,target_id,hunt_area) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT(leader_character_id) DO UPDATE SET min_level=EXCLUDED.min_level,max_level=EXCLUDED.max_level,vocation_mask=EXCLUDED.vocation_mask,team_slots=EXCLUDED.team_slots,free_slots=EXCLUDED.free_slots,party_members_join=EXCLUDED.party_members_join,client_timestamp=EXCLUDED.client_timestamp,team_type=EXCLUDED.team_type,target_id=EXCLUDED.target_id,hunt_area=EXCLUDED.hunt_area")
            .bind(character.as_slice())
            .bind(i32::from(listing.min_level))
            .bind(i32::from(listing.max_level))
            .bind(i16::from(listing.vocation_mask))
            .bind(i32::from(listing.slots))
            .bind(i32::from(listing.free_slots))
            .bind(listing.party_members_join)
            .bind(i64::from(listing.timestamp))
            .bind(kind)
            .bind(i32::from(first))
            .bind(area)
            .execute(&mut **tx).await?;
        // Normally the terminal/generation trigger has already retired an old
        // occurrence. Also handle an old row during migration/recovery safely.
        sqlx::query("DELETE FROM game_team_finder_active_listings WHERE leader_character_id=encode($1,'hex')::uuid AND (leader_game_session_id<>encode($2,'hex')::uuid OR connection_generation<>$3::text::numeric(20,0) OR character_lease_generation<>$4::text::numeric(20,0) OR scope_ownership_generation<>$5::text::numeric(20,0) OR world_id<>encode($6,'hex')::uuid OR channel_id<>encode($7,'hex')::uuid)")
            .bind(character.as_slice())
            .bind(authority.game_session_id().as_bytes().as_slice())
            .bind(authority.connection_generation().get().to_string())
            .bind(authority.character_lease_generation().to_string())
            .bind(authority.scope_generation().to_string())
            .bind(scope.0.as_slice()).bind(scope.1.as_slice())
            .execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_team_finder_active_listings(leader_character_id,world_id,channel_id,leader_game_session_id,connection_generation,character_lease_generation,scope_ownership_generation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric(20,0),$6::text::numeric(20,0),$7::text::numeric(20,0)) ON CONFLICT(leader_character_id) DO NOTHING")
            .bind(character.as_slice())
            .bind(scope.0.as_slice()).bind(scope.1.as_slice())
            .bind(authority.game_session_id().as_bytes().as_slice())
            .bind(authority.connection_generation().get().to_string())
            .bind(authority.character_lease_generation().to_string())
            .bind(authority.scope_generation().to_string())
            .execute(&mut **tx).await?;
        if listing.party_members_join {
            // Crystal protocolgame.cpp:3019-3029 inserts current party members
            // as accepted without overwriting an existing applicant's status.
            for member in party.members {
                if member.character == character {
                    continue;
                }
                sqlx::query("INSERT INTO game_team_finder_members(leader_character_id,world_id,channel_id,member_character_id,status) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,3) ON CONFLICT(leader_character_id,member_character_id) DO NOTHING")
                    .bind(character.as_slice()).bind(scope.0.as_slice()).bind(scope.1.as_slice()).bind(member.character.as_slice()).execute(&mut **tx).await?;
            }
        }
    } else {
        // Both active membership tables cascade from the existing S14a row.
        sqlx::query("DELETE FROM game_team_finder_leader_listings WHERE leader_character_id=encode($1,'hex')::uuid")
            .bind(character.as_slice()).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_team_finder_leader_receipts(game_session_id,command_id,leader_character_id,accepted,binding) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),encode($3,'hex')::uuid,true,$4)")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .bind(character.as_slice()).bind(binding.as_slice())
        .execute(&mut **tx).await?;
    Ok(true)
}
