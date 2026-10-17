# LOOP-TRAIN-1 — offline training statue and exercise weapons (skeleton task record)

```yaml
task_id: LOOP-TRAIN-1
title: "LOOP-TRAIN-1: offline training statue and exercise weapons (skeleton)"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
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
  - apps/game-server/src/training/offline.rs
  - apps/game-server/src/durability/character_build.rs
  - docs/agents/tasks/LOOP-TRAIN-1.md
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

## D1108 native settlement source increment (2026-10-16)

This checkpoint supersedes the earlier planner-only source inventory. CP D1108
(`6219643346`, relayed by the family lead) authorizes the native Logout/OfflineSettlement
writer; the lead allocated only `training/offline.rs`, the indispensable narrow
`durability/character_build.rs` extension and this card on PR2174's exact head
`7bea1e956dd8ebb21066ab90b7ae223f56a229bb`. No shared authority, DB, progression,
sequencer, registration, migration or caller file is changed in this increment.

The native semantic-pass source independently validates recovery and gameplay
fences, serializes Character/lease/cause, resolves first-marker replay before new
intent/time/formula checks, and reads the DB clock once after locking the native root.
It updates the same build row and v2 before/after marker receipt, advancing root and
progression once; failures roll back the entire transaction, including D88 initialization.
Ordinary v1 build bindings/replays remain unchanged and preserve the OFFLINE fields.
The latest build-carrying chain includes death; an older logout cannot mint offline time.

Settlement calls the existing VITALS stamina helper through a private typed participant,
checks its before value and u16 after value, and records both in that same transaction
and marker receipt. The actual training-due path computes OFFLINE-0 §5's integer
attack/mana gains and calls native `relevel` for the selected family and conditional
shielding. Rate facts are vocation/content/formula-bound. Same-level percent changes
come from a typed native Content port; no guessed percent or copied advancement loop
is introduced. Missing source facts refuse before mutation and preserve selected
skill/pool, rather than silently discarding earned training.

This is still **PARTIAL / uncompiled source**, with these concrete integration seams:

- FND must implement the sealed SQL-transaction clock port using independently DB-stamped
  current session/lease creation/acquisition and the actual immediate predecessor relation.
  Reconnect predecessor generation, receipt values and `lease - 1` are not substitutes.
- The native pinned Content loader must implement top-vocation attack/mana facts and
  the accepted percent comparison under the same content revision/formula digest.
  The current compiled build-training profile contains curves, not these fields.
- External handoff patches supply minimal D88-in-TX extraction, sequencer dispatch,
  integrity-chain fields and visibility/reexports for their existing sole owners.
  They are source proposals, not installed methods or weakened authority guards.
- The CP schema participant must evolve the same build row, build/death before/after
  receipts, marker version/cause/no-op constraints, replay uniqueness and full deferred
  chain guard. No literal DDL, clock columns or migration number is invented here.
- Native admission/logout callers, live flush/actor retirement, house/bed and final
  registration remain separate integration work. Statue activation and atomic exercise
  charges/tries/last-charge are a later commit after settlement.

The exact source/preimage/patch hashes and required native/PG commands are recorded
outside the checkout in `/workspace/oteryn-family/evidence/d1108-offline/manifest.json`
and `FND-SCHEMA-HANDOFF.md`. The preceding packet is preserved under
`generations/typed-stamina-generation-1/`. Two new arithmetic-only tests cover primary
advances, distance floors, conditional shielding, mana zero-tick fallback and missing
facts. Rustfmt/parser passed; these new tests, native typecheck and PostgreSQL cases
were NOT RUN. Earlier 11-test evidence qualifies only the earlier planner source.
All full-task acceptance boxes remain pending.


## 2026-10-16 — D1122 draft checkpoint: NOT_FULLY_QUALIFIED

This is one coherent checkpoint of the existing draft PR #2174 under
[CP D1122](https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6224930201),
not a fully qualified review head. Earlier planner-only/uncompiled/preparation
statements above describe their historical generations. The active checkpoint
owns exactly training/offline.rs, the narrow character_build.rs extension and
this task record; no exercise source, shared module registration or migration is
included. The two Rust files are the unchanged frozen canonical generation3
source, including its two already reviewed reexport attributes.

Actual qualification and open results:

- **PASS — separate private native generation6:** 23/23 exact selected tests,
  zero ignored, including two real native PG cases with mandatory disposable
  PostgreSQL 17.6; the retained source/ELF postflight passed. Inventory is
  11 planner + 2 training arithmetic + 8 stamina arithmetic + 2 PG. The retained
  ELF SHA256 is 49489ebe8c799420442ad8ee885a63fc5224e8caa5e1659985b1407da20b9f6f;
  executed-source SHA256 is
  d4f5c95aa092703d3b976507c836aadd513540f4d73f442c16d70b51a1a32b5c.
  This proof uses the disclosed private registration/shared-owner composition,
  SQL-clock/schema and finite policy fixtures. It does not qualify the canonical
  module graph, actual FND clocks, positive elapsed settlement or runtime delivery.
- **RED — canonical generation3 strict all-target Clippy:** exit101, E0433 at
  character_build.rs:1074 (`crate::training` is absent from the canonical root).
  Source3 postflight remained exact. This is the held shared registration slot,
  not permission to add or hide a module in this checkpoint.
- **NOT_RUN — canonical full-package suite and canonical PG qualification.**
  No private native23 result is promoted to a canonical/full-package PASS.
- **NOT_RUN — complete positive-elapsed/runtime/full-feature acceptance.**
  Actual FND DB-stamped acquisition/creation clocks and immediate predecessor
  source remain held. Independently pinned Content rate/percent ports,
  positive-elapsed end-to-end settlement, native admission/logout flush/retirement
  callers and the final schema/chain/registration bundle remain separate work.
  Statue activation and exercise charges/tries/last-charge are not implemented
  or newly allocated by this checkpoint.
- **PASS — checkpoint pre-push gates:** root ran owned-file formatting with
  exact-base Rust1.94 / edition2024 / rustfmt.toml and git diff --check for the
  three changed paths; both passed before the one authorized checkpoint commit.
  These bounded gates do not replace the RED/NOT_RUN canonical checks above.

The registration dependency is routed at
[CP #2131 comment6224922436](https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6224922436):
`crate::training` / `training::offline` requires an assigned native module slot;
no shared owner path is edited without its handoff. Actual FND clock/predecessor,
positive-elapsed/runtime and final schema/CP delivery remain held as listed above.

The PR remains DRAFT and NOT_FULLY_QUALIFIED. CP D1122 permits only the existing
one coherent checkpoint push after root review and passing owned fmt/diff checks;
it does not authorize ready/merge, a review trigger or CI restarts. Full applicable
checks still precede a frozen review head. No full task completion, production
clock provenance, process/server restart or canonical integration PASS is claimed.

## 2026-10-17 — lost-source rebuild: NOT_FULLY_QUALIFIED

CP [#2131 comment6230877600](https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6230877600)
records that the later private source has no recoverable copy. This increment is
rebuilt from published head `41fffdfe9d1562f30b165f5fc1e93f54d15cbe05`; it is not
that lost generation. The historical PASS statements above do not qualify this
rebuilt source. No private migration0104 or earlier six-PG proof is recovered,
installed or reused, and no new migration number is allocated.

The bounded source increment adds a typed `CommittedOfflineMarker` acknowledgement
for both fresh and replayed native receipts. The transaction-local outcome remains
private; the acknowledgement is constructed only after successful semantic COMMIT.
Its read-only receipt is historical completion, never current gameplay authority.
The native reader additionally refuses zero/mismatched stored marker leases and
non-successor committed revisions. Key replay still precedes fresh intent, clock,
formula and mutation checks, with independent recovery validation retained.

The native clock participant now checks its independently supplied Character,
session, World and lease binding through one reusable validation boundary. An
owned test covers eight independent identity/time/predecessor mutations and the
absence of a predecessor. It does not implement or qualify a clock source.
Exact DB-stamped lease/session microseconds and the actual immediate predecessor
relation remain BLOCKED: no epoch-seconds multiplication, `lease-1`, maximum-lower
lease or receipt-derived acquisition is substituted.

D1222's lost real statue-selection producer cannot be reconstructed from a known
native admission/content owner in this checkout. Its source/contract remains held;
this checkpoint adds no guessed statue IDs, fabricated admission or shared logout
caller. Native marker/schema/chain registration, positive-elapsed settlement,
statue activation and exercise charges/tries/last-charge remain pending. No shared
registration, migration, content or caller path is changed.

Qualification: canonical native typecheck, Cargo tests, PG and runtime/full-feature
acceptance are **NOT_RUN** for this increment. Only bounded owned-source formatting
and diff checks are applicable to this source-publication checkpoint. The existing
PR remains DRAFT / NOT_FULLY_QUALIFIED; no queue, ready, merge or full completion is
claimed. Fresh exact-head independent review and required runtime qualification
remain prerequisites for any later acceptance.
