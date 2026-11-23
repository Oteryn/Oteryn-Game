use super::*;
use sqlx::{PgConnection, Row};

pub(super) async fn online_count(conn: &mut PgConnection, worlds: &[Uuid]) -> Result<i64, E> {
    sqlx::query_scalar("SELECT count(*) FROM admin_read.online_characters WHERE world_id=ANY($1)")
        .bind(worlds)
        .fetch_one(conn)
        .await
        .map_err(db_error)
}

pub(super) async fn search(
    conn: &mut PgConnection,
    worlds: &[Uuid],
    target: &Target,
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
    let rows = sqlx::query_scalar::<_, Json<PlayerSummary>>(
        "WITH candidates AS MATERIALIZED (
            SELECT r.character_id,r.world_id,r.name FROM admin_read.character_roots r
            WHERE r.world_id=ANY($1)
              AND ($2::text IS NULL OR r.name_key LIKE $2 || '%')
              AND ($3::uuid IS NULL OR r.character_id=$3)
              AND ($4::uuid IS NULL OR r.account_id=$4)
              AND ($5::uuid IS NULL OR r.character_id=$5)
              AND ($6::uuid IS NULL OR r.account_id=$6)
              AND ($7::uuid IS NULL OR r.character_id>$7)
            ORDER BY r.character_id LIMIT $8
         )
         SELECT jsonb_build_object('character_id',r.character_id,'world_id',r.world_id,
                  'name',r.name,'level',COALESCE(p.level,1),'online',online.character_id IS NOT NULL)
         FROM candidates r LEFT JOIN admin_read.character_progression p USING(character_id)
         LEFT JOIN admin_read.online_characters online ON online.character_id=r.character_id AND online.world_id=r.world_id
         ORDER BY r.character_id"
    )
    .bind(worlds).bind(prefix)
    .bind(optional_uuid(params.character_id.as_deref())?)
    .bind(optional_uuid(params.account_ref.as_deref())?)
    .bind(optional_uuid(target.character_id.as_deref())?)
    .bind(optional_uuid(target.account_ref.as_deref())?)
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

pub(super) async fn check_offline_target(
    conn: &mut PgConnection,
    target: &Target,
) -> Result<(), E> {
    let character = uuid(required(&target.character_id)?)?;
    let world = uuid(required(&target.world_id)?)?;
    // Check the current durable holders in the same snapshot as any new result.
    let busy: bool = sqlx::query_scalar(
        "SELECT busy FROM admin_read.character_roots
         WHERE character_id=$1 AND world_id=$2 AND ($3::uuid IS NULL OR account_id=$3)",
    )
    .bind(character)
    .bind(world)
    .bind(optional_uuid(target.account_ref.as_deref())?)
    .fetch_optional(conn)
    .await
    .map_err(db_error)?
    .ok_or(E::AdmTargetNotFound)?;
    if busy {
        return Err(E::AdmPreconditionFailed);
    }
    Ok(())
}

pub(super) async fn character(conn: &mut PgConnection, target: &Target) -> Result<Character, E> {
    let id = uuid(required(&target.character_id)?)?;
    let world = uuid(required(&target.world_id)?)?;
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
              SELECT 1 FROM admin_read.premium_account_fences f
              JOIN admin_read.premium_evidence e USING(account_id,authority_revision)
              WHERE f.account_id=r.account_id AND f.conflict_authority_revision IS NULL
                AND e.entitlement_state=1
                AND EXTRACT(EPOCH FROM statement_timestamp())*1000000 >= greatest(e.effective_from_us,e.authority_issued_at_us)
                AND EXTRACT(EPOCH FROM statement_timestamp())*1000000 < least(e.effective_until_us,e.authority_valid_until_us)
           ),
           'guild',(SELECT g.name FROM admin_read.guild_members m JOIN admin_read.guilds g USING(guild_id)
                    WHERE m.character_id=r.character_id AND g.world_id=r.world_id AND g.state<>4),
           'house',(SELECT h.house_key FROM admin_read.house_properties h
                    WHERE h.world_id=r.world_id AND h.owner_character_id=r.character_id
                    ORDER BY h.house_key LIMIT 1),
           'bank_balance',COALESCE(bank.balance,0)::text,
           'sanctions','[]'::jsonb,
           'last_logins',ARRAY(SELECT to_timestamp(i.started_ms/1000.0)
               FROM admin_read.online_intervals i
               WHERE i.character_id=r.character_id AND i.owner_account_id=r.account_id
                 AND i.world_id=r.world_id AND i.state=2 AND NOT i.withheld
                 AND i.expires_ms > EXTRACT(EPOCH FROM statement_timestamp()) * 1000
               ORDER BY i.started_ms DESC,i.statistics_id DESC LIMIT 20))
         FROM admin_read.character_roots r
         JOIN admin_read.character_positions l USING(character_id,world_id)
         LEFT JOIN admin_read.character_progression p USING(character_id)
         LEFT JOIN admin_read.character_build b USING(character_id)
         LEFT JOIN admin_read.bank_balances bank USING(account_id,world_id)
         WHERE r.character_id=$1 AND r.world_id=$2",
    )
    .bind(id).bind(world)
    .fetch_optional(&mut *conn).await.map_err(db_error)?.ok_or(E::AdmTargetUnavailable)?;
    character.sanctions = sanctions(
        conn,
        target,
        &[world],
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
    let id = uuid(required(&target.character_id)?)?;
    let world = uuid(required(&target.world_id)?)?;
    // Same custody roots and edges as the Cyclopedia holdings projection, with
    // instance IDs retained. Stash balances have no item instance IDs.
    let stash_cursor = params
        .cursor
        .as_deref()
        .and_then(|cursor| cursor.strip_prefix("stash/"));
    let rows = if stash_cursor.is_some() {
        vec![]
    } else {
        sqlx::query(
            "WITH RECURSIVE roots AS (
            SELECT item_instance_id, 'backpack' AS location FROM admin_read.container_slots
             WHERE character_id=$1 AND world_id=$2
            UNION ALL SELECT item_instance_id, 'equipment:' || slot FROM admin_read.equipment_slots
             WHERE character_id=$1 AND world_id=$2
            UNION ALL SELECT item_instance_id, 'depot' FROM admin_read.depot_locations
             WHERE character_id=$1 AND world_id=$2
            UNION ALL SELECT item_instance_id, 'inbox' FROM admin_read.inbox_locations
             WHERE character_id=$1 AND world_id=$2
         ), tree(item_instance_id,location,parent_id,depth) AS (
            SELECT item_instance_id,location,NULL::uuid,0 FROM roots
            UNION ALL
            SELECT e.item_instance_id,t.location,t.item_instance_id,t.depth+1 FROM tree t
             CROSS JOIN LATERAL (
              SELECT item_instance_id FROM admin_read.container_entries
               WHERE world_id=$2 AND character_id=$1 AND parent_item_instance_id=t.item_instance_id
              UNION ALL SELECT item_instance_id FROM admin_read.nested_entries
               WHERE world_id=$2 AND parent_item_instance_id=t.item_instance_id
             ) e WHERE t.depth < 8
         )
         SELECT i.item_instance_id::text, i.definition_production_key,
                i.quantity::text, t.location, t.parent_id::text
         FROM tree t JOIN admin_read.item_instances i USING(item_instance_id)
         WHERE i.world_id=$2 AND i.lifecycle=1 AND ($3::uuid IS NULL OR i.item_instance_id>$3)
         ORDER BY i.item_instance_id LIMIT $4",
        )
        .bind(id)
        .bind(world)
        .bind(optional_uuid(params.cursor.as_deref())?)
        .bind(i64::from(params.limit) + 1)
        .fetch_all(&mut *conn)
        .await
        .map_err(db_error)?
    };
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
    let (items, mut next_cursor) = page(items, params.limit, |row| row.instance_id.clone());
    let mut stash_balances = vec![];
    if next_cursor.is_none() {
        let remaining = params.limit - items.len() as u16;
        let after = stash_cursor
            .map(str::parse::<u32>)
            .transpose()
            .map_err(|_| E::AdmInvalidParams)?
            .map(i64::from);
        let rows = sqlx::query_scalar::<_, Json<StashBalance>>(
            "SELECT jsonb_build_object('type_id',type_id,'count',sum(quantity)::text)
             FROM admin_read.stash_balances
             WHERE character_id=$1 AND world_id=$2 AND ($3::bigint IS NULL OR type_id>$3)
             GROUP BY type_id ORDER BY type_id LIMIT $4",
        )
        .bind(id)
        .bind(world)
        .bind(after)
        .bind(i64::from(remaining) + 1)
        .fetch_all(conn)
        .await
        .map_err(db_error)?;
        if remaining == 0 && !rows.is_empty() {
            next_cursor = items.last().map(|item| item.instance_id.clone());
        } else {
            (stash_balances, next_cursor) = page(json_rows(rows), remaining, |row| {
                format!("stash/{}", row.type_id)
            });
        }
    }
    Ok(Inventory {
        items,
        stash_balances,
        next_cursor,
    })
}

pub(super) async fn check_sanction_visibility(
    conn: &mut PgConnection,
    worlds: &[Uuid],
    sanctions: &[Sanction],
    target: &Target,
) -> Result<(), E> {
    // A retained account-wide row omits the Character from the signed selector.
    let selected = target.character_id.as_deref();
    let characters = sanctions
        .iter()
        .map(|row| optional_uuid(row.character_id.as_deref().or(selected)))
        .collect::<Result<Vec<_>, _>>()?;
    let accounts = sanctions
        .iter()
        .map(|row| optional_uuid(row.account_ref.as_deref()))
        .collect::<Result<Vec<_>, _>>()?;
    let visible: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS (
            SELECT 1 FROM unnest($1::uuid[], $2::uuid[]) AS s(character_id,account_id)
            WHERE NOT EXISTS (
                SELECT 1 FROM admin_read.character_roots r
                WHERE r.world_id=ANY($3) AND r.account_id=s.account_id
                  AND (s.character_id IS NULL OR r.character_id=s.character_id)))",
    )
    .bind(characters)
    .bind(accounts)
    .bind(worlds)
    .fetch_one(conn)
    .await
    .map_err(db_error)?;
    if !visible {
        return Err(E::AdmTargetNotFound);
    }
    Ok(())
}

pub(super) async fn sanctions(
    conn: &mut PgConnection,
    target: &Target,
    worlds: &[Uuid],
    params: Page,
) -> Result<SanctionPage, E> {
    let rows = sqlx::query_as::<_, (String, Json<Sanction>)>(
        "WITH records AS (
           SELECT s.key,s.id,s.character_id,s.account_id,s.kind,s.starts_at_ms,s.ends_at_ms,s.lifted
           FROM admin_read.sanctions s
           WHERE ($2::uuid IS NULL OR s.account_id=$2)
             AND (
               (s.world_id=ANY($3) AND ($1::uuid IS NULL OR s.character_id=$1))
               OR (s.character_id IS NULL AND EXISTS (
                   SELECT 1 FROM admin_read.character_roots r
                   WHERE r.account_id=s.account_id AND r.world_id=ANY($3)
                     AND ($1::uuid IS NULL OR r.character_id=$1)))
             )
         )
         SELECT key, jsonb_build_object('sanction_id',id,'character_id',character_id,'account_ref',account_id,
                 'kind',kind,'starts_at',to_timestamp(starts_at_ms/1000.0),
                 'expires_at',to_timestamp(ends_at_ms/1000.0),
                 'active',NOT lifted AND kind<>'notation' AND starts_at_ms <= EXTRACT(EPOCH FROM statement_timestamp()) * 1000
                         AND (ends_at_ms IS NULL OR ends_at_ms > EXTRACT(EPOCH FROM statement_timestamp()) * 1000))
         FROM records WHERE ($4::text IS NULL OR key COLLATE \"C\">$4)
         ORDER BY key COLLATE \"C\" LIMIT $5",
    )
    .bind(optional_uuid(target.character_id.as_deref())?)
    .bind(optional_uuid(target.account_ref.as_deref())?)
    .bind(worlds).bind(params.cursor).bind(i64::from(params.limit)+1)
    .fetch_all(conn).await.map_err(db_error)?;
    let (rows, next_cursor) = page(rows, params.limit, |(key, _)| key.clone());
    let sanctions = rows.into_iter().map(|(_, Json(row))| row).collect();
    Ok(SanctionPage {
        sanctions,
        next_cursor,
    })
}
