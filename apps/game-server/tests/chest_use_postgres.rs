#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for D39 (chest `USE` wiring to the CHEST-1 reward-claim
// MINT). Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when the routed database is
// absent. CHEST-1's own suite (`reward_claim_mint_postgres.rs`) proves the D40-D42/D92 MINT
// matrix; these cases prove the D39 layer (`interaction_chest_use`) end to end.
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
#[path = "../src/combat.rs"]
pub mod combat;
// A top-level module, not `combat::pickup`: see `src/combat/pickup.rs`'s own header comment for
// why (`combat.rs` is recompiled standalone in other, Content-free contexts).
#[allow(dead_code, unused_imports)]
#[path = "../src/combat/pickup.rs"]
pub mod combat_pickup;
// `support/content_shim.rs` mirrors `src/content/mod.rs`'s own wiring (see its header comment
// for why it is not a plain `#[path = "../src/content/mod.rs"]` inclusion).
#[allow(dead_code, unused_imports)]
#[path = "support/content_shim.rs"]
pub mod content;
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

#[allow(dead_code, unused_imports)]
#[path = "../src/achievement_catalogue.rs"]
pub mod achievement_catalogue;
#[allow(dead_code, unused_imports)]
#[path = "../src/interaction/mod.rs"]
pub mod interaction;
#[allow(dead_code, unused_imports)]
#[path = "../src/interaction/chest_use.rs"]
pub mod interaction_chest_use;

// Reuses the B3-1 harness (Database/register/scope/seed_character/Harness).
#[allow(dead_code)]
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;

#[path = "support/chest_use_postgres_cases.rs"]
mod chest_use_postgres_cases;
