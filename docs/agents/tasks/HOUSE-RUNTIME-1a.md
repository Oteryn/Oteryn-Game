# HOUSE-RUNTIME-1a — house interior shapes and item placement (skeleton task record)

```yaml
task_id: HOUSE-RUNTIME-1a
title: "HOUSE-RUNTIME-1a: house interior shapes and item placement"
mode: IMPLEMENT
status: local_preparation_pending_qualification
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row HOUSE-RUNTIME-1a; §7 'HOUSE-RUNTIME-1a/1b/1c' (HOUSE_RUNTIME0 ACCEPTED)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/house-runtime-1a
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Durable house interior shapes and item placement inside a house through the one-item transaction, with custody and audit (packet §2.1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

First of HOUSE-RUNTIME-1a -> 1b -> 1c. Releases TIMED-HOUSE-1, MAP-OVERLAY-1c and WRITE-1 house items.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| SCOPE-HANDOFF-1 | MERGED (#1782) |
| HOUSE-CUSTODY-1 | MERGED (archive OTV2-20260930-house-custody-1) |
| HOUSE_RUNTIME0 | ACCEPTED (plan §7 status list) |
| one-item operation tag and migration number | leased by CP at the slot |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/house_interior.rs`
- `apps/game-server/src/durability/house_interior_audit.rs`
- `apps/game-server/tests/house_interior_postgres.rs`
- `apps/game-server/tests/support/house_interior_postgres_cases.rs`
- `apps/game-server/tests/support/house_test_fixture.rs`
- `docs/agents/tasks/HOUSE-RUNTIME-1a.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/NNNN_house_interior_shapes.sql
- durability/item_mint_audit.rs arm
- native_one_item_transaction.proto (one-item operation tag, leased by CP)
- character_authority_postgres.rs path-mod line
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

- durability/mod.rs registration
- durability/item_mint_audit.rs operation arm
- native_one_item_transaction.proto oneof (tag leased by CP)
- GAME_EVENT_FOUNDATION_REGISTRY.json and RESOURCE_LIMITS_REGISTRY.json rows

## Inseparable bundles (never split across PRs)

- migration + house_interior writer + audit arm + proto tag + registries (packet §2.1)

## Acceptance criteria

- [ ] Placing/removing an item in a house is one one-item transaction with audit; no duplication under concurrency.
- [ ] House custody (HOUSE-CUSTODY-1) respected: OWNER, SUBOWNER and GUEST may place items from their own inventory/equipment with a current storage/access grant; placement without that current grant is refused with a typed code.
- [ ] Interior shape limits enforced via RESOURCE_LIMITS.
- [ ] State survives restart.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server house_interior
cargo test -p oteryn-game-server house_custody
cargo test -p oteryn-game-server item_transfer
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md §2.1
- PR #1782; archive OTV2-20260930-house-custody-1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## Local preparation — 2026-10-15 (unpublished)

- Source: `a9e0277c8c79071bcfe7b8aa7ec7b78f2592232d`, existing draft #2163,
  `skeleton/house-runtime-1a`. Local diffs only; no commit, push, freeze, ready flag,
  integration, migration, tag lease, registry or content change is authorized here.
- HOUSE_RUNTIME0 is accepted through merged #1771 despite its historical CANDIDATE
  heading. SCOPE-HANDOFF-1 and HOUSE-CUSTODY-1 are merged. Packet §2.1 has
  `candidate_bases: []`; 1b/1c, HOUSE-1a, MAP-LOAD and ADMIT do not block this early
  structural preparation. Production access remains migration 0074's no-row stub.
- Prepared `house_interior.rs`: closed house-side placement/take/tile-move facts,
  location bounds, item/house/placer/placement binding, same-house scope-generation
  equality, reclaim preservation and checked +1 provenance revision. The §5.2 role
  table is a pure policy rule, never a current-authority token or a commit gate.
- Prepared `house_interior_audit.rs`: immutable house-side audit relationships only.
  It allocates no protobuf tag and provides no envelope encoder/decoder. The character
  source/destination, complete event and worst-case encoded bounds remain slot work.
- Prepared the standalone test wrapper, focused semantic cases and §10 test-only
  access fixture. The fixture is guarded to a dedicated `house_interior_*` database,
  test administrator and PostgreSQL 17.6; it reads independently supplied fixture
  properties/grants under FOR SHARE. It supplies no session/scope authority and no
  shape function. Production access, table privileges and all migrations are untouched.
- Acceptance boxes above remain open. These sources do not implement or qualify a
  production shape writer, event round trip, restart/replay or live-authority fence.

### cross_domain_finding — required indivisible slot bundle (REPORT_ONLY)

The accepted packet §1.3/§2.1 requires three SECURITY DEFINER shape functions, runtime
EXECUTE-only grants and transaction-local insertion/removal proofs. Those changes
belong to the slot-only migration, not to the owned Rust files. Existing `0011`
reservation `destination_kind` and receipt `shape` constraints, its placement proofs,
`0014`'s `game_item_transfer_consistency_guard` and source evidence must gain the house
clauses and nullable ChannelId without weakening their existing bodies. Migration
`0025` currently forbids runtime table writes and requires same-transaction provenance;
`0074` supplies only the fail-closed access stub. Implementing a Rust table writer or
inventing future SQL functions here would bypass that accepted closure.

CP/architect qualification must approve the complete shape signatures/transaction
owner, tile-source signature, leased operation tag and additive event grammar,
reservation/receipt changes, retained provenance proof, registry bounds and module/test
registration. When the cases join the protected aggregate, `apps/game-server/Cargo.toml`
must also mark the standalone wrapper `test = false` under the repository duplicate-test
rule; that manifest is outside these owned paths and needs CP routing. Until the slot,
none of these integration edits are implemented or assumed by this preparation.
The future writer must preserve recovery/admission → cause/replay → live session,
assignment/incarnation and guards → character_root → locked house access → item-id
order, with house scope generation and all authorization revisions re-read in the same
commit. The current-session generation must be resolved independently of prepared
command/audit facts. Missing/stale facts, session/scope replacement, ACL/guild revision
changes, provenance substitution, transaction reuse, concurrent item moves and
restart/replay all remain required writer qualification cases; structural tests do
not discharge them.

### Local validation evidence

Final source-accurate isolated validation passed: 7 structural cases and 1 real
PostgreSQL 17.6 case (all existing migrations, no-row production stub, runtime table
privilege closure, fixture roles and ACL revocation lock conflict). Isolated Clippy
passed with the native root lint table; that isolated run did not compile the server
library. The later native-wrapper qualification below uses the real server library.
Workspace formatting, diff whitespace, governance validation and 124 governance unit
tests passed. Initial offline patch resolution and unconditional test mounting failures
were outside-tree harness setup issues; their logs are retained separately, not counted
as product RED evidence.

The outside-tree harness mounts unchanged prepared sources, literal native dependency
excerpts and the real protocol crate; it records exact source hashes and excerpt ranges
in `/workspace/oteryn-prep/house-runtime-1a-evidence/sources.json`. Separate semantic,
PostgreSQL and Clippy logs plus the actual test binary are retained there. This is not
whole-server Clippy, protected CI, independent review or final writer qualification.
CP and architect qualification remain required before any slot.


### Documentation repair — 2026-10-15

The coordination review verified a pre-existing P2 in the acceptance checklist:
“non-owner placement refused” contradicted accepted HOUSE_RUNTIME0 §5.2. The
checklist now explicitly allows OWNER, SUBOWNER and GUEST to place from their own
inventory/equipment with a current storage/access grant, and requires a typed
refusal without that current grant. The writer must still independently re-read
current authority and authorization revisions under the accepted transaction locks.

This repair changes only the checklist and this note. The correct Rust policy,
structural validator, audit facts, tests and fixture are unchanged; all acceptance
boxes remain pending. All five Rust file hashes match round-1 RESULT.json and the
source-bound retained scope evidence; the four directly mounted task sources also
match sources.json. No Rust, PostgreSQL or Clippy rerun is required or claimed for
this documentation-only correction. Prior runtime evidence retains its original
limited scope; CP/architect and slot qualification remain pending.


### Native standalone-wrapper qualification — 2026-10-15

The initial precautionary wrapper removal was reversed after reading the actual
`check_duplicate_test_sources.py` implementation: it checks test-bearing sources
compiled into multiple test targets. An auto-discovered standalone wrapper without
a `test = false` entry is not, by itself, proof of duplication. The unchanged
wrapper is restored for qualification through the real server library; none of the
four implementation/support Rust files, Cargo manifests or integration owners change.
The byte-identical outside-tree wrapper copy remains reconstruction evidence.

The first native invocation stopped before compiling the wrapper or running tests:
the restored sparse clone lacked 58 unique baseline include inputs, producing 60
missing-file compiler diagnostics. Every missing input is tracked at the exact base;
this is a clone setup failure, not a wrapper/import defect or product RED. The exact
restoration list is retained in `house-runtime-1a-evidence/native-wrapper-missing-inputs.json`.
Root restored every missing input from the exact base, then the corrected command
`cargo test --locked -p oteryn-game-server --test house_interior_postgres --quiet`
passed: **8 passed, 0 failed, 0 ignored, 0 filtered out** (7 structural cases and
1 real PostgreSQL 17.6 fixture/closure case), in 1.21 seconds of test execution.
It used the actual server library and native public types, without type substitution,
under the shared Rust lock, shared target and two build jobs. The first native log
is preserved as `native-wrapper-setup-failure.log`; the passing result is retained
as `native-wrapper-test.log`. This is additional standalone-wrapper qualification,
not a reinterpretation of the earlier isolated harness results.

The byte-identical standalone wrapper remains in the publication patch. No actual
duplicate-source violation was established; the guard's 13 Python tests passed,
while the complete compiled CI source-graph guard remains CI work. When these cases
join a protected aggregate in the CP slot, wrapper/Cargo/aggregate registration
must prevent actual double compilation of test-bearing sources. No Cargo or shared
module wiring was changed. This does not qualify the future shape writer, registered
event, session/scope mutation fences or full package; all acceptance boxes remain pending.
