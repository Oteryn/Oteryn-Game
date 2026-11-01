use super::*;

/// Donor browse sends all listings in the server scope, with level/vocation and
/// target metadata. Those fields are client filters, not admission restrictions.
pub async fn browse_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    after: Option<[u8; 16]>,
    limit: i64,
) -> Result<Vec<ListingRow>> {
    let limit = page_limit(limit)?;
    let (scope, _) = owner(tx, authority).await?;
    let rows = sqlx::query("SELECT l.*,r.name,uuid_send(a.leader_character_id) AS character,uuid_send(a.leader_game_session_id) AS session,a.connection_generation::text AS generation,COALESCE(mine.status,0)::smallint AS member_status,(SELECT count(*) FROM game_team_finder_members m WHERE m.leader_character_id=a.leader_character_id AND m.status=3) AS accepted_count FROM game_team_finder_active_listings a JOIN game_team_finder_leader_listings l USING(leader_character_id) JOIN game_character_roots r ON r.character_id=a.leader_character_id JOIN game_durability_reconnect_sessions s ON s.game_session_id=a.leader_game_session_id JOIN game_durability_admission_character_guards cg ON cg.character_id=s.character_id JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id LEFT JOIN game_team_finder_members mine ON mine.leader_character_id=a.leader_character_id AND mine.member_character_id=encode($3,'hex')::uuid WHERE a.world_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid AND ($4::bytea IS NULL OR a.leader_character_id>encode($4,'hex')::uuid) AND s.character_id=a.leader_character_id AND s.world_id=a.world_id AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=a.world_id AND s.runtime_scope_channel_id=a.channel_id AND s.session_state=2 AND s.current_generation=a.connection_generation AND s.character_lease_generation=a.character_lease_generation AND s.scope_ownership_generation=a.scope_ownership_generation AND a.scope_ownership_generation=$6::text::numeric(20,0) AND cg.account_id=r.account_id AND s.account_id=r.account_id AND cg.world_id=r.world_id AND cg.world_id=a.world_id AND cg.eligible AND cg.lease_generation=s.character_lease_generation AND cg.holder_game_session_id=s.game_session_id AND ag.presence_character_id=s.character_id AND ag.holder_game_session_id=s.game_session_id ORDER BY a.leader_character_id LIMIT $5")
        .bind(scope.0.as_slice()).bind(scope.1.as_slice())
        .bind(authority.character_id_bytes().as_slice())
        .bind(after.map(|id| id.to_vec())).bind(limit).bind(scope.2.to_string())
        .fetch_all(&mut **tx).await?;
    rows.into_iter()
        .map(|row| {
            let accepted = u64::try_from(row.try_get::<i64, _>("accepted_count")?)
                .map_err(|_| reject("team finder accepted count"))?;
            // Keep the S14a template's authored free-slot baseline on the wire;
            // occupied_slots carries the current global projection (Q5A).
            let listing = stored_listing(&row)?;
            let occupied_slots = occupied_slots(listing, accepted);
            let leader_target = target(&row)?;
            Ok(ListingRow {
                leader_target,
                name: row.try_get("name")?,
                listing,
                occupied_slots,
                status: status(row.try_get("member_status")?)?,
            })
        })
        .collect()
}

fn applicant(row: PgRow) -> Result<ApplicantRow> {
    let target = target(&row)?;
    Ok(ApplicantRow {
        target,
        name: row.try_get("name")?,
        level: u16::try_from(row.try_get::<i64, _>("level")?)
            .map_err(|_| reject("team finder applicant level"))?,
        vocation: client_vocation(&row.try_get::<String, _>("vocation")?)?,
        status: status(row.try_get("member_status")?)?,
    })
}

/// Leader first on the first page (status 3), then current online applicants in
/// stable Character order; offline member map entries remain until leader logout.
pub async fn applicants_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    after: Option<[u8; 16]>,
    limit: i64,
) -> Result<(Option<LeaderListing>, u16, Vec<ApplicantRow>)> {
    let limit = page_limit(limit)?;
    let (scope, _) = owner(tx, authority).await?;
    let character = authority.character_id_bytes();
    let Some(row) = active_listing(tx, scope, character).await? else {
        return Ok((None, 0, Vec::new()));
    };
    let accepted = u64::try_from(row.try_get::<i64, _>("accepted_count")?)
        .map_err(|_| reject("team finder accepted count"))?;
    let listing = stored_listing(&row)?;
    let occupied = occupied_slots(listing, accepted);
    let mut members = Vec::new();
    if after.is_none() {
        let row = sqlx::query("SELECT uuid_send(r.character_id) AS character,uuid_send(s.game_session_id) AS session,s.current_generation::text AS generation,r.name,p.level,COALESCE(b.vocation,'none') AS vocation,3::smallint AS member_status FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) LEFT JOIN game_character_build_state b USING(character_id) JOIN game_durability_reconnect_sessions s USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid AND s.game_session_id=encode($2,'hex')::uuid AND s.session_state=2")
            .bind(character.as_slice())
            .bind(authority.game_session_id().as_bytes().as_slice())
            .fetch_one(&mut **tx).await?;
        members.push(applicant(row)?);
    }
    let remaining =
        limit - i64::try_from(members.len()).map_err(|_| reject("team finder page size"))?;
    if remaining > 0 {
        let rows = sqlx::query("SELECT uuid_send(r.character_id) AS character,uuid_send(s.game_session_id) AS session,s.current_generation::text AS generation,r.name,p.level,COALESCE(b.vocation,'none') AS vocation,m.status AS member_status FROM game_team_finder_members m JOIN game_character_roots r ON r.character_id=m.member_character_id JOIN game_character_progression_state p ON p.character_id=r.character_id LEFT JOIN game_character_build_state b ON b.character_id=r.character_id JOIN game_durability_reconnect_sessions s ON s.character_id=r.character_id AND s.session_state=2 JOIN game_durability_admission_character_guards cg ON cg.character_id=s.character_id JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id WHERE m.leader_character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=encode($2,'hex')::uuid AND s.runtime_scope_channel_id=encode($3,'hex')::uuid AND ($4::bytea IS NULL OR m.member_character_id>encode($4,'hex')::uuid) AND s.scope_ownership_generation=$6::text::numeric(20,0) AND cg.account_id=r.account_id AND s.account_id=r.account_id AND cg.world_id=r.world_id AND cg.world_id=s.world_id AND cg.eligible AND cg.lease_generation=s.character_lease_generation AND cg.holder_game_session_id=s.game_session_id AND ag.presence_character_id=s.character_id AND ag.holder_game_session_id=s.game_session_id ORDER BY m.member_character_id LIMIT $5")
            .bind(character.as_slice()).bind(scope.0.as_slice()).bind(scope.1.as_slice())
            .bind(after.map(|id| id.to_vec())).bind(remaining).bind(scope.2.to_string())
            .fetch_all(&mut **tx).await?;
        for row in rows {
            members.push(applicant(row)?);
        }
    }
    Ok((Some(listing), occupied, members))
}
