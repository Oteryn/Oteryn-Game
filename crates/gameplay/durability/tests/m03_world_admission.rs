//! Focused M03 PostgreSQL target reuses the authoritative server admission case.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;

#[allow(
    dead_code,
    reason = "shared native fixture; this target uses only world admission"
)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[path = "../../../../apps/game-server/tests/support/authority_cases/qm03_world_admission.rs"]
mod world_admission;
