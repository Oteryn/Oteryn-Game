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
owner: Admission family I (user-delegated; CP D1081-D1084)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-16
candidate_state: AUTHORING
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/world_required_capabilities.rs (new)
  - apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs (new)
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/src/durability/fresh_admission_composition.rs
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/node/config.rs (ReadinessConfig and its validation)
  - apps/game-server/src/node/serve.rs (boot validation and bound effective set only)
  - apps/game-server/tests/world_required_capabilities*.rs
  - tools/qualification/node_boot/run.sh
  - tools/qualification/login_local/run.sh
  - docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md (D1083 map successor correction only)
  - docs/agents/tasks/ADMIT-CAP-1.md (task packet/notes)
slot_only_paths:
  - capabilities.rs (required set)
  - connection.rs admit path (`admit_frame` l.1101, `required_capability` l.363)
  - PROTOCOL_OTERYN_V1_REGISTRY.json
public_contracts: []
depends_on:
  - "ADMIT0 decision ACCEPTED: satisfied on main e2da9805 via ACCEPT-BATCH-2026-10-15"
  - "D1083 resolves map successor WORLD_MAP_VIEW_V1=18 and closed WORLD_INTERACTION_V1=11; FIELD has no ID and stays excluded until registered/offered"
  - "A2 slot waits for #2197 and #2163 merged/closed; merge main only after #2208 merges"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

The World admission refuses a session that does not select the capabilities its actual scope offers and requires, per ADMIT0 and CP D1083. D1084 delivery is fresh admission and authenticated V2 same-session recovery; lane G2 owns future native fast/post-grace callers. No partial admission or degraded selection.

## Current allocation (2026-10-16, CP D1081-D1084)

Authoritative allocation: https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6218747873 .
A1 authoring is allocated now. Node configuration/boot/effective-set hunks have one author.
Do not merge main until #2208 is merged. A2 shared paths stay untouched until #2197 and #2163 are merged or closed.
D1083 accepts WORLD_MAP_VIEW_V1=18 as the map successor; WORLD_INTERACTION_V1=11 requires [15,18] and remains unoffered.
WORLD_SPATIAL_FIELDS has no number; no alias to 6 or unknown-name configuration is permitted.
The floor and native-profile requirements are derived solely from registered, actually offered build/scope facts.
ADMISSION_CAPABILITY_REQUIRED=1116 and FND-04 tables already exist; leave them unchanged.

## Owned paths (builder edits freely)

- apps/game-server/src/gameplay_transport/world_required_capabilities.rs (new)
- apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs (new)
- D1081 additions are enumerated in the YAML owned_paths above.
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
last_progress: D1081 real boot/config and locked fresh/V2 owners implemented locally; native tests authored, not executed
status: authoring
branch: skeleton/admit-cap-1
blocker: A2 shared slot/main merge and actual native Rust/PostgreSQL qualification remain pending
next_action: independent source review; approved native full-context assembly and owner tests; final A2 protocol/registry integration at CP slot
```

## Historical local authoring packet (2026-10-15, draft #2187)

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

## D1081 authoring packet (2026-10-16)

Lifecycle returned to AUTHORING at unchanged branch head `3d7bd91d622a836b03d416b4c34305e9f2a9b225`.
No commit, push, main merge, shared-path edit, ready/merge or paid review occurred.

Implemented A1: explicit numeric configuration beside policy revision; unknown/unregistered,
zero, duplicate, unsorted and oversized declarations refuse; actual scope offered graph
is checked against the compiled registry; mandatory floor and native-profile roots come
from registered and actually offered names, never config defaults. Boot validates before
readiness and binds scope/generation/policy to an immutable result.

The same real `FreshAdmissionStore::check_current_runtime_capabilities` reads the current
runtime guard with its full mirror/history validator under the deciding transaction's locks.
Fresh composition recomputes current owner facts before calling it. V2 PREPARE/COMMIT call it
after registered credential revalidation and before reservation, nonce or session changes.
Both production durable entrypoints require an explicit callback; the raw fixture-only
`FreshAdmissionStore::commit` is now `cfg(test)`, closing its production bypass. Exact historical receipts retain their previous retry disposition.
The authenticated V2 owner refuses before legacy mismatch/release and forces current required
selection without widening optional selection. Fast/post-grace remain G2-owned; House exit
and fallback remain refused.

Native tests authored: real compiled registry/offer/floor and every missing map closure member;
World/Channel/policy/older/newer-generation mismatches; required selection and optional resume
retention; D1 shape/registration. `tests/world_required_capabilities_postgres.rs` reuses the
existing independently signed fresh fixture and trimmed V2 control-loss/recovery setup,
actual guard publishers/migrations/atomic stores. Its fresh and V2 PREPARE/COMMIT negatives
assert unchanged claims, current session, budget, nonce/receipt/reservation/membership counts,
then accept the original nonce and preserve exact replay. A separate current-owner test
publishes a new runtime generation after request preparation and requires a refusal
from the independently read current guard before the support predicate runs. This is store-boundary coverage;
it does not claim real Platform/TLS or transport authentication-order E2E.

A2 assembly is an exact unapplied patch outside the checkout:
`/workspace/oteryn-prep/admit-cap-1-d1081/slot-interface.patch` with source/mount SHA256 manifest.
It registers actual modules and passes bound requirements/current support through actual
callers and native fixtures. This is PRIVATE ASSEMBLY ONLY, not a publication or final wire
patch: D1082 still owns code allocation/registration/wire mapping for reconnect/recovery,
ADMIT0-RL-01, and replacement of obsolete preauthentication-singleton tests. A1 source references
those mandatory A2 fields/refusal variants and is not independently assembled on this branch.

Checks: `git diff --check` and `bash -n` for both modified qualification scripts pass.
Governance validated 22 policy documents and 9 lanes; its unit suite passed 124/124.
Native Rust 1.94 formatting of the eight exact held Rust files succeeded with
`skip_children=true`; the returned patch SHA256 is
`4e57c9b714de53b889aa99c69fdec44ab69698c23c25ab8a17e48b31d5f310cf`.
All input and output hashes were checked before authoring the formatting-only patch.
The first formatter attempt lacked two donor module blobs; the successful attempt included
the exact Git-blob-checked baseline leaves. This is formatter/parser evidence only.
The independently reviewed `cfg(test)` raw-commit hardening and its documentation were added
subsequently; those formatting-neutral lines have not had a separate formatter invocation.
Rust tests, PostgreSQL cases and Clippy have NOT run for this changed candidate.
The cloud environment has neither Cargo nor rustfmt; no heavy native job/global slot was used.
Historical 10/0 isolated-source evidence above applies only to the old kernel and does not
qualify the current A1 implementation. Final native validation must use the real crate and
exact private mount, including `--lib`, `--test world_required_capabilities_postgres`,
`--test durability_postgres --no-run`, and `--test wp5_s3b_composition --no-run`; selected
PG cases require the actual approved PostgreSQL 17.6 fixture. Do not count skipped tests.
