# BED-1 — house beds: sleeper, bed USE, owner wake, settlement

```yaml
task_id: BED-1
title: "BED-1: house beds: sleeper, bed USE, owner wake, settlement"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/bed-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row BED-1 (row 121); App. 7 LOOP-VITALS-1, LOOP-TRAIN-1, house beds; §5 Wave 5"
class: "hard (persistence, protocol review)"
needs_migration: yes
needs_registry: maybe
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house_bed/ (new)"
  - "apps/game-server/src/durability/bed_sleeper.rs (new)"
  - "apps/game-server/src/house_bed/*_tests.rs"
  - "docs/agents/tasks/BED-1.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_bed_sleepers.sql"
  - "apps/game-server/src/gameplay_transport/item_use.rs / connection.rs (bed USE arm)"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg confirmed)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## 2026-10-16 D1122 checkpoint — NOT_FULLY_QUALIFIED

This checkpoint retains source head
`1a9faa28b046a674c5da5a56c9ace6cd6c54267a` on existing draft #2147.
The implemented component is private sleeper storage/read primitives: native
House/Character identity and uniqueness constraints, immutable occupied-to-freed
state, first wake time/cause preservation, ownership-change fallback, transactional
occupancy revisions and caller-transaction row locking. The authored reconnect and
rollback cases check storage persistence and atomicity; they do not supply a
production restart, logout marker or offline-settlement integration.

| Check | Current-source disposition |
|---|---|
| Authored native cases | Nine: one pure and eight PostgreSQL17.6 cases, including competing free/reconnect and insert commit/rollback coverage. Authorship is not execution. |
| Native compilation and selected native Clippy | NOT_RUN; no current-generation successful compiler evidence is claimed. |
| Native test discovery and all nine case executions | NOT_RUN; the source-only native recipe and mount proposal are not execution evidence. |
| PostgreSQL17.6 qualification | NOT_RUN for this source generation; historical old-unit results and portable/facsimile checks cannot qualify these cases. Missing/non-Unicode URL errors in source rather than reporting skipped success. |
| Production bed USE, wake, logout/settlement and restart | NOT_RUN / incomplete; no production writer or runtime authority is delivered by these raw storage primitives. |

The exact source recipe is
`/workspace/house-family-handoff/2147-validation/manifest.json`
(SHA-256 `acd28e3a8b7c534bd34f5269dc304105a35ac158157f1a44f5759096c0d610e4`).
Its three owned Rust hashes match the current files; it explicitly records
`no_jobs_executed` and does not establish native PASS. No current-generation native
test RED or PASS is inferred from earlier results.

CP D1171 assigns OFFLINE/logout-marker/activation to #2174, durable food/soul to
#2196, and the caller-TX sleeper step and BED-CONTENT-1 head/foot leaf to #2147.
The native call in training/offline.rs follows #2174, and any map/facts.rs accessor
follows #2156. Those unimplemented integrations remain incomplete; the initial
interface request is at
[CP2131 comment6224909931](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224909931).
[CP D1171](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225337534)
resolves the allocation, not the missing implementation or evidence.
The specific-cause free must still precede ownership UPDATE in the same owning
transaction. No caller boolean, raw admin SQL, stored coordinate or separate bed
training backend replaces those native seams. D1162's registration order remains
#2163 → #2141 → #2137 → #2147; no registration or shared wiring is added here.

Fresh focused formatter check on all three current owned Rust files passed:
`rustfmt --edition 2024 --config skip_children=true --check`, official Rust1.94.0.
The exact hash-bound receipt is
`/workspace/house-family-handoff/checkpoint-publication-policy/2147-exact-fmt.json`
(SHA-256 `e0817b37ce1c34d15303eaffad2a8ae6598cc1d3152a1c484882eecd5a655016`). Package fmt remains NOT_RUN.

[CP D1122](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201)
allows one coherent checkpoint push on the existing draft with explicit
NOT_FULLY_QUALIFIED / RED / NOT_RUN accounting and passing fmt/whitespace checks;
the lead verified the current three source hashes and formatter result.
The PR remains draft, with no ready action, `@codex review` request or manual CI
restart. Full checks remain required before a frozen review candidate. All
original acceptance items remain open.

## Goal

Sleeper table, bed USE and sleep, owner wake and offline settlement per BED0 §4-§8.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| BED0 acceptance | ACCEPTED: ACCEPT_BATCH §1, owner item 5 (#2209) | acceptance gate released; child dependencies remain |
| OFFLINE-1 | NOT MERGED | **BLOCKING** |
| STATUE-1 (shared activation path) | NOT MERGED | **BLOCKING** |
| HOUSE-RUNTIME-1a, 1b, 1c | NOT MERGED (Wave 3) | **BLOCKING** |
| HOUSE-VIEW-1 | NOT MERGED | **BLOCKING** |
| WORLDINT-WIRE-1 | MERGED (#2094) | ok |
| BED-CONTENT-1 | MERGED (#1861) | ok |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house_bed/ (new)
- apps/game-server/src/durability/bed_sleeper.rs (new)
- `apps/game-server/src/house_bed/*_tests.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_bed_sleepers.sql
- apps/game-server/src/gameplay_transport/item_use.rs / connection.rs (bed USE arm)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg confirmed)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/content/item_bed_promotion.rs
- apps/game-server/src/gameplay_transport/item_use.rs (USE map_item arm, WORLDINT-WIRE-1)
- offline activation path (STATUE-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability

## Acceptance criteria

- [ ] sleep ends the session through the fenced logout path
- [ ] owner wake and settlement idempotent across restart
- [ ] only the house ACL may use a bed
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house_bed
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md §4-§8
- `docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md`
- `docs/agents/evidence/OTV2-20261005-bed-facts-v1.json`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded local preparation — 2026-10-15, draft #2147

Exact floor `ac3a40da2b2b783b7015b90f3d6d9eb56bcab3f3`, branch
`skeleton/bed-1`. The adjacent four-path allocation governs this preparation;
no historical broad directory grant or registration exception expands it.
Accepted BED0 was read at protected main
`ac9f172836f00226b68088aa04db95ccf1ab994e`; no merge/rebase is performed.
QUEUE_READINESS admits a private sleeper component, not an atomic native writer.

### Implemented private schema/read subset

- `house_bed/sleeper.rs` carries raw persisted rows, the four exact freed causes,
  native appearance::Sex variant, nonzero lease/revision and `StoredBedKey`. Its identity uses the
  existing native `HouseId` (WorldId + revision-free house content key) and native
  `LogicalCell`, never a generated bed UUID or a Channel-local house identity.
  A stored coordinate is explicitly not proof of a valid active-map head pair.
- `durability/bed_sleeper.rs` provides private schema SQL: native property/Character
  FKs, Character-to-World binding, one Character row including freed rows, one
  open sleeper per native BedKey, nonzero u64 lease, closed variant/free causes,
  paired/timestamp-ordered free fields, immutable sleep identity and a one-way
  occupied→freed transition. Freed rows cannot be overwritten/reopened.
- Private `AFTER UPDATE OF owner_character_id` fallback on the actual native
  ownership table frees only open rows with `owner_change` at the database clock,
  after taking the house revision row lock. Same-owner revisions do not wake;
  already-freed rows retain their first time/cause. The native ownership guard
  and writer source/permissions are unchanged. This DDL is not installed.
- Insert/free/delete trigger the house occupancy revision in the same transaction;
  rejected writes and rollback retain the prior revision/state. Revision overflow
  fails the whole mutation. This is storage bookkeeping, not marker provenance.
- Actual single-statement raw read includes the house revision. The caller-owned
  transaction variant takes `FOR UPDATE OF s`, pins an existing row and neither
  starts/commits an independent transaction nor frees/deletes/awards anything.
  Missing revision, bad lease/identity/variant/time/free pairing fail closed.
- `sleeper_tests.rs` exercises the exact owned source and actual native public
  types/SQLx/Tokio, native migrations and isolated PostgreSQL 17.6 fixtures. Native
  root/property fixture insertion is migration-owner setup, not an admission,
  acquisition or current-grant witness. Admin SQL tests private row lifecycle;
  no executable production insert/wake/settlement writer is offered.

`slept_at` and `freed_at` use **Unix milliseconds**, matching the actual native
`character_build.rs` `committed_at = floor(epoch * 1000)` representation. The
owner-change fallback uses database `clock_timestamp()` in those same units.
The OFFLINE owner must own typed elapsed-second conversion/settlement arithmetic;
this component introduces none. A real PG regression with native millisecond
`slept_at` exposed the old seconds fallback (6 PASS / 1 FAIL), now corrected.
Earlier six-test GREEN and native checks are explicitly the superseded old-unit
variant, not evidence for this final source.

No schema is installed in production; no grants, numbered migration, shared
registration, runtime effects, food consumption or vitals/skill truth are added.
All family acceptance checkboxes remain open.

### cross_domain_findings / precise integration packet

1. Native `HouseId` is already public and World-global. Native qualified house
   placement knows House content identities and logical cells, but no active-map
   bed pair/head normalizer is exposed here. BED-CONTENT-1 supplies authoring
   `ReferenceItemBed` facts; these alone do not prove an active pair. USE, choice,
   reservation, insert, view and bed-removed checks must share the real current
   bundle's reciprocal same-house head/foot resolver. Raw stored coordinates
   cannot substitute for that qualification.
2. `game_house_ownership` / `house_access_for_character` supply native ownership
   and ACL reads, but the current property row lacks BED0's OWNED /
   MOVE_OUT_PENDING / content-fence model, and this lease has no accepted guild-
   derived revision grant carrier. The writer must take the actual property
   `FOR SHARE` plus granting guild/membership locks, revalidate current revisions
   and current session/lease/scope/Premium/block/reach conditions. A caller's
   boolean or this raw row is not current authority. Owner-wake/bed-removed remain
   house-runtime-only; access revocation is not a wake cause.
3. `character_build::BuildCause` and migration 0030 admit training,
   vocation_choice and promotion, not logout/offline_settlement markers or
   offline activation storage. Reuse the existing build state/formula/receipt
   owner; do not create a separate bed training or vitals backend. The current
   `FreshAdmissionStore::release` owns terminal release, final position and its
   lifecycle receipt, with no sleeper/build-marker transaction hook. OFFLINE-1 /
   STATUE shared activation and real house logout/admission hooks are required.
4. Logout bundle: exact first-receipt replay before mutations; actual marker
   timestamp/lease and activation, and the native Character sex at insert; cleanup of the Character's older sleeper row;
   qualified new head insert. Bed unique conflict must commit the marker without
   sleeper or activation, not leave training enabled. Canonical multi-house
   revision locking must include the old row's house when cleanup moves between
   houses. Triggers preserve revision atomicity but cannot manufacture the
   required marker/property/grant provenance or lock order.
5. Owner/disposition bundle: real house runtime or ownership transaction acquires
   its property/house-revision locks, then frees only open rows at the database
   clock with the exact accepted cause. A second wake is a no-op preserving its
   first time/cause. The private fallback supplies only actual owner-column changes;
   ACL revocation does not wake. **Mandatory CP ordering:** a disposition or other
   specific-cause free must run BEFORE the native ownership UPDATE in the same
   transaction. Otherwise the generic AFTER fallback would record `owner_change`,
   and the one-way guard correctly refuses rewriting that first cause. There is
   no native operation-cause seam here and no invented context flag. Production
   installation remains blocked until the actual property/disposition writer
   guarantees this order (or owns an accepted explicit native cause seam).
6. Admission bundle: actual offline-settlement transaction reads/locks the row;
   only matching previous lease plus that lease's latest logout build receipt
   count. It deletes any row, including stale/freed, with revision advancement
   and first-receipt replay. `sleep_s=min(d,freed_duration_s when freed, otherwise d)`, cap
   21 days; training still runs over OFFLINE0's d after owner wake. No plain
   caller-supplied latest-marker flag or standalone delete is accepted as this
   integration. Apply that accepted seconds-duration rule through the OFFLINE
   owner's conversion from native Unix millisecond timestamps; this raw component
   does not guess settlement conversion/rounding. BED-REGEN remains separate until
   DUR-02 durable vitals: no food
   consumed and no regeneration coefficients promoted from PARITY_PENDING.
7. CP slot must install schema + permissions/marker provenance guards + actual
   native writer/hooks inseparably, then revalidate then-current main and test
   fence/revision changes, marker conflicts, ambiguous commit/replay, disposition,
   stale previous lease, head/foot equivalence and settlement/value conservation.
   No registry, wire, shared module or content/pin edits are prepared in this lease.

### Evidence

Commands, original logs, initial/final source hashes, native snapshot registration
manifest and scope/self-review are in
`/workspace/oteryn-prep/bed-1-evidence/index.md`. This is private lifecycle/read
qualification only, not live sleep, wake, offline training or settlement delivery.

## Owner-assigned authoring repair — 2026-10-16

The owner directly confirmed exclusive assignment of this existing draft to the
current family lead and its sole BED-1 author; the previous team will not write.
No additional CP lease ID is required. This instruction supersedes the earlier
missing-allocation-artifact uncertainty; it does not broaden this round's four
paths: `house_bed/sleeper.rs`, `house_bed/sleeper_tests.rs`,
`durability/bed_sleeper.rs` and this card. The authoring floor remains exact PR
head `2d5be266946baae97f988f7cc46f6415464b1ba7`, worktree
`/workspace/bed-1`. No shared registration, migration, publication or review
trigger is authorized in this round.

CP finding P2 (silent PostgreSQL success) is **fixed in source, execution pending**:
`run_case` now returns an error when `OTERYN_TEST_POSTGRES_ADMIN_URL` is missing
or not Unicode. There is no success/skip fallback. The existing canonical local
admin-route restriction and exact PostgreSQL `server_version_num=170006` check
remain mandatory. No credential is printed by the new error.

Two additional PostgreSQL cases exercise the actual existing private schema:

- Competing owner-wake/bed-removed storage transitions: an uncommitted first free
  blocks the loser (`55P03`), the committed first time/cause remains unchanged,
  and the losing retry affects zero rows with no extra occupancy revision. A new
  connection reloads the same first result and repeats the losing no-op.
- Competing inserts into one bed: a pending winner blocks its competitor; commit
  preserves uniqueness (`23505`) without an extra revision. A second house case
  rolls back the initial insert, lets the competitor succeed, and verifies the
  rolled-back Character row/revision did not survive or affect the other house.

These are migration-owner storage tests, not runtime authority or a real process
restart qualification. The new connection reload is not a claim that restart,
OFFLINE receipt replay, current access or a production wake writer is delivered.

On this exact authoring floor `BuildCause` still contains only `Training`,
`VocationChoice` and `Promotion`; no native logout/offline marker transaction
exists. `FreshAdmissionStore::release` has no sleeper-marker hook. No registered
house runtime supplies current house/node/assignment, active head-pair or
owner-wake/bed-removed proof on this branch. An ACL read alone cannot provide
those missing facts. Therefore production sleep/free/delete APIs are not added:
the affected integration remains with OFFLINE/STATUE and HOUSE owners. A missing
accepted owner seam is not replaced by caller booleans, raw admin SQL or a new
authority trait. The disposition-before-ownership-UPDATE requirement above
remains unchanged.

Validation in this round: `git diff --check` and the governance validator pass;
all 124 governance unit tests pass. The family lead formatted the owned
`sleeper_tests.rs` source with verified official Rustfmt 1.94
(`/workspace/house-tools/rustfmt`, edition 2024, `skip_children`); focused source
formatting passes. Package `cargo fmt`, Clippy, all nine sleeper tests and
PostgreSQL remain unrun: D1011 requires the owner-PC executor and no global PC
slot is available. The executor must register/qualify these owned modules under the
actual native test manifest and run all nine sleeper tests (one pure, eight
PostgreSQL 17.6), plus required package formatting/Clippy. Missing URL must fail
the PG cases; canonical 17.6 must execute them, not report skipped success.
All family acceptance checkboxes remain open; this repair does not close BED-1.
