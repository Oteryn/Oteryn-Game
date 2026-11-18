use super::*;

// Node-boot owns these disposable Characters and is their only admission
// caller. Wait for the previous owner to release before touching its saved cell.
pub(super) async fn reset_fixture_to_town(clients: &SeamClients<'_>, index: usize) -> TestResult {
    wait_character_released(clients, index).await?;
    tokio::time::timeout(FIXTURE_COMMAND_DEADLINE, async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        reset_saved_fixture_position(
            &mut connection,
            &clients.characters[index],
            &clients.world,
            &clients.accounts[index],
        )
        .await
    })
    .await
    .map_err(|_| "fixture town reset deadline")?
}

async fn reset_saved_fixture_position(
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
    world: &[u8; 16],
    account: &str,
) -> TestResult {
    // A fresh fixture has no saved row and will use the configured first-entry
    // temple. Preserve the position frame, lease generation and all other state.
    let eligible = sqlx::query_scalar::<_, bool>(
        "WITH fixture AS (
             SELECT r.character_id, r.world_id FROM game_character_roots r
             WHERE r.character_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid
             AND r.account_id=$3::text::uuid AND NOT EXISTS (
                 SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.world_id=r.world_id
                 AND s.session_state<>3 AND (s.account_id=r.account_id OR s.character_id=r.character_id))
         ), moved AS (
             UPDATE game_character_last_positions p
             SET tile_x=$4, tile_y=$5, tile_floor=$6, write_sequence=p.write_sequence+1,
                 house_key=NULL, origin_channel_id=NULL, origin_tile_x=NULL,
                 origin_tile_y=NULL, origin_tile_floor=NULL
             FROM fixture f WHERE p.character_id=f.character_id AND p.world_id=f.world_id
             AND ((p.tile_x,p.tile_y,p.tile_floor) IS DISTINCT FROM ($4,$5,$6) OR p.house_key IS NOT NULL)
         )
         SELECT EXISTS(SELECT 1 FROM fixture)",
    )
    .bind(character.as_slice())
    .bind(world.as_slice())
    .bind(account)
    .bind(i32::from(TOWN.x))
    .bind(i32::from(TOWN.y))
    .bind(i16::from(TOWN.floor))
    .fetch_one(connection).await?;
    if !eligible {
        return Err("fixture town reset requires the expected released Character".into());
    }
    Ok(())
}

#[tokio::test]
async fn qualification_town_reset_updates_only_released_fixture_position() -> TestResult {
    let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let mut connection = sqlx::PgConnection::connect(&url).await?;
    sqlx::raw_sql(
        "CREATE TEMP TABLE game_character_roots (character_id uuid, world_id uuid, account_id uuid);
         CREATE TEMP TABLE game_durability_reconnect_sessions
            (character_id uuid, world_id uuid, account_id uuid, session_state smallint);
         CREATE TEMP TABLE game_character_last_positions (character_id uuid, world_id uuid,
            tile_x int, tile_y int, tile_floor smallint, write_sequence numeric, health int,
            house_key text, origin_channel_id uuid, origin_tile_x int, origin_tile_y int,
            origin_tile_floor smallint);
         INSERT INTO game_character_roots VALUES
            ('01010101-0101-0101-0101-010101010101','03030303-0303-0303-0303-030303030303','04040404-0404-0404-0404-040404040404');
         INSERT INTO game_character_last_positions VALUES
            ('01010101-0101-0101-0101-010101010101','03030303-0303-0303-0303-030303030303',32296,32333,-8,3,42,NULL,NULL,NULL,NULL,NULL),
            ('02020202-0202-0202-0202-020202020202','03030303-0303-0303-0303-030303030303',32296,32333,-8,3,42,NULL,NULL,NULL,NULL,NULL);
         INSERT INTO game_durability_reconnect_sessions
            SELECT character_id, world_id, account_id, 2 FROM game_character_roots",
    ).execute(&mut connection).await?;
    let account = "04040404-0404-0404-0404-040404040404";
    assert!(
        reset_saved_fixture_position(&mut connection, &[1; 16], &[3; 16], account)
            .await
            .is_err()
    );
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
        .execute(&mut connection)
        .await?;
    reset_saved_fixture_position(&mut connection, &[1; 16], &[3; 16], account).await?;
    reset_saved_fixture_position(&mut connection, &[1; 16], &[3; 16], account).await?;
    let positions = sqlx::query_as::<_, (i32, i32, i16, i32, i32)>(
        "SELECT tile_x,tile_y,tile_floor,write_sequence::int,health
         FROM game_character_last_positions ORDER BY character_id",
    )
    .fetch_all(&mut connection)
    .await?;
    assert_eq!(
        positions,
        [(32369, 32238, -7, 4, 42), (32296, 32333, -8, 3, 42)]
    );
    Ok(())
}
