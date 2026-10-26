//! GUILD-WAR-1 migration qualification mounted with the dispatch PostgreSQL suite.
#[test]
fn guild_war_schema_is_embedded_for_dispatch() {
    let migration = include_str!("../../../../../../apps/game-server/migrations/0232_guild_wars.sql");
    assert!(migration.contains("CREATE TABLE game_guild_wars"));
    assert!(migration.contains("game_guild_war_commands"));
    assert!(migration.contains("CREATE TABLE game_guild_war_frags"));
    assert!(migration.contains("death_occurrence_id UUID PRIMARY KEY"));
}

#[test]
fn guild_war_frag_and_end_writes_are_mounted_with_dispatch() {
    let writer = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../durability/src/durability/guild_war.rs"
    ));
    assert!(writer.contains("attribute_guild_war_frag_in_transaction"));
    assert!(writer.contains("ON CONFLICT(death_occurrence_id) DO NOTHING"));
    assert!(writer.contains("guild_war_end(limit"));
}
