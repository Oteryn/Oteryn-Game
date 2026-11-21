//! Dispatch harness selected explicitly by the parallel PR archives.

// Keep ordinary test and Clippy invocations on the library's harness.
#![cfg(all(test, oteryn_dispatch_test_archive = "dedicated"))]

// Keep these declarations in sync with lib.rs; including them through a macro changes name lookup.
pub mod gameplay_transport;
pub mod observability;

include!("dispatch.rs");
