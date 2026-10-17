# DAILY-1 — daily reward (skeleton task record)

```yaml
task_id: DAILY-1
title: "DAILY-1: daily reward"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row DAILY-1 (row 47); §7 'DAILY-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/daily-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Reward state, joker, claim and line tables; claim transaction; item binding table; line delivery to Inbox and wildcard balance (DAILY-1) with content, bind refusals and `DAILY_REWARD_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

DAILY-CONTENT-1, DAILY-1 (hard), DAILY-BIND-1, DAILY-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| NPC-ECON-1 | BLOCKING |
| MARKET-1 (`CharacterInbox` usage) | BLOCKING (Wave 4) |
| HUNT-1 / PREY-1 (wildcard balance) | BLOCKING (Wave 4) |
| ADR-0021 World reset epoch | ADR present on main (docs/architecture/ADR-0021-world-map-runtime-loading.md); reset-epoch runtime: verify |
| PREMIUM-1 | MERGED (#2110) |
| WO-1 | MERGED |
| EXERCISE-CONTENT-1, RUNE-USE content | verify (RUNE-USE-0 accepted, archive OTV2-20261014-accept-rune-use-0) |
| WORLDINT-USE-1 (DAILY-WIRE-1) | verify (WORLDINT-WIRE-1 archived) |
| acceptance of DAILY_REWARD0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/daily_reward.rs`
- `apps/game-server/src/durability/daily_reward.rs`
- `apps/game-server/src/durability/daily_reward_tests.rs`
- `apps/game-server/src/gameplay_transport/daily_reward.rs`
- `crates/protocol-oteryn/src/daily_reward.rs`
- `crates/protocol-oteryn/src/daily_reward_tests.rs`
- `apps/game-server/tests/daily_reward_postgres.rs`
- content authoring inputs for shrine bindings, rune/potion lists, training weapons, scrolls
- `docs/agents/tasks/DAILY-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- durability/character_inbox.rs delivery
- PREY-1 wildcard balance writer
- connection.rs dispatch; capabilities.rs `GATED` (`DAILY_REWARD_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + claim writer + durability
- claim = line MINT into Inbox + wildcard credit + ledger in one transaction
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] One claim per reset epoch; streak and joker rules per §3.
- [ ] Bound items refused at every §6 site.
- [ ] Claim idempotent across reconnect.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server daily
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md §3-§7
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded local preparation — 2026-10-15 (not full DAILY-1 acceptance)

Authority basis: protected-main `3ef900c160c9713092a86bfcd1a8377c7587c898`
accepts DAILY_REWARD0 without changing §§3–5 rules. The original skeleton and
historical dependency/status rows above are preserved. The active allocation
is `daily-1-allocation.json`, branch `skeleton/daily-1`, exact source
`170c17c575d3623d9074025c94d872b5feb3bedc`; only the allocated domain and durability
files, adjacent tests and this record are written. `QUEUE_READINESS.md` nominates
bounded state/claim/pending-line preparation, not delivery-enabled claims.
No existing native daily owner was found. Generic reward-chest MINT remains its
own owner and is not copied or repurposed.

Prepared component:

- Domain lane/streak planning from explicit stored values: next-day wrap 7→1,
  missed epochs do not advance the lane, all-or-nothing joker coverage, first
  streak 1, saturated streak ceiling 100,000, equal/older epochs refused and
  full u64 gap arithmetic without overflow. This function is data arithmetic;
  it neither claims an epoch nor grants/spends a joker.
- An embedded, uninstalled SQL schema candidate for state, Account+World jokers,
  immutable joker entries, immutable epoch-keyed claims and ordered reward
  lines. It delegates the actual World-reset source and UTC grant-month owner;
  no replacement epoch table, wildcard balance, reward definitions or content
  facts are introduced. Native Character-root RESTRICT references and the native
  `TypedDefinitionRef` are reused. Character+World and Account+World state has
  no Channel scope. Schema installation/numbering remains a CP slot.
- Caller-transaction readers expose stored state, an existing locked joker row
  (missing remains missing), and exact request-binding receipt classification.
  A stored transaction identity survives reload/replay; a differing SHA-256
  binding yields `BindingMismatch`, never a new receipt or delivery.
- Pending rows lock in line order with PostgreSQL `FOR UPDATE`, without
  `SKIP LOCKED`; callers must hold the locks through sink+status commit/rollback.
  Runtime claims/ledger have SELECT+INSERT only, no UPDATE/DELETE. Lines expose
  UPDATE only on status/delivering transaction/excess, and terminal lines reject
  both downgrade and receipt rewrite. No mutation API is exposed.
- A line's requested quantity must be positive. A fully capped wildcard credit
  is represented by DELIVERED with `uncredited_excess == quantity`, recording
  zero credited value without leaving the line retryable. Pending lines have
  no delivering transaction or excess; other line kinds cannot carry excess.
  These are storage constraints, not evidence that any value sink ran.

Integration obligations (mandatory, incomplete):

1. The current World-reset owner must provide the independently current epoch;
   crash restart/session-generation changes cannot invent a new window. The
   UTC calendar-month/reset history source must resolve lazy joker grants and
   keep the immutable entry chain/balance in agreement. Raw epoch, month, stored
   balance, receipt and arithmetic-plan values confer no current authority.
2. The eventual claim writer must use native admission/recovery locks and
   current session/lease/scope evidence (`character_item_fence_is_current`,
   composition rule 2), then Character root and Account+World joker locks.
   Shrine reach, current Account/World ownership and admitted content/picks
   remain independently checked. Reconciliation needs its current recovery
   fence. A request binding is opaque storage data here; canonical validated
   picks/planned-output SHA-256 binding must be frozen before the first attempt.
3. Reward state/jokers/entries/claim/complete line set and every available sink
   remain one inseparable transaction, including refusal rollback. No separate
   claim reservation, periodic worker or valuable grant is enabled. The writer
   must create every line with its claim, never append lines on replay, enforce
   per-claim item output bound 20, definition/vocation/count rules, immutable
   intent and atomic Item binding rows. SQL constraints here do not prove those
   aggregate/cross-table/content or ledger-chain invariants.
4. Privileged Inbox delivery, closed `DailyRewardCause`, canonical mint/ledger/
   outbox, item-binding lifecycle and PREY's actual Account wildcard writer are
   required typed seams. XP remains pending until its accepted sink. No public
   Inbox API, invented audit cause, admitted reward data or sink stub is added.
   Free lane remains required until PREM-3; the receipt Premium field is history,
   not current evidence. Joker UTC and lane parity remain PARITY_PENDING.
5. CP must integrate migration/role grants with the inseparable writer, module
   registrations, content, registries and wire; route these PostgreSQL tests
   into the real PG lane (currently an unregistered adjacent test module).
   Runtime raw SQL/table privileges alone do not qualify production authorization.

Validation and source-bound evidence are recorded under
`/workspace/oteryn-prep/daily-1-evidence/`; the disposable native validation
snapshot registers these modules outside the branch only. Current-authority
mutation operators are NOT_APPLICABLE to this candidate's read/lock/arithmetic
APIs: no production grant, controller, PREPARE/COMMIT permission or session
replacement is returned. Binding/scope substitution, stale epoch/replay,
concurrent insertion, lock replacement/rollback and terminal-state mutations
are tested at their actual data/storage boundaries. Full writer/fence/valuable
sink/E2E qualification and independent architect acceptance remain incomplete.

### Actual local validation result

- PASS: 7/7 focused original owned test bodies: 3 domain cases and 4 isolated
  PostgreSQL 17.6 cases. They cover exact/insufficient joker gaps, lane wrap,
  first/equal/regressed/full-u64 epochs, streak ceiling/max+1, state/joker
  bounds, rollback of state+joker+entries+claim+lines, receipt reload and exact
  binding substitution, competing epoch uniqueness, runtime line lock timeout/
  rollback/reacquisition, zero-credit terminal status, and immutable-column/
  terminal downgrade/receipt-rewrite refusals. No real reward was granted.
- PASS: Clippy metadata for the original untransformed native `--test` unit
  target, retaining its actual Cargo extern/cfg/lint inputs and `-D warnings`.
  No binary codegen or global unit suite success is claimed by this check.
- PASS: actual native `cfg(test)` normal rlib plus a seven-case external test
  runner. Validation-only snapshots remove only owned test attributes, expose
  parent visibility and append forwarding functions; complete test bodies and
  native fixtures/types stay unchanged. The four SQLx test attributes' database
  lifecycle is replaced by a disclosed wrapper creating isolated real PG17.6
  databases and applying the same actual native migrations. All transformations,
  actual commands, public compile environment, body identity checks and hashes
  are retained. The cfg(test) rlib reports baseline unused-helper warnings;
  original unit Clippy is independently clean.
- PASS: owned rustfmt, branch and original registered snapshot fmt; source
  binding before/after for 82 actual owned/native/migration inputs. Module
  registration changes exist outside the candidate only. Governance validation,
  its 124 tests, owned-path and whole-diff/whitespace checks pass.
- Preserved setup/resource failures separately: the first snapshot copy caught
  a concurrent task-card append and stopped before Cargo (zero tests); the one
  original global unit-codegen attempt stopped with tool exit 137 before any
  test result. No unchanged global-unit retry occurred. Capturing the real
  native unit invocation deliberately stopped its compiler before codegen;
  the resulting command is used for original metadata Clippy and the bounded
  normal-rlib runner. These are not behavioral RED outcomes.

Evidence: `/workspace/oteryn-prep/daily-1-evidence/index.md`,
`source-binding-before.json`, `source-binding-after.json`,
`bridge-transformations.json`, `bridge-body-verification.json`,
`bridge-only.diff`, `tests.log`, actual status/command JSON and final owned hash
manifest. No commit, push, shared registration, migration installation,
production activation, full DAILY-1 acceptance or architect approval occurred.

## Fresh native Cargo qualification (2026-10-17)

This entry records the fresh native run reported in
[PR #2151](https://github.com/Oteryn/Oteryn-Game/pull/2151), bound to source head
`e266247a2f6b447181d3b9442cd8dff9b048acc5`. The earlier bridge, metadata-only
Clippy and setup/resource results above remain historical evidence.

An isolated checkout on Molehill-PC / WSL Ubuntu-24.04 added only
`pub mod daily_reward;` in `apps/game-server/src/domain/mod.rs` and
`apps/game-server/src/durability/mod.rs` for validation. The three owned Rust
files and every original test body and attribute were unchanged. These two
registration lines remain outside this PR; this task-card append publishes
the evidence for the component without registering or activating DAILY.

- PASS: `cargo test --locked -p oteryn-game-server --lib daily_ --quiet -- --test-threads=1`.
  The original Cargo cases returned 7 passed (3 domain and 4 PostgreSQL),
  0 failed, 0 ignored and 3,179 filtered out, in 3.78 seconds.
- The original `#[sqlx::test(migrations = "./migrations")]` cases used
  Cargo/SQLx's native database lifecycle against PostgreSQL 17.6. This fresh
  run retained the original bodies, attributes and migration lifecycle.
- PASS: `cargo fmt --all -- --check`.
- PASS: `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`
  in the same isolated validation checkout.
- Source hashes were checked before and after tests and Clippy. The run used
  Rust/Cargo 1.94.0 without a `RUST_MIN_STACK` override.
- The first preparation script stopped at an absent registration anchor
  before editing source or running tests. That setup stop is retained
  separately from behavioral test outcomes.

Retained evidence is under
`/root/oteryn-six-pr-20261017/evidence-daily/` on Molehill-PC: the two-line
validation-only patch, source hashes, original Cargo output, formatting and
Clippy results. The fresh results qualify the unchanged arithmetic and
storage/locking component. Full-package behavioral tests, current-main
integration, the actual claim writer and its current fences, valuable sinks,
E2E qualification and architect acceptance remain incomplete. Production
schema installation, shared registration and runtime activation remain
integration work in the assigned CP slot; DAILY-1 remains a partial draft.

## Joker ledger consistency preparation (2026-10-17 continuation)

This source batch starts from `ce3f8a3f74473aa24e91b3bff702eb8dbea3559c`.
Its accepted basis is DAILY_REWARD0 §3.3, §3.4 and §5, including the
BANK-0 §3 deferred balance/entry guard and immutable ledger pattern. It
tightens the existing uninstalled schema candidate and adjacent SQLx tests;
the arithmetic and read/lock APIs are unchanged. Current source comments also
follow the newly resolved grant-month policy below.

The candidate adds deferred checks that the stored joker balance and cursor
match the latest entry, that every entry has its exact Account+World
predecessor (or the zero opening balance), and that GRANT/SPEND arithmetic
matches that predecessor. The function rereads the final transaction state
instead of treating an earlier trigger's row image as the final head. Each
check reads one state, one indexed latest entry and at most one predecessor,
without a scan of the complete ledger. UPDATE, DELETE and TRUNCATE cannot
rewrite its entries; the guard's schema binding prevents a same-named
temporary table from replacing the installed state.

Four new original `#[sqlx::test(migrations = "./migrations")]` cases cover
multi-entry transactions in both state/ledger write orders, out-of-order
insertion, deferred commit refusals, Account/World isolation, temporary-table
substitution, immutable history, rollback and real row-lock contention with
a stale losing debit. The previous four SQLx test bodies and three domain
cases are preserved. Native PostgreSQL 17.6 execution of this successor is
pending the root's isolated qualification run; the earlier seven-case
evidence remains bound to its original source and validation registrations.

These are stored-data invariants, not a grant or delivery authorization.
[CP D1317-D1319](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232842478)
resolve the next integration boundaries: D1257 Berlin10 supersedes the
historical UTC wording, epoch0 does not count, and a grant month requires a
committed reset newer than the previous grant. The committed-month port stays
with #2167 and is consumed after its merge; this leaf adds no substitute.
The transaction Inbox seam stays with #2133. PREY_WILDCARD lines remain
PENDING until HUNT supplies its actual sink in the assigned slot. The claim
writer still must bind entries and the complete claim/line/sink transaction
to native current authority, the shrine and admitted content. Migration
installation, shared registration, runtime activation and full DAILY-1
acceptance remain open.

## 2026-10-17 — qualified joker ledger integrity successor (D1329)

The schema candidate now enforces the Account+World joker state against its final ledger tip at transaction commit. Deferred checks reload the final locked state, validate indexed predecessor arithmetic and reject gaps or mismatched scope/balance. The ledger rejects UPDATE, DELETE and TRUNCATE. Function name resolution remains bound to the installed schema even with a caller search_path or same-named temporary tables.

Exact qualified Rust sources:

| Owned path | SHA256 |
| --- | --- |
| `domain/daily_reward.rs` | `16263d20166ba6074c5ff5652205c24ba3d24116b449042bf0a2ff0f23af10bc` |
| `durability/daily_reward.rs` | `f2bc406d81698dcde14a68a5054cb88ec0bf8cccba56fcdc8be1b04008f57437` |
| `durability/daily_reward_tests.rs` | `d72f29b46b8d1ee7f84d7ca2b4200c1804e75dd06d2aaf9d4308b29fe9829c3e` |

Paths above are under `apps/game-server/src/`. The normal isolated Cargo assembly is based on `ce3f8a3f74473aa24e91b3bff702eb8dbea3559c` plus these owned changes and the two unchanged validation-only `pub mod daily_reward;` registrations. Domain root SHA256 `f876b61f0107a196ffc9664243dc4a54929fad8a28b96cd580acb62dfe7f09de`; durability root `fae0f83cf157099b6dd97003a3aa1d526cf4ad80db14967f008f8a29c7af2e60`. Cargo.lock, workspace/server manifests and rust-toolchain input are pinned unchanged in the retained before/after snapshots.

### Executed qualification and correction history

- Original-store RED genuinely reproduced the unchanged selected regression: commit accepted a balance differing from the latest entry, yielding `balance differs from latest entry: Ok(())`, exit101, exactly 1 failed. Its original store blob is `dba29b4913c4a533c0352476b6cdee3bdddbff99`, and the source/log binding is retained.
- The first attempted GREEN is **NOT qualified**: executable inspection proved that a shared Cargo target reused the RED binary across different worktrees (binary `c42cd35cfdc2f84942b68bbedea816a6b9b5680a8a572f587180ad3e828f186a`, RED depfile path, absent new guard strings). Its apparent 8/11 result is not a result for the new store. That evidence remains intact.
- The separate SQLSTATE test defect was corrected: the first real helper retains the row lock; a second native SQLx transaction checks exact raw 55P03 before DurabilityError intentionally discards unregistered states. Winner commit, subsequent real helper read, stale-debit 23514 refusal and final ledger assertions are unchanged. Independent one-hunk review PASS; actual formatting PASS.
- Corrected GREEN uses a new, initially absent `target-daily-green-corrected`, without copying build artifacts. **11/11 original Cargo-selected tests PASS**, 3 domain and 8 PostgreSQL, zero failed/ignored. These cover final multi-update/multi-entry state, out-of-order ledger insertion, rollback, head/chain/scope errors, name-resolution shadowing, immutable history and real concurrent locking/stale debit.
- Strict server all-target Clippy and workspace fmt PASS. The original SQLx migration lifecycle and test attributes are unchanged. `RUST_MIN_STACK` is unset.

Under [D1329](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233047791), `cargo fetch --locked` completed first; native PG commands used the installed `/root/.oteryn/in-pg-netns.sh` (SHA256 `368fd9867c97ca1d59aa8b9f7123b77f0e23fa8b6a16fbb5ace4d18b6e1bfd99`). Shared PostgreSQL reports 170006 and image `sha256:f3bd19c606e442c3d7bdfa8002e03fe260a1023351e0ea4598032022b68dd6e3`. The official namespace wrapper supplies the canonical fixture route to the existing shared service. No PG instance is created by this packet.

Final native receipt SHA256: `32dfd064161ad14f8a0b4777d9583527bcc3cf8d0114f90b358b840eabfa7dac`.

| Command | Qualification | Finished UTC | Log SHA256 |
| --- | --- | --- | --- |
| `daily-red` | EXPECTED_BEHAVIORAL_RED | `2026-10-17T11:15:41.218859+00:00` | `e1fc48f9757ef3abd08cde326f9543cd0b18d3ff0495041f209107a0a7868455` |
| `daily-green-corrected` | PASS | `2026-10-17T11:54:54.244361+00:00` | `7c88d9239699ab0c18146def6395deea07380d434d87219d50902c84d77e0ddf` |
| `daily-clippy` | PASS | `2026-10-17T11:57:47.752879+00:00` | `984a0050d7b0091cd2c5701c8ec6e6b74b8587417704637c594d8849234b24a1` |
| `daily-fmt` | PASS | `2026-10-17T11:58:21.229112+00:00` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

Native output, exact commands, before/after sources and the discarded-attempt diagnosis are retained on Molehill-PC under `/root/oteryn-six-pr-20261017/`. These are component results; the schema candidate is still uninstalled and is not a claim writer, current-authority proof, delivery sink or full E2E qualification.

D1317 binds lazy joker grants to #2167's committed-reset month history with Berlin10 semantics (epoch 0 bootstrap excluded); D1318 supplies #2133's real same-TX Inbox seam; D1319 assigns #2149's PREY sink. Those production interfaces and shared registration remain subject to the existing CP integration order. This owned successor does not select a migration number or mutate shared registrations.
