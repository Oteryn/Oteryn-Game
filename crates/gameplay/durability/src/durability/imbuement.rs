//! ITEM-IMBUE-1 storage preparation, owned here under ADR-0022.
//!
//! The installed migration is the single schema definition; no private parallel
//! table or alternate item owner. Runtime access is read-only until Apply/Clear
//! can compose the shared item-only fee owner, and checkpoint/expiry have real
//! runtime callers with admitted elapsed provenance and native audit support.
//! A retained zero row is occupied state, never permission for bare deletion.

/// Installed by the normal GAME_MIGRATOR; this module does not execute DDL.
pub const IMBUEMENT_MIGRATION_VERSION: i64 = 150;
