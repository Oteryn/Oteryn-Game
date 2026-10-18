use super::*;
use sqlx::{Connection, Executor, PgConnection};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn world(tag: u8) -> Result<WorldId, crate::domain::DomainError> {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tag;
    WorldId::from_bytes(bytes)
}

async fn version(connection: &mut PgConnection) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(connection)
        .await?;
    assert_eq!(version, "170006", "native PostgreSQL 17.6 is required");
    Ok(())
}

// Explicit native-schema fixture data. Trigger bypass is local to seeding only;
// recipient reads/locks run normally and no production mutation is qualified.
async fn seed(
    connection: &mut PgConnection,
    world: WorldId,
    name: &str,
) -> TestResult<CharacterId> {
    let mut tx = connection.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    let account:String=sqlx::query_scalar("INSERT INTO game_character_account_guards(account_id) VALUES(game_character_uuid_v7()) RETURNING account_id::text").fetch_one(&mut *tx).await?;
    let id:String=sqlx::query_scalar("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(game_character_uuid_v7(),$3::uuid,encode($1,'hex')::uuid,1,1,'p1','r1','c1','s1',$2) RETURNING character_id::text")
        .bind(world.as_bytes().as_slice()).bind(name).bind(account).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_name_reservations(name_key,naming_policy_revision,character_id,reserved_at) VALUES(game_character_name_key($1),1,$2::uuid,0)")
        .bind(name).bind(&id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(CharacterId::from_bytes(super::uuid_text(&id)?)?)
}

pub(crate) async fn mail_recipient_same_world_self_and_unknown_are_privacy_safe(
    pool: sqlx::PgPool,
) -> TestResult {
    let mut connection = pool.acquire().await?;
    version(&mut connection).await?;
    let own_world = world(1)?;
    let other_world = world(2)?;
    let recipient = seed(&mut connection, own_world, "Al Dric").await?;
    seed(&mut connection, other_world, "Foreign Name").await?;
    sqlx::query("INSERT INTO game_character_name_reservations(name_key,naming_policy_revision,character_id,reserved_at) VALUES('previousname',1,encode($1,'hex')::uuid,0)")
        .bind(recipient.as_bytes().as_slice()).execute(&mut *connection).await?;
    let address = CharacterName::parse("ALDRIC").map_err(|_| "invalid fixture name")?;
    let mut tx = connection.begin().await?;
    assert_eq!(
        mail_recipient_hint(&mut tx, own_world, &address).await?,
        Some(recipient)
    );
    // There is no sender/account/online/Channel exclusion for letters, including self.
    assert!(lock_mail_recipient_root(&mut tx, own_world, recipient, &address).await?);
    for name in ["Foreign Name", "Unknown Name", "Previous Name"] {
        let address = CharacterName::parse(name).map_err(|_| "invalid fixture name")?;
        assert_eq!(
            mail_recipient_hint(&mut tx, own_world, &address).await?,
            None
        );
    }
    assert_eq!(
        mail_recipient_hint(&mut tx, other_world, &address).await?,
        None
    );
    assert!(!lock_mail_recipient_root(&mut tx, other_world, recipient, &address).await?);
    tx.rollback().await?;
    Ok(())
}

pub(crate) async fn mail_stale_hint_rechecks_rename_lifecycle_and_world_independently(
    pool: sqlx::PgPool,
) -> TestResult {
    let mut connection = pool.acquire().await?;
    version(&mut connection).await?;
    let own_world = world(1)?;
    let recipient = seed(&mut connection, own_world, "Aldric").await?;
    let address = CharacterName::parse("Aldric").map_err(|_| "fixture name")?;
    let mut tx = connection.begin().await?;
    let hint = mail_recipient_hint(&mut tx, own_world, &address).await?;
    assert_eq!(hint, Some(recipient));
    tx.commit().await?;
    for update in [
        "name='Aldrik'",
        "world_id='00000000-0000-7000-8000-000000000002'::uuid",
    ] {
        // Each mutation changes one invariant from the same valid starting state.
        let mut mutation = connection.begin().await?;
        mutation
            .execute("SET LOCAL session_replication_role=replica")
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE game_character_roots SET {update} WHERE character_id=encode($1,'hex')::uuid"
        )))
        .bind(recipient.as_bytes().as_slice())
        .execute(&mut *mutation)
        .await?;
        assert!(!lock_mail_recipient_root(&mut mutation, own_world, recipient, &address).await?);
        assert_eq!(
            mail_recipient_hint(&mut mutation, own_world, &address).await?,
            None
        );
        mutation.rollback().await?;
    }
    // Native roots currently permit only lifecycle=1 (0005). Use actual row
    // absence for the terminal/missing-root case; invent no lifecycle encoding.
    let mut removed = connection.begin().await?;
    removed
        .execute("SET LOCAL session_replication_role=replica")
        .await?;
    sqlx::query("DELETE FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(recipient.as_bytes().as_slice())
        .execute(&mut *removed)
        .await?;
    assert!(!lock_mail_recipient_root(&mut removed, own_world, recipient, &address).await?);
    assert_eq!(
        mail_recipient_hint(&mut removed, own_world, &address).await?,
        None
    );
    removed.rollback().await?;
    Ok(())
}

pub(crate) async fn mail_recipient_root_share_lock_blocks_terminal_change_until_transaction_ends(
    pool: sqlx::PgPool,
) -> TestResult {
    let mut connection = pool.acquire().await?;
    version(&mut connection).await?;
    let world = world(1)?;
    let recipient = seed(&mut connection, world, "Aldric").await?;
    let address = CharacterName::parse("Aldric").map_err(|_| "fixture name")?;
    let mut held = connection.begin().await?;
    assert!(lock_mail_recipient_root(&mut held, world, recipient, &address).await?);
    let mut other = pool.acquire().await?;
    let mut contender = other.begin().await?;
    contender
        .execute("SET LOCAL session_replication_role=replica; SET LOCAL lock_timeout='50ms'")
        .await?;
    let attempt =
        sqlx::query("DELETE FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(recipient.as_bytes().as_slice())
            .execute(&mut *contender)
            .await;
    assert_eq!(
        attempt
            .as_ref()
            .err()
            .and_then(|error| error.as_database_error())
            .and_then(|error| error.code())
            .as_deref(),
        Some("55P03")
    );
    contender.rollback().await?;
    held.commit().await?;
    let mut after = other.begin().await?;
    after
        .execute("SET LOCAL session_replication_role=replica")
        .await?;
    assert_eq!(
        sqlx::query("DELETE FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(recipient.as_bytes().as_slice())
            .execute(&mut *after)
            .await?
            .rows_affected(),
        1
    );
    after.rollback().await?;
    Ok(())
}
