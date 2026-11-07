# Finding code without reading it all

The repository has about 2,700 Rust files; 36 are over 100 KB and the largest
(`apps/game-server/tests/durability_postgres.rs`) is about 370 KB, roughly
90k tokens if read whole. Data files under `content/`, `imports/` and
`docs/agents/evidence/` run to tens of MB. Read the lines you need, never the
file.

## How to read

1. Locate first: `rg -l '<symbol>' <dir>` to find files, then
   `rg -n '<symbol>' <file>` for the line numbers.
2. Read the range: the Read tool with `offset` and `limit` around the
   function (about 60-150 lines), not the whole file.
3. When the session has LSP tools (the `rust-analyzer-lsp` plugin; they
   appear in the tool list), use go-to-definition,
   find-references and hover instead of grepping for call sites.
4. A question that needs many files ("where is X used", "which crates touch
   Y") goes to one `Explore` subagent; it returns the answer, and the file
   contents stay out of this context.
5. A file already read in this session is not read again unless it changed.
6. Build and test output goes to a file; read its tail and the `error` /
   `FAILED` / `panicked` lines.
7. The crate list is the `members` array in the root `Cargo.toml`
   (`cargo metadata --no-deps --format-version 1` for paths and names); the
   top-level layout is `docs/agents/REPOSITORY_MAP.md`. This file does not
   repeat them.

## Where things live

| looking for | start here |
|---|---|
| a wire command by id `N` | `docs/contracts/protocol-oteryn/v1/registry/command_types/<NNNN>-*.json`; its fragment: `rg -l '^commands=(.*,)?N(,\|$)' crates/gameplay/dispatch/mechanics/` |
| how a mechanic is wired | its `crates/gameplay/dispatch/mechanics/<ordinal>-<name>.mechanic` (`name=`, `transport=`, `[route]`); the route in `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/<name>.rs`; the body in `crates/gameplay/dispatch/src/gameplay_transport/<name>*.rs` |
| a capability by id `N` | `crates/protocol-oteryn/capabilities/<NNNN>-*-v1.json`, the wire types in `crates/protocol-oteryn/src/<mechanic>.rs` (constant `CAPABILITY_<NAME>_V1`), the client folder `crates/client-ui/src/features/<NNNN>-<slug>/` |
| the selected-capability limit | `SELECTED_CAPACITY` in `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`; protocol ceiling `MAX_CAPABILITY_COUNT` in `crates/protocol-oteryn/src/lib.rs` |
| who owns a table | `rg -l 'CREATE TABLE <table>' apps/game-server/migrations/`, then `rg -l '<table>' crates/gameplay/durability/src` for the durable owner |
| durable rules (pure) vs durable writes | rules in `crates/gameplay/{domain,combat-rules,spell-rules,encounter-rules,social-rules,economy}`; SQL writes in `crates/gameplay/durability/src/durability/` |
| server startup, schedulers, config | `apps/game-server/src/node/` (`serve.rs`, `config.rs`, `*_scheduler.rs`) |
| content loading and generators | `crates/gameplay/content/`; generators and checks in `tools/content-migration/` (`regenerate_content.py`) and `tools/world-bundle-compiler/` |
| tests for a function | `rg -l '<fn_name>' crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/ apps/game-server/src/lifted_tests/auto/ apps/game-server/tests/` |
| rules for a directory | the nearest `AGENTS.md` (root, `apps/game-server/`, `crates/simulation-determinism/`, `docs/agents/`) and the directory's `README.md` (e.g. `crates/gameplay/dispatch/mechanics/`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/`, `crates/protocol-oteryn/capabilities/`) |
| a mechanic's status | the ledger `docs/agents/tasks/MECHANICS_STATUS.md` points to; find the row with `rg -n '<mechanic name>'`, not by reading the table |
| a reference rule | `docs/reference/` and the source registry (`content-world.md`, *Reference sources*) |

## Large data files

JSON under `content/`, `imports/` and `docs/agents/evidence/` is often one
multi-megabyte line. The rules are `docs/agents/CONTEXT_ROUTING.md`, *Large
documents and command output*; in short: find it with `rg -l`, extract the
record with `jq` or a few lines of Python, and print only the fields you need.
