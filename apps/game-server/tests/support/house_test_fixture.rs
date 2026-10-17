//! Accepted §10 fixture: access is replaced only inside a dedicated test database.
//! No fake session/scope authority, shape function or production migration is installed.

use sqlx::PgConnection;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// A House case's own database: the stub case's `house_interior_<nanos>`
/// (unless `harness_only`) or the native harness's `it_<tag>_<nanos>` for a
/// `house_interior_*` or `house_loader_*` tag, as the canonical test admin on 17.6.
async fn dedicated_database(connection: &mut PgConnection, harness_only: bool) -> TestResult<bool> {
    let (database, actor, version): (String, String, String) = sqlx::query_as(
        "SELECT current_database(), session_user::text, current_setting('server_version_num')",
    )
    .fetch_one(&mut *connection)
    .await?;
    let Some((stem, nanos)) = database.rsplit_once('_') else {
        return Ok(false);
    };
    let harness_tag = |prefix: &str| {
        stem.strip_prefix(prefix).is_some_and(|tag| {
            !tag.is_empty()
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
    };
    let named = !nanos.is_empty()
        && nanos.bytes().all(|b| b.is_ascii_digit())
        && ((!harness_only && stem == "house_interior")
            || harness_tag("it_house_interior_")
            || harness_tag("it_house_loader_"));
    Ok(named && actor == "oteryn_test_admin" && version == "170006")
}

pub async fn install(connection: &mut PgConnection) -> TestResult {
    if !dedicated_database(connection, false).await? {
        return Err("house fixture requires a dedicated PostgreSQL 17.6 test database".into());
    }
    sqlx::raw_sql(
        "CREATE TABLE test_house_properties (
             world_id UUID NOT NULL, house_key TEXT NOT NULL,
             acl_revision NUMERIC(20,0) NOT NULL CHECK (acl_revision > 0),
             content_fenced BOOLEAN NOT NULL,
             PRIMARY KEY(world_id, house_key));
         CREATE TABLE test_house_grants (
             world_id UUID NOT NULL, house_key TEXT NOT NULL, character_id UUID NOT NULL,
             role TEXT NOT NULL CHECK (role IN ('OWNER','SUBOWNER','GUEST')),
             guild_revisions JSONB,
             PRIMARY KEY(world_id, house_key, character_id),
             FOREIGN KEY(world_id, house_key) REFERENCES test_house_properties);
         CREATE TABLE test_house_tiles(world_id UUID NOT NULL,house_key TEXT NOT NULL,position BYTEA NOT NULL,PRIMARY KEY(world_id,house_key,position));
         REVOKE ALL ON test_house_tiles FROM PUBLIC;
         CREATE OR REPLACE FUNCTION game_house_access_tile(p_world UUID,p_house TEXT,p_position BYTEA)
         RETURNS BOOLEAN LANGUAGE sql AS $$ SELECT EXISTS(SELECT 1 FROM test_house_tiles WHERE world_id=p_world AND house_key=p_house AND position=p_position) $$;
         REVOKE ALL ON test_house_properties, test_house_grants FROM PUBLIC;
         CREATE OR REPLACE FUNCTION game_house_access(
             p_world_id UUID, p_house_key TEXT, p_character_id UUID)
         RETURNS TABLE(role TEXT, content_fenced BOOLEAN, acl_revision NUMERIC(20,0),
                       guild_revisions JSONB)
         LANGUAGE plpgsql AS $$
         DECLARE property test_house_properties%ROWTYPE;
         BEGIN
             SELECT p.* INTO property FROM test_house_properties p
              WHERE p.world_id = p_world_id AND p.house_key = p_house_key FOR SHARE;
             IF NOT FOUND THEN RETURN; END IF;
             RETURN QUERY SELECT g.role, property.content_fenced, property.acl_revision,
                                 g.guild_revisions
               FROM test_house_grants g
              WHERE g.world_id = p_world_id AND g.house_key = p_house_key
                AND g.character_id = p_character_id FOR SHARE;
         END $$;
         DO $$ BEGIN
             EXECUTE format('ALTER FUNCTION game_house_access(UUID,TEXT,UUID) SET search_path = %I, pg_temp', current_schema());
         END $$;",
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Independent, inactive occupancy DATA for the cardinality boundary only. This is
/// not a native mint/placement history, current grant, session, assignment or shape proof.
/// Mirrors the existing HOUSE-CUSTODY test seeding idiom. All guards are restored before
/// returning; the tested actor/source, mint, transfer, handoff and shape use real APIs.
pub async fn seed_inactive_occupancy(
    connection: &mut PgConnection,
    world: &[u8],
    house: &str,
    subject: &[u8],
    count: i32,
) -> TestResult {
    use sqlx::Connection;
    if !dedicated_database(connection, true).await? || !(0..=1999).contains(&count) {
        return Err("inactive occupancy fixture only".into());
    }
    let mut tx = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE TEMP TABLE test_house_inactive_seed ON COMMIT DROP AS SELECT game_character_uuid_v7() item,game_character_uuid_v7() mint,game_character_uuid_v7() placement,g::numeric ordinal FROM generate_series(1,$1) g").bind(count).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT item,encode($1,'hex')::uuid,'Item','fixture:inactive.occupancy','definition-r1',1,1,mint FROM test_house_inactive_seed").bind(world).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations SELECT item,encode($1,'hex')::uuid,$2,'\\x010203'::bytea,ordinal,placement FROM test_house_inactive_seed").bind(world).bind(house).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) SELECT item,encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,placement,1 FROM test_house_inactive_seed").bind(world).bind(house).bind(subject).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_placement_transactions SELECT placement,item FROM test_house_inactive_seed").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    let orphans:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_house_interior_locations h LEFT JOIN game_item_instances i ON(i.item_instance_id,i.world_id)=(h.item_instance_id,h.world_id) LEFT JOIN game_item_house_reclaim_provenance p ON(p.item_instance_id,p.world_id,p.house_key,p.placement_transaction_id)=(h.item_instance_id,h.world_id,h.house_key,h.placed_transaction_id) LEFT JOIN game_character_roots c ON c.character_id=p.reclaim_subject_character_id LEFT JOIN game_item_house_placement_transactions l ON(l.placed_transaction_id,l.item_instance_id)=(h.placed_transaction_id,h.item_instance_id) WHERE i.item_instance_id IS NULL OR p.item_instance_id IS NULL OR c.character_id IS NULL OR l.item_instance_id IS NULL").fetch_one(&mut *tx).await?;
    assert_eq!(
        orphans, 0,
        "all actual custody/provenance/subject/ledger FK targets exist"
    );
    tx.commit().await?;
    let guards: String = sqlx::query_scalar("SHOW session_replication_role")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(guards, "origin");
    Ok(())
}

/// Synthetic retained-audit corruption/absence DATA after a genuine commit. This does
/// not qualify the native retention clock or pruning writer. Only the audit row changes;
/// the permanent reservation, receipt, identity, hash and custody remain untouched.
pub async fn retained_audit_data_probe(
    connection: &mut PgConnection,
    transaction: &[u8],
    event: &[u8],
    replacement: Option<&[u8]>,
) -> TestResult {
    use sqlx::Connection;
    if !dedicated_database(connection, true).await? {
        return Err("retained audit DATA fixture only".into());
    }
    let mut tx = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let result = if let Some(bytes) = replacement {
        sqlx::query("UPDATE game_item_audit_outbox SET envelope=$3,envelope_sha256=sha256($3) WHERE transaction_id=encode($1,'hex')::uuid AND event_id=encode($2,'hex')::uuid").bind(transaction).bind(event).bind(bytes).execute(&mut *tx).await?
    } else {
        sqlx::query("DELETE FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid AND event_id=encode($2,'hex')::uuid").bind(transaction).bind(event).execute(&mut *tx).await?
    };
    assert_eq!(
        result.rows_affected(),
        1,
        "exact genuine audit row DATA probe"
    );
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let guards: String = sqlx::query_scalar("SHOW session_replication_role")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(guards, "origin");
    Ok(())
}

/// Same stable EventId, independently corrupted/restored transaction DATA; every
/// operational reader runs after restoration of all native guards.
pub async fn retained_audit_transaction_data_probe(
    connection: &mut PgConnection,
    event: &[u8],
    transaction: &[u8],
) -> TestResult {
    use sqlx::Connection;
    if !dedicated_database(connection, true).await? {
        return Err("retained audit DATA fixture only".into());
    }
    let mut tx = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let result=sqlx::query("UPDATE game_item_audit_outbox SET transaction_id=encode($2,'hex')::uuid WHERE event_id=encode($1,'hex')::uuid").bind(event).bind(transaction).execute(&mut *tx).await?;
    assert_eq!(result.rows_affected(), 1);
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let guards: String = sqlx::query_scalar("SHOW session_replication_role")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(guards, "origin");
    Ok(())
}

/// Inactive reclaim-subject DATA only. No session, admission mirror, node, access grant
/// or writer proof is created. The genuine active actor remains independently fenced.
pub async fn seed_inactive_reclaim_subject(
    connection: &mut PgConnection,
    active_actor: &[u8],
) -> TestResult<Vec<u8>> {
    use sqlx::Connection;
    if !dedicated_database(connection, true).await? {
        return Err("inactive reclaim-subject DATA fixture only".into());
    }
    let mut tx = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let subject:Vec<u8>=sqlx::query_scalar("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT game_character_uuid_v7(),account_id,world_id,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Inactive Reclaim' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid RETURNING uuid_send(character_id)").bind(active_actor).fetch_one(&mut *tx).await?;
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let guards: String = sqlx::query_scalar("SHOW session_replication_role")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(guards, "origin");
    Ok(subject)
}
