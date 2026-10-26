//! GUILD-WAR-1 migration qualification mounted with the dispatch PostgreSQL suite.
#[test]
fn guild_war_schema_is_embedded_for_dispatch() {
    let migration = include_str!("../../../../../../apps/game-server/migrations/0232_guild_wars.sql");
    assert!(migration.contains("CREATE TABLE game_guild_wars"));
    assert!(migration.contains("game_guild_war_commands"));
}
