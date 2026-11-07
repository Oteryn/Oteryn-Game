---
name: oteryn-coding
description: Writing Rust, SQL and test code in Oteryn-Game - scope discipline, a red-then-green check per task, Rust rules beyond the workspace lints, runtime-lock and async rules, PostgreSQL queries and migrations, test stack size and waiting. Load before writing or changing code here, including delegated work.
---

# Writing code in Oteryn-Game

Mechanic layout, numbers and file sets are in `oteryn-dev`
(`references/mechanic.md`); this skill is how to write the code itself.

## Scope: surgical and minimal

- Every changed line traces to the task. Adjacent code, comments and
  formatting stay as they are; new code follows the surrounding style.
  Unrelated dead code is reported in the PR and left in place; code your own
  change orphaned is removed.
- Build what the task asks and nothing beside it: no extra feature, option,
  abstraction or error branch. Before pushing, shrink a long draft to the
  shortest version that stays clear.
- When the task reads two ways, pick the reading that fits the task card and
  the parity table, state it in one line in the PR or report, and carry on.
  The owner prefers a stated assumption to a question; stop only for a choice
  that cannot be undone.

## Every task has a check that goes red, then green

Before production code, write or name the check that proves the task:

- a fix: a test that reproduces the bug on the current tree;
- a rule or mechanic slice: a test that drives the real caller (dispatch,
  route or durable owner), not only a pure function;
- a refactor: the tests that pass before and after, unchanged.

Run it red first and keep the output. A test that passes on the tree before
your change, or never reaches the new code path (a PostgreSQL test that never
calls the new query), proves nothing. For several steps, write
`1. <step> -> check: <command>` so you can loop without asking.

## Rust in this workspace

The workspace lints already deny `unwrap`, `expect`, `panic!`, `todo!`,
`unimplemented!`, `dbg!` and `unsafe` (root `Cargo.toml`), so fallible paths
return `Result` and propagate with `?`.

- Every new `#[allow(...)]` sits on the narrowest item and carries a comment
  saying why; an unwired rules slice carries `dead_code` with a reason and its
  PR says PARTIAL.
- Borrow before cloning: `&str` / `&[T]` parameters, no `.clone()` in loops,
  no intermediate `.collect()` that is only iterated again.
- A `TODO` names its issue or task id.

## Runtime, sessions and async

- Read actor state only through its owner under the runtime lock, and
  re-prove the exact actor and session generation before writing
  (session-generation fencing).
- A `std::sync::Mutex` / `RwLock` guard, or the runtime lock, is dropped
  before the next `.await`: finish the critical section, then await.
- Best-effort enrichment (boosts, statistics, cosmetics) never fails movement
  or combat; fail closed only on authority.
- A refused path rolls back what it started (inventory full, a pending
  exchange cancelled), and every end path (logout, kick, death, shutdown)
  closes or hands off the durable state the session opened.
- Registrations go in sorted position, not at the end of a file.

## PostgreSQL and migrations

The new-table template (one migration file, `REVOKE ALL ... FROM PUBLIC`, the
two role grants) is in `oteryn-dev` `references/mechanic.md`, *Migrations*.
Beyond it:

- A foreign-key column that is not the leading column of an existing index
  gets its own index; `ON DELETE CASCADE` and joins scan the table otherwise.
- Keep transactions short: no network calls, waits or content loading inside
  one. Lock the rows you will change (`FOR UPDATE`) in a stable order (by key)
  before mutating dependent rows, so two writers cannot deadlock.
- Work queues take rows with `FOR UPDATE SKIP LOCKED`; a cap (slots, stacks,
  offers) is counted and enforced in the same statement or transaction that
  writes the row.
- Every read over players, worlds or history has a `LIMIT`, a page or a
  proven small bound; a paged query and its count twin both match an index.
- Compare nullable values with `IS DISTINCT FROM`; aggregates over an empty
  set return `NULL`, so handle it (`COALESCE` or an explicit branch).
- sqlx runs each migration in one transaction unless the file opts out with
  `-- no-transaction`; no migration here does, so `CREATE INDEX CONCURRENTLY`
  is not used. Keep migrations small, re-runnable and forward-only.
- Extend an existing guard chain function rather than replacing it.

## Tests

- New test files go in the auto-mounted directories listed in `oteryn-dev`
  `references/mechanic.md`; name each test after the behaviour it proves.
- A test that drives a full connection, dispatch scenario or PostgreSQL case
  runs on its own thread with at least a 64 MiB stack
  (`std::thread::Builder::new().stack_size(64 << 20)`; the scenario tests
  here use 64 or 256 MiB); default-stack overflows appear only on some
  runners.
- Tests wait for the condition they care about, never for a guessed duration:
  a channel, `Notify` or readiness handle, or a poll with a deadline; drive
  timers with paused time (`tokio::time::pause` / `start_paused`).
- A busy durable holder is retryable: retry it with a bound instead of
  failing (DEFLAKE-2..4 were missing retries; DEFLAKE-1 and the field-runes
  barrier fixes were a fixed sleep or a missing readiness wait).
- Assertions stay as strict as before; a changed golden count is recorded in
  the PR, never loosened to fit.

## Sources

Adapted, not copied verbatim: Andrej Karpathy's coding guidelines as written up
in forrestchang/andrej-karpathy-skills (MIT); apollographql/skills
`rust-best-practices` (MIT); supabase/agent-skills
`supabase-postgres-best-practices` (MIT); the condition-based-waiting note in
obra/superpowers `systematic-debugging` (MIT); Oteryn's own
`docs/agents/review/CODEX-CHECKLIST.md` and CI history.
