# ITEM-STORAGE-1 — item storage: depot, supply stash, writable items (skeleton task record)

```yaml
task_id: ITEM-STORAGE-1
title: "ITEM-STORAGE-1: item storage: depot, supply stash, writable items"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
pr: 2156
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row ITEM-STORAGE-1; §7 'ITEM-STORAGE-1' (DEPOT0, SUPPLY_STASH0, WRITE0 child graphs)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-storage-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Characters store items durably in a World-wide per-Character depot and a supply stash, and read/write books, scrolls and blackboards, all through the ITEM-MOVE-2 transaction family.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

DEPOT-CONTENT-1 -> DEPOT-1 -> DEPOT-WIRE-1; STASH-CONTENT-1 -> STASH-1 -> STASH-WIRE-1; WRITE-CONTENT-1 -> WRITE-1 -> WRITE-MAP-1 -> WRITE-MOD-1. WRITE-WIRE-1 is excluded from this completion set.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING (Wave 1, not merged) |
| acceptance of DEPOT0 (plan §7 gate) | ACCEPTED on protected main 3ef900c; stash/write are outside this allocation |
| HOUSE-CUSTODY-1 exclusivity guard (DEPOT-1) | VERIFIED migration 0025 exists; depot join remains CP integration work |
| ITEM-MOVE-2a / 2b (DEPOT-1, STASH-1, WRITE-1) | BLOCKING (ITEM-LOOP-1 children) |
| ITEM-VIEW-1b (DEPOT-WIRE-1) | MERGED (archive OTV2-20261003-item-view-1b) |
| MAP-LOAD-1, MAP-WIRE-1 (MAP-WIRE-2) | MERGED (archive MAP-LOAD-1, #1810) |
| WO-1, BANK-RET-0 | MERGED (#1748) |
| MARKET-CONTENT-1 (STASH-CONTENT-1) | BLOCKING (MARKET-1, Wave 4) |
| MAIL-TEXT-1, BAGS-1 (WRITE-1) | BLOCKING (no record on main) |
| HOUSE-RUNTIME-1a/1b/1c (WRITE-1 house items) | BLOCKING (Wave 3, not merged) |
| LEVER-1 (WRITE-MAP-1) | BLOCKING (no record on main) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (exclusive bounded root allocation)

- `apps/game-server/src/durability/character_depot.rs`
- `apps/game-server/src/durability/character_depot_tests.rs`
- `apps/game-server/src/gameplay_transport/depot.rs`
- `apps/game-server/tests/item_storage_postgres.rs`
- `apps/game-server/src/map/facts.rs` — CP D1142: exact depot attribute/placement/definition only
- `docs/agents/tasks/ITEM-STORAGE-1.md`

No item_text, stash, protocol or content path is allocated. D1147/D1160/D1182
below grant only the listed target/module declarations and protected pin repair;
D1122 permits the disclosed existing-draft checkpoint. Additional shared hunks,
migrations, native transfer/audit hooks and runtime wiring remain CP slot work.

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

- connection.rs dispatch chain (DEPOT/STASH intents near NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` l.165 and offered sets l.76/104/132
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568
- durability/mod.rs registration; HOUSE-CUSTODY-1 exclusivity guard call

## Inseparable bundles (never split across PRs)

- DEPOT-1 / STASH-1: migration + container writer + move transaction + audit
- WRITE-1: text payload limit (RESOURCE_LIMITS) + writer + tests
- each *-WIRE-1: wire + registry + capability + tests

## Acceptance criteria

- [ ] Depot contents survive restart and are scoped to Character + World, shared across towns and channels (DEPOT0 §3). Stash remains outside this allocation.
- [ ] Every store/withdraw is one ITEM-MOVE-2 transaction; no item duplication under concurrent moves.
- [ ] Depot access excluded while the same item is under house custody.
- [ ] Written text respects the RESOURCE_LIMITS cap and is sanitized.
- [ ] Older clients receive no depot/stash domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Family integration validation commands (not claimed for inactive preparation)

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server depot
cargo test -p oteryn-game-server stash
cargo test -p oteryn-game-server item_text
cargo test -p oteryn-game-server item_transfer
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md
- docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md
- PRs #1810, #1748
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded DEPOT-1 packet, before code (2026-10-15)

Root supplied META 3.1.0 and the exact five-path lease on existing draft #2156,
`skeleton/item-storage-1`, base 89f5598db8db824e5793ae3e349dc4197bf2bed0.
DEPOT0 is ACCEPTED on protected main 3ef900c160c9713092a86bfcd1a8377c7587c898;
this supersedes only the historical acceptance blocker above. Family/child
integration gates and all acceptance boxes remain open. Scope is the accepted
first slice, not BAGS trees, Inbox, stash, writing or mail.

### Source-grounded component

DEPOT0 §§3,5,6,8: one World+Character custody, no Channel/town/runtime scope;
boxes 1..17, positive never-renumbered ordinal, empty items only, one row per item
regardless of quantity; 15,000 hard entries, 2,000 Free / 15,000 current Premium;
32 entries in one page; each of two TRANSFER shapes is 1 item / 2 locations /
3 work units. Taking out is allowed after Premium lapses, with no stack merges.
Native PremiumGate is a raw planning input, not a new current-authority proof.

Prepare a private uninstalled table/guard proposal, validated custody/read models,
bounded summary/page reads and capacity planning. Same-World integrity must bind
both native item FK and Character root; capacity guard locks the root and counts
rows. PostgreSQL tests apply the exact proposed SQL in an isolated 17.6 database;
no production migration/schema/grant is installed. Reads do not open views or
prove ownership. They refuse inconsistent/corrupt counts and limit page reads.

Prepare only raw shape planning over actual native item/definition/backpack
models: direct main-backpack entry into a box, or depot entry out to the empty
container slot when eligible, otherwise one new direct entry. Reuse the native
stack and container-slot planner checks; no fabricated definition facts, existing
merge/top-up path, guessed shape number or independent item relocation.

### cross_domain_findings / withheld value writer

- HOUSE-CUSTODY migration 0025 exists: its item exclusivity function counts the
  native location families, not a depot table. Join depot (and reject contents)
  in that function, including later native amendments, in the CP migration.
- Native item_transfer source/destination/TransferShape and audit locations are
  closed to Ground/Corpse -> container slot/backpack and merge shapes. Add the
  accepted two depot shapes, depot source/destination_box and nullable custody
  channel to reservation/receipt/audit proof composition; preserve every prior
  body and guard. No native enum is changed in this lease.
- Backpack deletion already has non-depot proof clauses (latest replacement 0083:
  equipment, spell/temporal, timed, bank, trade/use and fee cases), but no depot
  TRANSFER clause or callable native depot primitive. CP must preserve that complete
  union while composing entry removal and depot
  insertion/removal evidence, game_item_placement_proven and consistency/exclusivity
  in the same receipt/audit transaction. This component supplies no dead writer.
- Current character_item_fence_is_current is explicitly Channel scoped. Depot
  custody is World scoped; accepted composition rule 2 still requires current
  session/lease/root checks, but not depot runtime ownership. The owner must reuse
  the appropriate current fence boundary without manufacturing scope authority.
- No typed current depot locker/open-view/item-handle owner exists. Recheck current
  reach (Chebyshev distance 1, same floor) and generation/session-bound live view
  at command execution, including after reconnect, before any future value commit.
- Root -> items in ID order -> container-slot lock, replay/ambiguity reconciliation,
  item-only CharacterRevision behavior, capacity at commit and all atomic proof
  obligations remain part of the future shared writer. Raw planner success is not
  permission, reservation, item lock, receipt or committed transfer.

Durable evidence: `/workspace/oteryn-prep/item-storage-1-evidence/`.

### Ordinal representation and validation setup

The proposed schema/read/planner use native B3 u64 ordinal representation (0011
placement_ordinal and destination_ordinal range 1..18446744073709551615; native
ContainerEntryPosition is u64), including checked overflow refusal. DEPOT0 names
NUMERIC(20), without a separate decimal-ordinal API. This is explicit private
native-adapter preparation, not a new resource registry bound or an architecture
claim about values beyond native u64. Preserve the owning ordinal representation
at CP integration; a wider decimal domain would require its actual adapter.

Seven literal owned rule/planner tests passed in a cfg(test)=false production-native
rlib bridge using unchanged actual native types/private planner; no global unit ELF
was linked. The first separate PostgreSQL-target compile failed SQLx0.9 E0521/E0597:
controlled dynamic fixture Execute strings require AssertSqlSafe. Only that target
was corrected; no PostgreSQL behavior ran in the failed setup and no behavioral
RED is claimed. Original target source/argv/diagnostics and the seven-case source
binding are retained. The unstarted-waiter cancellation check raced with completion;
no process was signaled. No unchanged seven-case rerun is needed for this fixture fix.

### Actual reader regression and REPORT_ONLY sibling finding

Canonical PG17.6 exposed a real first-page failure: the projection
ordinal::text AS ordinal made unqualified ORDER BY ordinal sort text, e.g.
9999,...,9990,999,9989. The preserved behavioral RED is 3 pure passes / 1 PG
failure (InvalidStoredState), distinct from the earlier SQLx setup compile error.
The minimal repair qualifies the numeric table column in the ORDER BY. Final
by-path PG qualification, not the old native read symbol, executes that reader.
qualification-source-matrix.json binds unchanged native 4 planner/test file and
unchanged called storage helper/type bytes; no final-whole-file native 7 run is
claimed. The final PG target repeats the 3 storage pure cases against final source.

REPORT_ONLY P2, outside lease: character_inbox.rs read_character_inbox lines 129/132
and read_character_inbox_deliveries lines 178/181 use the same text projection /
unqualified numeric ORDER BY alias. Native floor 89f5598 and protected main ac9f172
are byte-identical; exact full SHA256 is recorded in
inbox-report-only-finding.json. The read-only 170006 SQL probe returns 1,10,2; an
explicit numeric table qualifier returns 1,2,10. Delivery-order readers misorder
cross-digit ordinals, and any future consumer paginating those vectors inherits
wrong page membership/order; no shipped Inbox pagination or value mutation is
asserted. Minimal owner patch: ORDER BY game_item_character_inbox_locations.ordinal
and ORDER BY game_character_inbox_deliveries.ordinal respectively. Root carries
this finding to CP/architect; no unowned file was edited.

### Boundary classification

Identity/binding checks cover Character+World and item+World integrity, box/ordinal
bounds and uniqueness, item definition equality and raw source/contents rules.
Storage capacity consumes independent database counts under a Character root lock;
it does not infer current ownership from a prepared count or row. Current session,
lease/generation, locker/view liveness, admission and provenance are not granted by
any planner/read return and remain future owner boundaries. Expired/replaced fences,
reconnect substitution, reservation replay and native audit causation are therefore
NOT_APPLICABLE to this component as mutation operators; they are explicitly not
qualified for a future writer. Ordinal overflow, immutable-row mutation and database
count/World substitutions are applicable and covered. There is no new timer or
CharacterRevision mutation. Withdraw ignores raw PlanInput.destination and selects
the accepted slot/direct-entry branch from the raw slot state; no new command choice
or public wire semantics is assigned.

Final source sweep corrects the historical fee-only statement in the original
before-code packet: migration 0083 is the latest backpack-removal proof union;
0076 is the latest physical exclusivity union (including equipment and Inbox).
The actual missing delta is a depot TRANSFER clause/family, not the absence of
all non-fee inventory deletion. Existing equipment-specific removal proof cannot
serve as depot placement/audit proof. CP must extend these latest complete bodies,
plus placement/reservation/receipt/audit source/destination unions, preserving all
prior admitted shapes. The immutable before-code packet is historical evidence;
this final task record carries the corrected source-grounded delta.


### Final bounded qualification (2026-10-15)

Eight unique owned scenarios are qualified: seven raw/native-planner cases before
an SQL-only reader repair, and one final proposed-schema PostgreSQL scenario. The
final standalone real-public-import PG target passed 4/0 on PostgreSQL 17.6
(server_version_num 170006), including three repeated storage cases. The source
matrix binds unchanged planner and called helper bodies; final whole-file native
seven execution is not claimed. The actual ordering regression RED 3/1 and both
setup failures are preserved separately.

Final original-unit-context native Clippy metadata passed with warnings denied,
including the exact PG target source in a validation-only self-alias root. The
standalone PG compilation/run independently proves real external public imports.
A coherent Cargo graph refresh intentionally stopped the root compiler at sentinel
86 (Cargo exit 101), before global unit codegen: this capture is not a test/check
pass. Immediate metadata Clippy under the same lock returned 0. Two duplicate_mod
allowances exist only at duplicated snapshot import boundaries; no owned production
bytes or lint settings were weakened. Original registrations remain unchanged.
Final source bindings distinguish the refreshed lint dependency graph from earlier
runtime artifacts rather than claiming a single unchanged graph throughout.

Direct formatting of all four unregistered Rust files and workspace format checks
passed. Governance validated 22 documents/9 lanes and the unchanged validator suite
passed 124/0. Whole owned diff self-review and exact five-path/empty-index/base checks
are retained. Durable final source copies, original roots, restoration inputs,
all RED/GREEN/setup/lint logs, hashes and local.patch are in the evidence folder
above. No native value writer, installed schema/grants, runtime integration or family
acceptance is claimed; CP composition and independent architecture review remain due.

### Owned continuation (2026-10-16, authoring from `0dbcda9024a1f75b7479ab645bef7a2709fa933d`)

The lead supplied exclusive family authoring authority and retained this record's five
owned paths; publication and shared-path composition remain with the lead/CP. The prior
missing-lease concern is superseded. No STASH, WRITE or item_text work is included.

Corrected an inconsistent raw withdrawal snapshot: an empty container slot accompanied
by facts for an equipped backpack now returns `InvalidSnapshot`. Previously it reached
the native container-slot planner, which intentionally does not examine backpack facts,
and accepted the withdrawal. The regression changes only those facts from a valid spare
container withdrawal; source identity, definition, quantity and empty contents stay valid.

The proposed-schema PostgreSQL qualification now attempts a competing depot insertion,
instead of manually requesting a root lock. The insertion or its forced deferred guard
must time out on the winner's Character root lock; rollback must preserve the winner's
uncommitted count. This qualifies proposed storage contention only, not a native move.
The native existing `item_storage_postgres` target passed 4/4 on PostgreSQL 17.6
(server_version_num 170006), including the actual competing INSERT and rollback case.
In a disposable validation copy, test-only registrations of the exact owned modules
connected the native library to the withdrawal regression. RED failed as expected:
the pre-fix condition returned `Ok(BoxToBackpack)` instead of `InvalidSnapshot`.
After restoring the corrected source, GREEN passed all seven depot tests (three
durability owner tests and four planner tests; zero ignored, 3,179 filtered). The
restored planner SHA-256 is
`aba5f796dcea322114916799fabfa68583efb7ee52089ef62131fdc43a02433e`.
Rustfmt and `git diff --check` passed; strict Clippy remains pending. These native
validation overlays do not install a production depot owner or qualify a native move.
No branch registration, visibility widening, copied native planner or shared-source
edit is included in the candidate.

The native transaction remains blocked by shared composition, not by authoring authority:
`ItemTransferDestination` admits only `ContainerSlot` and `MainBackpack`;
`commit_item_transfer` and `character_item_fence_is_current` currently encode Channel
pickup authority, while depot custody is Character + World. Existing placement/removal
proofs and physical exclusivity do not admit depot rows, and native receipts/audit have
no depot shape. These shared owners must change together before either depot move can
commit. The raw readers/planners confer no current session, locker reach or open-view
authority. No duplicate SQL owner or alternate writer is introduced to bypass those gates.

An owned-target planner routing investigation found a concrete visibility barrier:
`item_transfer::{PlanInput, plan_transfer, stack_maximum}` are `pub(crate)` and cannot
be imported by reexporting the native library into `item_storage_postgres`. Path-loading
`item_transfer.rs` would duplicate its test-bearing source into a second strict harness,
contrary to `tools/repository/check_duplicate_test_sources.py`; the shrinking allowlist
does not include this target. The inspected `character_forge_dust_postgres` wrapper
has `test = false`; that is a different target and establishes no manifest fact about
`forge_postgres`. No Forge-target claim is needed for the visibility/duplication barrier.
No copied planner or test shim was added. Production planner connectivity still needs
shared library registration; that edit is outside the owned-path packet. Qualification uses test-only registrations in
a disposable validation copy of the native library, with an exact delta manifest
retained by the lead. This validation routing does not install the depot owner or
authorize a native move.


### Terminal private qualification checkpoint (2026-10-16)

The frozen private test input is `PC_SOURCE_PACKET.json` SHA-256
`ce0c7dc10623a9b54971a7b5ddff1f9cf2101aed34b4123efc9f2c8675ea792d`,
full source manifest `75fe38c2dd11e827a26d637c0747722175141c6ff8ea86e8ec88ff2065883e4b`.
Its historical task-card bytes precede this documentation candidate.

Historical combined run `evidence-run-1792167456282407854` in
`PC_FINAL_EVIDENCE.json` passed fmt and strict package all-target Clippy but
failed package tests (exit 101): library 3150 passed / 29 ignored; later
`character_authority_postgres` 1490 passed / 1 failed at
`postgres_target_aggregation::every_postgres_target_runs_its_cases_in_a_ci_run_target`.
No focused PG stage executed in that historical run. Launch 041 subsequently
held before tests on the occupied canonical PG lock (observation 043), neither
a focused test PASS nor a test FAIL.

Actual focused run `evidence-run-1792176419124648929` is retained in
`PC_FOCUSED_FINAL_EVIDENCE.json`, including exact native commands, logs and
hash-verified ELF archive receipts. Runner V3 SHA-256
`6f86ba9abef7c0dbb611a9bb59b93f3fcc39531253f58016ce67907700fac14a`;
recipe V3 `8aedcff0eda55fb0d6b61ffa023aaa118e7bad1bc95e42064d7298bc30dace56`.
Forge passed 9/9, including three actual PostgreSQL 17.6 cases. Imbue passed
22/22 (12 pure + 10 actual PostgreSQL 17.6 cases, each asserting 170006),
including complete-before-binding, with the same unchanged Root deadlines.
Storage passed 7/7 pure native depot tests in the test-only registration overlay;
its four actual PostgreSQL cloud cases remain separate historical evidence.
The earlier cloud Imbue 21/22 `RootPassDeadlineExceeded` failure remains retained;
this later PASS establishes no causal diagnosis for that earlier failure.

Cleanup is clean, source mode plain, with no remaining owned process group.
`PC_CALL_047_RESULT.json` attests `RELEASED_NO_ACTIVE_CLAIM`, supervisor 163594,
start ticks 2603394, boot `1de2caf4-8501-4962-98e7-73177714c673`.
These are private fixture results, not production installation or exact-head CI.

The exact shared CI remedy `CP_PG_TARGET_REGISTRATION.patch` and rationale `.md`
remain UNAPPLIED: register the existing Storage/Forge targets in
`.github/workflows/merge-group-gate.yml` through the existing helper. This shared
workflow slot and qualifying package/CI results remain required for this target.

Other existing product integration slots and full mechanic acceptance remain gated.
No Rust, workflow or shared source changed here; no commit, publication, new PR,
ready, merge or complete package qualification is claimed.

### Local recovery checkpoint (2026-10-16)

Owned Rust bytes retain the qualified native depot7/7 pure result and the separate
cloud4/4 actual PostgreSQL result. Imbue's later normal-package PASS belongs to
its different frozen input. This Storage-only full normal package is NOT_RUN;
the combined input's actual aggregation-guard FAIL above remains RED.
Publication is held under root AGENTS.md47 (run local changed-path checks before
pushing and fix all failures of a run in one push) and CONTEXT_ROUTING.md68
(Rust changes require fmt, strict all-target Clippy and normal package tests).
The two-line shared workflow registration remedy remains unapplied pending its
CP slot. The owned candidate is secured as a local commit and verified recovery
bundle outside this checkout; exact identifiers are in the family recovery record.
No shared file, ready, merge or rebase is part of this checkpoint.

### D1122/D1147/D1160 checkpoint — NOT_FULLY_QUALIFIED

CP comment6224930201 (D1122) explicitly permits one coherent checkpoint push
to this existing draft before full qualification, with fmt/diff checks and honest
RED/NOT_RUN disclosure. This supersedes the publication hold above only; it does
not grant ready, review, merge, CI restarts or product authority.

CP comment6224931148 (D1147) assigns this PR its own existing PG target
registration now. The workflow adds only `item_storage_postgres` through the
unchanged `run_registered_target` helper; Forge is not registered here. CP
comment6224912485 (D1160) allows registration-only lines for this PR's own
modules: `durability::character_depot` and `gameplay_transport::depot` now join
the actual library. No other shared module code changes in this checkpoint.

The unchanged native std-only `postgres_target_aggregation.rs` guard is executed
against this actual checkout before and after registration. Its previous RED
specifically named this unregistered Storage target; exact receipts and all
checkpoint checks are kept outside the checkout in
`/workspace/oteryn-family/storage-checkpoint-d1122/`.

Retained source-compatible evidence: depot7/7 pure and separate cloud4/4 actual
PG17.6; prior combined all-target Clippy/fmt PASS and aggregation RED remain
historical, not an exact checkpoint PASS. Full normal package, all-target Clippy,
new canonical module compilation and exact-head CI are NOT_RUN for this new
input unless separately recorded below. Protected workflow pins are not changed;
any validator RED remains explicit. This is NOT_FULLY_QUALIFIED.

Runtime destination/fence/audit/receipt/guards remain after #2190 and #2163 per
D1120/D1142. Migration0102 is reserved by D1158, not installed here. Exact depot
placement/definition binding belongs to this PR's `map/facts.rs`; locker/container
must come from an admitted world-bundle, otherwise UNKNOWN, with no substitute
fixture. Native depot value moves and complete mechanic acceptance remain gated.

Actual registration check: the unchanged native aggregation guard compiled from
its complete source with rustc1.94, then passed 1/1 after registration (before:
0/1, with only `item_storage_postgres` named). This isolated std-only guard is
not package/PG execution. Repository policy is RED/E8002 (reviewed MQ blob
contract); the matching workflow regression is RED at its protected blob pin
`d7cfbf90b2b81c43f92c010660f41d242a3cf840`. Both are retained without changing
pins or validators. Exact owner request: CP comment6225410275. D1122 permits
this disclosed checkpoint; it remains NOT_FULLY_QUALIFIED.

### D1182 pin repair — NOT_FULLY_QUALIFIED

CP comment6225580190 explicitly gives `pin OK` after checking the exact one-line
Storage workflow diff and blob `50d0ff3c1f2edc545a79c7fc59d85b0654c9a18c`.
It assigns this PR both protected blob constants: `APPROVED` in
`test_validate_merge_group_pg_sim.py` and `EXPECTED_MERGE_GROUP_GATE_BLOB` in
`validate_repository_policy_core.py`. Only those constants change; validator
logic, guards and other pins remain untouched. This supersedes the pin-owner
block above, while its actual earlier RED receipts remain historical evidence.
The workflow itself is unchanged from checkpointc40a5469; Rust source/registration
and successful full-format evidence are also unchanged. Exact repair checks are
kept in `storage-checkpoint-d1122/pin-repair/`. Full normal package, all-target
Clippy and PG/runtime qualification remain NOT_RUN for the assembled checkpoint.

Actual pin-repair checks: repository policy now PASS (23 files, 63 workflows),
governance and diff-check PASS. The matching workflow regression passes its
policy/Atlas/World-bundle stages, then remains RED because this executor has no
`pwsh` for WP1 native failure-propagation canaries. That stage and the following
stages are not qualified; no complete workflow-regression PASS is claimed.
Successful Rust fmt is retained against byte-identical Rust files from c40a5469.


### D1142 depot placement facts — NOT_FULLY_QUALIFIED

The existing qualified map visitor now preserves the exact top-level depot
attribute as `Option<u16>`, including zero. `BundleFacts::depot_placement` resolves
only an explicitly observed materialized Item placement: strict native placement
key, actual Item palette family, exact compact ID and the already-qualified Item
definition reference. Missing attributes, wrong placements/definitions, nested
entries, Terrain attributes, dropped teleports and duplicate recorded keys refuse.
Existing bound/count/subtype behavior remains unchanged; no palette-ID fallback
creates a depot observation.

This is raw immutable map data. Callers must bind it to the active bundle
generation. It grants no locker/container admission, custody, session authority,
view handle or durable transfer permission. Actual admitted locker/container
content remains required, otherwise UNKNOWN. D1142 authorizes this exact
`map/facts.rs` increment; no new shared slot or migration is used.

Four new native-source unit tests exercise the actual map visitor with decoded
engineering bundles. They cover observed attribute extrema and base behavior,
wrong identity/position/bounds, Terrain/nested/missing attributes, dropped teleport
and duplicate keys. These fixtures are not deployed-world or content parity
proof. Independent source review, complete Rust fmt, diff and governance PASS.
Actual normal native map-facts target validation is recorded separately in
`/workspace/oteryn-family/storage-d1142/`; no new PG/runtime/fullpackage or
strictalltarget qualification is claimed. Earlier RED receipts and their exact
source scope remain preserved.


Actual same-source native validation: `cargo test --locked --offline -p oteryn-game-server --lib map::facts::tests -- --nocapture` PASS: 9/9, zero failed/ignored, 3181 filtered. Four cases are
new; five existing map-facts tests also pass. The actual normal server library
test target compiled in this invocation, elapsed482.377s including tests;
logSHA256 `82ca2d6c8d90c40d4aae0a698355376971e9f43fb4748144594ac310957ed76c`.
Five existing unused depot declarations still warn; no lint suppression or
visibility relaxation was added. This focused result is not a fullpackage,
strictalltargetClippy, PostgreSQL or runtime PASS.

### Exact-head Clippy repair (2026-10-17)

CI Clippy at `2db7e6529` was RED only on `dead_code` (`-D warnings`) for the
unwired DEPOT0 declarations in `durability/character_depot.rs` and
`gameplay_transport/depot.rs`; the aggregate merge-gate and game-gate REDs
followed from it. Both owned modules now carry a module-level
`#![allow(dead_code, reason = ...)]` naming the pending CP-composed TRANSFER
caller, matching the existing `ability/condition_spell.rs` precedent. No
visibility, logic or test change. This supersedes the "no lint suppression"
note above; the protected-base merge-authority audit RED (protected main still
pins merge-group gate blob `d7cfbf90…` against this PR's CP-approved
`50d0ff3c…`) needs an owner-authorized audit rotation on main and is not
changed here.


## 2026-10-17 native registration repair on current draft

Current authoring base: `46fd398195cb7ae24403fdf837b8597672ac4293`.
The coordinator's actual native Clippy run on that head passed. The Cargo
artifact/dep-info duplicate-source guard then rejected
`src/durability/character_depot_tests.rs`: its three pure cases were mounted
both by the native library and by the path-loaded owner in
`item_storage_postgres`. This is a separate executed guard failure, not a
PostgreSQL qualification result; the PG suite had not started.

This bounded repair removes only the nested test registration from the native
owner and mounts the same three pure cases explicitly in the existing
`item_storage_postgres` target. The target therefore owns the same four cases
once: three pure bounds/classification cases and one PG schema/read case.
The test imports name the exact path-loaded owner and the library's real
`domain::premium` types. All three pure function bodies, the PG function and
its SQL fixtures/assertions are unchanged. Native owner definitions and SQL,
module visibility, target registration, workflow and guard allowlists are
unchanged.

The current user authorized completing this named draft; root is the sole
publication and remote qualification coordinator. This is AUTHORING for a new
candidate, not a claim that the earlier freeze already passed the new guard.
Independent source review, native duplicate-source GREEN and the exact four
canonical cases remain pending until their receipts are attached. PostgreSQL
execution must follow D1329 using the existing shared PostgreSQL17.6 and its
namespace wrapper; no private server/container is started. The scope remains
private schema/read/capacity qualification. Native DEPOT0 TRANSFER, exclusivity
and actual current-locker/open-view admission are not qualified by this repair.

### D1329 native registration repair receipt (2026-10-17)

This receipt supersedes the pending qualification above. The exact three Rust
files passed workspace formatting, governance and all 124 governance tests,
strict all-target Clippy, and the guard using the actual Cargo artifact and
dep-info output on Molehill-PC with Rust 1.95.0. The guard observed 56 harness
units and the existing 11 exceptions. No exception or validator was changed.

The canonical `item_storage_postgres` target then passed all four cases once:
three unchanged pure cases and the actual PostgreSQL 17.6 schema/read case.
The complete PG log contains its qualified version marker, with zero failures
or ignored cases. Locked dependency fetch preceded the unchanged D1329 owner
`in-pg-netns.sh`; database traffic used its namespace route to the existing
shared server on host 127.0.0.1:55432. No PG server or container was started.

| Receipt | Result | SHA-256 of complete log |
| --- | --- | --- |
| Strict all-target Clippy | PASS | `fb5f108fbbfe064d6c2fa561d830a92012227017f4857b97ce2d6a11f8b011d9` |
| Actual duplicate-source guard | PASS, 56 harness units | `db5b5f4338644ceb286e11783c17465770d3b88c1b71c9d8362e456c0414f2f2` |
| Canonical depot target | 4 passed, 0 failed, 0 ignored | `0edc2250e34d3dd8f49bff6ff55dfab6575f8d863881739dc63172b16acbe37f` |

Independent source review passed. Full normal-package execution is running at
this publication checkpoint; the new successor's CI is not yet qualified.
The separate reviewed native TRANSFER/exclusivity packet and its isolated
root-order regression require their own composed execution. These results do
not establish actual current-locker or open-view runtime admission.

## 2026-10-17 composed Depot TRANSFER native qualification

This is a new authored successor of storage head
`dd92a9eac7349a36668048dfe0f5966bfde4a89b`, composed with the full qualified
main/HOUSE input `f010472bcb6794a2cbc56dfaf755bd122c10225e` and the exact
Items repair `ea7dab45bcb9b9ce7edcd66ac08eb8c5162cc35d`. All three histories
are retained. Private preparation HEAD is
`0fb9fd34bdeabd828112c611b705c9563158adce`, with 82 numbered migrations and
the actual corrected 0113 blob `7bb7ff5d5179adc926cd8b7f47276c3dc513c2f2`.
The six qualified source files form native Git tree
`7a9cf0f5e7ccba977e8140dcad2f2a233658d83c`; an independently reconstructed
GitHub tree matches it exactly. This task-card entry is a strict append after
native completion, checked separately with diff and governance.

### Resulting behavior

The actual native TRANSFER owner now implements the two bounded whole-item
main-backpack/Depot shapes with the existing operation3. It binds Character,
World, source custody, destination, numeric placement ordinal, actual item
quantity, command, same physical transaction receipt and native audit. The
Character root precedes globally ordered item locks and subsequent slot locks.
Withdrawal creates one new main-backpack entry and does not scatter or merge.
Retry uses the committed receipt. The final proposal preserves the complete
HOUSE/Items proof bodies and includes Depot in exclusive live custody.

The native source packet extends TRANSFER fields8–10, shapes8–9, source kinds5–6
and destination6. It retains World-only Depot scope and the private 15,000-entry
bound, with ordinal max+1. No new outer operation tag or replacement migration
number is allocated. Positive slot withdrawal still requires the existing
HouseExit admission; the qualified roundtrip uses the actual main backpack.

The SQL remains an explicitly unnumbered proposal installed by its real native
test fixture. The packet does not create a gameplay Locker/USE/window caller,
current Premium authority or admitted Content binding. Raw map placement and
source observations remain data; missing runtime admission is not manufactured.

### Immutable source and repair lineage

The original five-file root-order successor remains frozen: manifest
`a1dbcb9fca8d9ee546d1a9aec4008cd06811ff089df0703fa1f2a825e1089dff`,
patch `661b26cba81fa818aba0dd02e698afd5b62b1fa73d8a1ad3293c357b125696b5`.
Its first actual Clippy invocation exposed missing test-fact weight and a
panic rejected by the canonical path-loaded target. That failed attempt is
retained as failed evidence and does not qualify PostgreSQL.

The explicit two-file repair has manifest
`a1e6a2b1acbe232d339a715ed1ddbf30e93d4329770405a6b2e53e157c0071db`
and patch `a29051ef69580a0e741b355418976f53b6fdd5edfc7fd97df0998a4aa709ac89`.
It supplies the missing optional weight in the existing test implementation
and replaces the test helper panic with an equivalent assertion. Production
logic and old case bodies are preserved. The successor source receipt is
`f653e5cd32848473c6e035a4e1d437c594c569acc7cb9a5c1150b242b2bc6d2d`;
the original source receipt and v1 logs are unchanged.

The complete original/prepared/formatted/repair chain passed forward/reverse
checks. Actual v2 formatting changed only assertion wrapping in the audit
test helper. An independent byte-level review matched all six exported files
to the actual formatted source receipt
`dab571265aa0c26c2b54b033924e772c28d9ab24ac6aa93c38e2154fd72190b9`.
A read-only export observed during an intentional SQL RED overlay was rejected;
only the later complete GREEN export
`bf84ed7e1728b32587f869bd762edc0c6f119174b78ce713125780dd40a7efd7`
is a publication input.

### Executed qualification on Molehill-PC

Rust1.95.0, two Cargo jobs and a separate native target were used. Locked fetch,
actual package edition discovery, owned formatting, full workspace fmt check,
governance, strict all-target Clippy and the duplicate-source guard all passed.
The guard consumed the actual Cargo artifact/dep-info output and observed
57 test-harness units with the existing 11 exceptions; its rules were unchanged.

| Native stage | Observed result | Complete log SHA-256 |
| --- | --- | --- |
| Strict all-target Clippy | PASS | `7e89f83286398ad7372465972eef9e24deb4f20dcf67587781e4ff7c0bcee03c` |
| Actual duplicate-source guard | PASS, 57 units / 11 existing exceptions | `a70bce278550167d18c832ac3e36b222cd22f92e1e18e24ef5fea07531778d5e` |
| Exact pure roundtrip/resources | 1 passed, 0 failed, 0 ignored | `c0be4123b3df22a55d90c48c81cddd1031e3eb7f5e1559ea1dc2d8e5e96027b6` |
| Exact pure custody/command refusals | 1 passed, 0 failed, 0 ignored | `4eefa4be7c96fcb824a9bceec8bfb79ec2a7d6bb55588cf0e535aaa2a7928d47` |
| Exact pure unsigned/closed-shape bounds | 1 passed, 0 failed, 0 ignored | `7d360b20bb39e5e28c52bfcb3a758d5039c46608f393de030314351042222346` |
| Isolated root-order RED | Intended case failed, actual PG17.6 reached | `33dbaa9a0da0b6764993e2dcc7e559f275c8caeeab43c9e1f8fcb077b34593e9` |
| Isolated exclusivity RED | Intended case failed, actual PG17.6 reached | `277b98459fa8fd7b9a494ad67133423542622ae3fa28edc7b80c92bdcaa9b2fd` |
| Restored canonical Depot PG cases | 4 passed, 0 failed, 0 ignored; 4 actual170006 markers | `af5178f3dbaebb0f44d571103fbdc2689b499d0904ef37fb366509a8713483c1` |
| Full normal game-server package | 10,303 passed, 0 failed, 37 ignored; 59 groups;833.22s | `faffcd450a89fb3d1dd67181eb7c0e318d5d815dcdc46406f422d1c44711d28f` |

Both RED operators modify only their isolated SQL invariant, not their tests.
They reached the exact intended failure; their exit101 is recorded separately
from the restored GREEN. All subsequent source hashes match restored SQL
`8d50b1b03eea0009c288a257e9a1e5da527ac5f4866e245cee544de0a7b3a546`.

The complete 29-record native evidence bundle is SHA-256
`c7768aa2bbf20c871b72f2769b0f83fbdbd52e5dec26163d4a4f0f65b276f41a`;
the actual runner summary is
`abcca7761b18b100a3fc1e1c943e0590237da8b58d302ca5a8d567a774c8c900`.
Source, command, full-log hashes and separate stage results are retained at
`/home/mole/work/pr4-20261017-1135/logs/feature2156-depot-v2-*`.

D1329 was followed throughout: locked fetch preceded the owner's unchanged
`in-pg-netns.sh`; all PG cases used its route to existing shared PostgreSQL17.6
at real host127.0.0.1:55432. No server/container was created and real host5432
was untouched. Cleanup matched only database creation markers from these exact
invocations; all such databases are absent, without inspecting or changing
other databases. Normal-package totals are not a substitute for the separately
observed PG qualification.

### Publication and remaining integration

This completed native checkpoint supersedes the prior pending TRANSFER and
normal-package statuses on its exact inputs. The later main Ground successor
`ce7bccab973625d5d3d0d0c566426903229d2373` arrived after those inputs were
frozen; its shared-source composition is a subsequent authored qualification,
not silently covered by these receipts. Exact new published-head CI is pending
until observed. Draft status is retained.

Final installed Depot proof/migration composition and actual current Locker,
USE, session-bound view, Content and Premium admission remain explicit
integration dependencies. Stash and writable-item children keep their accepted
separate scope. These native tests do not establish those runtime producers.

### 2026-10-17 — Ground composition and exact test-weight adapter qualification

This receipt closes the earlier checkpoint's outstanding composition with
pinned Ground input `ce7bccab973625d5d3d0d0c566426903229d2373`. It is the
specific Ground input tested here, not a claim about the latest main. Later
main successors require their own explicit integration. Published Depot
`047d922583f4e1410ad0b9a3aa91b17c3f58547e` was composed with that complete
Ground tree in private HEAD `b92a989e409583841e9cc55d09e2c3c287c59a91`,
base tree `ab39a25d03bfec4d02fa117ba2668d877987c23a`, with ordered parents
`[047d9225, ce7bccab]`.

#### Preserved failed attempt and minimal repair

The first complete Ground/Depot attempt failed strict all-target Clippy on ten
compile errors in the existing Ground test fixtures: four missing optional
`ItemDefinitionFacts.weight` fields, two missing request generation digests,
and four missing native weight-lookup arguments in freeze/commit calls. Its
source, failure summary and Clippy receipt remain immutable:

- Source: `bec45ed835f8391b54a718a85f0ce009ed9120e624cf532596a63a303468d5dc`.
- Failed summary: `88b2bee90dd1247f051f2522d64562a543c1fce4feb0a878093947a0be5988ef`.
- Failed Clippy: `c6f8a7fdbc8c52a5e29e96ccbd5978cb2cdff08b3f13a52f6a9b563e32b5c059`.

The separate repair changes only
`apps/game-server/src/durability/ground_cell_limit_tests.rs`: original Git blob
`9c9c8740ac45872f2a92d9d2afdbbc1f4fcd9f2a` becomes
`3ff84e916a4d8c8c147cb8600e4e4a4679262489`, SHA-256
`52d3668302893e428b66fcd1bafe74de6a73100c163badb329f0bec39cf3ebc3`.
Its typed local fixture lookup resolves the existing Ground root fixture and
delegates every other definition and the generation digest to the real native
`TEST_WEIGHTS` fixture catalogue. The four facts, two requests and four native
call sites use that consistent lookup. The same 28 case names, assertions and
fixture identities remain. No production Content lookup, weight fallback,
SQL, gameplay owner, shared catalogue, guard or test-ignore path was changed.

The reviewed one-file manifest is
`cd4275b313e30278f3c51a2a17faa79c085144dd846cca4114db544bef943e8f`;
patch `eda376585ce4c9246597ea58e04d23f7ddcd5b5e212bca91b0c0a2de7a1ac478`.
Application kept the existing detached worktree, real index and cache in
place. Its source receipt is
`40b959c7dfbd55a20ebcdd67b8e7f85c901be498716331614bc63d60872b2d4e`,
with applied tree `9352731f0aaf95e40d53236bbc7dd3ff0b3ac4a9`.
The historical failed attempt qualifies no stage of this successor.

#### Fresh native completion

All nine fresh gates below are bound to that exact repaired complete source.
The helper pair is apply
`b05bdfcce81034b2ecc32bada35914e2d8c4fa8f7351164a7d5c1573de3eb5a7`
and qualifier
`64eff50c4db763e33f1cfab77fbe2a5ab33fe56fa90f24cea52147684cf6fcdd`.
The full composition, source inventory, Cargo.lock and real index remained
unchanged across every stage. Rust 1.95.0 used two Cargo jobs, the existing
core build lock and the existing separate `targets/feature2156` cache.

The complete 18,922-path inventory stayed bound to
`2a32eec0fe472039a037b8189f425b7e3063b6dc2a871985d48bbf8dcf2f0343`. Cargo.lock remained
Git blob `eba8094c71724362b911f309d6fa13262b2b7279`, SHA-256
`a3b9800ad3199130b2ddc363b6b3972b1ecf119d22e595c243701fcd00f5a35e`; the real index stayed
`26dc92181288ccfc79cd1323e2df04d67e5b39132dddcecdb0bd95f7fa78817f`.

| Repaired Ground/Depot stage | Actual result | Complete log SHA-256 |
| --- | --- | --- |
| Fresh locked fetch | PASS | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Full workspace fmt check | PASS; no source formatting mutation | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| Repository governance | PASS on native source | `ed5de0b194f14550187fbb13e78364869d9be5848032f691f807fecb2abe0a1f` |
| Strict all-target Clippy | PASS | `0398a22dedfe988eb8914998db242da003e53a8919034e302ddb85c41d0e195b` |
| Actual Cargo dep-info duplicate-source guard | PASS; 57 units | `a70bce278550167d18c832ac3e36b222cd22f92e1e18e24ef5fea07531778d5e` |
| Native whole-item Depot cases | 4 passed, 0 failed, 0 ignored; 4 actual PG 170006 creation markers | `72ed82847d25db685b50787d7dcc898e043cd8893b8f89779bccd49fe30bc941` |
| Existing Storage target | 4 passed, 0 failed, 0 ignored: 3 pure + 1 actual PG case | `9f04e3c4af2ab07807e02fdaa9590e2227e06808e9c1952598f9a058892bb207` |
| Ground canonical regression group | 28 passed, 0 failed, 0 ignored: 12 pure + 16 actual PG cases | `94327fc3a592e2bab379d935a31d984b805e6ff9e604fca8e34a81a8adab7216` |
| Full normal game-server package | 10,331 passed, 0 failed, 37 ignored; 59 groups; 825.14 s | `2d3f8889de87598307971cdcbe213de98201b7ddc5b09d8d85e0d74a5d28f6d5` |

The Depot group supplied the exact four expected names and per-case database/
170006 markers. Ground supplied its exact 28 names and the expected tag plus
170006 marker in each of its 16 PG bodies. The unchanged Storage target retains
its actual PostgreSQL170006 assertion; its other three cases are pure. The
normal package ran without a PG URL and does not add database coverage to these
separately observed groups. The earlier root-order and custody-exclusivity RED
receipts remain historical evidence for unchanged Depot logic; no new RED
operator was required by this fixture-only adapter.

D1329 locked fetch preceded the unchanged owner `in-pg-netns.sh`, using only the
existing shared PostgreSQL 17.6 at real host 127.0.0.1:55432. No server/container
was created and real host5432 was untouched. Actual cleanup results are:

The Depot controller checked exactly the four database names printed in the
current Depot test log. All four were already absent after native harness
cleanup; no additional DROP was issued:

- `it_depot_native_atomic_1792262138896174790`.
- `it_depot_native_custody_1792262134647886136`.
- `it_depot_native_root_1792262136708068640`.
- `it_depot_native_roundtrip_1792262140994762967`.

All three same-route controls reported server_version_num=170006, empty
cleanup errors, no observed or remaining logged databases, and zero additional
DROP operations. The unchanged Storage and Ground harnesses supplied no DB
names to their logs and completed their own cleanup. Their controls did not
infer ownership, query or delete unlogged databases; their empty logged-name
inventories are not independent censuses of every created database.

| Actual PG group | Database-control receipt SHA-256 |
| --- | --- |
| Depot 4 | `fb79be4770b6131ac9fe25406486c89a1cba585c98fa1c2788103f67d6d01871` |
| Storage 1 PG within 4 cases | `066b5110cfdf5891d65acc6039f59fc6f2fdedcf34b6515e95ca78498c3ded08` |
| Ground 16 PG within 28 cases | `83c43fb8f57943c999f6947aa03581580d1a87ec20e7dec3ce7cf7b5e94d1776` |

The complete immutable native bundle is `78baca56f7cc8a076a22feba328c27ee7dd0e17220dde00874a9b1e678fabfe7`, containing
17 records; summary `12d4033ce018be7f29358d69549b90ec0bd867e87facb1d2ba7e499e478165a1`; full-package receipt
`341a4fa443908c79e1c2c70c839d33680e1fd9db9da027aad8c98375b49fe1e6`. The actual complete source export is
`b98a4d226dae4d2ec6329afba6492115f4ae04f8a08ff56d05bc4d7a97605fe3`, and the subsequently captured native tree is
`9352731f0aaf95e40d53236bbc7dd3ff0b3ac4a9`, capture receipt `76f0a6aecf1c4aae5cbd64131ce70a37c6fb624741cf90801bc562ae5431f6b3`.
All prior failed and successful source records remain unchanged.

#### Publication and retained boundaries

Publication carries the complete pinned Ground/Depot tree plus this strict
task-card append, preserving all 43,372 preceding card bytes as an exact prefix.
The sole public parent is `047d9225`; that commit identity differs from the
private two-parent native composition. Final-card diff/governance and exact
new-head CI are separate publication evidence and are not asserted by this
native receipt. Existing worktrees, caches and historical artifacts are kept.

This fixture repair does not install or renumber the Depot proposal, allocate
an operation tag or CP lease, or create current Locker, USE, session-bound view,
Content, Premium or HouseExit admission. Whole-item withdrawal remains one new
main-backpack entry within the accepted native TRANSFER operation3 scope.
Stash and writable-item children retain their separate accepted boundaries.
The draft is retained; these results do not claim full storage-family runtime
integration or ready/merge status.
