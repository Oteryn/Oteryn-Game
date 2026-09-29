#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for B3-2 (Combat ground pickup; definition facts bound
// to the current Content generation instead of caller-supplied `ItemDefinitionFacts`). Ordinary
// workspace runs report PRE-ROUTING/NONCANONICAL when the routed database is absent.
//
// B3-1's own PG suite (`item_transfer_postgres.rs`) already proves the D80-D83 admission/merge
// matrix inside `durability::item_transfer`; these cases exist to prove the new content-binding
// layer (`combat_pickup`) end to end, not to re-prove that matrix.
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

// Reuses the B3-1 harness (Database/register/scope/seed_character/Harness) instead of
// duplicating PostgreSQL/admission bootstrap; see that file's own header comment.
#[path = "support/item_transfer_postgres_cases.rs"]
mod item_transfer_postgres_cases;

#[path = "support/combat_pickup_postgres_cases.rs"]
mod combat_pickup_postgres_cases;
