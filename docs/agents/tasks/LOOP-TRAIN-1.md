# LOOP-TRAIN-1 — offline training statue and exercise weapons (skeleton task record)

```yaml
task_id: LOOP-TRAIN-1
title: "LOOP-TRAIN-1: offline training statue and exercise weapons (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-train-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 row LOOP-TRAIN-1 (rows 34, 35); App. 7 LOOP-VITALS-1, LOOP-TRAIN-1, house beds; §5 Wave 5"
class: "hard"
needs_migration: yes
needs_registry: no
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/training/ (new: offline, exercise) + tests"
  - "docs/agents/tasks/LOOP-TRAIN-1.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_offline_training.sql"
  - "content regeneration outputs and pins (statue/dummy content)"
  - "apps/game-server/src/gameplay_transport/item_use.rs / connection.rs"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Offline training (Premium statue) and exercise weapons/dummies; beds are BED-1.

## Dependencies (acceptance reconciled with protected main@ac9f1728, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| PREMIUM-1 | MERGED (#2110) | ok |
| CHAR-POSITION-1 | MERGED (#2006) | ok |
| OFFLINE0, EXERCISE0 acceptance | ACCEPTED: effective ACCEPT-BATCH #2209 §1 | owning accepted rules |
| LOOP-VITALS-1 | NOT MERGED (Wave 2) | **BLOCKING** |
| LOOP-PROGRESS-1 | NOT MERGED (Wave 1) | **BLOCKING** |
| WORLDINT-USE-1 | NOT MERGED | **BLOCKING** |
| OFFLINE-1, STATUE-1 | NOT MERGED | **BLOCKING** |
| EXERCISE-1 children | NOT MERGED | **BLOCKING** |
| ITEM-TIMED-1 (`charges`) | NOT MERGED (Wave 3) | **BLOCKING** |
| WO-2, OFFLINE-CONTENT-1, EXERCISE-CONTENT-1 content | NOT MERGED | **BLOCKING** |
| slot in the shared guard-rewrite order | CP lease | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/training/ (new: offline, exercise) + tests

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_offline_training.sql
- content regeneration outputs and pins (statue/dummy content)
- apps/game-server/src/gameplay_transport/item_use.rs / connection.rs
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/training_save.rs (W2b, #2109)
- durable skill build state (LOOP-PROGRESS-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- content + regeneration + pins by one author

## Acceptance criteria

- [ ] offline skill gain computed once at login from durable timestamps
- [ ] exercise charges consumed exactly once
- [ ] statue Premium gate fails closed
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server training
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md §30-34
- docs/architecture/reviews/OTERYN_GAME_EXERCISE0_EXERCISE_WEAPONS_AND_DUMMIES_DECISION_2026-10-03.md §33-35
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Owned OFFLINE preparation (2026-10-15)

Exact allocated draft base `076cef9c26bd41e43830653d44a99865016185b8`, branch
`skeleton/loop-train-1`; no merge/rebase. Protected acceptance snapshot is
`ac9f172836f00226b68088aa04db95ccf1ab994e`, ACCEPT-BATCH #2209 §1: OFFLINE0 §§3–6 and
EXERCISE0 are accepted despite this older draft's headings. SKILLS0 remains CANDIDATE.
This round's exact lease covers only training/offline.rs, training/offline_tests.rs and this
card. It supersedes the broader skeleton training-directory/registration permission: no
exercise implementation, training/mod.rs, shared registration, migration, content or pins.

### Coherent transition plan, not a settlement writer

The private planner consumes the real native `CurrentCharacterGameplayFence`, existing
`DurableBuildState` and proposed OFFLINE extension columns of that SAME build row. No second
skill/stamina engine or parallel storage is introduced. Proposed marker/time/predecessor data
is explicitly raw: equality of proposed `as_of` and `committed_at` does not prove a database
clock, a native receipt, a current lease or any session/scope/node/recovery authority.

Every fresh settlement output retains a `StaminaOwnerWork`, `MarkerCommitRequired`, and,
when training is due, `TrainingWork::NeedsRates`. Raw native arithmetic preview still retains
the rate-owner stage and exposes `NeedsNativePercentProjection` with both conditional builds
when only progress changed. No output is an installable/complete settlement, native receipt,
BuildChangeRequest, current-authority token or actor activation. No database write occurs.

- The latest build-carrying receipt must be a proposed logout marker of the independently
  resolved immediate predecessor lease/session, same Character and World. The predecessor is
  not inferred as `lease_generation - 1`. Missing/older/non-logout latest receipts produce
  crash `d=0`; any selected skill is still cleared. Invalid identity/time data is refused.
- Offline time uses lease acquisition minus logout commit, floored to seconds, max with0 and
  capped at21 days. Later settlement `as_of` does not add offline time.
- A selected skill with `d<600` consumes/restores no pool; otherwise `t=min(d,pool,43200)`
  consumes training time and restores only `d-t`, capped43200. Below60 trained seconds grants
  no tries; all settlement plans clear the skill. Without a skill the pool restores `d`.
- Logout restores online pool only if THIS lease has a settlement marker and only from the
  current GameSession creation clock. A crash/replacement/skipped marker does not restore
  uncontrolled time. The final native tries/stamina flush remains an explicit owner stage.
- Marker-key replay is planned before raw request/time/column/activation binding checks:
  same (Character, lease, cause) requires returning the first native marker. A proposed key
  never substitutes for that receipt or its recovery validation. Same-lease reconnect makes
  no new marker. A no-change fresh lease still needs its marker/no-op exemption.
- Settlement waits for pending respawn and requests D88 initialization in the same transaction
  when progression is absent. Logout skips missing progression/respawn/terminally released
  actors and discards activation; it waits for actor removal before proposing its marker.
  Generic native Instance scopes are not assumed to be houses: their House owner is pending.
- Statue policy consumes the unchanged native PremiumStatus gate (only Current passes;
  NotActivated has no bypass), refuses logout/PZ locks, and returns only an activation
  proposal bound to Character/session, connection generation, lease, scope and scope generation.
  Native sequenced CharacterRevision advancement is permitted; none of these raw values proves
  current authority. The actual statue/WorldUse reach/PZ/cooldown/current Premium admission
  remains required; activation can persist only with the final logout marker.

### Native arithmetic and explicit unresolved owners

The preview calls existing `character_build::relevel` and `BuildFormula` over native build
families. It copies no advancement loop, curves, vocation multipliers or loss rules and
defines no new SKILLS0 rules. Tests use the actual native compiled training profile, not a
replacement formula engine. A separately sourced/admitted delta remains an owning obligation;
the `RawTrainingDelta` input is explicitly not its proof and cannot clear that obligation.
No magic/skills candidate result is committed or installed.

Rate stages name the accepted OFFLINE recipe precisely: top-vocation base attack speed times2
for fist/melee, times4 for distance; current-vocation mana amount and top-vocation ticks with
the1-second zero-tick fallback for magic. No admitted native top-vocation attack-speed/relation
projection was found. Native food mana rates alone do not establish that missing relationship.
If the trained level changes, the preview adds `floor(t/4)` shielding through native relevel;
if neither tuple component changes it adds none; progress-only changes retain BOTH branches
pending the missing native percent projection, without inventing percentage rounding.

Stamina regeneration remains `StaminaOwnerWork { before_minutes, offline_s }`, including the
39h/42h boundaries. The separately published VITALS stamina component is unmerged and is not
blindly imported/copied. No after-stamina value or complete settlement is fabricated here.
EXERCISE per-charge rates, timed-item checkpoint/retirement and new content remain out of this
allocation; charges and skill gains must eventually share their native composed transaction.

### Concrete shared integration hooks and validation

**cross_domain_finding / CP slot:** native 0030/character_build currently admit only Training,
VocationChoice and Promotion; no-change Training is rejected. Add Logout/OfflineSettlement
marker shapes and once-per-(Character,lease,cause) uniqueness/replay BEFORE binding comparison,
without translating them into a separate Training receipt. Extend the same build row,
before/after receipt fields, death all-NULL/all-non-NULL/equal OFFLINE fields, deferred chain
guards and verify_character_integrity; bump the native build binding version. Native recovery
and current session/generation/lease/scope/node/root revision checks stay in the owning writer.
DB-trigger clock stamping/`as_of=committed_at` and independently resolved lease acquisition,
immediate predecessor and current GameSession creation clocks must come from the actual FND/
DB owners. Proposed `*_us` inputs are explicitly integer microseconds; the current native build
receipt exposes `committed_at` as Unix milliseconds. The future native reader must perform
checked exact milliseconds-to-microseconds conversion (including overflow handling), never
reinterpret milliseconds as microseconds. Actual DB/FND precision, epoch identity and the
clock-source/row provenance are separate owner obligations; raw planner unit names establish
none of them. Reconnect `predecessor_generation` is not automatically a predecessor lease link.

The admission owner must initialize progression and commit settlement after respawn but before
playability, then pass that committed revision to all chain writers. The actor-end owner must
flush after actor removal and before terminal release, preserving native unknown-outcome
reconciliation/sequencing in training_save.rs. House/bed/statue/WorldUse/Premium and final
current-main logout consumers require CP integration/revalidation; no shared file is changed.

Validation snapshots contain only a literal root path registration outside the draft. Original
native unit-context Clippy checks unchanged test bodies/private imports as metadata. A smaller
production native rlib (cfg(test)=false) runs exact owned bodies through disclosed attribute/
forwarding bridges; no fake native types/authority/fixtures or full unit ELF. Evidence and
source manifests are in `/workspace/oteryn-prep/loop-train-1-evidence`. All full task acceptance
boxes remain pending, including production markers, statue activation and exercise charges.

Final local qualification: original native untransformed unit-context metadata Clippy passed
with `-D warnings`; a freshly captured coherent Cargo dependency graph and the production-native
rlib bridge ran all 11 exact owned test bodies, 11 passed/0 failed. The deliberate Cargo101 /
root sentinel86 is graph capture only, not a test pass. The earlier SQLx proc-macro E0463 is a
retained dependency setup failure, not product RED. No full native unit ELF or PostgreSQL
execution is claimed. Workspace/direct-owned formatting, governance and its 124 tests passed.
The duplicate-source validator lacks complete workspace artifact JSON here; an invalid usage
attempt is retained, with no guard pass claimed. No root wrapper or tracked registration added.
All runtime/DB integration and full task acceptance boxes remain pending.
