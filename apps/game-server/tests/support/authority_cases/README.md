# Auto-mounted `character_authority_postgres` cases

`apps/game-server/build.rs` mounts every `*.rs` file in this directory into the
`character_authority_postgres` target (`include!` at the end of
`tests/character_authority_postgres.rs`), sorted by name, as `mod <file stem>;`.

A new mechanic's PostgreSQL cases go here as one new file. Do not add a
`#[path] mod` line to `character_authority_postgres.rs`: that shared list was the
most frequent merge conflict between parallel mechanics.

- The file stem must be a snake_case module name.
- The module is a direct child of the test crate root, so `super::` reaches the
  harnesses mounted there (`fixture`, `item_transfer_postgres_cases`, ...).
- A case that needs routed PostgreSQL returns `Ok(())` when no admin URL is
  configured; the integration shards compile and run this target without PostgreSQL.
