// Local recompiled `content` module for B3-2's PG test binary (`combat_pickup_postgres.rs`),
// which locally recompiles `combat` (a private `mod combat;` in `lib.rs`, only reachable this
// way) and therefore needs `crate::content::...` to resolve the same way inside that local
// copy.
//
// This is deliberately a narrow slice of `src/content/mod.rs`'s own wiring, not a full
// `#[path = "../src/content/mod.rs"]` inclusion: `combat::pickup` and this binary's own test
// cases only ever name types from `digest`/`model`/`production`/`reference_playable` (checked:
// their own `use super::...`/`use crate::...` lines name only each other and
// `crate::foundation`). `production` is `support/production_lite.rs`, not the real
// `content/production.rs`: the real file's non-test code unconditionally needs
// `reference_artifact`, whose own `#[cfg(test)]` modules in turn need `project`/
// `cw2_b1_import`/`static_cell_engine` — none of it related to B3-2, and all of it dead code
// this binary would otherwise be forced to compile. `production_lite.rs`'s header explains
// exactly what it keeps.
//
// Every other `content` submodule is dropped entirely, which also avoids
// `encounter_map_item.rs`'s `#[cfg(test)]` module (needs `world_runtime`/`world_object_revert`/
// `interaction`, unrelated to B3-2).

#[path = "../../src/content/digest.rs"]
pub mod digest;
#[path = "../../src/content/model.rs"]
pub mod model;
#[path = "production_lite.rs"]
pub mod production;
#[path = "../../src/content/reference_playable.rs"]
mod reference_playable;

pub use model::*;
pub use production::{
    ContentLockBinding, ContentLockEntry, PackageManifestBinding, ProductionAtom, ProductionKey,
    Sha256HexDigest,
};
pub use reference_playable::*;
