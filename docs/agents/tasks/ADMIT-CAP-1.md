# ADMIT-CAP-1

```yaml
task_id: ADMIT-CAP-1
title: "ADMIT-CAP-1: World capability admission authority (skeleton)"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/admit-cap-1
issue: 1622
pr: 2187
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 ADMIT-1 (none, security prerequisite); App. 7 ADMIT; wave 1"
class: "hard (authority, protocol); protocol and security review"
migration: no
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/world_required_capabilities.rs (new)
  - apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs (new)
  - docs/agents/tasks/ADMIT-CAP-1.md (task packet/notes)
slot_only_paths:
  - capabilities.rs (required set)
  - connection.rs admit path (`admit_frame` l.1101, `required_capability` l.363)
  - PROTOCOL_OTERYN_V1_REGISTRY.json
public_contracts: []
depends_on:
  - "ADMIT0 decision ACCEPTED: satisfied on main e2da9805 via ACCEPT-BATCH-2026-10-15"
  - "numbers of MAP_STATE_V1, WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 reserved on #162: BLOCKING: CP verifies reservation (WORLDINT-WIRE-1 #2094 offers capability 11)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

The World admission refuses a session that does not select the capabilities a World requires (`MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS`, `WORLD_INTERACTION_V1`), per ADMIT0. One admission authority, fail closed, before SEC-CHAL-1/2.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ADMIT0 decision ACCEPTED | SATISFIED: main `e2da9805` ACCEPT-BATCH §1, item 5; the old branch header is historical |
| numbers of MAP_STATE_V1, WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 reserved on #162 | BLOCKING: CP verifies reservation (WORLDINT-WIRE-1 #2094 offers capability 11) |

**BLOCKING:** unresolved capability-number/build-floor adapters; numbers of MAP_STATE_V1, WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 reserved on #162. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/gameplay_transport/world_required_capabilities.rs (new)
- apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs (new)
- docs/agents/tasks/ADMIT-CAP-1.md (task packet/notes)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- capabilities.rs (required set)
- connection.rs admit path (`admit_frame` l.1101, `required_capability` l.363)
- PROTOCOL_OTERYN_V1_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `admit_frame` (l.1101) and `FreshAdmissionAuthority::required_capability` (l.363)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`

## Inseparable bundles

wire+registry+capability+handler+tests in one PR.

## Acceptance criteria

- [ ] A session missing any World-required capability is refused at admission with the ADMIT0 code; no partial admission.
- [ ] Registry entries and `capabilities_tests.rs` agree.
- [ ] Negative cases: each required capability missing alone.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

ADMIT0 §2-§4; plan App. 7 ADMIT; Reference login capability negotiation is not applicable (Oteryn-native). Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 6 ADMIT-1 (none, security prerequisite); App. 7 ADMIT.

## Builder instructions

- Work on branch `skeleton/admit-cap-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: local dependency-closure and support kernel prepared; isolated source harness 10/0 and all-target Clippy pass
status: authoring
branch: skeleton/admit-cap-1
blocker: production floor/profile adapters and authenticated current-generation slot integration remain unresolved
next_action: root reviews local diff and evidence; CP resolves adapters and runtime wiring before qualification
```

## Local authoring packet (2026-10-15, draft #2187)

Starting head `19ec21c299d0f1763b82aa1030032d0780703de5`, exclusive branch
`skeleton/admit-cap-1`. Residue checked read-only against main
`e2da9805d296a27b06b32335bf67394509d4910b`: ACCEPT-BATCH §1 item 5 accepts
ADMIT-0, including its effective accepted amendments. The historical CANDIDATE
heading here is not an acceptance blocker. Current main still lacks the required
capability module/closure; `capabilities.rs` is byte-identical to this draft.
The registry already carries `requires` on 10 of 18 rows; do not duplicate that
existing work. Immutable inputs/hashes are saved immediately under
`/workspace/oteryn-prep/admit-cap-1-evidence/`.

Bounded implementation before slot:

- Compute transitive dependency closure from the existing `OfferedCapability`
  graph, with registered IDs supplied by a trusted build adapter. Reject unknown,
  unoffered, cyclic and ambiguous duplicate-node graphs; preserve the accepted
  bounds of 16 declared roots and protocol `MAX_CAPABILITY_COUNT` closure members.
- Validate the explicit build mandatory floor at boot; no inferred floor,
  capability numbers, World classification, required set or transport profile.
  There is intentionally no production adapter/default until CP resolves them.
- Keep the effective set immutable with the borrowed build catalogue. At session
  checks, require the effective set plus the trusted profile's dependency closure,
  then reuse actual `SelectedCapabilities::select` and confirm forced selection.
  Existing selection capacity is not raised; unrepresentable sets fail closed.
- No identity, nonce, lease/session authority or scope generation is minted here.
  The eventual owner must bind the set beside policy revision to the current
  Channel ownership generation and invoke the same predicate after authentication
  and inside existing atomic revalidation at every accepted boundary.
- Small durable source harness: compile the real existing capabilities production
  body, omitting only its terminal full-server test registration; include both
  owned modules by their exact source paths. No fake negotiation/types, no server
  rebuild loop, no original registration-file edit. Preserve hashes and commands.

### cross_domain_findings / CP slot obligations

1. `MAP_STATE_V1` and `WORLD_SPATIAL_FIELDS` numbers, mandatory floor and trusted
   registry/build/profile adapters remain unresolved. The kernel never substitutes
   empty production sets or guesses a relationship to map/spatial capability IDs.
2. Existing `required_capability()` is a single-ID pre-`authority.admit` check using
   `CAPABILITY_MISMATCH`. It is not ADMIT0's authenticated/current-scope predicate.
   Slot integration must honor authentication/privacy ordering, the accepted three
   error codes, current authority and final atomic revalidation; no missing IDs
   may be published pre-authentication, no nonce consumed on this refusal.
3. `SelectedCapabilities` currently holds 32 entries, while ADMIT0 bounds closure
   at protocol 128. Reuse its real selector and refuse unrepresentable sets; a
   capacity change, if needed, belongs to an accepted owning-path change.
4. Registry normalization, boot configuration/readiness, fresh/reconnect/recovery,
   forced-selection acknowledgements/replacement snapshots, transfer refusal and
   current generation binding are unwired and remain slot/integration obligations.
   This leaf module does not claim a real admitted-session path or authority proof.

Worker publication remains forbidden: root alone may publish to the existing
owner-authorized draft after reviewing the final packet. No worker commit, push,
ready/merge, review trigger, registry/migration, content/pin or shared wiring edit.

### Local evidence and validation checkpoint

Durable source/config/log evidence is written directly to
`/workspace/oteryn-prep/admit-cap-1-evidence/`, not just `/tmp`.
The isolated `admit-cap-source-harness` compiles the actual two owned source files
and the actual `OfferedCapability`/`SelectedCapabilities` production body. Its
copy of `capabilities.rs` removes exactly the terminal three-line test-module
registration that imports full-server integration fixtures; no production byte,
type or negotiation function is replaced. The exact removed suffix, full-source
hash, production-prefix/copy hash and unchanged original wiring hashes are recorded.
The real protocol path dependency supplies protocol types and the bound constant.

Actual isolated unit tests: **10 passed, 0 failed**. Cases cover transitive diamond
closure, every missing effective member alone, missing/empty floor, unknown or
unoffered roots/dependencies, duplicate nodes, self/mutual cycles, malformed
canonical lists, declared 16/17, closure 128/129, existing selected capacity,
profile closure/error handling, required union overflow and optional-negotiation
compatibility. These are pure configuration/support checks, not real admission,
nonce, DB/session fence or cross-generation visibility evidence.

Root lifted the temporary disk HOLD. The unchanged ten-test source hashes are
retained; tests were not repeated. Harness all-target Clippy passed with
`--locked --all-targets --quiet -- -D warnings`, using Rust 1.94.0, the shared
Rust lock/target, two jobs and `CARGO_INCREMENTAL=0`. No broad server, workspace
build or E2E run is claimed. Direct formatting of both unregistered owned modules and `cargo fmt --all
--check` passed. Governance validated 22 policy documents and 9 lanes; its
unit suite passed 124/124. Exact commands, restoration instructions, local
patch and final source hashes are
preserved in the durable evidence folder. These checks do not qualify runtime
integration; the three runtime/registry acceptance criteria remain unchecked.
