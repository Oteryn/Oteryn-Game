//! Test-only access to the established sequencer request and PVP-1a root fixture.
use super::id;
#[path = "pvp_character_fixture.rs"]
mod character;
pub(crate) use character::seed_root;

#[path = "pvp_ledger_fixture.rs"]
mod ledger;
pub(crate) use ledger::insert as seed_ledger;
