//! ADR-0019 gameplay crate `oteryn-gameplay-durability`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

extern crate self as oteryn_gameplay_durability;

/// The game-server migrations, embedded once here so files that other crates
/// include by `#[path]` name one migrator whichever crate compiles them.
pub static GAME_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../../../apps/game-server/migrations");

#[cfg(test)]
#[test]
fn migration_numbers_are_unique_and_match_embedded_migrator()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../apps/game-server/migrations");
    let mut numbers = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|name| format!("non-UTF-8 migration filename: {name:?}"))?;
        let bytes = name.as_bytes();
        assert!(
            entry.file_type()?.is_file()
                && bytes.len() >= 10
                && bytes[..4].iter().all(u8::is_ascii_digit)
                && bytes[4] == b'_'
                && name.ends_with(".sql")
                && bytes[5..bytes.len() - 4]
                    .iter()
                    .all(|byte| byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || *byte == b'_'),
            "migration entry must match ^[0-9]{{4}}_[a-z0-9_]+\\.sql$: {name}"
        );
        let number: i64 = name[..4].parse()?;
        match numbers.entry(number) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(name);
            }
            std::collections::btree_map::Entry::Occupied(entry) => {
                return Err(format!(
                    "duplicate migration number {number:04}: {} and {name}; take the next free number",
                    entry.get()
                )
                .into());
            }
        }
    }
    let embedded: Vec<_> = GAME_MIGRATOR
        .iter()
        .map(|migration| migration.version)
        .collect();
    for pair in embedded.windows(2) {
        assert!(
            pair[0] < pair[1],
            "embedded migrations out of order: {pair:?}"
        );
    }
    assert_eq!(
        embedded,
        numbers.into_keys().collect::<Vec<_>>(),
        "embedded migration versions must match the migration directory"
    );
    Ok(())
}

/// `SHOW server_version_num` of the one PostgreSQL release every native
/// qualification runs against (18.6). CI service images, the qualification
/// scripts and this constant move together.
pub const CANONICAL_POSTGRES_SERVER_VERSION_NUM: &str = "180006";

pub mod character_bootstrap_intent;
pub mod durability;

#[cfg(test)]
#[allow(dead_code, reason = "shared native PG harness; DAILY uses a subset")]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

#[allow(unused_imports)]
use oteryn_gameplay_admission_evidence as admission_evidence;
#[allow(unused_imports)]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[allow(unused_imports)]
use oteryn_gameplay_content::content;
#[allow(unused_imports)]
use oteryn_gameplay_domain::domain;
#[allow(unused_imports)]
use oteryn_gameplay_entities as foundation;
#[allow(unused_imports)]
use oteryn_gameplay_native_admission_source as native_admission_source;

#[cfg(test)]
#[path = "durability/character_appearance/command_replay/command_replay_tests.rs"]
mod appearance_command_replay_tests;
