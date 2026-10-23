# LOOP-TRAIN-1 — offline training statue and exercise weapons (consolidated task card)

```yaml
task_id: LOOP-TRAIN-1
title: "LOOP-TRAIN-1: offline training statue and exercise weapons"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 1 row LOOP-TRAIN-1 (rows 34, 35); App. 7 LOOP-VITALS-1, LOOP-TRAIN-1, house beds; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_EXERCISE0_EXERCISE_WEAPONS_AND_DUMMIES_DECISION_2026-10-03.md
base_branch: main
branch: null
pr: null
class: hard
needs_migration: true
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Shared offline stamina arithmetic exists, but no offline-training marker/settlement owner from this card is retained. Evidence: `crates/gameplay/characters/src/stamina.rs`.

Remaining: Offline planner/marker and clock/content ports, admission/logout settlement, statue/exercise activation/charges and PG qualification remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2174, branch skeleton/loop-train-1

Status note: the draft built an offline-settlement planner and a native Logout/
OfflineSettlement marker writer on the pre-ADR-0022 `apps/game-server/src/training/` layout.
A private generation qualified 23/23 tests (incl. 2 PG on PG 17.6) but that source was lost;
the rebuilt source never compiled canonically (E0433 / unresolved durability symbols). No
statue activation, exercise charges, migration or caller exists. It is closing; the mechanic
is re-implemented fresh on the current layout. Beds are BED-1 (out of scope).

## Goal

Offline training (Premium statue) and exercise weapons/dummies.

## Server wiring gap list

Local audit, 2026-10-23 at `3c57781c321e643a67457374d1d42b6dbabd9133`
(before implementation; `MECHANICS_STATUS.md` E2E audit and this worktree inspected).
Client UI is outside this allocation.

| Layer | Present on this HEAD | Missing |
|---|---|---|
| L3 | Native build/relevel formula, hit/mana training, stamina regeneration | Offline pool/settlement planner and exercise scheduling/rates |
| L4 | Fenced, revision-serialized build writer; composed build transaction port; item timed charges | Offline markers, DB session/lease clocks and immediate predecessor link; atomic exercise charge/build settlement |
| Schema | Build row/receipts (0030), timed item state | OFFLINE0 build fields and marker invariants; reserved migration 0173 only |
| Protocol | Logout, item/world use and capability negotiation | Training codec/registry (reserved commands 66–67, capability 54) |
| Dispatch | Real admission, logout, item-use, build checkpoints | Training dispatch and admission/logout settlement callers |
| Content/offer | Native training curves; staged Reference item evidence | Admitted statue/dummy facts and charge rates; bundle capability offer |
| Qualification | Auto-mounted dispatch PostgreSQL test tree | Real command bytes → dispatch → durable write → result bytes, replay/fence/reload tests |

### Reference parity table

| Visible behavior | Reference evidence | Oteryn now |
|---|---|---|
| Premium statue selects a skill and logs out | Accepted OFFLINE0 §6; `training-content-1/facts.json`, `training.offline` | No statue caller or durable activation |
| Next login settles once, after at least 600 seconds offline | Accepted OFFLINE0 §5; `gameplay-source-reconciliation.json` retains the donor formula as unqualified | No settlement; only shared stamina arithmetic exists |
| Exercise weapon trains every two seconds and disappears at its last charge | Accepted EXERCISE0 §§2–5; staged charge counts are PROVEN | Generic timed-item owner exists; no exercise caller/composed build write |
| Skill/magic gain per charge | `gameplay-source-reconciliation.json`: melee 7 donor tries vs 7.2 community hit equivalents; bow units conflict; 600 base mana corroborated but final gain unqualified | No runtime rate selected |

### Current checkpoint and pending decision (2026-10-23)

`status: waiting` remains accurate. Only this card is changed; no implementation or
new player-visible server behavior is claimed. Audit classification: PROVEN for
the missing server layers above; UNKNOWN/CONFLICT for unresolved Reference gains.

The source reconciliation dated 2026-10-19 explicitly records
`SOURCE_RECONCILIATION_WORKING_DEFAULT_WITHDRAWN`,
`selected_for_native_runtime: false` and
`runtime_admission: BLOCKED_PENDING_NATIVE_QUALIFICATION` for offline formulas and
exercise gains, including the 600-mana candidate. Its README states that source
code alone does not meet the original official/controlled-gameplay evidence
requirement. The accepted briefs describe provisional formulas; the newer source
packet does not select them for runtime. The present instruction to fail closed
on missing source facts does not resolve that rate selection explicitly.

Owner question (pending; supersedes the earlier magic-only alternative): use the
accepted OFFLINE0 formulas and retained Canary exercise gains as provisional
Oteryn rules for this basic server task, retaining the accepted two-second tick
and documenting parity gaps, or keep rate selection blocked pending qualified
Reference values? Recommendation: explicitly select the provisional rules for
this task. No owner-funded Reference measurement or paid review was triggered.

Unfinished: L3 planner, FND clock/predecessor port, migration 0173, marker writer,
exercise charge/build composition, admitted content, wire/offer/dispatch,
admission/logout callers and PostgreSQL E2E. Next step after rate selection is
the mechanic-owned L3/content implementation followed by the composed durable
writer and real wire qualification; no backend-only completion is acceptable.

Validation for the current documentation-only delta:

- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `python3 tools/repository/validate_repository_policy.py`: PASS.
- `git diff --check`: PASS.
- Whole diff reviewed against `~/oteryn-cache/CODEX-CHECKLIST.md`: no new Rust
  enum variants, fields, trait methods or public functions; no production paths,
  tests, workflow policy, credentials or client files changed.
- Cargo test filters: none added or run; no Rust crate touched and no PostgreSQL
  qualification claimed. Focused unit/PG filters must be recorded when the
  implementation exists.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| OFFLINE0, EXERCISE0 acceptance | ACCEPTED (#2209 §1); SKILLS0 remains CANDIDATE |
| PREMIUM-1 | MERGED (#2110) |
| CHAR-POSITION-1 | MERGED (#2006) |
| LOOP-VITALS-1 (stamina helper) | MERGED (#2196, skeleton) |
| LOOP-PROGRESS-1 | MERGED (#2192, skeleton) |
| ITEM-TIMED-1 (`charges`) | MERGED (#2155, skeleton) |
| WORLDINT-USE-1 (statue/dummy WorldUse reach) | BLOCKING (no record on main; verify at allocation) |
| OFFLINE-1, STATUE-1, EXERCISE-1 children | BLOCKING (allocate from the briefs' child graphs) |
| WO-2 remainder, OFFLINE-CONTENT-1, EXERCISE-CONTENT-1 content | BLOCKING (WO-2a/2c merged #1319; rest verify) |
| FND DB-stamped session/lease clocks + immediate predecessor lease relation | BLOCKING (no owner API on main) |
| Slot in the shared guard-rewrite order | CP lease |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/domain/src/domain/character_build.rs` — `relevel`, `BuildFormula`,
  `DurableBuildState` (no copied advancement loop or curves).
- `crates/gameplay/durability/src/durability/character_build.rs` (986 lines) — build row and
  receipts; currently admits only Training/VocationChoice/Promotion. Wiring-only edits; the
  offline marker writer goes in a new file.
- `crates/gameplay/characters/src/stamina.rs` — `regenerate_stamina_minutes` (VITALS helper).
- `crates/gameplay/durability/src/durability/character_progression.rs` —
  `initialize_character_progression` (D88); an in-transaction variant is an owner hunk.
- `crates/gameplay/durability/src/durability/db.rs` — `begin_semantic_transaction` /
  `commit_semantic_transaction` (pub); `lock_admission_relations` is `pub(super)`.
- `crates/gameplay/durability/src/durability/character_authority.rs` —
  `assert_recovery_fence` is `pub(super)`; visibility changes are owner wiring hunks.
- `crates/gameplay/dispatch/src/gameplay_transport/training_save.rs` — actor-end flush;
  preserve its unknown-outcome reconciliation/sequencing.
- `apps/game-server/migrations/0030_character_build_state.sql` — existing schema (immutable).
- `apps/game-server/tests/support/character_build_postgres_cases.rs`,
  `apps/game-server/tests/loop_vitals_stamina.rs` — existing PG/test patterns.
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`, `capabilities.rs`
  (new capability not offered until CP decides) — wiring only.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/progression/src/offline_training.rs` (new) + `offline_training_tests.rs`
  (new) — L3 pure settlement planner/arithmetic (draft: `training/offline.rs` planner part).
- `crates/gameplay/progression/src/exercise_training.rs` (new) + tests (new) — L3 per-charge
  rates.
- `crates/gameplay/durability/src/durability/offline_settlement.rs` (new) — L4 Logout/
  OfflineSettlement marker writer (draft: `native_marker_writer`).
- `crates/gameplay/durability/src/durability/offline_settlement_tests.rs` (new) — PG cases.
- `crates/gameplay/dispatch/src/gameplay_transport/offline_training.rs` (new) — statue/
  exercise use handler only.
- `apps/game-server/migrations/<next>_offline_training.sql` (new) — migration number: next free
  number at allocation time (only one migration PR in the queue). Draft's private "0104" is void.

## Accepted design (OFFLINE0 §§3–6, EXERCISE0, as built in the draft)

Settlement arithmetic:

- Latest build-carrying receipt must be a logout marker of the independently resolved immediate
  predecessor lease/session (same Character and World); never inferred as `lease - 1`. Missing/
  older/non-logout latest receipts (death counts) → crash `d=0`; selected skill still cleared.
- Offline time = lease acquisition − logout commit, floored to seconds, ≥ 0, capped 21 days.
  Later settlement `as_of` adds nothing.
- Selected skill with `d<600` consumes/restores no pool; else `t=min(d,pool,43200)` trains and
  restores `d-t` (cap 43200). `t<60` grants no tries. All settlement plans clear the skill.
  No skill → pool restores `d`.
- Rates: top-vocation base attack speed ×2 fist/melee, ×4 distance; magic uses current-vocation
  mana amount and top-vocation ticks with 1-second zero-tick fallback. Level change adds
  `floor(t/4)` shielding via native relevel; same-level percent changes come from a typed native
  Content port (no guessed rounding). Missing source facts refuse before mutation and preserve
  selected skill/pool.
- Stamina regen via the VITALS helper (39h/42h boundaries) inside the same transaction; before
  and u16 after values recorded in the marker receipt.
- Logout restores online pool only if this lease has a settlement marker and only from the
  current GameSession creation clock.

Writer obligations:

1. Validate recovery and gameplay fences; serialize (Character, lease, cause); resolve
   first-marker replay BEFORE fresh intent/time/formula checks; read DB clock once after the
   root lock; one root + progression advance; whole-TX rollback incl. D88 initialization.
2. Settlement waits for pending respawn and initializes progression in the same TX when absent;
   logout waits for actor removal before its marker. Generic Instance scopes are not houses.
3. `CommittedOfflineMarker` acknowledgement only after COMMIT (fresh or replay); it is
   historical completion, never current authority. Reader refuses zero/mismatched marker leases
   and non-successor revisions.
4. Clocks: DB-stamped integer microseconds from FND; checked ms→µs conversion of
   `committed_at`; reconnect `predecessor_generation` is not a predecessor-lease link.
5. Schema: extend the same build row, build/death before/after receipts (death all-NULL/
   all-non-NULL OFFLINE fields), marker version/cause/no-op constraints, once-per-
   (Character, lease, cause) uniqueness, deferred chain guards, `verify_character_integrity`;
   bump the build binding version. Ordinary v1 bindings/replays unchanged.
6. Statue: only `PremiumStatus::Current` passes (no NotActivated bypass); refuse logout/PZ
   locks; activation proposal binds Character/session/connection generation/lease/scope and
   persists only with the final logout marker. WorldUse reach/PZ/cooldown admission required.
7. Exercise charges, tries and last-charge share one composed transaction.

## Review dispositions (from the draft)

- Canonical Clippy E0433 (`crate::training` absent) → module registration needed; on the new
  layout register via the crate's `lib.rs`/`mod.rs` (wiring only).
- Unresolved symbols after registration: missing in-TX progression init, VITALS helper (now on
  main), `pub(super)` db/authority helpers, unused re-export → each needs an owner wiring hunk,
  not a copy.
- Lost private source: no prior PASS qualifies new code.

## Remaining work

Everything: planner, marker writer, schema migration, FND clock port, Content rate/percent port,
admission/logout callers, statue activation, exercise charges, content, PG cases.

## Acceptance criteria

- [ ] Offline skill gain computed once at login from durable timestamps.
- [ ] Exercise charges consumed exactly once.
- [ ] Statue Premium gate fails closed.
- [ ] Marker replay returns the first receipt; no double settlement across reconnect.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- migration + marker writer + durability (build row, receipts, chain guards)
- exercise charge consumption + tries + last-charge in one transaction
- content + regeneration + pins by one author

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability offline
cargo test --locked -p oteryn-game-server --test durability_postgres offline -- --test-threads=1  # PG 17.6
python3 tools/content-migration/regenerate_content.py --check   # content slot only
python3 tools/agents/validate_governance.py
```

## Reference evidence

- OFFLINE0 §§3–6 (§30–34); EXERCISE0 §33–35; SKILLS0 (CANDIDATE)
- Completion plan §4, §5 (Wave 5), §7; coordination #2131 (D1108, D1122, D1160, D1220, D1222)
