//! Accepted §10 fixture: access is replaced only inside a dedicated test database.
//! No fake session/scope authority, shape function or production migration is installed.

use sqlx::PgConnection;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub async fn install(connection: &mut PgConnection) -> TestResult {
    let (database, actor, version): (String, String, String) = sqlx::query_as(
        "SELECT current_database(), session_user::text, current_setting('server_version_num')",
    )
    .fetch_one(&mut *connection)
    .await?;
    if !database.starts_with("house_interior_")
        || actor != "oteryn_test_admin"
        || version != "170006"
    {
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
