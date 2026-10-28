---
name: oteryn-mechanic
description: Build and land one Tibia game mechanic in Oteryn-Game end to end - definition of done, allocating migration, capability and command numbers, the exact file set for a new capability, PostgreSQL migrations and grants, conflict-free registration, and the parity-table PR. Use when implementing, finishing or reviewing a mechanic or a slice of one.
---

# Landing one mechanic

## What "done" means

A mechanic is **DONE** only when every element it is made of is done: rules,
durable storage, the in-game caller that reaches it, the wire command, the
client registration and tests that exercise each of those. A slice that ships
rules without a caller, or a command nobody can send, is **PARTIAL**. Say
"slice done", never "mechanic complete", until every element is done, and
count mechanics, not PRs, when reporting progress.

- One mechanic = one branch = one PR. Merge it when the mechanic is DONE.
- Partial work stays on a branch without a PR, or goes into a PR whose title
  and parity table say PARTIAL and name the missing elements.
- Behaviour must match the reference (current Tibia, checked against
  tibia.com and the pinned Canary/Crystal sources; `docs/reference/`). Record
  every rule in the PR's parity table with its source, and write `N/A` with a
  reason for rows that do not apply.

## Before writing code

1. Read the task card (`docs/agents/tasks/`) and the nearest `AGENTS.md` of
   every path you will touch.
2. Claim numbers against **current `main` and every open PR**, not against
   `main` alone:
   - migration: above the highest number in `apps/game-server/migrations/`
     and in open PRs. A migration that sorts before one already merged fails
     every PostgreSQL partition.
   - capability id and command id: the next free fragment under
     `docs/contracts/protocol-oteryn/v1/registry/{capabilities,command_types}/`
     and in open PRs.
   - offer order (`offer=<order>,...` in a `.mechanic` file): unique.
   Re-check the numbers right before you push; renumber yours if `main` took
   them.
3. Check that the selected-capability capacity still has room
   (`SELECTED_CAPACITY` in
   `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`).

## File set for a new capability

The arena slice (#2642) is the reference shape; a new capability with one
command touches these and nothing shared:

| File | Purpose |
|---|---|
| `crates/protocol-oteryn/src/<mechanic>.rs` + one `mod` line in `lib.rs` | wire types, `CAPABILITY_<NAME>_V1` constant |
| `crates/protocol-oteryn/capabilities/<id:04>-<slug>-v1.json` | capability definition (`offers`) |
| `crates/protocol-oteryn/src/tests/capabilities_golden/<id:04>-<slug>-v1.json` | golden copy of the definition |
| `docs/contracts/protocol-oteryn/v1/registry/capabilities/<id:04>-<slug>-v1.json` | registry entry (`offered`, `offer_gate`) |
| `docs/contracts/protocol-oteryn/v1/registry/command_types/<cmd:04>-<slug>.json` | one file per command |
| `docs/contracts/protocol-oteryn/v1/<slug>-v1.md` | contract text the schema refs point at |
| `crates/gameplay/dispatch/mechanics/<ordinal>-<name>.mechanic` | registration fragment (see that directory's `README.md`) |
| `crates/gameplay/dispatch/src/gameplay_transport/<name>.rs` | command body |
| `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/<name>.rs` | route function |
| `crates/client-ui/src/features/<id:04>-<slug>/registrations/handlers.json` and `fixtures/registrations.json` | client registration (folder = capability slug without `_V1`) |
| `tools/architecture-check/client-unsupported-baseline.d/<id:04>-<slug>.json` | `{"mechanic": "<id:04>-<slug>"}` while the client is unsupported |

- **Bundle-world only** (the usual first step): `offers` is
  `{"production": false, "item_view": false, "bundle_world": true}`, the
  registry entry has `"offered": false`, and its `offer_gate` text names
  `BUNDLE_WORLD_OFFERED_CAPABILITIES` (a test checks for that string).
- Offering the capability in production is a separate PR with its own golden
  and client handler change.
- JSON fragments: 2-space indent, one array element per line, trailing
  newline. Run `python3 tools/contracts/registry_fragments.py` after adding
  any.

## Migrations

- One new file `apps/game-server/migrations/<NNNN>_<name>.sql`. Never edit a
  merged migration; fix forward with a new one.
- Every new table:
  ```sql
  REVOKE ALL ON <table> FROM PUBLIC;
  GRANT SELECT, INSERT, UPDATE ON <table> TO oteryn_game_runtime;   -- DELETE only if the owner deletes rows
  GRANT SELECT ON <table> TO oteryn_game_control;
  ```
- Extend an existing guard chain function instead of replacing it; keep the
  migration re-runnable and use NULL-safe comparisons (`IS DISTINCT FROM`).

## Tests without shared-file edits

Put new test files in the auto-mounted directories (root `AGENTS.md`,
*Landing a mechanic without shared-file conflicts*):

- `apps/game-server/tests/support/authority_cases/` (PostgreSQL authority cases),
- `apps/game-server/src/lifted_tests/auto/` (game-server lib tests),
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/` (dispatch).

Do not add a new `tests/*_postgres.rs` target. A PostgreSQL case returns
`Ok(())` when `OTERYN_TEST_POSTGRES_ADMIN_URL` is absent.

A test must exercise the change: it fails on the tree before your change and
reaches the code path you added (a PostgreSQL test that never calls the new
query proves nothing). Record golden count changes explicitly; never loosen
an assertion to fit.

## Code rules that CI and review keep catching

- Read actor state only through its owner under the runtime lock, and re-prove
  the exact actor and session generation before writing (`session-generation`
  fencing).
- Best-effort enrichment (boosts, statistics, cosmetics) never fails movement
  or combat; fail closed only on authority.
- A refused path rolls back what it started (inventory full, a pending
  exchange cancelled).
- No `expect`/`unwrap` in production code. An unwired rules slice carries the
  `dead_code` allowance with a reason, and its PR says PARTIAL.
- Insert registrations in sorted position, never at the end of a file.

## The PR

- Template: `.github/pull_request_template.md`. Fill every heading, keep the
  five validation checkboxes verbatim, and add a `## Parity table` section.
- Validation lists what actually ran, with pass counts; unchecked boxes say
  why.
- Before the first push: merge current `origin/main`, then run the local checks
  (`oteryn-build-test`). Add the `heavy-tests` label only when the mechanic is
  DONE (`steward`).
