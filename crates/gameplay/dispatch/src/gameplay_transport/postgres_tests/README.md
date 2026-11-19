# Auto-mounted dispatch PostgreSQL tests

`crates/gameplay/dispatch/build.rs` mounts every `*.rs` file in this directory into
`gameplay_transport::monk_save::postgres_tests` (`include!` in
`monk_save_postgres_tests.rs`), sorted by name, as `mod <file stem>;`.

A new mechanic's live-dispatch PostgreSQL test goes here as one new file instead of
another `#[path] mod` line in `monk_save_postgres_tests.rs`, the shared list every
mechanic used to edit. The module is a sibling of `fixture`, `fresh_fixture` and the
other explicit mounts, so `super::fixture` keeps resolving. A test that needs routed
PostgreSQL returns early when no admin URL is configured.

Put new shared fixtures in test-free modules under `support/` so every shard can
use them. Keep cross-entry dependencies in support or the parent fixtures.

`OTERYN_DISPATCH_PG_SHARD=k/n` selects one group of entries by source size.
The explicit PostgreSQL tests run in shard 1. Leaving the variable unset mounts
the complete suite. Run `python3 tools/repository/check_dispatch_pg_shards.py 3`
from the repository root to check coverage and measure the mounted sources.
