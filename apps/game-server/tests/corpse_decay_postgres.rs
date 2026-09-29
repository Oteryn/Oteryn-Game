#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for D3-6 (DUR-03 DECAY_RETIRE of a
// decayed corpse: N+1 one-item steps at materialized_at + 60 s, resumable from
// durable state). Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when
// the routed database is absent.
extern crate self as oteryn_game_server;

#[allow(dead_code, unused_imports)]
#[path = "../src/admission_evidence.rs"]
pub mod admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/domain/mod.rs"]
pub mod domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

// Reuses the B3-1 harness and the D3-4 corpse fixtures (real corpse MINT,
// forged loot entries, database-clock time travel) instead of duplicating
// PostgreSQL/admission bootstrap.
#[path = "support/corpse_transfer_postgres_cases.rs"]
mod corpse_transfer_postgres_cases;
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;

// Standalone target for local focused runs; the protected PostgreSQL lane also
// includes these exact cases through character_authority_postgres.
#[path = "support/corpse_decay_postgres_cases.rs"]
mod corpse_decay_postgres_cases;
