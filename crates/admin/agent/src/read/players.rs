use super::*;
use sqlx::{PgConnection, Row};

// Durable connected presence needs matching character and account holders and lease generation.
// A detached or replaced transport must not look like a live player.
const ONLINE: &str = "
    SELECT s.character_id, s.world_id
    FROM game_durability_reconnect_sessions s
    JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id
    JOIN game_durability_admission_account_guards a ON a.account_id=s.account_id
    WHERE s.session_state=2 AND s.current_transport_ref IS NOT NULL
      AND c.eligible AND c.world_id=s.world_id AND c.account_id=s.account_id
      AND c.lease_generation=s.character_lease_generation AND c.holder_game_session_id=s.game_session_id
      AND a.presence_character_id=s.character_id AND a.holder_game_session_id=s.game_session_id";

pub(super) async fn online_count(conn: &mut PgConnection, worlds: &[Uuid]) -> Result<i64, E> {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM ({ONLINE}) online WHERE world_id=ANY($1)"
    )))
    .bind(worlds)
    .fetch_one(conn)
    .await
    .map_err(db_error)
}

pub(super) async fn search(
    conn: &mut PgConnection,
    worlds: &[Uuid],
    params: PlayerSearch,
) -> Result<PlayerPage, E> {
    if params.ip_hash.is_some() {
        return Err(E::AdmTargetUnavailable);
    }
    let prefix = params
        .name_prefix
        .as_deref()
        .map(|prefix| {
            let key = prefix.replace(' ', "").to_ascii_lowercase();
            if key.is_empty() || key.len() > 29 || !key.bytes().all(|b| b.is_ascii_lowercase()) {
                return Err(E::AdmInvalidParams);
            }
            Ok(key)
        })
        .transpose()?;
    let rows = sqlx::query_scalar::<_, Json<PlayerSummary>>(sqlx::AssertSqlSafe(format!(
        "WITH candidates AS MATERIALIZED (
            SELECT r.character_id,r.world_id,r.name FROM game_character_roots r
            WHERE r.world_id=ANY($1) AND r.lifecycle=1 AND NOT game_account_closure_detached(r.character_id)
              AND ($2::text IS NULL OR r.name_key LIKE $2 || '%')
              AND ($3::uuid IS NULL OR r.character_id=$3)
              AND ($4::uuid IS NULL OR r.account_id=$4)
              AND ($5::uuid IS NULL OR r.character_id>$5)
            ORDER BY r.character_id LIMIT $6
         )
         SELECT jsonb_build_object('character_id',r.character_id,'world_id',r.world_id,
                  'name',r.name,'level',COALESCE(p.level,1),'online',online.character_id IS NOT NULL)
         FROM candidates r LEFT JOIN game_character_progression_state p USING(character_id)
         LEFT JOIN ({ONLINE}) online ON online.character_id=r.character_id AND online.world_id=r.world_id
         ORDER BY r.character_id"
    )))
    .bind(worlds).bind(prefix)
    .bind(optional_uuid(params.character_id.as_deref())?)
    .bind(optional_uuid(params.account_ref.as_deref())?)
    .bind(optional_uuid(params.cursor.as_deref())?)
    .bind(i64::from(params.limit)+1)
    .fetch_all(conn).await.map_err(db_error)?;
    let (players, next_cursor) = page(json_rows(rows), params.limit, |row| {
        row.character_id.clone()
    });
    Ok(PlayerPage {
        players,
        next_cursor,
    })
}

async fn offline_target(conn: &mut PgConnection, target: &Target) -> Result<(Uuid, Uuid), E> {
    let character = uuid(required(&target.character_id)?)?;
    let world = uuid(required(&target.world_id)?)?;
    // Same guards as the Game's offline Character reads, in the same read-only snapshot
    // as the data. Any nonterminal session or outstanding holder excludes the offline path.
    let busy: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
                       WHERE s.character_id=r.character_id AND s.session_state IN (1,2))
             OR EXISTS(SELECT 1 FROM game_durability_admission_character_guards c
                       WHERE c.character_id=r.character_id AND c.holder_game_session_id IS NOT NULL)
             OR EXISTS(SELECT 1 FROM game_durability_admission_account_guards a
                       WHERE a.account_id=r.account_id AND a.presence_character_id=r.character_id)
         FROM game_character_roots r WHERE r.character_id=$1 AND r.world_id=$2
           AND r.lifecycle=1 AND NOT game_account_closure_detached(r.character_id)",
    )
    .bind(character)
    .bind(world)
    .fetch_optional(conn)
    .await
    .map_err(db_error)?
    .ok_or(E::AdmTargetNotFound)?;
    if busy {
        return Err(E::AdmTargetUnavailable);
    }
    Ok((character, world))
}

pub(super) async fn character(conn: &mut PgConnection, target: &Target) -> Result<Character, E> {
    let (id, world) = offline_target(conn, target).await?;
    let Json(mut character) = sqlx::query_scalar::<_, Json<Character>>(
        "SELECT jsonb_build_object(
           'character_id',r.character_id,'world_id',r.world_id,'name',r.name,
           'level',COALESCE(p.level,1),'vocation',COALESCE(b.vocation,r.creation_vocation,'none'),
           'skills',(SELECT jsonb_agg(jsonb_build_object('name',name,'level',level,'progress',0))
                     FROM (VALUES ('magic',COALESCE(b.magic_level,0)),
                        ('fist',COALESCE(b.fist_level,10)),('club',COALESCE(b.club_level,10)),
                        ('sword',COALESCE(b.sword_level,10)),('axe',COALESCE(b.axe_level,10)),
                        ('distance',COALESCE(b.distance_level,10)),
                        ('shielding',COALESCE(b.shielding_level,10)),
                        ('fishing',COALESCE(b.fishing_level,10))) skills(name,level)),
           'position',jsonb_build_object('x',l.tile_x,'y',l.tile_y,'z',l.tile_floor),
           'online',false,
           'premium',EXISTS(
              SELECT 1 FROM game_premium_account_fence f
              JOIN game_premium_evidence e USING(account_id,authority_revision)
              WHERE f.account_id=r.account_id AND f.conflict_authority_revision IS NULL
                AND e.entitlement_state=1
                AND EXTRACT(EPOCH FROM statement_timestamp())*1000000 >= greatest(e.effective_from_us,e.authority_issued_at_us)
                AND EXTRACT(EPOCH FROM statement_timestamp())*1000000 < least(e.effective_until_us,e.authority_valid_until_us)
           ),
           'guild',(SELECT g.name FROM game_guild_members m JOIN game_guilds g USING(guild_id)
                    WHERE m.character_id=r.character_id AND g.world_id=r.world_id AND g.state<>4),
           'house',(SELECT h.house_key FROM game_house_properties h
                    WHERE h.world_id=r.world_id AND h.owner_character_id=r.character_id
                    ORDER BY h.house_key LIMIT 1),
           'bank_balance',COALESCE(bank.balance,0)::text,
           'sanctions','[]'::jsonb,
           'last_logins',ARRAY(SELECT to_timestamp(i.started_ms/1000.0)
               FROM game_character_online_intervals i
               WHERE i.character_id=r.character_id AND i.owner_account_id=r.account_id
                 AND i.world_id=r.world_id AND i.state=2 AND NOT i.withheld
                 AND i.expires_ms > EXTRACT(EPOCH FROM statement_timestamp()) * 1000
               ORDER BY i.started_ms DESC,i.statistics_id DESC LIMIT 20))
         FROM game_character_roots r
         JOIN game_character_last_positions l USING(character_id,world_id)
         LEFT JOIN game_character_progression_state p USING(character_id)
         LEFT JOIN game_character_build_state b USING(character_id)
         LEFT JOIN game_account_bank_balances bank USING(account_id,world_id)
         WHERE r.character_id=$1 AND r.world_id=$2",
    )
    .bind(id).bind(world)
    .fetch_optional(&mut *conn).await.map_err(db_error)?.ok_or(E::AdmTargetUnavailable)?;
    character.sanctions = sanctions(
        conn,
        target,
        &[world],
        false,
        Page {
            limit: 100,
            cursor: None,
        },
    )
    .await?
    .sanctions;
    Ok(character)
}

pub(super) async fn inventory(
    conn: &mut PgConnection,
    target: &Target,
    params: Page,
) -> Result<Inventory, E> {
    let (id, world) = offline_target(conn, target).await?;
    // Same custody roots and edges as the Cyclopedia holdings projection, with
    // instance IDs retained. Stash balances have no item instance IDs.
    let rows = sqlx::query(
        "WITH RECURSIVE roots AS (
            SELECT item_instance_id, 'backpack' AS location FROM game_item_container_slots
             WHERE character_id=$1 AND world_id=$2
            UNION ALL SELECT item_instance_id, 'equipment:' || slot FROM game_character_equipment_slots
             WHERE character_id=$1 AND world_id=$2
            UNION ALL SELECT item_instance_id, 'depot' FROM game_item_character_depot_locations
             WHERE character_id=$1 AND world_id=$2
            UNION ALL SELECT item_instance_id, 'inbox' FROM game_item_character_inbox_locations
             WHERE character_id=$1 AND world_id=$2
         ), tree(item_instance_id,location,parent_id,depth) AS (
            SELECT item_instance_id,location,NULL::uuid,0 FROM roots
            UNION ALL
            SELECT e.item_instance_id,t.location,t.item_instance_id,t.depth+1 FROM tree t
             CROSS JOIN LATERAL (
              SELECT item_instance_id FROM game_item_container_entries
               WHERE world_id=$2 AND character_id=$1 AND parent_item_instance_id=t.item_instance_id
              UNION ALL SELECT item_instance_id FROM game_item_nested_entries
               WHERE world_id=$2 AND parent_item_instance_id=t.item_instance_id
             ) e WHERE t.depth < 8
         )
         SELECT i.item_instance_id::text, i.definition_production_key,
                i.quantity::text, t.location, t.parent_id::text
         FROM tree t JOIN game_item_instances i USING(item_instance_id)
         WHERE i.world_id=$2 AND i.lifecycle=1 AND ($3::uuid IS NULL OR i.item_instance_id>$3)
         ORDER BY i.item_instance_id LIMIT $4",
    )
    .bind(id).bind(world).bind(optional_uuid(params.cursor.as_deref())?).bind(i64::from(params.limit)+1)
    .fetch_all(conn).await.map_err(db_error)?;
    let items = rows
        .into_iter()
        .map(|row| {
            let definition: String = row.try_get("definition_production_key").map_err(db_error)?;
            let type_id = definition
                .strip_prefix("oteryn:item.tibia.i")
                .or_else(|| definition.strip_prefix("oteryn:item.i"))
                .and_then(|number| number.parse().ok())
                .unwrap_or(0);
            Ok(Item {
                instance_id: row.try_get("item_instance_id").map_err(db_error)?,
                type_id,
                count: row.try_get("quantity").map_err(db_error)?,
                location: row.try_get("location").map_err(db_error)?,
                parent_id: row.try_get("parent_id").map_err(db_error)?,
            })
        })
        .collect::<Result<Vec<_>, E>>()?;
    let (items, next_cursor) = page(items, params.limit, |row| row.instance_id.clone());
    Ok(Inventory { items, next_cursor })
}

pub(super) async fn sanctions(
    conn: &mut PgConnection,
    target: &Target,
    worlds: &[Uuid],
    global: bool,
    params: Page,
) -> Result<SanctionPage, E> {
    let rows = sqlx::query_as::<_, (String, Json<Sanction>)>(
        "WITH records AS (
           SELECT 'platform/' || p.sanction_stream_id AS key, p.sanction_stream_id AS id,
                  p.character_id,p.account_id,
                  (ARRAY['mute','jail','character_ban','account_ban','namelock','notation'])[p.kind] AS kind,
                  p.starts_at_ms,p.ends_at_ms,p.revoked AS lifted
           FROM game_account_sanction_projections p
           LEFT JOIN game_character_roots r ON r.character_id=p.character_id AND r.account_id=p.account_id
           WHERE ($1::uuid IS NULL OR p.character_id=$1)
             AND ($2::uuid IS NULL OR p.account_id=$2)
             AND (r.world_id=ANY($3) OR ($4 AND p.character_id IS NULL))
           UNION ALL
           SELECT 'staff/' || s.sanction_id || '/' || s.environment,s.sanction_id,
                  s.character_id,r.account_id,'mute',s.starts_at_ms,s.ends_at_ms,s.lifted_at_ms IS NOT NULL
           FROM game_character_sanctions s JOIN game_character_roots r USING(character_id)
           WHERE ($1::uuid IS NULL OR s.character_id=$1)
             AND ($2::uuid IS NULL OR r.account_id=$2) AND r.world_id=ANY($3)
         )
         SELECT key, jsonb_build_object('sanction_id',id,'character_id',character_id,'account_ref',account_id,
                 'kind',kind,'starts_at',to_timestamp(starts_at_ms/1000.0),
                 'expires_at',to_timestamp(ends_at_ms/1000.0),
                 'active',NOT lifted AND kind<>'notation' AND starts_at_ms <= EXTRACT(EPOCH FROM statement_timestamp()) * 1000
                         AND (ends_at_ms IS NULL OR ends_at_ms > EXTRACT(EPOCH FROM statement_timestamp()) * 1000))
         FROM records WHERE ($5::text IS NULL OR key COLLATE \"C\">$5)
         ORDER BY key COLLATE \"C\" LIMIT $6",
    )
    .bind(optional_uuid(target.character_id.as_deref())?)
    .bind(optional_uuid(target.account_ref.as_deref())?)
    .bind(worlds).bind(global).bind(params.cursor).bind(i64::from(params.limit)+1)
    .fetch_all(conn).await.map_err(db_error)?;
    let (rows, next_cursor) = page(rows, params.limit, |(key, _)| key.clone());
    let sanctions = rows.into_iter().map(|(_, Json(row))| row).collect();
    Ok(SanctionPage {
        sanctions,
        next_cursor,
    })
}
