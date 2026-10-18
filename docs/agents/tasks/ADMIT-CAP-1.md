# ADMIT-CAP-1

```yaml
task_id: ADMIT-CAP-1
title: "ADMIT-CAP-1: World capability admission authority (skeleton)"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
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
updated_at: 2026-10-18
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
  - apps/game-server/tests/support/world_required_capabilities_postgres_cases.rs (D1324: extracted cases only)
  - tools/qualification/node_boot/run.sh
  - tools/qualification/login_local/run.sh
  - docs/architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md (D1083 map successor correction only)
  - docs/agents/tasks/ADMIT-CAP-1.md (task packet/notes)
  - apps/game-server/src/gameplay_transport/map_bundle_domain_postgres_tests.rs (D1164: two callers and matching fixture binding only)
  # 2026-10-18 completion directive (lead; D1442): the A2 integration paths, hand-authored.
  - apps/game-server/src/gameplay_transport/connection.rs (admit/resume refusal wiring only)
  - apps/game-server/src/gameplay_transport/mod.rs (module, owner fields, fresh admit check only)
  - apps/game-server/src/gameplay_transport/capabilities.rs (D1442 constants and gated rows only)
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs (ADMIT0 transport cases only)
  - apps/game-server/src/gameplay_transport/qualification.rs (owner binding only)
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs (owner binding only)
  - apps/game-server/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs (owner binding only)
  - apps/game-server/tests/wp5_s3b_composition.rs (commit capability argument only)
  - crates/protocol-oteryn/src/lib.rs (1117/1118 and capability rows 29/30)
  - crates/protocol-oteryn/src/item_look_tests.rs (unregistered probe moved to 31)
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capabilities 29/30, errors 1117/1118)
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json (ADMIT0-RL-01)
  # 2026-10-18 CI follow-up (lead; D1324 canonical mount, PG-COVERAGE-1).
  - apps/game-server/tests/durability_postgres.rs (world_required_capabilities cases mount only)
slot_only_paths: []
public_contracts: []
depends_on:
  - "ADMIT0 decision ACCEPTED: satisfied on main e2da9805 via ACCEPT-BATCH-2026-10-15"
  - "D1083 resolves map successor WORLD_MAP_VIEW_V1=18 and closed WORLD_INTERACTION_V1=11; FIELD has no ID and stays excluded until registered/offered"
  - "D1322: A2 slot after #2197 and #2181 merge, reconciled against then-current main and shared-file order below"
  - "D1324: canonical PostgreSQL aggregate mount after #2181 merges; CP alone writes the Cargo test=false entry"
  - "D1442: MAP_STATE_V1=29 and WORLD_SPATIAL_FIELDS=30 take the next free registry numbers (next_unallocated 31)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## D1442 completion packet (2026-10-18)

Lead directive: merge only when the mechanic is complete and wired into the running game.
Every change below is hand-authored on the main merge `c3a1adab`; the frozen A2 bundle further
down is recovery data only and was not applied.

**D1442 registry numbers.** `PROTOCOL_OTERYN_V1_REGISTRY.json` gives `MAP_STATE_V1` 29 and
`WORLD_SPATIAL_FIELDS` 30, the next free numbers (`next_unallocated` 29 -> 31). Both require
`[6]` and are `offered: false`. 29 is registered only so the name is never reused: D1083 keeps
18 `WORLD_MAP_VIEW_V1` as the accepted map successor, so 29 is never offered. 30 stays behind
the FIELD-WIRE gate. `protocol-oteryn` `REGISTERED_CAPABILITY_IDS_V1` and
`CAPABILITY_REQUIREMENTS_V1`, and `capabilities.rs` `GATED`, carry the same rows.

**Build-floor adapter.** The minimal adapter that matches the existing admission code is
name-based. The floor is every `MANDATORY_FLOOR_NAMES` row (`WORLD_MAP_VIEW_V1`,
`WORLD_SPATIAL_FIELDS`, `WORLD_INTERACTION_V1`) that is registered AND offered by the scope's
real offer (`offered_for_scope`). It fails closed in three ways:
- A floor name missing from the build registry fails boot with `InvalidFloorList`.
- A closed gate contributes nothing until its offer opens, and then it binds automatically.
- Declaring an unoffered ID (11, 29, 30) fails boot with `Unoffered`.

Today a bundle World's floor is `[18]` (closure 4, 6, 18). A fixture World's floor is empty.

**Wiring (running game).**
- **D1 configuration.** `readiness.required_gameplay_capabilities` is a required field. Boot
  (`serve.rs`) binds it to scope, generation and policy against the real offer before readiness,
  and any error is `BootError::Readiness`. `login_local` declares `[18]` for the bundle World.
- **Fresh admission.** The pre-authentication `CAPABILITY_MISMATCH` singleton is removed.
  `ComposedFreshAdmission::admit` checks after grant authentication and before capacity
  reservation or commit, so no nonce is consumed. The refusal is
  `ADMISSION_CAPABILITY_REQUIRED` (1116). The commit rechecks under its locks.
- **Recovery (authenticated V2 same-session).** `resume.rs` refuses before PREPARE and before
  the legacy release. `connection.rs` sends one `RECOVERY_CAPABILITY_REQUIRED` (1118,
  `OPERATION_TERMINAL`, D1187).
- **Reconnect.** `RECONNECT_CAPABILITY_REQUIRED` (1117) is registered in the registry and
  `protocol-oteryn`. Main has no native fast/post-grace reconnect transport path; per D1084
  lane G2 owns those callers and must use 1117.
- **Transfer.** The only transfer into a Channel scope on main is the House exit (and its §4.3
  fallback), refused with `ExitNotAdmitted`. The test
  `every_transfer_into_a_channel_scope_stays_refused` pins it, as ADMIT0 §4 requires until the
  lifting conditions hold.
- **ADMIT0-RL-01.** Registered in `RESOURCE_LIMITS_REGISTRY.json` (hard maximum 16, applied
  before closure).

**Validation (2026-10-18, Molehill-PC, shared PG 17.6 via `in-pg-netns.sh`).**
`cargo fmt --all --check` clean; `cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn
--all-targets -- -D warnings` clean. Tests: `protocol-oteryn` 260/260; `--test
world_required_capabilities_postgres` 1180/1180; `--test wp5_s3b_composition` 1161 passed, 1
ignored (pre-existing); `--test durability_postgres` 1298/1298; `oteryn-game-server --lib`
3489 passed, 3 failed in untouched files. The two `node::secure_file` cases fail only under the
netns uid mapping and pass outside it (4/4); the `party::shared_xp_postgres_tests` block-limit
case passed on its rerun (1/1). The
`world_required_capabilities_postgres` wrapper now also mounts `bestiary_postgres_harness`,
which main's durability NPC travel fixture needs.

**CI follow-up (2026-10-18, head after `93815212`).**
- **PG-COVERAGE-1.** Both merge-queue failures (integration-rest and the Durability harness)
  were one guard, `postgres_target_aggregation`: `support/world_required_capabilities_postgres_cases.rs`
  was mounted only by its own wrapper, which CI does not run. `durability_postgres.rs` (a CI-run
  target) now mounts it the same way as the sibling `*_cases.rs` files; nothing is exempted.
  The other PostgreSQL errors in the harness log are expected-error output of passing tests.
- **Node template (Codex P1).** CP D1447: production infra is oteryn-vps only and Synology is
  out of scope, so `deploy/synology-game/` is unchanged. The repository has no oteryn-vps node
  template. A node document whose `[readiness]` omits the field now fails closed with
  the key `readiness.required_gameplay_capabilities` (no default). The
  VPS batch deploy adds the field: `[18]` for a bundle World, `[]` for a fixture World.
- **Owner answers (lead).** Registry ID 29 (`MAP_STATE_V1`) stays reserved and is never
  offered. The 1117 `RECONNECT_CAPABILITY_REQUIRED` caller belongs to lane G2.

**Tests.**
- **Wire handshake** (`capabilities_tests.rs`, through `admit_frame`). A bundle World refuses a
  client lacking any closure member with exactly one 1116 and admits one that has them. A
  recovery lacking one gets exactly one 1118 and resumes once it supports them.
- **PostgreSQL** (`world_required_capabilities_postgres`). Fresh and V2 refusals leave the
  nonce, claims, reservations and receipts unchanged, and then admit the same nonce.
- **Not covered.** No in-process test drives `ComposedFreshAdmission::admit` itself, because it
  needs a live mTLS Platform evidence producer. The `login_local` qualification covers that.

## D1322–D1324 checkpoint — partial / NOT_FULLY_QUALIFIED (2026-10-17)

Authoring base: public #2187 head
`0c4e6dfc0ff73b3fb39aa25b896c9f54c702958c`. The lead remains the sole publisher.
The [current CP decision](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232842478)
grants the two-file F3 extraction now and supersedes the earlier allocation
requests; it does not release all shared integration paths.

**D1324 — F3 extraction, first stage:** the existing
`tests/world_required_capabilities_postgres.rs` wrapper now mounts
`tests/support/world_required_capabilities_postgres_cases.rs`. The new module
uses the harness crate root's same owners and fixtures. Its 566-line suffix from
`require_postgres_17_6` through all three ADMIT0 cases is byte-identical to the
public base, SHA256
`824db05965fac778275a92c03ddb69e1790faf2d23bdb41542cf4f3cdd29cf44`.
The wrapper's path-loaded production modules and reconnect fixture builders are
unchanged. Module-root imports and the local `TestResult` alias move with the
cases; the three test names acquire the module prefix
`world_required_capabilities_postgres_cases::`.

This is a partial checkpoint. The standalone integration target is still active;
`tests/durability_postgres.rs` does not mount the cases yet, and no Cargo
`test = false` entry is added. The canonical mount comes only after #2181 merges;
CP is the sole Cargo writer. The duplicate-source failure remains unresolved,
and this extraction claims no whole-guard PASS. The earlier native PostgreSQL
PASS remains bound to recovered `d0a262f4`; it does not qualify the extracted
module or a current-main composition.

Source/byte preservation, changed-path and diff checks passed, as did governance
validation (22 policy documents / 9 lanes) and the existing 124-test governance
unit suite. Independent extraction review found no material issue. The lead
also ran Rust 1.94.0 `rustfmt --check --edition 2024 --config skip_children=true`
on exact copies of both changed Rust files on Molehill-PC: exit 0, with matching
SHA256 before and after. The wrapper is
`78b520f14149be27d077d9ff0b42a34913967f4f5812d8105c947c230fc190c8`;
the support module is
`dac39ff13885e97d2757dae683c701c4e7aa5127d44de124b7c7705c2a521993`.
No native build, Clippy, PostgreSQL, duplicate-source guard or S3-B run is
attributed to this checkpoint; the formatter check does not establish compilation.

**D1322 — A2 remains a later integration slot:** reconcile the allocated hunks
against then-current main after #2197 and #2181 merge, not against recovered
`d0a262f4`. Shared-file order is binding:

- `gameplay_transport/mod.rs`: #2197 → QUEST #2198 → A2 #2187.
- `gameplay_transport/monk_save_postgres_tests.rs`: QUEST → A2.
- `tests/wp5_s3b_composition.rs`: #2181 → A2.
- `tests/durability_postgres.rs`: #2181 → #2187.
- Registry JSONs wait for CP regeneration; CP alone writes Cargo manifests/lock.

**D1323 — F2 deferred:** no `deploy/synology-game/*` hunk is authorized, including
`deploy-ops.sh`. The infrastructure target is `oteryn-vps`; CP will identify the
topology/declaration validation location. The previously reported declaration
gap remains evidence only. No Synology implementation, pin change, installed
configuration change or deployment is part of this checkpoint.

## Historical continuation checkpoint — NOT_FULLY_QUALIFIED (2026-10-17, F1)

Fresh-read public source: `c26d8a3150683539361202c94b08e3f82d89b8af`.
The continuation is AUTHORING; this repair changes only the owned login-local
configuration renderer and this task record. Root owns publication. The recovered
private A2 source remains `d0a262f47c0e3afc1fcc80fb4cbe007d0781b8d4`;
its 17,276-byte Git bundle and 40,003-byte, 13-file integration patch reproduce
the recovery SHA256 values retained below. No shared runtime hunk is published.

**F1 — fixed:** the actual `write_node_config()` emitted
`required_gameplay_capabilities = []` even with `LOGIN_LOCAL_WORLD_BUNDLE=1`.
That bundle offer has the accepted map floor 18, so boot refuses the empty
declaration with `MissingFloor([18])` before readiness. The renderer now declares
`[18]` for bundle mode and `[]` for the fixture build. The existing immutable
builder still computes closure `[4, 6, 18]`; its floor and refusal are unchanged.

Validation executes the actual renderer in a disposable directory for modes 0
and 1, intercepting only the final privileged installation. It parses both
outputs as TOML and checks the declaration, map revision and bundle-table
presence. Original source blob `5547e2be1764d566ecadf7da29ef704fcadc845b`
fails the bundle case (RED, exit 1); repaired source blob
`cdedcf24d79b2fc708c982700a89ff989cb501fe` passes both cases (GREEN, exit 0).
`bash -n tools/qualification/login_local/run.sh` and `git diff --check`: PASS.
Local `tools/agents/validate_governance.py`: PASS (22 documents, 9 lanes);
`python -m unittest discover -s tools/agents/tests`: PASS (124 tests).
ShellCheck: NOT_RUN, executable unavailable in the authoring environment.
Repaired script SHA256:
`744e5e53bd739322a6d01b46fbf6cb8d7f7a96baf834f7d3c7ac9f8903475663`.
The lead retains the disposable renderer regression beside the source packet.

**F2 — cross-domain finding, deferred to the deployment owner:**
`deploy/synology-game/node.toml.template` and `preprod-topology.toml` at the
reviewed source omit the new mandatory field. `NodeConfig` intentionally rejects
its absence. The shipped preproduction bundle needs `[18]`; the template's default
fixture needs `[]` with an explicit update when bundle mode is enabled. Their
owning templates, `deploy-ops.sh` manifest validation, existing topology tests and
rollout configuration need the matching declaration. These paths are outside this
allocation; no installed configuration or deployment has been changed.

**F3 — test aggregation finding, confirmed by native guard:** the new
`world_required_capabilities_postgres` target path-loads test-bearing production
modules but has no thin-wrapper exclusion in `apps/game-server/Cargo.toml`, and
the three ADMIT0 cases are not registered in `tests/durability_postgres.rs`.
The actual Cargo JSON/dep-info guard reports 112 violating source paths:

| Source paths | Strict test targets in the retained guard log |
| --- | --- |
| 109 test-bearing `src/` files | Library, `npc_econ_bank_postgres`, `world_required_capabilities_postgres` |
| `tests/support/item_transfer_postgres_cases.rs` and `npc_econ_bank_postgres_cases.rs` | `character_authority_postgres`, `npc_econ_bank_postgres` |
| `tests/support/postgres.rs` | `durability_postgres`, `world_required_capabilities_postgres` |

Admission appears in 110 of these paths. The independent NPC duplication remains
even if the Admission wrapper is excluded. The existing guard and its
only-shrinking allowlist remain unchanged. Retained `duplicate_sources.log` SHA256:
`f2d68e47be87be0ad6e3d4acabca6b0568467aad7b532aebe9a274fd2c6474f3`.

A scratch-only four-file proposal moves the unchanged three cases to
`tests/support/world_required_capabilities_postgres_cases.rs`, mounts them in the
canonical aggregate, and marks the focused wrapper `test = false`. Its 566-line
case/helper body is byte-identical to the recovered source, and `git apply --check`
passes on `d0a262f4`. This proposal is not compiled or published; the support path,
Cargo entry and aggregate mount require shared-path routing. It addresses the
Admission duplication only; no whole-guard PASS follows from this proposal.

**Native continuation — selected tests PASS, aggregate guard FAIL:** the lead's PC run
`/root/oteryn-pr-i-20261017-b2bde334/admission-validation/run-PIQIEdLt` binds
recovered `d0a262f4` plus the repaired script by exact hashes. Actual renderer
RED/GREEN (2/2 repaired modes), Bash syntax, diff check, Cargo format, strict
server Clippy (`--all-targets -- -D warnings`) and strict protocol Clippy: PASS.
The selected server tests pass: capability closure/floor/profile 14/14,
explicit registered/canonical NodeConfig 1/1, typed recovery wire 1/1, fresh
bundle wire 1/1, and resume 4/4. This is 21 selected server tests, not the full
server suite. The protocol library passes 247/247. All reported test runs have
zero failed and zero ignored tests; the runner checks the positive counts.

The PostgreSQL target compiles and all three exact ADMIT0 cases pass: fresh
atomic refusal, independently current runtime replacement, and V2 PREPARE/COMMIT
refusal. Each reports 1 passed, 0 failed, 0 ignored and its existing
`ADMIT0_PG17_6_PASS` marker after cleanup. These are signed-fixture store-boundary
checks, not a Platform/TLS/transport E2E qualification. The freshly compiled
executable SHA256 is
`f61b323826aac15dae92783dba81f5b61659ed5fd4b8ce0a5f9c63c49f1bf0cc`.

The run used build slot B with two Cargo jobs and serialized PostgreSQL under the
native PG lock in the dedicated PostgreSQL 17.6 container's network namespace.
The host's unrelated PostgreSQL endpoint and all fixture guards remained unchanged.
Before/after manifests for all 18,876 tracked files are byte-identical, SHA256
`e2ccdcad6c086621c2b1bfda625d2fdef291a905cc3cb29dcec000462056fb9b`:
SOURCE_REHASH PASS. The runner exits 1 solely because the real duplicate-source
guard fails; its other nonzero result is the deliberately expected original
renderer RED. The lead retains the full logs and exact compiled-PG record.

This recovered predecessor composition is not current-main or current-dependency
qualification: its NPC predecessor is `427fe6224d30557af4b70afb651768b90a07446d`,
while the lead's fresh read finds #2197 still draft/open at
`5ef65a81bea8ee3d7c8f7b11b008a0a73f1dd0f8`. Historical native PASS below retains
its original source scope.

Public Merge gate run `38976459975`, job
`117227528614`, fails before tests because the held A2 module, owner fields,
refusal variant and caller arguments are not yet published. CP confirms #2163
merged while #2197 remains held; the A2 publication hold and draft status remain.

## Historical checkpoint — NOT_FULLY_QUALIFIED (2026-10-16)

The boot-bound `BoundRequiredCapabilities::effective()` read port is published
on #2187 at `c8b5cb16f54ea54bf5e4a2cc64137c59aafbba13`.
[D1164](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225159415)
now allocates only the two obsolete MAP test callers and their matching fixture
binding in `map_bundle_domain_postgres_tests.rs`, before A2. Other shared A2
Git files remain unchanged and their existing slot holds still apply.

The new cloud strict-Clippy attempt against the private c8 assembly is historical
**RED** at those two removed `required_capability()` calls. This checkpoint
repairs both through the real `for_channel_boot` builder and the same explicit
scope/generation/policy fixture source used for fresh-session preparation. It
checks the boot-bound effective set: control `[]`, bundle dependency closure
`[4, 6, 18]`. Independent review accepted P1: the bundle must explicitly
declare `[CAPABILITY_WORLD_MAP_VIEW_V1]`; an empty declaration fails the
builder's `MissingFloor` check. The declaration is repaired; the control remains
empty and the expected closure is unchanged. The fixture snapshot grants no current authority. This repair is
**NOT_YET_REQUALIFIED**; no new Clippy, PostgreSQL or runtime PASS is claimed.

Historical 16 pure plus 3 PostgreSQL PASS results remain evidence only for
`3b80d9a5`; they do not qualify c8 or this successor. Earlier harness/format
results below retain their original source scopes. The new narrow formatting
and `git diff --check` PASS concern only this checkpoint's changed files.
The MAP test source SHA-256 is `fb815efd2a08d98b33bf279653bab8ec3bb3166817dcec0ba949e6d728de33cd`.
The prior private assembly manifest and RED log remain retained; lead owns
independent validation and publication. No ready/merge claim follows.

## Goal

The World admission refuses a session that does not select the capabilities its actual scope offers and requires, per ADMIT0 and CP D1083. D1084 delivery is fresh admission and authenticated V2 same-session recovery; lane G2 owns future native fast/post-grace callers. No partial admission or degraded selection.

## Current allocation (2026-10-16, CP D1081-D1084)

Authoritative allocation: https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6218747873 .
A1 authoring is allocated now. Node configuration/boot/effective-set hunks have one author.
Do not merge main until #2208 is merged. A2 shared paths stay untouched until #2197 and #2163 are merged or closed.
D1083 accepts WORLD_MAP_VIEW_V1=18 as the map successor; WORLD_INTERACTION_V1=11 requires [15,18] and remains unoffered.
WORLD_SPATIAL_FIELDS has no number; no alias to 6 or unknown-name configuration is permitted. *(Superseded 2026-10-18 by CP D1442: it is 30, registered and unoffered.)*
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

- [x] A session missing any World-required capability is refused at admission with the ADMIT0 code; no partial admission.
- [x] Registry entries and `capabilities_tests.rs` agree.
- [x] Negative cases: each required capability missing alone.
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
last_progress: D1442 completion packet hand-authored on the main merge; fmt, clippy -D warnings and the affected native/PostgreSQL lanes run on Molehill-PC (2026-10-18)
status: validating
branch: skeleton/admit-cap-1
blocker: none in this packet; native fast/post-grace reconnect (1117 caller) is lane G2-owned per D1084
next_action: CI and one Codex round on the pushed head; CP merge-queue turn
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


## Recoverable frozen source backup — 2026-10-17

User-requested code backup in this existing draft. This section contains the exact frozen private source bundle and readable integration patch; it is material source recovery data, not a new runtime/ready claim. The published task-branch runtime files keep their allocated integration order. Unified diffs below are encoded as JSON arrays of exact lines (including newlines); concatenate `patch_lines` to recover the byte-identical patch without Markdown trailing-whitespace changes.

Prepared composition: game-server/protocol all-target strict Clippy PASS at59f24c9d; final protocol Clippy and247tests PASS; two new server boundary cases PASS. Final delta to59f onlytwo protocol assertions. Old16pure+3PG PASS were not rerun. G callee composition remains NOT_RUN; runtime/PG/integrated qualification remains separate.

```json
{
  "private_candidate": "d0a262f47c0e3afc1fcc80fb4cbe007d0781b8d4",
  "bundle_file": "admit-private-d0a262f4-current-predecessors.bundle",
  "bundle_bytes": 17276,
  "bundle_sha256": "e54d5c638f70dfeedcf3e8a3e0445a3fcb29bc530abff0b1a10b417945ff4d92",
  "integration_patch_sha256": "9106f4eaf489b7e66aae56fb3afad81c49dae407619011ba9f0523ed7dc7eedb",
  "source_manifest_sha256": "25af0dc18ccc61430bf3ba8a56b42326fdb675a0728189828fc21b53a4ff0518",
  "restoration": "Base64-decode the bundle block, verify SHA256, then git bundle verify against the published prerequisites below; fetch its HEAD into an isolated checkout. The bundle preserves private commits and merge resolutions.",
  "shared_runtime_integration": "Private source backup only; the allocated runtime/shared-file slot still applies.",
  "status": "NOT_FULLY_QUALIFIED",
  "published_prerequisites": [
    "1d16eda0b7de19e240e8fd5a20c2d476ee103967",
    "967d972d29feba8307cff84f3f3b7929dea240cb",
    "427fe6224d30557af4b70afb651768b90a07446d"
  ]
}
```

<details><summary>Admission A2 — exact Git bundle (base64)</summary>

```text
IyB2MiBnaXQgYnVuZGxlCi05NjdkOTcyZDI5ZmViYTgzMDdjZmY4NGYzZjNiNzkyOWRlYTI0MGNi
IHdvcmsoaG91c2UtcnVudGltZS0xYS1zbG90LTIpOiBwcmVwYXJlIG93bmVkIGRyYWZ0IHNjb3Bl
Ci0xZDE2ZWRhMGI3ZGUxOWUyNDBlOGZkNWEyMGMyZDQ3NmVlMTAzOTY3IHRlc3QoYWRtaXNzaW9u
KTogZGlzYW1iaWd1YXRlIGVtcHR5IGNhcGFiaWxpdHkgYXNzZXJ0aW9uCi00MjdmZTYyMjRkMzA1
NTdhZjRiNzBhZmI2NTE3NjhiOTBhMDc0NDZkIHRlc3QobnBjLWVjb24pOiByZWdpc3RlciBiYW5r
IGNhc2VzIGluIGNhbm9uaWNhbCBQRyB0YXJnZXQKZDBhMjYyZjQ3YzBlM2FmYzFmY2M4MGZiNGNi
ZTAwN2QwNzgxYjhkNCBIRUFECgpQQUNLAAAAAgAAAD+VEHiclY1BTgMxDEX3OYX3SChxk0mNKgSC
A/QKruOISHQyZDxQbk/nCLzNl5709WyoQpw4swrKhUqsIfCx1omIRA4xawwiCRNOyS08dDZIVDEK
FZ+KL+gVJckBU00XKSw5ckal4KPjzT76gLde9AYn2eelLzpze5R+fYaQCcNxBx78HXe312am//m4
82jfbAqv+ARceDHQG4t9/oL9dBj6tbWhK/C66rDW5xW2ueyJM7yHQNn9ATg4UEmSEXiclUxbbsMg
EPznFPtfpWKJCVBVVateIFdYlrWKFHstTJL29iFH6PzMQzPTmwiIpewCuxjI5QljiOTRCs3JYplT
Dkz+xJ7NRk3WDojJl5CcJckusjtynFFyLiinoeZJgrBL1tC1/2iDby3yC+/8pE/dZKX6yrp8AI4T
jHFKHl7sgBnpUnuX/2zMudUbdYEvB6Ox6V571fUNsmo/ZL2uBagsdd9HCvR0l4vyWBRownqT9gf3
2gSa3nfzAMLAVvebEniclY5NSgQxEIX3fYrsBUkqqVQiIsJcwCtUUhWmkek06YzM8W0FD+DbPHg/
8M2hajhZl3NqKD5xlViBpSiRr5S9c5qiI6Siy85Dt2kqQHUWC4MXqiW01FKwmBNiQggZIHph5r99
AGoaAYJ4i0jcQiHLrUR0FFPJli2FEGXh+7z2YS5d9GFe64+99103Xp9rv70ZRxlcShatebKnljO9
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
pN5ZxZw=
```
</details>

<details><summary>Admission A2 — exact frozen integration patch lines (JSON)</summary>

```json
{
  "patch_lines": [
    "diff --git a/apps/game-server/src/gameplay_transport/capabilities_tests.rs b/apps/game-server/src/gameplay_transport/capabilities_tests.rs\n",
    "index 4693aec1..739edba2 100644\n",
    "--- a/apps/game-server/src/gameplay_transport/capabilities_tests.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/capabilities_tests.rs\n",
    "@@ -386,8 +386,10 @@ impl ConnectionIdentifiers for Identifiers {\n",
    " /// connection's own refusal is observable. `offered` is `None` for the production set.\n",
    " struct NegotiatingAuthority {\n",
    "     offered: Option<&'static [OfferedCapability]>,\n",
    "-    /// MAP-CUTOVER-1b: the capability a bundle World requires at fresh admission.\n",
    "-    required: Option<u32>,\n",
    "+    /// Immutable requirements for this transport fixture; production authenticates first.\n",
    "+    required: Option<\n",
    "+        std::sync::Arc<super::super::world_required_capabilities::BoundRequiredCapabilities>,\n",
    "+    >,\n",
    "     lost: Cell<Option<AdmittedSession>>,\n",
    "     steps: Cell<u32>,\n",
    "     /// ITEM-USE-WIRE-1: what an item USE observes; the backpack it serves under capability 4.\n",
    "@@ -432,8 +434,17 @@ fn session(continuity: SessionContinuity) -> Result<AdmittedSession, Box<dyn Err\n",
    " impl FreshAdmissionAuthority for NegotiatingAuthority {\n",
    "     async fn admit(\n",
    "         &self,\n",
    "-        _attempt: FreshAdmissionAttempt<'_>,\n",
    "+        attempt: FreshAdmissionAttempt<'_>,\n",
    "     ) -> Result<AdmittedSession, AdmissionRefusal> {\n",
    "+        // The fixture owner consumes the same forwarded support as the real authenticated\n",
    "+        // owner. This verifies transport refusal/selection, not grant authentication.\n",
    "+        if let Some(required) = &self.required {\n",
    "+            required.authenticate_support(attempt.supported_capabilities).map_err(|error| match error {\n",
    "+                super::super::world_required_capabilities::SessionCapabilityError::MissingSupport(_) =>\n",
    "+                    AdmissionRefusal::Classified(FoundationProtocolError::AdmissionCapabilityRequired),\n",
    "+                _ => AdmissionRefusal::Unavailable,\n",
    "+            })?;\n",
    "+        }\n",
    "         session(SessionContinuity::FRESH).map_err(|_| AdmissionRefusal::Unavailable)\n",
    "     }\n",
    " \n",
    "@@ -448,10 +459,6 @@ impl FreshAdmissionAuthority for NegotiatingAuthority {\n",
    "         self.offered.unwrap_or(PRODUCTION_OFFERED_CAPABILITIES)\n",
    "     }\n",
    " \n",
    "-    fn required_capability(&self) -> Option<u32> {\n",
    "-        self.required\n",
    "-    }\n",
    "-\n",
    "     fn world_clock_utc_ms(&self) -> Option<u64> {\n",
    "         Some(NOON_UTC_MS)\n",
    "     }\n",
    "@@ -770,8 +777,21 @@ fn the_bundle_world_offered_set_is_3_the_item_view_set_and_18_requiring_4_and_6(\n",
    " fn a_bundle_world_admits_only_a_client_that_selects_18() -> Result<(), Box<dyn Error>> {\n",
    "     run(async {\n",
    "         let mut authority = NegotiatingAuthority::new(Some(BUNDLE_WORLD_OFFERED_CAPABILITIES));\n",
    "-        authority.required = Some(CAPABILITY_WORLD_MAP_VIEW_V1);\n",
    "-        let mismatch = encode_protocol_error(FoundationProtocolError::CapabilityMismatch, 0)?;\n",
    "+        authority.required = Some(\n",
    "+            super::super::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(\n",
    "+                &[CAPABILITY_WORLD_MAP_VIEW_V1],\n",
    "+                crate::foundation::RuntimeScopeRefV1::channel(\n",
    "+                    WorldId::decode(&WORLD)?,\n",
    "+                    ChannelId::decode(&CHANNEL)?,\n",
    "+                ),\n",
    "+                1,\n",
    "+                \"capability-transport-fixture\",\n",
    "+                BUNDLE_WORLD_OFFERED_CAPABILITIES,\n",
    "+            )\n",
    "+            .map_err(|error| format!(\"capability fixture boot: {error:?}\"))?,\n",
    "+        );\n",
    "+        let mismatch =\n",
    "+            encode_protocol_error(FoundationProtocolError::AdmissionCapabilityRequired, 0)?;\n",
    "         // 4, 6 and 18 select 18 with the rest of the production set the client supports.\n",
    "         let (admitted, frames) = admit(&authority, &bootstrap(&[4, 6, 13, 17, 18])?).await?;\n",
    "         assert_eq!(accepted_selection(&frames)?, [4, 6, 13, 17, 18]);\n",
    "@@ -782,14 +802,16 @@ fn a_bundle_world_admits_only_a_client_that_selects_18() -> Result<(), Box<dyn E\n",
    "                 .selected_capabilities\n",
    "                 .contains(CAPABILITY_WORLD_MAP_VIEW_V1)\n",
    "         );\n",
    "-        // A client without 18, and one selecting 18 without its required 4, are refused with\n",
    "-        // exactly one CAPABILITY_MISMATCH before the owner admits anything.\n",
    "-        for supported in [&[4, 6, 13, 17][..], &[6, 13, 17, 18], &[]] {\n",
    "+        // Missing 18 or either transitive requirement is refused by the fixture owner with\n",
    "+        // exactly one ADMISSION_CAPABILITY_REQUIRED, preserving negotiated transport behavior.\n",
    "+        for supported in [&[4, 6, 13, 17][..], &[6, 13, 17, 18], &[4, 13, 17, 18], &[]] {\n",
    "             let (refused, frames) = admit(&authority, &bootstrap(supported)?).await?;\n",
    "             assert!(matches!(\n",
    "                 refused,\n",
    "                 Err(ConnectionEnd::AdmissionRefused(\n",
    "-                    AdmissionRefusal::Classified(FoundationProtocolError::CapabilityMismatch)\n",
    "+                    AdmissionRefusal::Classified(\n",
    "+                        FoundationProtocolError::AdmissionCapabilityRequired\n",
    "+                    )\n",
    "                 ))\n",
    "             ));\n",
    "             assert_eq!(frames, std::slice::from_ref(&mismatch));\n",
    "@@ -807,7 +829,7 @@ fn a_fixture_world_offers_exactly_the_production_set_and_never_18() -> Result<()\n",
    "             authority.offered_capabilities(),\n",
    "             PRODUCTION_OFFERED_CAPABILITIES\n",
    "         );\n",
    "-        assert_eq!(authority.required_capability(), None);\n",
    "+        assert!(authority.required.is_none());\n",
    "         let (admitted, frames) = admit(&authority, &bootstrap(&[4, 6, 13, 17, 18])?).await?;\n",
    "         let selected = accepted_selection(&frames)?;\n",
    "         assert!(!selected.contains(&CAPABILITY_WORLD_MAP_VIEW_V1));\n",
    "diff --git a/apps/game-server/src/gameplay_transport/connection.rs b/apps/game-server/src/gameplay_transport/connection.rs\n",
    "index 1eb5b4f0..f832a644 100644\n",
    "--- a/apps/game-server/src/gameplay_transport/connection.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/connection.rs\n",
    "@@ -136,6 +136,7 @@ pub(crate) const SERVER_SCHEMA_REVISION: u32 = 1;\n",
    " pub(crate) struct FreshAdmissionAttempt<'a> {\n",
    "     pub(crate) character_id: CharacterId,\n",
    "     pub(crate) admission_material: &'a [u8],\n",
    "+    pub(crate) supported_capabilities: &'a [u32],\n",
    "     pub(crate) game_session_id: GameSessionId,\n",
    "     pub(crate) transport: AuthenticatedTransportRefV1,\n",
    " }\n",
    "@@ -328,6 +329,8 @@ pub(crate) enum AdmissionRefusal {\n",
    "     /// Refused with an N8 admission refusal code (ARCH-LOGIN-FIRST-PACKETS-V1 \u00a71.2); nothing\n",
    "     /// was admitted. The client receives the code in one `ProtocolError` before the close.\n",
    "     Classified(FoundationProtocolError),\n",
    "+    /// Authenticated ADMIT0 V2 refusal, mapped to D1187 code1118 without releasing claims.\n",
    "+    RecoveryCapabilityRequired,\n",
    " }\n",
    " \n",
    " /// N8: the one pre-admission refusal frame is written and flushed within this bound, or dropped.\n",
    "@@ -364,13 +367,6 @@ pub(crate) trait FreshAdmissionAuthority {\n",
    "         PRODUCTION_OFFERED_CAPABILITIES\n",
    "     }\n",
    " \n",
    "-    /// A capability every fresh admission must select, or `None`. A bundle World requires 18\n",
    "-    /// `WORLD_MAP_VIEW_V1` (MAP-CUTOVER-1b): it serves its map only as domain 17, so a client\n",
    "-    /// without it is refused with `CAPABILITY_MISMATCH` before the owner admits anything.\n",
    "-    fn required_capability(&self) -> Option<u32> {\n",
    "-        None\n",
    "-    }\n",
    "-\n",
    "     /// The admitted actor's current own-actor observation for the initial snapshot, or `None`\n",
    "     /// when this authority serves no gameplay (transport-only fixtures).\n",
    "     fn observe(\n",
    "@@ -1160,7 +1156,7 @@ where\n",
    "             let Some(transport) = identifiers.transport_ref() else {\n",
    "                 return Err(ConnectionEnd::ResumeUnavailable);\n",
    "             };\n",
    "-            let resumed = authority\n",
    "+            let resumed = match authority\n",
    "                 .resume(ResumeAttempt {\n",
    "                     game_session_id: resume.game_session_id,\n",
    "                     recovery_material: resume.reconnect_material,\n",
    "@@ -1169,7 +1165,27 @@ where\n",
    "                     supported_capabilities: resume.supported_capabilities,\n",
    "                 })\n",
    "                 .await\n",
    "-                .map_err(|_| ConnectionEnd::ResumeUnavailable)?;\n",
    "+            {\n",
    "+                Ok(resumed) => resumed,\n",
    "+                Err(AdmissionRefusal::RecoveryCapabilityRequired) => {\n",
    "+                    // The owner produces this variant only after credential authentication,\n",
    "+                    // safe ownership/world classification and before PREPARE/COMMIT.\n",
    "+                    let _ = tokio::time::timeout(\n",
    "+                        ADMISSION_REFUSAL_WRITE_BOUND,\n",
    "+                        send_error(\n",
    "+                            stream,\n",
    "+                            FoundationProtocolError::RecoveryCapabilityRequired,\n",
    "+                            0,\n",
    "+                        ),\n",
    "+                    )\n",
    "+                    .await;\n",
    "+                    return Err(ConnectionEnd::AdmissionRefused(\n",
    "+                        AdmissionRefusal::RecoveryCapabilityRequired,\n",
    "+                    ));\n",
    "+                }\n",
    "+                // Before authentication no semantic capability diagnostics are disclosed.\n",
    "+                Err(_) => return Err(ConnectionEnd::ResumeUnavailable),\n",
    "+            };\n",
    "             let continuity = resumed.continuity;\n",
    "             // CAP-NEG-1: the resumed session keeps its original selection. The owning authority\n",
    "             // refuses a resume that cannot keep it before committing; this never acknowledges one.\n",
    "@@ -1215,19 +1231,10 @@ where\n",
    "             AdmissionRefusal::Unavailable,\n",
    "         ));\n",
    "     };\n",
    "-    if let Some(required) = authority.required_capability()\n",
    "-        && !selected.contains(required)\n",
    "-    {\n",
    "-        let error = FoundationProtocolError::CapabilityMismatch;\n",
    "-        let _ =\n",
    "-            tokio::time::timeout(ADMISSION_REFUSAL_WRITE_BOUND, send_error(stream, error, 0)).await;\n",
    "-        return Err(ConnectionEnd::AdmissionRefused(\n",
    "-            AdmissionRefusal::Classified(error),\n",
    "-        ));\n",
    "-    }\n",
    "     let attempt = FreshAdmissionAttempt {\n",
    "         character_id: bootstrap.character_id,\n",
    "         admission_material: bootstrap.admission_material,\n",
    "+        supported_capabilities: bootstrap.supported_capabilities,\n",
    "         game_session_id,\n",
    "         transport,\n",
    "     };\n",
    "@@ -4289,8 +4296,8 @@ mod tests {\n",
    " \n",
    "     /// Drive one connection with the given client frames; return the end state\n",
    "     /// and every frame the server wrote.\n",
    "-    async fn drive(\n",
    "-        authority: &Authority,\n",
    "+    async fn drive<A: FreshAdmissionAuthority>(\n",
    "+        authority: &A,\n",
    "         client_frames: &[Vec<u8>],\n",
    "     ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {\n",
    "         let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);\n",
    "@@ -5580,6 +5587,59 @@ mod tests {\n",
    "         })\n",
    "     }\n",
    " \n",
    "+    struct ResumeRefusalAuthority(AdmissionRefusal);\n",
    "+    impl FreshAdmissionAuthority for ResumeRefusalAuthority {\n",
    "+        async fn admit(\n",
    "+            &self,\n",
    "+            _attempt: FreshAdmissionAttempt<'_>,\n",
    "+        ) -> Result<AdmittedSession, AdmissionRefusal> {\n",
    "+            Err(AdmissionRefusal::Unavailable)\n",
    "+        }\n",
    "+        async fn resume(\n",
    "+            &self,\n",
    "+            _attempt: ResumeAttempt<'_>,\n",
    "+        ) -> Result<AdmittedSession, AdmissionRefusal> {\n",
    "+            Err(self.0)\n",
    "+        }\n",
    "+    }\n",
    "+\n",
    "+    /// Wire-boundary coverage only: the real owner authenticates before producing the typed refusal.\n",
    "+    #[test]\n",
    "+    fn authenticated_recovery_capability_refusal_is_typed_and_generic_refusals_stay_silent()\n",
    "+    -> Result<(), Box<dyn Error>> {\n",
    "+        run(async {\n",
    "+            for refusal in [\n",
    "+                AdmissionRefusal::Rejected,\n",
    "+                AdmissionRefusal::Unavailable,\n",
    "+                AdmissionRefusal::RecoveryCapabilityRequired,\n",
    "+            ] {\n",
    "+                let (end, frames) = drive(&ResumeRefusalAuthority(refusal), &[resume()]).await?;\n",
    "+                if refusal == AdmissionRefusal::RecoveryCapabilityRequired {\n",
    "+                    assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));\n",
    "+                    assert_eq!(\n",
    "+                        frames,\n",
    "+                        vec![error_frame(\n",
    "+                            FoundationProtocolError::RecoveryCapabilityRequired,\n",
    "+                            0\n",
    "+                        )]\n",
    "+                    );\n",
    "+                    let view = oteryn_protocol_oteryn::decode_protocol_error(\n",
    "+                        decode_wire_envelope(&frames[0])?.payload(),\n",
    "+                    )?;\n",
    "+                    assert_eq!(view.error_code, 1118);\n",
    "+                    assert_eq!(\n",
    "+                        view.disposition,\n",
    "+                        crate::foundation::ProtocolDisposition::OperationTerminal\n",
    "+                    );\n",
    "+                } else {\n",
    "+                    assert_eq!(end, ConnectionEnd::ResumeUnavailable);\n",
    "+                    assert!(frames.is_empty());\n",
    "+                }\n",
    "+            }\n",
    "+            Ok(())\n",
    "+        })\n",
    "+    }\n",
    "+\n",
    "     #[test]\n",
    "     fn admission_resume_is_not_served_and_admits_nothing() -> Result<(), Box<dyn Error>> {\n",
    "         run(async {\n",
    "diff --git a/apps/game-server/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs b/apps/game-server/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs\n",
    "index d6aa557d..0af348fc 100644\n",
    "--- a/apps/game-server/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/kill_reward_postgres_e2e_tests.rs\n",
    "@@ -223,6 +223,17 @@ fn scenario() -> TestResult {\n",
    "             evidence: &evidence,\n",
    "             world_id: world,\n",
    "             channel_id: channel,\n",
    "+            // This kill-only fixture has no admission caller; its explicit empty-floor build\n",
    "+            // does not stand in for the new capability-admission qualification.\n",
    "+            required_capabilities:\n",
    "+                crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(\n",
    "+                    &[],\n",
    "+                    crate::foundation::RuntimeScopeRefV1::channel(world, channel),\n",
    "+                    1,\n",
    "+                    \"policy-1\",\n",
    "+                    crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,\n",
    "+                )\n",
    "+                .map_err(debug)?,\n",
    "             runtime: &runtime,\n",
    "             movement_cells: room.movement_cells(),\n",
    "             door: &door,\n",
    "diff --git a/apps/game-server/src/gameplay_transport/mod.rs b/apps/game-server/src/gameplay_transport/mod.rs\n",
    "index 75a282b0..b5e4c90d 100644\n",
    "--- a/apps/game-server/src/gameplay_transport/mod.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/mod.rs\n",
    "@@ -27,6 +27,7 @@ mod last_position;\n",
    " mod logout;\n",
    " #[cfg(test)]\n",
    " mod logout_tests;\n",
    "+pub(crate) mod world_required_capabilities;\n",
    " pub(crate) use last_position::LastPositionFlush;\n",
    " mod map_door;\n",
    " mod monk_save;\n",
    "@@ -400,6 +401,7 @@ pub struct GameplaySeamOwners<'a, 'f, 's> {\n",
    "     pub evidence: &'a FreshEvidenceSource,\n",
    "     pub world_id: WorldId,\n",
    "     pub channel_id: ChannelId,\n",
    "+    pub(crate) required_capabilities: Arc<world_required_capabilities::BoundRequiredCapabilities>,\n",
    "     /// First-slice runtime authority remains crate-owned and cannot be supplied\n",
    "     /// by a transport or client caller.\n",
    "     pub(crate) runtime: &'a Mutex<ChannelRuntimeV1>,\n",
    "@@ -526,6 +528,7 @@ pub async fn serve_gameplay(\n",
    "         evidence: owners.evidence,\n",
    "         world_id: owners.world_id,\n",
    "         channel_id: owners.channel_id,\n",
    "+        required_capabilities: owners.required_capabilities,\n",
    "         runtime: owners.runtime,\n",
    "         movement_cells: owners.movement_cells,\n",
    "         door: owners.door,\n",
    "@@ -705,6 +708,7 @@ pub(crate) struct ComposedFreshAdmission<'a, 'f, 's> {\n",
    "     pub(crate) evidence: &'a FreshEvidenceSource,\n",
    "     pub(crate) world_id: WorldId,\n",
    "     pub(crate) channel_id: ChannelId,\n",
    "+    pub(crate) required_capabilities: Arc<world_required_capabilities::BoundRequiredCapabilities>,\n",
    "     pub(crate) runtime: &'a Mutex<ChannelRuntimeV1>,\n",
    "     pub(crate) movement_cells: &'a NativeEntryMovementCells,\n",
    "     /// The one entry-room door's bound runtime (#162 5868482467, M2b). Always locked after\n",
    "@@ -3052,16 +3056,10 @@ impl FreshAdmissionAuthority for ComposedFreshAdmission<'_, '_, '_> {\n",
    "     /// only a client that selects 18. MAP-ITEM-REF-1: any other World offers capability 4 (once\n",
    "     /// Part B sets its gate) only when the active generation pins a non-empty Item key set.\n",
    "     fn offered_capabilities(&self) -> &'static [capabilities::OfferedCapability] {\n",
    "-        if self.bundle_map().is_some() {\n",
    "-            capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES\n",
    "-        } else {\n",
    "-            item_ref_admission::offered_capabilities(self.active_generation)\n",
    "-        }\n",
    "-    }\n",
    "-\n",
    "-    fn required_capability(&self) -> Option<u32> {\n",
    "-        self.bundle_map()\n",
    "-            .map(|_| oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)\n",
    "+        world_required_capabilities::offered_for_scope(\n",
    "+            self.bundle_map().is_some(),\n",
    "+            self.active_generation,\n",
    "+        )\n",
    "     }\n",
    " \n",
    "     /// MAP-CUTOVER-1b: domain 17 of a bundle World at the actor's current position. The position\n",
    "@@ -3978,6 +3976,24 @@ impl FreshAdmissionAuthority for ComposedFreshAdmission<'_, '_, '_> {\n",
    "             let transition =\n",
    "                 FreshAdmissionClaimTransitionV1::prepare(&composition, &authorization, now)\n",
    "                     .map_err(|_| Rejected)?;\n",
    "+            // ADMIT0: authenticated grant, equal revisions and safe current ownership/world\n",
    "+            // precede any capability-specific refusal. The deciding transaction repeats it.\n",
    "+            let (scope, generation, policy) = composition.runtime_capability_binding();\n",
    "+            if !self\n",
    "+                .required_capabilities\n",
    "+                .matches_current(scope, generation, policy)\n",
    "+            {\n",
    "+                return Err(Rejected);\n",
    "+            }\n",
    "+            let capabilities = self\n",
    "+                .required_capabilities\n",
    "+                .authenticate_support(attempt.supported_capabilities)\n",
    "+                .map_err(|error| match error {\n",
    "+                    world_required_capabilities::SessionCapabilityError::MissingSupport(_) => {\n",
    "+                        Classified(N8::AdmissionCapabilityRequired)\n",
    "+                    }\n",
    "+                    _ => Unavailable,\n",
    "+                })?;\n",
    "             let mut flow = FreshAdmissionDurabilityFlowV1::begin(authorization, transition)\n",
    "                 .map_err(|_| Rejected)?;\n",
    "             let mut prepared = PreparedRequest(None);\n",
    "@@ -3994,6 +4010,7 @@ impl FreshAdmissionAuthority for ComposedFreshAdmission<'_, '_, '_> {\n",
    "                     self.holder,\n",
    "                     &composition,\n",
    "                     &request,\n",
    "+                    capabilities.current_check(),\n",
    "                 )\n",
    "                 .await\n",
    "             {\n",
    "diff --git a/apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs b/apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs\n",
    "index e4aad206..17b15728 100644\n",
    "--- a/apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs\n",
    "@@ -203,6 +203,19 @@ impl Dependencies {\n",
    "             evidence: &self.evidence,\n",
    "             world_id: self.admitted.world_id,\n",
    "             channel_id: self.admitted.channel_id,\n",
    "+            // Explicit no-floor build for this save-only fixture; no admission is performed.\n",
    "+            required_capabilities:\n",
    "+                crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(\n",
    "+                    &[],\n",
    "+                    crate::foundation::RuntimeScopeRefV1::channel(\n",
    "+                        self.admitted.world_id,\n",
    "+                        self.admitted.channel_id,\n",
    "+                    ),\n",
    "+                    1,\n",
    "+                    \"policy-1\",\n",
    "+                    crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,\n",
    "+                )\n",
    "+                .expect(\"explicit save fixture build\"),\n",
    "             runtime: &self.runtime,\n",
    "             movement_cells: self.room.movement_cells(),\n",
    "             door: &self.door,\n",
    "diff --git a/apps/game-server/src/gameplay_transport/qualification.rs b/apps/game-server/src/gameplay_transport/qualification.rs\n",
    "index d5af5115..92019af0 100644\n",
    "--- a/apps/game-server/src/gameplay_transport/qualification.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/qualification.rs\n",
    "@@ -1589,6 +1589,20 @@ async fn seam_flow(accounts: &[String; 2], key_id: &str, signing: &SigningKey) -\n",
    "             evidence: &fresh_evidence,\n",
    "             world_id: world,\n",
    "             channel_id: channel,\n",
    "+            required_capabilities:\n",
    "+                super::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(\n",
    "+                    &[],\n",
    "+                    scope,\n",
    "+                    scope_generation,\n",
    "+                    \"policy-s3b-1\",\n",
    "+                    super::world_required_capabilities::offered_for_scope(\n",
    "+                        false,\n",
    "+                        spell_input\n",
    "+                            .as_ref()\n",
    "+                            .and_then(|_| content_controller.active()),\n",
    "+                    ),\n",
    "+                )\n",
    "+                .map_err(|error| format!(\"capability boot: {error:?}\"))?,\n",
    "             runtime: &runtime,\n",
    "             movement_cells: &movement_cells,\n",
    "             door: &door,\n",
    "diff --git a/apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs b/apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs\n",
    "index 6f269629..771ed697 100644\n",
    "--- a/apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs\n",
    "@@ -478,3 +478,38 @@ fn independently_current_binding_is_required_by_the_durable_callback() {\n",
    "         );\n",
    "     }\n",
    " }\n",
    "+\n",
    "+#[test]\n",
    "+fn the_declared_limit_is_registered_and_applies_before_closure() {\n",
    "+    let registry: serde_json::Value = serde_json::from_str(include_str!(\n",
    "+        \"../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json\"\n",
    "+    ))\n",
    "+    .unwrap();\n",
    "+    let row = registry[\"entries\"]\n",
    "+        .as_array()\n",
    "+        .unwrap()\n",
    "+        .iter()\n",
    "+        .find(|row| row[\"id\"] == \"ADMIT0-RL-01\")\n",
    "+        .unwrap();\n",
    "+    assert_eq!(\n",
    "+        row[\"hard_maximum\"].as_u64(),\n",
    "+        Some(MAX_DECLARED_REQUIRED_CAPABILITIES as u64)\n",
    "+    );\n",
    "+    assert_eq!(row[\"configurable_range\"][\"maximum\"].as_u64(), Some(16));\n",
    "+    let over_bound: Vec<u32> = (1..=17).collect();\n",
    "+    assert_eq!(\n",
    "+        validate_declared(&over_bound),\n",
    "+        Err(BootCapabilityError::DeclaredLimitExceeded)\n",
    "+    );\n",
    "+    let protocol: serde_json::Value = serde_json::from_str(include_str!(\n",
    "+        \"../../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json\"\n",
    "+    ))\n",
    "+    .unwrap();\n",
    "+    assert!(\n",
    "+        protocol[\"capabilities\"]\n",
    "+            .as_array()\n",
    "+            .unwrap()\n",
    "+            .iter()\n",
    "+            .all(|row| row[\"requires\"].is_array())\n",
    "+    );\n",
    "+}\n",
    "diff --git a/apps/game-server/tests/wp5_s3b_composition.rs b/apps/game-server/tests/wp5_s3b_composition.rs\n",
    "index 57f5b489..858939b0 100644\n",
    "--- a/apps/game-server/tests/wp5_s3b_composition.rs\n",
    "+++ b/apps/game-server/tests/wp5_s3b_composition.rs\n",
    "@@ -882,7 +882,9 @@ fn composition_flow(\n",
    "                 now,\n",
    "             )?;\n",
    "             let store = FreshAdmissionStore::from_root(root.clone());\n",
    "-            match root.commit_composed_fresh_admission(&authority, &node_a, &composition, &request).await? {\n",
    "+            match root.commit_composed_fresh_admission(&authority, &node_a, &composition, &request,\n",
    "+                std::sync::Arc::new(move |current, generation, policy| current == scope\n",
    "+                    && generation == first.assignment.ownership_generation && policy == \"policy-s3b-1\")).await? {\n",
    "                 FreshAdmissionDurableOutcomeV1::Committed(_) => {}\n",
    "                 other => return Err(format!(\"admission 1 not committed: {other:?}\").into()),\n",
    "             }\n",
    "@@ -962,7 +964,9 @@ fn composition_flow(\n",
    "             // An owner change between composition and commit (a newer S2 floor)\n",
    "             // is caught at the commit boundary: the prepared request is stale.\n",
    "             ingest(&root, &node_b, descriptor, &accounts[1], &key_id).await?;\n",
    "-            match root.commit_composed_fresh_admission(&authority, &node_b, &composition, &request).await? {\n",
    "+            match root.commit_composed_fresh_admission(&authority, &node_b, &composition, &request,\n",
    "+                std::sync::Arc::new(move |current, generation, policy| current == scope\n",
    "+                    && generation == second.assignment.ownership_generation && policy == \"policy-s3b-1\")).await? {\n",
    "                 FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority => {}\n",
    "                 other => return Err(format!(\"owner change not revalidated at commit: {other:?}\").into()),\n",
    "             }\n",
    "@@ -979,7 +983,9 @@ fn composition_flow(\n",
    "                 AuthenticatedTransportRefV1::decode(&[0x53; 16])?,\n",
    "                 now,\n",
    "             )?;\n",
    "-            match root.commit_composed_fresh_admission(&authority, &node_b, &composition, &request).await? {\n",
    "+            match root.commit_composed_fresh_admission(&authority, &node_b, &composition, &request,\n",
    "+                std::sync::Arc::new(move |current, generation, policy| current == scope\n",
    "+                    && generation == second.assignment.ownership_generation && policy == \"policy-s3b-1\")).await? {\n",
    "                 FreshAdmissionDurableOutcomeV1::Committed(_) => {}\n",
    "                 other => return Err(format!(\"admission 2 not committed: {other:?}\").into()),\n",
    "             }\n",
    "diff --git a/crates/protocol-oteryn/src/lib.rs b/crates/protocol-oteryn/src/lib.rs\n",
    "index bb840077..8e9f584e 100644\n",
    "--- a/crates/protocol-oteryn/src/lib.rs\n",
    "+++ b/crates/protocol-oteryn/src/lib.rs\n",
    "@@ -61,6 +61,10 @@ pub enum FoundationProtocolError {\n",
    "     AdmissionIncumbentProtected = 1114,\n",
    "     AdmissionCapacityExceeded = 1115,\n",
    "     AdmissionCapabilityRequired = 1116,\n",
    "+    /// D1187 / ADMIT0: terminal for the reconnect candidate; current authority is preserved.\n",
    "+    ReconnectCapabilityRequired = 1117,\n",
    "+    /// D1187 / ADMIT0: terminal recovery refusal with no authority mutation.\n",
    "+    RecoveryCapabilityRequired = 1118,\n",
    " }\n",
    " \n",
    " #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n",
    "@@ -110,7 +114,9 @@ impl FoundationProtocolError {\n",
    "             | Self::SnapshotLimitExceeded\n",
    "             | Self::BootstrapLimitExceeded\n",
    "             | Self::InvalidCapabilitySet => ProtocolDisposition::SessionFatal,\n",
    "-            Self::PayloadLimitExceeded\n",
    "+            Self::ReconnectCapabilityRequired\n",
    "+            | Self::RecoveryCapabilityRequired\n",
    "+            | Self::PayloadLimitExceeded\n",
    "             | Self::TooManyOutstandingCommands\n",
    "             | Self::AccountDataIntegrity => ProtocolDisposition::OperationTerminal,\n",
    "             Self::CommandOutcomeExpired\n",
    "@@ -168,13 +174,15 @@ impl Display for FoundationProtocolError {\n",
    "             Self::AdmissionIncumbentProtected => \"character is already active\",\n",
    "             Self::AdmissionCapacityExceeded => \"admission capacity exceeded\",\n",
    "             Self::AdmissionCapabilityRequired => \"a required capability is missing\",\n",
    "+            Self::ReconnectCapabilityRequired => \"a required reconnect capability is missing\",\n",
    "+            Self::RecoveryCapabilityRequired => \"a required recovery capability is missing\",\n",
    "         })\n",
    "     }\n",
    " }\n",
    " \n",
    " impl Error for FoundationProtocolError {}\n",
    " \n",
    "-/// The codes FND-04A \u00a711 fresh-admission refusals use (ARCH-LOGIN-FIRST-PACKETS-V1 \u00a71.2); 1117..\n",
    "+/// The codes FND-04A \u00a711 fresh-admission refusals use (ARCH-LOGIN-FIRST-PACKETS-V1 \u00a71.2); 1119..\n",
    " /// 1199 are reserved for later admission rows and never reused.\n",
    " pub const ADMISSION_REFUSAL_CODES: std::ops::RangeInclusive<u32> = 1100..=1199;\n",
    " \n",
    "@@ -205,7 +213,7 @@ const ADMISSION_REFUSAL_ROWS: [(\n",
    "     FoundationProtocolError,\n",
    "     AdmissionProgression,\n",
    "     AdmissionPublicClass,\n",
    "-); 17] = {\n",
    "+); 19] = {\n",
    "     use AdmissionProgression::{Retryable, SecurityTerminal, Terminal};\n",
    "     use AdmissionPublicClass::{\n",
    "         AuthenticationRequired, CharacterAlreadyActive, ClientUpdateRequired, RetryLogin,\n",
    "@@ -282,6 +290,16 @@ const ADMISSION_REFUSAL_ROWS: [(\n",
    "             Terminal,\n",
    "             ClientUpdateRequired,\n",
    "         ),\n",
    "+        (\n",
    "+            E::ReconnectCapabilityRequired,\n",
    "+            Terminal,\n",
    "+            ClientUpdateRequired,\n",
    "+        ),\n",
    "+        (\n",
    "+            E::RecoveryCapabilityRequired,\n",
    "+            Terminal,\n",
    "+            ClientUpdateRequired,\n",
    "+        ),\n",
    "     ]\n",
    " };\n",
    " \n",
    "@@ -4119,6 +4137,14 @@ mod tests {\n",
    "                 E::AdmissionCapabilityRequired,\n",
    "                 \"ADMISSION_CAPABILITY_REQUIRED\",\n",
    "             ),\n",
    "+            (\n",
    "+                E::ReconnectCapabilityRequired,\n",
    "+                \"RECONNECT_CAPABILITY_REQUIRED\",\n",
    "+            ),\n",
    "+            (\n",
    "+                E::RecoveryCapabilityRequired,\n",
    "+                \"RECOVERY_CAPABILITY_REQUIRED\",\n",
    "+            ),\n",
    "         ]\n",
    "         .into_iter()\n",
    "         .map(|(error, name)| {\n",
    "@@ -4153,7 +4179,12 @@ mod tests {\n",
    "                 assert_eq!(admission_refusal_class(code), None, \"{code}\");\n",
    "                 continue;\n",
    "             }\n",
    "-            assert_eq!(entry[\"default_disposition\"], \"TRANSPORT_FATAL\");\n",
    "+            let expected_disposition = if matches!(code, 1117 | 1118) {\n",
    "+                \"OPERATION_TERMINAL\"\n",
    "+            } else {\n",
    "+                \"TRANSPORT_FATAL\"\n",
    "+            };\n",
    "+            assert_eq!(entry[\"default_disposition\"], expected_disposition);\n",
    "             let (crate_progression, crate_class) =\n",
    "                 admission_refusal_class(code).expect(\"admission code\");\n",
    "             let crate_progression = match crate_progression {\n",
    "@@ -4176,7 +4207,7 @@ mod tests {\n",
    "             );\n",
    "             admission.push(code);\n",
    "         }\n",
    "-        assert_eq!(admission, (1100..=1116).collect::<Vec<_>>());\n",
    "+        assert_eq!(admission, (1100..=1118).collect::<Vec<_>>());\n",
    "         assert_eq!(\n",
    "             ADMISSION_REFUSAL_ROWS\n",
    "                 .map(|(error, _, _)| error.code())\n",
    "@@ -4185,7 +4216,7 @@ mod tests {\n",
    "         );\n",
    "     }\n",
    " \n",
    "-    /// The 17 rows of \u00a71.2, written out once more so a table edit cannot pass unnoticed.\n",
    "+    /// The 17 fresh rows and two D1187 recovery rows, written out once more so a table edit cannot pass unnoticed.\n",
    "     #[test]\n",
    "     fn admission_refusal_class_table() {\n",
    "         use AdmissionProgression::{Retryable as R, SecurityTerminal as S, Terminal as T};\n",
    "@@ -4208,12 +4239,14 @@ mod tests {\n",
    "             (1114, T, CharacterAlreadyActive),\n",
    "             (1115, R, TemporarilyUnavailable),\n",
    "             (1116, T, ClientUpdateRequired),\n",
    "+            (1117, T, ClientUpdateRequired),\n",
    "+            (1118, T, ClientUpdateRequired),\n",
    "         ];\n",
    "         for (code, progression, class) in rows {\n",
    "             assert_eq!(admission_refusal_class(code), Some((progression, class)));\n",
    "         }\n",
    "         // Unassigned in range: generic, no retry. Outside the range: not an admission refusal.\n",
    "-        for code in [1117, 1150, 1199] {\n",
    "+        for code in [1119, 1150, 1199] {\n",
    "             assert_eq!(\n",
    "                 admission_refusal_class(code),\n",
    "                 Some((T, TemporarilyUnavailable))\n",
    "diff --git a/crates/protocol-oteryn/src/npc_service_tests.rs b/crates/protocol-oteryn/src/npc_service_tests.rs\n",
    "index 2f0b474c..03c10160 100644\n",
    "--- a/crates/protocol-oteryn/src/npc_service_tests.rs\n",
    "+++ b/crates/protocol-oteryn/src/npc_service_tests.rs\n",
    "@@ -681,7 +681,7 @@ fn the_registries_bind_the_npc_service_constants() {\n",
    "     let capability = find(\"capabilities\", CAPABILITY_NPC_SERVICE_V1);\n",
    "     assert_eq!(capability[\"name\"], \"NPC_SERVICE_V1\");\n",
    "     assert_eq!(capability[\"offered\"], false);\n",
    "-    assert!(capability.get(\"requires\").is_none());\n",
    "+    assert_eq!(capability[\"requires\"], serde_json::json!([]));\n",
    "     assert!(\n",
    "         capability[\"offer_gate\"]\n",
    "             .as_str()\n",
    "diff --git a/crates/protocol-oteryn/src/quest_log_tests.rs b/crates/protocol-oteryn/src/quest_log_tests.rs\n",
    "index b5af43ac..ab4a5ccf 100644\n",
    "--- a/crates/protocol-oteryn/src/quest_log_tests.rs\n",
    "+++ b/crates/protocol-oteryn/src/quest_log_tests.rs\n",
    "@@ -400,7 +400,7 @@ fn registries_bind_capability_16_and_its_limits() {\n",
    "     let capability = find(\"capabilities\", CAPABILITY_QUEST_LOG_V1);\n",
    "     assert_eq!(capability[\"name\"], \"QUEST_LOG_V1\");\n",
    "     assert_eq!(capability[\"offered\"], false);\n",
    "-    assert!(capability.get(\"requires\").is_none());\n",
    "+    assert_eq!(capability[\"requires\"], serde_json::json!([]));\n",
    "     assert_eq!(\n",
    "         capability[\"command_types\"],\n",
    "         serde_json::json!([COMMAND_TYPE_QUEST_LOG_QUERY])\n",
    "diff --git a/docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json b/docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json\n",
    "index 9e96ed18..e61e5a0e 100644\n",
    "--- a/docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json\n",
    "+++ b/docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json\n",
    "@@ -45,6 +45,7 @@\n",
    "     {\n",
    "       \"id\": 1,\n",
    "       \"name\": \"BESTIARY_CHARMS_V1\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"CHARM-5 wire, CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1 (Sol protocol acceptance #162 5907282001; CharmViewV1 bound and effect_active: Sol ruling #162 5913269950, owner D235)\",\n",
    "       \"owner\": \"Character Authority\",\n",
    "       \"command_types\": [4, 5],\n",
    "@@ -55,6 +56,7 @@\n",
    "     {\n",
    "       \"id\": 3,\n",
    "       \"name\": \"NPC_SERVICE_V1\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"NPC-0 \u00a74 (docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md) with ARCH-NPC-PACKETS \u00a71.3 and \u00a72.3 (docs/architecture/reviews/OTERYN_GAME_ARCH_NPC_PACKETS_2026-10-05.md); numbers fixed by NPC-0 \u00a74 and registered by NPC-WIRE-1\",\n",
    "       \"owner\": \"current ChannelRuntime (the NPC conversations and trade windows of the channel)\",\n",
    "       \"command_types\": [7, 8],\n",
    "@@ -76,6 +78,7 @@\n",
    "     {\n",
    "       \"id\": 6,\n",
    "       \"name\": \"WORLD_SPATIAL_ENTITIES\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"MOVE-RL-11-VISIBILITY-V1 (D84-D87, #162 comment 5875312123), VIS-2\",\n",
    "       \"owner\": \"current ChannelRuntime\",\n",
    "       \"command_types\": [],\n",
    "@@ -86,6 +89,7 @@\n",
    "     {\n",
    "       \"id\": 7,\n",
    "       \"name\": \"CHAT_V1\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"CHAT-0 CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1 \u00a77 (docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md, #1375); numbers leased to CHAT-1 by the #162 control plane\",\n",
    "       \"owner\": \"World communication (current ChannelRuntime for local speech)\",\n",
    "       \"command_types\": [13],\n",
    "@@ -96,6 +100,7 @@\n",
    "     {\n",
    "       \"id\": 8,\n",
    "       \"name\": \"ACHIEVEMENT_NOTICES_V1\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"ACHIEVEMENT-0 ACHIEVEMENT0-GRANT-RUNTIME-V1 \u00a75 (docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md, D292); numbers leased to ACH-NOTIFY-1 by the #1622 control plane\",\n",
    "       \"owner\": \"Achievement domain module\",\n",
    "       \"command_types\": [],\n",
    "@@ -106,6 +111,7 @@\n",
    "     {\n",
    "       \"id\": 10,\n",
    "       \"name\": \"ANALYSER_V1\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"ANALYSERS-0 ANALYSERS0-HUNTING-ANALYSERS-V1 \\u00a74 and \\u00a76 (docs/architecture/reviews/OTERYN_GAME_ANALYSERS0_HUNTING_ANALYSERS_DECISION_2026-10-03.md, D298); numbers leased to ANALYSER-WIRE-1 by the #1622 control plane\",\n",
    "       \"owner\": \"Analyser module (facts derived from committed combat, loot, item-use and progression outcomes)\",\n",
    "       \"command_types\": [],\n",
    "@@ -140,6 +146,7 @@\n",
    "     {\n",
    "       \"id\": 13,\n",
    "       \"name\": \"PACED_MOVEMENT_V1\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"CONDITIONS-0 \u00a74.3 (docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md, as amended by CREATURE-AI-0 \u00a75.1); number allocated by the #1622 control plane and registered by SPEED-1; result field step_wait_ms by the owner's acceptance of OQ-1 (CLIENT-COMPLETION R1), registered by STEP-PACE-WIRE-0\",\n",
    "       \"owner\": \"current ChannelRuntime (Movement owner step pacing)\",\n",
    "       \"command_types\": [],\n",
    "@@ -172,6 +179,7 @@\n",
    "     {\n",
    "       \"id\": 16,\n",
    "       \"name\": \"QUEST_LOG_V1\",\n",
    "+      \"requires\": [],\n",
    "       \"owner_decision\": \"QUEST-GATE-0 \\u00a77 (docs/architecture/reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md); numbers allocated by the #1622 control plane (D485) and registered by QUEST-LOG-WIRE-1\",\n",
    "       \"owner\": \"current ChannelRuntime (the projection of the session's quest copy)\",\n",
    "       \"command_types\": [22],\n",
    "@@ -913,7 +921,9 @@\n",
    "     {\"code\": 1113, \"name\": \"ADMISSION_ACCOUNT_CHARACTER_CONFLICT\", \"category\": \"CONFLICT\", \"default_disposition\": \"TRANSPORT_FATAL\", \"progression\": \"TERMINAL\", \"public_class\": \"SESSION_UNAVAILABLE\"},\n",
    "     {\"code\": 1114, \"name\": \"ADMISSION_INCUMBENT_PROTECTED\", \"category\": \"CONFLICT\", \"default_disposition\": \"TRANSPORT_FATAL\", \"progression\": \"TERMINAL\", \"public_class\": \"CHARACTER_ALREADY_ACTIVE\"},\n",
    "     {\"code\": 1115, \"name\": \"ADMISSION_CAPACITY_EXCEEDED\", \"category\": \"CAPACITY_EXCEEDED\", \"default_disposition\": \"TRANSPORT_FATAL\", \"progression\": \"RETRYABLE\", \"public_class\": \"TEMPORARILY_UNAVAILABLE\"},\n",
    "-    {\"code\": 1116, \"name\": \"ADMISSION_CAPABILITY_REQUIRED\", \"category\": \"UNSUPPORTED_REVISION\", \"default_disposition\": \"TRANSPORT_FATAL\", \"progression\": \"TERMINAL\", \"public_class\": \"CLIENT_UPDATE_REQUIRED\"}\n",
    "+    {\"code\": 1116, \"name\": \"ADMISSION_CAPABILITY_REQUIRED\", \"category\": \"UNSUPPORTED_REVISION\", \"default_disposition\": \"TRANSPORT_FATAL\", \"progression\": \"TERMINAL\", \"public_class\": \"CLIENT_UPDATE_REQUIRED\"},\n",
    "+    {\"code\": 1117, \"name\": \"RECONNECT_CAPABILITY_REQUIRED\", \"category\": \"UNSUPPORTED_REVISION\", \"default_disposition\": \"OPERATION_TERMINAL\", \"progression\": \"TERMINAL\", \"public_class\": \"CLIENT_UPDATE_REQUIRED\"},\n",
    "+    {\"code\": 1118, \"name\": \"RECOVERY_CAPABILITY_REQUIRED\", \"category\": \"UNSUPPORTED_REVISION\", \"default_disposition\": \"OPERATION_TERMINAL\", \"progression\": \"TERMINAL\", \"public_class\": \"CLIENT_UPDATE_REQUIRED\"}\n",
    "   ],\n",
    "   \"allocation_ranges\": {\n",
    "     \"message_type\": {\"invalid\": 0, \"foundation_reserved\": [1, 255], \"future_extension\": [256, 2147483647]},\n",
    "diff --git a/docs/contracts/RESOURCE_LIMITS_REGISTRY.json b/docs/contracts/RESOURCE_LIMITS_REGISTRY.json\n",
    "index b874ca6b..b3d880ae 100644\n",
    "--- a/docs/contracts/RESOURCE_LIMITS_REGISTRY.json\n",
    "+++ b/docs/contracts/RESOURCE_LIMITS_REGISTRY.json\n",
    "@@ -11839,6 +11839,29 @@\n",
    "         \"apps/game-server/tests/support/house_interior_postgres_cases.rs (real PostgreSQL 17.6 boundary)\"\n",
    "       ],\n",
    "       \"notes\": \"Accepted HOUSE-OWN-0 \u00a7\u00a77/12 (HOUSEOWN0-RL-14); HOUSE_RUNTIME0 \u00a76.1 and HOUSE_RT_INBOX_PACKETS \u00a71.3/\u00a72.1. This storage count is separate from the 200-Character scope admission limit HOUSERT0-RL-03. Test occupancy is synthetic inactive DATA, never current actor/session/access authority or a 100-Character capacity claim.\"\n",
    "+    },\n",
    "+    {\n",
    "+      \"id\": \"ADMIT0-RL-01\",\n",
    "+      \"owner_contract\": \"../architecture/reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md\",\n",
    "+      \"resource\": \"Declared Channel required_gameplay_capabilities before dependency closure\",\n",
    "+      \"unit\": \"declared capability ids per Channel scope\",\n",
    "+      \"hard_maximum\": 16,\n",
    "+      \"configurable_range\": {\n",
    "+        \"minimum\": 0,\n",
    "+        \"maximum\": 16\n",
    "+      },\n",
    "+      \"failure_category\": \"INVALID_INPUT\",\n",
    "+      \"allocation_impact\": \"Validate the sorted unique declared list before closure; reject an over-bound or unregistered list before readiness. Dependency closure is separately bounded by MAX_CAPABILITY_COUNT and must contain the build/scope mandatory floor. An empty declaration is valid only for a build/scope with an empty floor.\",\n",
    "+      \"client_visible\": false,\n",
    "+      \"boundary_tests\": [\n",
    "+        \"world_required_capabilities::tests::the_declared_limit_is_registered_and_applies_before_closure\",\n",
    "+        \"world_required_capabilities::tests::real_scope_offer_enforces_the_registered_d1083_floor\"\n",
    "+      ],\n",
    "+      \"consumers\": [\n",
    "+        \"apps/game-server/src/gameplay_transport/world_required_capabilities.rs MAX_DECLARED_REQUIRED_CAPABILITIES\",\n",
    "+        \"apps/game-server/src/node/config.rs ReadinessConfig\"\n",
    "+      ],\n",
    "+      \"notes\": \"ADMIT0 section3.1. The limit does not let the operator omit the mandatory offered floor; no FIELD id is allocated here.\"\n",
    "     }\n",
    "   ]\n",
    " }\n"
  ]
}
```
</details>

<details><summary>I-authored G/#2150 player-origin hunk — D1194</summary>

This exact separately reviewed/formatted patch is assigned to G as the single writer by [CP](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225719708). Combined callee compilation is NOT_RUN pending G source.

```json
{
  "authority": "https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225633260",
  "author": "I/#2187",
  "integration_pr": 2150,
  "base_head": "4e80528e79382807f25395d3465d0418e578bb5a",
  "path": "apps/game-server/src/gameplay_transport/world_item_cast.rs",
  "before_sha256": "fc61cd1a5086cea0d50005f6dc560c8ad495be07f2e215887ddc06ee33dc62b1",
  "after_sha256": "674495174762f6023f9491371c28ca5301fb94cf14cdc8d704aa058ceb168542",
  "patch_sha256": "dbb3a8bc3a481975af3b5e9700a6d3749f7e60c83e999cea2396bdad970aa3d6",
  "callee_signature": "D1194 five arguments ending SpellFieldOrigin::Player(&BoundRequiredCapabilities)",
  "A2_dependency": "ComposedFreshAdmission.required_capabilities, already prepared in private A2",
  "runtime_semantics": "G owns player field/wall-only check after receipt under current authority locks; ordinary item/monster/system paths must retain behavior",
  "status": "FROZEN_PATCH_NOT_COMPILED until G writer available; no shared/G branch writes"
}
```

```json
{
  "patch_lines": [
    "--- a/apps/game-server/src/gameplay_transport/world_item_cast.rs\n",
    "+++ b/apps/game-server/src/gameplay_transport/world_item_cast.rs\n",
    "@@ -1057,7 +1057,20 @@\n",
    "             // Release after S: the COMMIT and the post-commit transaction hold only the lane.\n",
    "             *guards=None;\n",
    "             let attempt=pending.as_ref().ok_or(DurabilityError::Unavailable)?;\n",
    "-            let outcome=match items::apply_spell_items_in_transaction_guarded(&mut tx,&authority,&attempt.request,verdict).await{\n",
    "+            // D1193/D1194: this is the positively resolved player ingress. Unknown origin\n",
    "+            // is never relabelled Monster/System, including retained prepared attempts.\n",
    "+            let Some(origin) = attempt.request.caster_origin.as_ref() else {\n",
    "+                return Err(DurabilityError::Unavailable);\n",
    "+            };\n",
    "+            if origin.actor != actor\n",
    "+                || origin.character_lease_generation != attempt.fence.character_lease_generation\n",
    "+            {\n",
    "+                return Err(DurabilityError::Unavailable);\n",
    "+            }\n",
    "+            let outcome=match items::apply_spell_items_in_transaction_guarded(\n",
    "+                &mut tx,&authority,&attempt.request,verdict,\n",
    "+                items::SpellFieldOrigin::Player(owner.required_capabilities.as_ref()),\n",
    "+            ).await{\n",
    "                 Ok(outcome)=>outcome,\n",
    "                 Err(items::SpellItemError::Rejected(WORLD_ITEM_NEW_GRANT_REFUSED))=>{\n",
    "                     // The writer holds the exact-command lock and found no historical receipt.\n"
  ]
}
```
</details>
