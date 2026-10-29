# Landing one mechanic

## What "done" means

A mechanic is **DONE** only when every element it is made of is done: rules,
durable storage, the in-game caller that reaches it, the wire command, the
client registration and tests that exercise each of those. A slice that ships
rules without a caller, or a command nobody can send, is **PARTIAL**. Say
"slice done", never "mechanic complete", until every element is done, and
count mechanics, not PRs, when reporting progress.

The progress ledger is the file `docs/agents/tasks/MECHANICS_STATUS.md` points
to in its first paragraph (on 2026-10-29: the Crystal re-audit,
`docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`, with
rows marked `DONE`, `PARTIAL`, `MISSING`, `OWNER-EXCLUDED`, `OUT-OF-SCOPE`).
Follow that pointer rather than a remembered path: the ledger has moved once
already, and the 2026-10-27 table in `MECHANICS_STATUS.md` is now a frozen
snapshot. Use the ledger's own words and definitions so reports and the table
agree, report progress from its totals line, and update the mechanic's row in
the PR that changes its state. The element list above is the checklist for
reaching the ledger's `DONE`, not a second definition.

- One mechanic = one branch = one PR. Merge it when the mechanic is DONE.
- Partial work stays on a branch without a PR. A PARTIAL slice gets a PR only
  when the owner asked for that slice; its title and parity table say PARTIAL
  and name the missing elements.
- Behaviour must match the reference (`content-world.md`, *Reference
  sources*). Record every rule in the PR's parity table with its source, and
  write `N/A` with a reason for rows that do not apply.

## Before writing code

1. Read the task card (`docs/agents/tasks/`) and the nearest `AGENTS.md` of
   every path you will touch.
2. Claim numbers against **current `main` and every open PR**, not against
   `main` alone. `scripts/next-free-numbers.py` prints the next free value of
   each kind and which open PR holds a number; it exits non-zero when two open
   PRs claim the same one. The kinds:
   - migration: above the highest number in `apps/game-server/migrations/`
     and in open PRs, so files apply in merge order. A reused number fails the
     migration-directory check in `crates/gameplay/durability/src/lib.rs`.
   - capability id and command id: the next free fragment under
     `docs/contracts/protocol-oteryn/v1/registry/{capabilities,command_types}/`
     and in open PRs.
   - offer order (`offer=<order>,...` in a `.mechanic` file): unique.
   Re-check the numbers right before you push (run the script again);
   renumber yours if `main` took them. The directory check only rejects
   duplicates, so a number below `main`'s highest still builds (#2676 merged
   `0411` when `main` was at `0445`); taking the next free number keeps the
   apply order equal to the merge order.
3. Check that the selected-capability capacity still has room
   (`SELECTED_CAPACITY` in
   `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs`) counting
   the capabilities of open PRs too. It has been raised again and again
   (32, 64, 112, 120: CAPCAP-1, CAPCAP-2); if it is full, search open PRs for
   a pending raise and depend on it instead of opening another one.
   `SELECTED_CAPACITY` must stay below `MAX_CAPABILITY_COUNT`
   (`crates/protocol-oteryn/src/lib.rs`, 128 at the time of writing); when a
   raise would reach it, stop and raise it with the owner, because the next
   step is a protocol change, not another constant bump.

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
- A test that drives a full connection, dispatch scenario or PostgreSQL case
  runs on a dedicated thread with at least a 64 MiB stack
  (`std::thread::Builder::new().stack_size(64 << 20)`), as about 80 test files
  already do. Default-stack overflows in CI were fixed one by one (the
  FLOOR-STACK-1 fix was copied into 8 PRs) and show up only on some runners.
- Tests never depend on wall-clock timing: drive timers with paused or
  virtual time, wait on an explicit readiness signal instead of a sleep, and
  treat a busy durable holder as retryable. DEFLAKE-1..4 (2026-10-29) and the
  field-runes barrier fixes (copied into 15 PRs) were all this.

## The PR

- Template: `.github/pull_request_template.md`. Fill every heading, keep the
  five validation checkboxes verbatim, and add a `## Parity table` section.
- Validation lists what actually ran, with pass counts; unchecked boxes say
  why.
- Before the first push: merge current `origin/main`, then run the local checks
  (`local-checks.md`). Add the `heavy-tests` label only when the mechanic is
  DONE (`ci-merge.md`).
- Never edit the PR body after opening it; post the tested head SHA and any
  changed validation as a comment.
- A mechanic PR that changes a row's state carries the ledger update
  (*What "done" means*).
