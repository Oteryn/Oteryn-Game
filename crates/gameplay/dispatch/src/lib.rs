//! ADR-0019 gameplay crate `oteryn-gameplay-dispatch`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

// The common CI archive uses the original Cargo selection, but its Dispatch
// test harness is empty. Non-test builds still compile the library normally.
#![cfg(not(all(test, oteryn_dispatch_test_archive = "common")))]

// Keep these declarations in sync with ci_unit_archive.rs; including them through a macro changes name lookup.
pub mod gameplay_transport;
pub mod observability;

include!("dispatch.rs");
