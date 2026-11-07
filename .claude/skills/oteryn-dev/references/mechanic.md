# Landing one mechanic

## What "done" means

The progress ledger is the file `docs/agents/tasks/MECHANICS_STATUS.md` points
to in its first paragraph (on 2026-10-29: the Crystal re-audit,
`docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`, rows
`DONE`, `PARTIAL`, `MISSING`, `OWNER-EXCLUDED`, `OUT-OF-SCOPE`). Follow the
pointer, not a remembered path: the ledger has moved before. Its definitions
are the only ones: a row is `DONE` when a production path and relevant tests
exist for the bounded slice the row names, with no missing connection; it
does not require a native-client UI. Anything less is `PARTIAL`: say "slice
done", count mechanics rather than PRs, and report from the ledger's totals
line.

To reach `DONE`, check that the row's slice has each element it needs: rules,
durable storage, the in-game caller that reaches it, the wire command, and
tests that exercise them. Client registration belongs to the slice only when
the row names client behaviour.

Mechanic PRs do not edit the ledger, because every one of them would conflict
on it: after each wave one docs PR updates the wave's rows with donor and
Oteryn file:line evidence (owner decision, 2026-10-29). A keyword grep is not
evidence.

- One mechanic = one branch = one PR, merged when the mechanic is DONE.
- Exception (owner decision, 2026-10-29): a mechanic that needs more than one
  migration, or server and client together, splits into `a` then `b`. Slice
  `a` merges as a named PARTIAL (title and parity table say PARTIAL and name
  what `b` adds); `b` starts from `main` after `a` merges, never stacked.
- Any other partial work stays on a branch without a PR, unless the owner
  asked for that slice.
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
   the capabilities of open PRs too. It has been raised several times
   (32, 64, 112: CAPCAP-1, CAPCAP-2; a raise to 120, #2685, closed unmerged);
   if it is full, search open PRs for
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

- One new file `apps/game-server/migrations/<NNNN>_<name>.sql`.
- Every new table:
  ```sql
  REVOKE ALL ON <table> FROM PUBLIC;
  GRANT SELECT, INSERT, UPDATE ON <table> TO oteryn_game_runtime;   -- DELETE only if the owner deletes rows
  GRANT SELECT ON <table> TO oteryn_game_control;
  ```
- How to write the SQL itself (guard chains, NULL-safe comparisons, locks,
  indexes): `oteryn-coding`, *PostgreSQL and migrations*.

## Tests without shared-file edits

Put new test files in the auto-mounted directories (root `AGENTS.md`,
*Landing a mechanic without shared-file conflicts*):

- `apps/game-server/tests/support/authority_cases/` (PostgreSQL authority cases),
- `apps/game-server/src/lifted_tests/auto/` (game-server lib tests),
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/` (dispatch).

Do not add a new `tests/*_postgres.rs` target. A PostgreSQL case returns
`Ok(())` when `OTERYN_TEST_POSTGRES_ADMIN_URL` is absent.

Record golden count changes explicitly in the PR.

## Code and test rules

How to write the code and its tests (scope, Rust, async, PostgreSQL, stack
size, timing) is the `oteryn-coding` skill; load it before writing code.

## The PR

- Template: `.github/pull_request_template.md`. Fill every heading, keep the
  five validation checkboxes verbatim, and add a `## Parity table` section.
- Validation lists what actually ran, with pass counts; unchecked boxes say
  why.
- Before the first push run the local checks (`local-checks.md`); merge
  `origin/main` only as *Before every push* in `ci-merge.md` says. Add the
  `heavy-tests` label only when the mechanic is DONE (`ci-merge.md`).
- Never edit the PR body after opening it; post the tested head SHA and any
  changed validation as a comment.
- The PR description names the ledger row it moves and to which state, so the
  wave's docs PR can collect it (*What "done" means*).
