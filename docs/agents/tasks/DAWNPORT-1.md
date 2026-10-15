# DAWNPORT-1

```yaml
task_id: DAWNPORT-1
title: "DAWNPORT-1: Dawnport content and vocation choice (skeleton)"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/dawnport-1
issue: 1622
pr: 2191
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 DAWNPORT-1 (row 38); App. 7 Supporting (A13); wave 1 (migration chain #2)"
class: "impl (content)"
migration: maybe
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - content/world/ Dawnport inputs (new family files only)
  - content/npcs/ Dawnport vocation NPC dialogue (new files)
  - apps/game-server/src/interaction/vocation_choice.rs (new)
  - apps/game-server/tests/dawnport_*.rs (new)
slot_only_paths:
  - regenerate_content.py outputs, content/project.json, manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins (one author, one commit)
  - migration number if a vocation-choice receipt is needed
public_contracts: []
depends_on:
  - "CHAR-BUILD-1: MERGED #1393 + 1b writer"
  - "W2b: MERGED #2109"
  - "A13 ACCEPTED: effective on main e2da9805 via ACCEPT-BATCH #2209"
  - "ITEM-LOOP-1 (migration chain order): BLOCKING: chain order, migration slot after ITEM-LOOP-1"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Dawnport content and the vocation-choice interaction; choosing a vocation keeps cumulative progress (SKILLS-0 §3.3).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CHAR-BUILD-1 | MERGED #1393 + 1b writer |
| W2b | MERGED #2109 |
| A13 ACCEPTED | ACCEPTED on main e2da9805 via ACCEPT-BATCH #2209 |
| ITEM-LOOP-1 (migration chain order) | BLOCKING: chain order, migration slot after ITEM-LOOP-1 |

**BLOCKING for the final migration slot:** ITEM-LOOP-1 (migration chain order). A13 acceptance is effective on main `e2da9805`; this local allocation permits the detached request adapter without assuming any future schema. Slot-only paths remain held until CP clears their gates.

## Owned paths (builder edits freely)

- content/world/ Dawnport inputs (new family files only)
- content/npcs/ Dawnport vocation NPC dialogue (new files)
- apps/game-server/src/interaction/vocation_choice.rs (new)
- apps/game-server/tests/dawnport_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- regenerate_content.py outputs, content/project.json, manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins (one author, one commit)
- migration number if a vocation-choice receipt is needed
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `durability/character_progression.rs` (build state writer)
- `content/character_progression_content.rs`
- `tools/content-migration/regenerate_content.py` then `content/content.lock.json`, `content/manifest.json`, `content/world/pins/*` rotated by one author in one commit

## Inseparable bundles

content+regeneration+all pins by one author; migration+writer if needed.

## Acceptance criteria

- [ ] Vocation choice commits through the build-state writer; progress kept.
- [ ] Content validators pass; pins rotated once.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

A13 §; SKILLS-0 §3.3; Reference Dawnport map/NPC data. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 DAWNPORT-1 (row 38); App. 7 Supporting (A13).

## Builder instructions

- Work on branch `skeleton/dawnport-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: local adapter prepared; five real-library tests and focused Clippy pass
status: authoring
branch: skeleton/dawnport-1
blocker: qualified native Dawnport interaction/departure and stance-fit facts; ITEM-LOOP-1 final migration-slot order
next_action: CP/architect qualify native facts, independent review and integration slot
```

## Bounded local preparation (2026-10-15)

Existing draft #2191; branch `skeleton/dawnport-1`, source
`aaeffba3e6982ba721afa147b82fd8242731ddff`. The current allocation owns exactly
`interaction/vocation_choice.rs`, `tests/dawnport_vocation.rs` and this record.
The skeleton's broader content allocation is not used in this batch. No content,
imports, pins, generation outputs, migration, registry or shared wiring is changed.
The worker makes no commit/push/ready/merge or external review request; root alone
may publish a completed local package to the existing draft.

### Accepted basis and residue

The lead confirmed effective ACCEPT-BATCH #2209. Read-only inspection of primary
main `e2da9805d296a27b06b32335bf67394509d4910b` confirms A13's **ACCEPTED** marker.
Its §4.4 assigns DAWNPORT-1 the choice interaction, exactly one build receipt and
the combined stance fields; stance fit remains UNKNOWN. SKILLS-0 §3.3 supersedes
raw magic/skill equality: each family's cumulative progress is preserved and
re-levelled by the writer's content formula. Dawnport-specific cap/departure facts
are not supplied by these general build-state contracts.

On this supplied skeleton the choice adapter is absent. The existing public
`BuildChangeRequest`, `DurableBuildState`, `BuildFormula` and `convert_vocation`
already provide the accepted writer seam; they are unchanged between this source
and the inspected accepted main. `RevisionSlot::commit_build` routes to the fenced
writer, which checks the current before-state, revision, cause and conversion and
owns occurrence replay/binding. No new schema or separate vocation/stance receipt
is required by this adapter, and no future schema is assumed.

### Prepared behavior and its boundary

`vocation_choice_request` derives `after` by calling the existing conversion, then
returns the actual `BuildChangeRequest` with `VocationChoice`, the original before
state/occurrence and the owner's optional stance-prune decision. It does not copy
raw levels/tries as the successor, reset progress, recompute the conversion, infer
stance fit, or create a second receipt. Conversion errors are returned unchanged.
`BuildFormula::required(None)` retains its existing unreachable-level meaning;
there is no first-level probe or invented vocation-admission rule.

This is request assembly, not an authority/admission decision or durable commit.
The eventual owner must qualify the selected vocation and current interaction from
active native content, flush pending training, read current build/stance, resolve
the still-unknown fit rule, and submit through the existing revision queue and
session/recovery fences. Existing writer checks are not replaced by this adapter.
No island/NPC/tile, level threshold, departure fact or magic-level cap is invented
from Reference. Native choice/departure qualification and actual caller integration
remain blocked until their owning facts are supplied. ITEM-LOOP-1 still orders any
final migration slot; this batch neither requests nor assigns one.

### Validation boundary

The owned integration test path-loads only the unregistered adapter and imports
the real public `oteryn_game_server::durability` library. No copied dependency types,
writer surrogate, dependency excerpt tests, registration overlay or extra wrapper
is used. Fixtures are explicitly synthetic formula data, not native Dawnport facts.
Tests cover all eight cumulative families, the existing combined prune field,
deterministic occurrence/request assembly, real conversion errors and preservation
of the unreachable-level semantics. They do not assert actual native admission,
session fencing, durable replay, a committed revision or departure behavior.

The focused Rust checks use Rust 1.94.0 under
`flock /tmp/oteryn-prep-rust.lock`,
`CARGO_TARGET_DIR=/workspace/oteryn-prep/cargo-target`, `CARGO_BUILD_JOBS=2`.
Logs and final source hashes are retained under
`/workspace/oteryn-prep/dawnport-1-evidence/`. Final module/test placement, qualified
content and pins, PostgreSQL/integration validation, independent exact-head review
and CP/architect acceptance remain future slot work. Once the adapter is wired
into the library, the test inclusion must be reconciled with duplicate-source policy.

Native validation setup evidence: the first Cargo attempt stopped before tests on
60 missing `include_*` paths in the sparse checkout; root restored exact-base tracked
inputs and all reported paths were verified present. The retry compiled the actual
server library, then Cargo's implicit game-server/game-migrate binary links failed
with linker signal 7 while only 371 MiB remained free. Neither attempt is a behavioral
RED or a passing test run. Complete logs are preserved as
`test-initial-setup-failure.log` and `test-cargo-link-failure.log` in the evidence folder.
After root freed disk, a direct `rustc --test` linked the unchanged actual server
rlib and ran **5 passed, 0 failed, 0 ignored**. Focused `clippy-driver --test
--emit=metadata` passed with repository deny lints, including `-D warnings`,
`clippy::all`, panic/unwrap/expect/todo/unimplemented/dbg prohibitions and forbidden
unsafe code. Both used `CARGO_INCREMENTAL=0`; no further Cargo invocation or implicit
binary link was used. No source/error/type surrogate or manifest/wiring change is
used to work around the environment. These focused checks do not replace the full
Cargo, durable-state and content validation required at integration.

Owned-source `rustfmt --check`, governance validation (22 documents, 9 lanes),
the governance suite (**124 passed**) and `git diff --check` passed. Self-review
confirmed exactly the three allocated changed paths, no staged content/commit,
and no module wiring, content/pin/schema change or durable-authority claim.

SHA-256 bindings (full provenance and commands are in the external evidence folder):

- Adapter: `2b9c3e9be16ed6efa65b0ae5126c5bfc1a642326d217c0c1637b2c2b885bc7c9`.
- Tests: `19d6496cf4e72553a7b3bcb15317ac203f51e7638a44a50f6fc6144927fcd773`.
- Actual server rlib: `75fc517e5ad7d15462174567fc868ab14d22ea34e24605843e40040b05c0f6f5`
  (`liboteryn_game_server-c4a23c339eaa9aaa.rlib`, unchanged before/after checks).
- Retained native test executable: `5ed024df660efb77b4aa71037734278785223ba4d19de9db519599efb06ae0a8`.

No acceptance claim is made for current DB/session authority, native availability,
stance fit, departure, durable replay or exactly one committed receipt: these need
the future qualified caller, writer execution and independent integration review.
