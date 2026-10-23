# DAILY-1 — daily reward (consolidated task card)

```yaml
task_id: DAILY-1
title: "DAILY-1: daily reward"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row DAILY-1 (row 47); §7 'DAILY-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md
base_branch: main
branch: w5/daily-1
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Current lane checkpoint — 2026-10-22, run 1c

**Incomplete DAILY implementation; bounded storage/rules/claim-core slice validated.**
This checkpoint supersedes run 1b's statements that migration 0140 and DAILY PG
cases do not exist. The capability remains unoffered, and no acceptance box for
the complete mechanic is being closed. Local HEAD remains
`9e5770362a96b2e0e269abf1480f7de3a17163fe`; no candidate freeze or PR.

Changed in this run (run 1b's protocol/registry/gating edits are preserved):

- `apps/game-server/migrations/0140_daily_reward.sql`: Character+World state
  (next lane, bounded streak, last claim epoch and Berlin reset day), immutable
  epoch-keyed claim receipts and World-owned committed reset history. Includes
  Character-root RESTRICT references, full-u64 positive epochs, exact 32-byte
  binding, unique TransactionId, day uniqueness, indexed latest/exact lookups,
  immutable-history triggers and deferred final projection checks. Runtime can
  read reset history, never publish it; state updates have column-only grants.
- `crates/gameplay/progression/src/daily_reward.rs` and
  `daily_reward_tests.rs`, plus registration in `src/lib.rs`: first/consecutive
  streaks, 7→1 lane wrap, ceiling 100,000, all-or-nothing gap coverage and checked
  full-u64 arithmetic. The pure reset-day rule consumes already-resolved Berlin
  civil time; PostgreSQL's upstream timezone database resolves Europe/Berlin.
  Neither clock time nor session/restart changes manufacture an epoch.
- `crates/gameplay/durability/src/durability/daily_reward.rs`,
  `daily_reward_claim.rs`, `daily_reward_tests.rs`, plus wiring-only changes in
  `durability/mod.rs` and `src/lib.rs`: native Root semantic transaction/deadline,
  recovery and admission locks, existing `character_item_fence_is_current`,
  Character-root serialization and World-history publication lock. Fresh claims
  atomically write receipt and projection; duplicates return the original receipt
  without writes, changed bindings refuse, stale fences refuse even on replay,
  and unavailable/older/future windows refuse. Missing epochs reset the streak
  to 1 while advancing the lane only once. Until its native ledger is composed,
  this **uncomposed core uses zero jokers and the Free lane**. It is not the
  complete reward claim transaction and must not be enabled as one.

### Validation — run 1c

Every Cargo command used `/home/mole/oteryn-cache/codex-bin/cargo` and `--locked`.
Only filtered DAILY PostgreSQL cases ran; no whole PG suite ran.

```bash
cargo test --locked -p oteryn-gameplay-progression --lib daily_reward -- --nocapture
cargo test --locked -p oteryn-gameplay-durability --lib daily_reward -- --nocapture --test-threads=1
cargo test --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-dispatch --lib daily_reward -- --nocapture
cargo clippy --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability -p oteryn-protocol-oteryn -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo clippy --locked -p oteryn-gameplay-durability --all-targets --quiet -- -D warnings
cargo fmt --all -- --check
```

Results: **4 progression tests, 5 durability PG tests, 1 protocol test and 2
real-dispatch gating tests passed**. Both strict clippy commands and final fmt
check passed. The second clippy run covers the final durability test edits.
The existing vendored Tokio dependency still emits its missing-doc warning.
The 5 PG cases are mounted once from the durability library, using the existing
native Bestiary harness and installed SQLx migrations, with no new PG target.
Each emitted an actual isolated-database start/version marker and cleanup marker:

| Test | Database marker (`server_version_num=170006` for every row) |
|---|---|
| `daily_reward_postgres_claim_replay_gap_and_restart` | `bp_daily_claim_1792690948501260310` |
| `daily_reward_postgres_concurrent_claim_serializes` | `bp_daily_race_1792691008457095927` |
| `daily_reward_postgres_insert_failure_rolls_back_state` | `bp_daily_rollback_1792691012270959070` |
| `daily_reward_postgres_refusals_write_nothing` | `bp_daily_refusals_1792691015874185594` |
| `daily_reward_postgres_schema_constraints_privileges_and_dst` | `bp_daily_schema_1792691019694604702` |

Coverage includes restart/reload receipts, duplicate binding mismatch, persisted
session-generation replacement, two independent native Root writers, rollback
after the state update when receipt insertion fails, schema/role refusals,
full-u64 epochs and Berlin's spring/autumn DST boundaries including fractional
seconds immediately before 10:00. Initial validation caught and repaired a
PL/pgSQL CASE parse error and a concurrency fixture that used one Root; the final
five-case run passed without skips. No unrelated contention failure remains.

Re-read the whole existing diff and all new files against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; workspace `rg` sweeps of `apps/`,
`crates/`, `tools/` covered every new DAILY type, constructor, match and impl.
No existing enum or trait changed. New rule functions have the claim-core caller;
the public claim seam's production composition and positive dispatch E2E remain
explicitly unfinished below. No test was deleted, weakened or ignored. Shared
parents receive wiring only; no existing large module gained mechanic logic.
Governance validation, all 124 agent-tool tests and `git diff --check` passed;
documentation checks are repeated after this last task-record edit.

### Unfinished / next slice

- Native World-reset publication and its runtime/recovery composition: history
  storage exists, but only migration-owner PG fixtures publish rows in this run.
  No daily-local epoch source or runtime reset-publication privilege was added.
- Account+World joker state/immutable ledger and lazy committed-month grants;
  full source-owned canonical picks/output binding and shrine/content checks;
  complete ordered reward lines, actual Inbox MINT, item bindings and §6 refusals;
  pending wildcard/XP delivery and Premium activation; content/regeneration and
  DAILY codecs/wall/handler composition.
- `commit_daily_reward_claim` has **no production caller or positive real-dispatch
  claim E2E yet**. The existing dispatch tests prove reservation refusal only.
  Owner clarification was requested about retaining this bounded uncomposed API
  under run 1c's scope versus the general every-public-function composition rule;
  no exception has been assumed or approved in this session.
- Independent exact-candidate review and CI remain for the control plane's later
  candidate. No publication, review trigger, readiness or full-mechanic completion
  is claimed. No forbidden Git operation, credential change, workflow/repository
  routing edit, new PG target or external mutation occurred.

## Prior lane checkpoint — 2026-10-22, run 1b after CP allocation

**Incomplete implementation.** This checkpoint supersedes the pre-allocation audit
below. The CP decision resolves the wire-number question; no further allocation
question or new authority blocker is being raised. The authorized end-to-end
implementation has not been completed. All acceptance boxes remain unchecked.

Implemented in the working tree:

- Registered capability **32 DAILY_REWARD_V1**, command **32 DAILY_REWARD_CLAIM**,
  command **33 DAILY_REWARD_QUERY** (the anywhere/view-only wall), and state domain
  **24 DAILY_REWARD**. No other lane's IDs or existing registry entries changed.
- Added the protocol constants, module registration, capability ownership table
  entry and reserved protobuf layouts. **No DAILY codecs or handler are composed.**
- Kept capability 32 unoffered. Added child tests using the existing negotiation
  fixture and real `serve_admitted` dispatch: supporting 32 does not select it,
  both commands reject with empty payloads, and domain 24 is absent from snapshots.
- Replaced the Item Look acceptance test's now-allocated unknown ID 32 with
  `u32::MAX`, preserving both unknown-capability rejection assertions; the DAILY
  registry test independently checks that acceptance recognizes registered 32.

Remaining authorized work: the native World-reset/month-history prerequisite;
L3 lane/streak/joker rules; authored shrine, pick, training and scroll content with
regeneration; **0140_daily_reward.sql** (reserved, not created); reward state,
immutable joker ledger, claim and complete ordered line storage; frozen request
binding and native fenced claim transaction; actual Inbox MINT and atomic item
bindings; all §6 binding refusals and split/merge preservation; wildcard/XP pending
lines; DAILY codecs, wall/USE/claim composition and positive real-dispatch DAILY PG
E2E. No daily-local epoch, parallel wildcard balance or Premium activation was
introduced. The missing sinks retain the accepted pending-line/Free-lane design.

### Validation of the reservation change

Every Cargo command used `/home/mole/oteryn-cache/codex-bin/cargo`:

- `test --locked -p oteryn-protocol-oteryn --quiet`: **286 passed**; includes
  `daily_reward_registry_uses_only_cp_allocated_ids` and the preserved unknown-ID
  rejection test. The exact DAILY-only filter is `daily_reward`.
- `test --locked -p oteryn-gameplay-dispatch --lib --quiet capabilities`:
  **62 passed**, including both new `daily_reward` child cases. The narrower
  `daily_reward` filter also ran and passed both cases.
- `test --locked -p oteryn-game-server --test character_authority_postgres --quiet native_inbox_ -- --test-threads=1 --nocapture`:
  **14 passed, 0 failed, 0 ignored**, with real body and cleanup markers. Example:
  `MAIL-NATIVE-PG database=bp_mail_mint_twenty_1792683398725587335 server_version_num=170006`.
  This qualifies the existing MAIL/native Inbox prerequisite, **not DAILY claims**.
  No DAILY PG target or migration exists yet; earlier zero-match runs are not
  counted as qualification.
- `clippy --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`:
  passed. The existing vendored Tokio dependency emitted its missing-doc warning;
  neither changed package failed strict clippy.
- `fmt --all -- --check`: passed after applying the formatter's requested layout.
- Governance validator, all **124** agent-tool unit tests and `git diff --check`
  passed; documentation checks are repeated after this record edit.

Re-read the complete tracked diff and new files against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Workspace `rg` sweeps covered `apps/`,
`crates/` and `tools/`, including the former unknown-ID assertions. The final diff
adds no Rust enum variant, struct field, trait method or public function; there
are no new constructor/match/impl consumers to update. Existing shared parents
receive registration only; no file over 1000 lines grew by more than 150 lines.
Existing tests and fixtures remain in place. No Git index, commit, push, reset,
checkout, stash, credential or external mutation occurred. The previous audit was
preserved below, and this task record was the last file edited.

Next: implement the already-authorized prerequisite and claim/content/binding
bundle on the native owners, then add and run the positive DAILY PG/dispatch
cases before offering capability 32. No candidate freeze, PR or completion claim.

## Prior prerequisite audit — before the CP wire allocation

Baseline: `9e5770362a96b2e0e269abf1480f7de3a17163fe` (the allocated
worktree HEAD); no pre-existing working-tree changes. Local file edits only:
no Git index, commit, branch, push, external write or publication operation.
Migration `0140_daily_reward.sql` is reserved for this lane by the task packet.

### Module layout planned before implementation

- L3 progression: `daily_reward.rs` with `daily_reward_tests.rs` for lane,
  streak and all-or-nothing joker arithmetic.
- L1 content: a dedicated daily reward module and child tests under the existing
  content owner, with authored shrine/pick/training/scroll inputs and native
  regeneration; reuse the admitted item-definition and interaction owners.
- L4 durability: separate daily reward storage, claim, joker-grant and item-binding
  modules with child PG test files; migration 0140 owns the inseparable schema.
  Existing Inbox mint and fence owners remain authoritative.
- Missing reset-history prerequisite: a dedicated World-reset module in the
  existing World runtime owner and its durable adapter, consuming the accepted
  reset/drain/assignment boundaries; never a daily-local epoch source.
- Wire/composition: `protocol-oteryn/src/daily_reward.rs` and its child tests;
  `dispatch/src/gameplay_transport/daily_reward.rs` for handler composition;
  `apps/game-server/tests/daily_reward_postgres.rs` for real dispatch E2E.
- Shared parents, registries, capability tables and connection slots receive
  registration/wiring only. Binding restrictions belong in their existing
  transfer/split/merge/mail/trade/stash/NPC owners through dedicated child modules.

### Prerequisite audit

- PROVEN: the native Inbox bridge is present at
  `crates/gameplay/durability/src/durability/character_inbox/reward_claim/mint.rs`.
  Reuse its `prepare_reward_claim_inbox_mint` and
  `commit_reward_claim_inbox_mint`, borrowed `transaction()` and retained replay
  outcome. It requires 1..20 actual item outputs; non-item days need their own
  native fenced claim transaction, not an empty Inbox plan or dummy item.
- PROVEN: `character_item_fence_is_current` exists in durability's
  `item_transfer.rs`; reuse the native recovery/admission/session/lease/scope
  checks and Character-root lock rather than copying their implementation.
- PROVEN: migrations through 0130 contain no committed World-reset/month-history
  tables, PREY wildcard balance or `game_item_bindings`. The current map overlay
  owns pickup/rebuild, and loop map pickup still checks reset epoch zero. A new
  daily-local calendar/epoch or parallel wildcard balance would violate the card.
- PROVEN: the accepted brief explicitly keeps claims Free until PREM-3 and permits
  pending wildcard/XP lines while their accepted sinks are absent. Do not invent
  Premium activation evidence or a replacement PREY owner.
- PROVEN: the native PG fixture already exists at
  `apps/game-server/tests/support/postgres.rs` (`IsolatedPostgres`); the existing
  MAIL/native-item fixtures must be reused, with a minimal test-only exposure if
  required. No fixture or harness has been copied or added.
- PROVEN: neither the current protocol registry, protocol capability tables,
  task card nor the referenced closed DAILY draft #2151 (body and ten comments)
  supplies a DAILY wire-number reservation. The draft explicitly leaves wire
  work open. DAILY_REWARD0 section 7 assigns that reservation to the control plane.

### Integrator decision 2026-10-22: prerequisite work in this lane

The packet authorizes implementing missing prerequisites on their existing
owners before the daily scope. No prerequisite implementation has been written:
the control-plane wire allocation is an unresolved authority boundary, not a
missing helper to replace. No new module, public function, field, enum variant,
trait method, SQL function or test registration exists at this checkpoint.

Historical question (resolved by the CP allocation above): provide the `DAILY_REWARD_V1` capability,
command and state-domain numbers, or explicitly delegate reserving unused numbers
to this lane. The accepted brief requires the reservation before DAILY-WIRE-1;
this worker has not self-assigned or offered any capability.

### Validation and file-size record

Implementation and all acceptance boxes remain unfinished. No DAILY test suite
exists on this baseline, so none of the card's `daily` / `daily_reward` Cargo
filters constitutes implementation evidence. No Cargo or PostgreSQL test has run
in this checkpoint; there is no database-start marker to report.

Documentation checks: `python3 tools/agents/validate_governance.py` passed;
`python3 -m unittest discover -s tools/agents/tests` passed all 124 tests;
`git diff --check` passed. Re-read the complete working-tree diff against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: no code/schema/wire/test changes,
so constructor/match/impl sweeps, Cargo clippy, rustfmt and PG qualification remain
not applicable to this documentation-only checkpoint, not implementation passes.

Only this task card changed. No file grew by more than 200 lines; no runtime or
shared-slot file changed. Existing code was traced for planned reuse above, with
no copied implementation, extracted wrapper or changed caller.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2151, branch skeleton/daily-1

Status note: the draft qualified a bounded component (claim arithmetic + an uninstalled
storage schema candidate with read/lock APIs, 11/11 tests incl. 8 PG on PG 17.6) on the
pre-ADR-0022 `apps/game-server/src/...` layout. No claim path was enabled. It is closing;
the mechanic is re-implemented fresh on the current layout.

## Goal

Reward state, joker, claim and line tables; claim transaction; item binding table; line
delivery to Inbox and wildcard balance (DAILY-1) with content, bind refusals and
`DAILY_REWARD_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):
DAILY-CONTENT-1, DAILY-1 (hard), DAILY-BIND-1, DAILY-WIRE-1 (Reg).

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| DAILY_REWARD0 acceptance | ACCEPTED (#2209) |
| NPC-ECON-1 | MERGED (#2197) |
| Reward Inbox bridge (MAIL-1, D1318 same-TX seam) | MERGED (#2133) — `prepare/commit_reward_claim_inbox_mint` |
| PREMIUM-1 | MERGED (#2110) |
| WO-1 | MERGED (#1748) |
| Committed World-reset month history (MAP-OVERLAY-1c, D1317) | BLOCKING (draft #2167 open, not on main) |
| HUNT-1 / PREY wildcard sink (D1319) | BLOCKING — re-implementation pending (draft #2149 closing) |
| PREM-3 (Free lane until then) | BLOCKING for Premium lane (no record on main) |
| EXERCISE-CONTENT-1, RUNE-USE content | verify at allocation (RUNE-USE-0 accepted) |
| WORLDINT-USE-1 (DAILY-WIRE-1) | verify at allocation (WORLDINT-WIRE-1 archived) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/durability/src/durability/character_inbox/reward_claim/mint.rs` —
  `prepare_reward_claim_inbox_mint` / `commit_reward_claim_inbox_mint` (Inbox line delivery).
- `crates/gameplay/durability/src/durability/item_transfer.rs` —
  `character_item_fence_is_current` (current session/lease/scope; composition rule 2).
- `crates/gameplay/durability/src/durability/` mod registration (wiring only).
- `crates/protocol-oteryn/src/lib.rs` — `pub mod` + registry constants (wiring only).
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`, `capabilities.rs`
  (`GATED`: `DAILY_REWARD_V1`) — wiring only.
- `apps/game-server/tests/durability_postgres.rs` — registered PG target that executes
  durability-mounted PG cases once in CI.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/progression/src/daily_reward.rs` (new) + `daily_reward_tests.rs` (new)
  — L3 lane/streak/joker arithmetic (draft: `domain/daily_reward.rs`).
- `crates/gameplay/durability/src/durability/daily_reward.rs` (new) — L4 storage, readers,
  line locks.
- `crates/gameplay/durability/src/durability/daily_reward_claim.rs` (new) — L4 claim writer.
- `crates/gameplay/durability/src/durability/daily_reward_tests.rs` (new) — PG cases.
- `crates/protocol-oteryn/src/daily_reward.rs` (new) + `daily_reward_tests.rs` (new) — wire.
- `crates/gameplay/dispatch/src/gameplay_transport/daily_reward.rs` (new) — handler only.
- `apps/game-server/tests/daily_reward_postgres.rs` (new) — e2e target.
- Content inputs: shrine bindings, rune/potion lists, training weapons, scrolls.
- `apps/game-server/migrations/<next>_daily_reward.sql` (new) — migration number: next free
  number at allocation time (only one migration PR in the queue).

## Accepted design (DAILY_REWARD0 §§3–7 and CP decisions, as built in the draft)

Arithmetic (pure data; neither claims an epoch nor grants/spends a joker):

- Next-day lane wrap 7→1; missed epochs do not advance the lane.
- All-or-nothing joker coverage of a gap; first streak 1; streak ceiling 100,000.
- Equal/older epochs refused; full u64 gap arithmetic without overflow.
- D1317: Berlin 10:00 reset (D1257) supersedes historical UTC wording; epoch 0 does not count;
  a joker grant month requires a committed reset newer than the previous grant.

Storage schema (uninstalled candidate in the draft):

- Character+World reward state; Account+World jokers with immutable joker entries;
  immutable epoch-keyed claims; ordered reward lines. No Channel scope.
- Reuses native Character-root RESTRICT references and `TypedDefinitionRef`. No replacement
  epoch table, wildcard balance, reward definitions or content facts.
- Joker ledger (BANK-0 §3 pattern): deferred commit guard checks stored balance/cursor ==
  latest entry, exact Account+World predecessor (or zero opening), GRANT/SPEND arithmetic;
  re-reads final transaction state; O(1) rows per check; UPDATE/DELETE/TRUNCATE rejected;
  schema-qualified to defeat same-named temp tables.
- Receipts: stored transaction identity survives reload/replay; differing SHA-256 request
  binding → `BindingMismatch`, never a new receipt.
- Pending lines lock in line order `FOR UPDATE` (no `SKIP LOCKED`); held through sink+status
  commit. Runtime claims/ledger SELECT+INSERT only; lines UPDATE only status/delivering
  transaction/excess; terminal lines reject downgrade and receipt rewrite.
- Requested quantity positive; a fully capped wildcard credit is DELIVERED with
  `uncredited_excess == quantity`; pending lines carry no transaction/excess.

Integration obligations (mandatory):

1. Current epoch from the World-reset owner; restart/session-generation changes cannot invent
   a window. Lazy joker grants from committed reset-month history (#2167 port).
2. Claim writer: native admission/recovery locks, current session/lease/scope, then Character
   root and Account+World joker locks. Shrine reach, Account/World ownership, admitted content
   and picks checked independently. Freeze canonical picks/planned-output SHA-256 binding before
   the first attempt.
3. One transaction: state, jokers, entries, claim, complete line set and every available sink,
   including refusal rollback. Create every line with its claim; never append on replay.
   Per-claim item output bound 20; definition/vocation/count rules; atomic item binding rows.
4. Typed seams: Inbox delivery, closed `DailyRewardCause`, canonical mint/ledger/outbox, item
   binding lifecycle, PREY Account wildcard writer. XP stays pending until its accepted sink.
   PREY_WILDCARD lines remain PENDING until the HUNT sink exists. Joker UTC/lane parity:
   PARITY_PENDING.
5. PG cases follow the repository convention (no `#[sqlx::test]`): without
   `OTERYN_TEST_POSTGRES_ADMIN_URL` report PRE-ROUTING/NONCANONICAL and return; routed runs
   create an isolated DB, apply native migrations, run, drop.

## Review dispositions (from the draft)

- Gap-review P2: PG cases never ran in CI → mounted via durability so `durability_postgres`
  runs them once.
- SQLSTATE test defect: second native transaction checks raw 55P03 before error mapping.

## Remaining work

Schema migration; claim writer on the MAIL-1 Inbox bridge; lazy joker grants; item binding
table and §6 refusals (DAILY-BIND-1); PREY wildcard sink; content (DAILY-CONTENT-1);
wire/protocol (DAILY-WIRE-1); `tests/daily_reward_postgres.rs`.

## Acceptance criteria

- [ ] One claim per reset epoch; streak and joker rules per §3.
- [ ] Bound items refused at every §6 site.
- [ ] Claim idempotent across reconnect.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- content + regeneration + pins by one author
- migration + claim writer + durability
- claim = line MINT into Inbox + wildcard credit + ledger in one transaction
- wire + registry + capability + handler + tests

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability daily
cargo test --locked -p oteryn-game-server --test durability_postgres daily -- --test-threads=1  # PG 17.6
cargo test --locked -p protocol-oteryn daily_reward
python3 tools/content-migration/regenerate_content.py --check   # content slot only
python3 tools/agents/validate_governance.py
```

## Reference evidence

- DAILY_REWARD0 §3–§7; BANK-0 §3 ledger pattern
- Completion plan §4, §5 (Wave 4), §7; coordination #2131 (D1257, D1317, D1318, D1319, D1329)
