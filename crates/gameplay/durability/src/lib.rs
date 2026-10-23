//! ADR-0019 gameplay crate `oteryn-gameplay-durability`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

extern crate self as oteryn_gameplay_durability;

/// The game-server migrations, embedded once here so files that other crates
/// include by `#[path]` name one migrator whichever crate compiles them.
pub static GAME_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../../../apps/game-server/migrations");

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
#[path = "../../../../apps/game-server/src/lifted_tests/auto/bed_sleep_postgres_cases.rs"]
mod bed_sleep_postgres_cases;
