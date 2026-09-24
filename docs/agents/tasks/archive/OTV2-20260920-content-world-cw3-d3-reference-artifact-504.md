> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #694 is merged on protected main as `2b667d98afaacfd0985d1941787c1c8c362d6231`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260920-content-world-cw3-d3-reference-artifact-504

```yaml
task_id: OTV2-20260920-content-world-cw3-d3-reference-artifact-504
title: D3 bounded Reference playable artifact compiler and loader
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-d3-reference-artifact-504
issue: 162
pr: pending
base_sha: e026eeddd687709309589d4a15a796c28cadfe2c
head_sha: pending
final_head_sha: pending
final_head_frozen_at: pending
owner: "Oteryn: content world build"
created_at: 2026-09-20T07:25:00Z
updated_at: 2026-09-20T07:45:33Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/artifact.rs
  - apps/game-server/src/content/compiler.rs
  - apps/game-server/src/content/production.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/reference_artifact.rs
  - apps/game-server/tests/content_reference_artifact.rs
  - docs/agents/tasks/active/OTV2-20260920-content-world-cw3-d3-reference-artifact-504.md
public_contracts:
  - OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v1
depends_on:
  - "owner Option A disposition: issue #162 comments 5748127537 and 5748251837"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
production_authority: NONE
```

## Outcome

Implement the smallest real D3 successor path for the protected one native Item:
canonical six-document World Project -> existing strict project parser -> existing
`link_reference_playable` validator/linker -> existing Content compiler entrypoint ->
deterministic server-authoritative and client-safe
`OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v1` pair -> bounded production-lineage loader ->
non-authoritative stage.

The fixture compiler and fixture artifact loader remain unchanged. The existing
FIRST_PRODUCTION compiler, bytes, staging and activation controller remain unchanged. The
Reference stage has no activation conversion, controller slot, fallback, rollback or authority
mutation API.

## Architecture and source of truth

- **PROVEN:** owner Option A in issue #162 comment `5748127537`, corrected and accepted by
  comment `5748251837`, selects one three-section Reference artifact profile, production Content
  codec/compiler/loader/controller lineage, positive client allowlisting, no compression or
  spatial chunks, and the exact 8,715 / 8,675 / 17,390 byte first-slice caps.
- **PROVEN:** protected base `e026eeddd687709309589d4a15a796c28cadfe2c` contains PR #693's
  superseding rule that licensing/access audit metadata does not itself block technical compile,
  load, stage or qualification.
- **PROVEN:** the protected CW2-B1 adapter binds
  `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`,
  `data/items/items.xml` item `2876`, node digest
  `b7c5c457cdccf047b251313e27cf283442a7346ddc556eaece8c2fdc30d655c3`, to
  `oteryn:item.decor.vase@definition-r1`. Its exact source and mapper provenance remain in the
  canonical project documents and are transitively committed by the source-manifest and package
  provenance digests in both artifacts.
- **DERIVED:** the exact protected corpus is six canonical documents / 6,268 bytes and compiles
  to 788 server bytes plus 776 client bytes (1,564 pair bytes). These are measurements, not new
  maxima. The accepted caps are bounded one-Item profile limits, not full-world, map, project or
  product limits.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task compiles, loads and holds borrowed non-authoritative content evidence.
It performs no production mutation, PREPARE/COMMIT, session/lease/fence operation, controller
installation/restoration, fallback, rollback, durable write or recovery interpretation. Existing
activation authority remains sealed to FIRST_PRODUCTION `GenerationIdentity` and
`FirstProductionExpectation`.

## Artifact and loader shape

- Fixed `OTRPA01\0` / version 1 header and exactly three critical sections: manifest, typed
  identity index and body.
- Each section carries an item count and SHA-256; a whole-artifact SHA-256 trailer covers the
  header, section table and all section bytes.
- The manifest binds source profile, capability, package identity/revision, semantic schema,
  licensing metadata, exact source-manifest digest, typed `WorldId`, coordinate frame, Content
  Lock token, package-provenance digest, compiler profile, canonicalization profile and
  projection.
- The index stores exact `(DefinitionFamily, ProductionKey, DefinitionRevisionRef)` identity plus
  bounded body offset/length/digest. Loading validates the manifest and index without decoding the
  body. Positive lookup decodes only the selected record; missing lookup returns `None` without a
  body scan.
- The server body carries physical class, materialization, stack class and legal destination.
  The client body grammar positively admits only physical and stack classes. Pair staging requires
  identical generation provenance and typed identity, plus equality of those shared fields.
- Exactly one Item, one client definition and one immutable root Content Lock entry are admitted;
  placements, ordering and transitions remain zero.

## Acceptance criteria

- [x] Protected source -> canonical project -> existing linker -> compile -> load/stage preserves
  exact typed Item semantics and provenance.
- [x] Repeated compilation and reversed input document enumeration emit identical exact bytes.
- [x] Server/client projection pair is consistent and the client body is a positive allowlist.
- [x] Positive and missing exact typed lookup use the index without eager whole-body decoding.
- [x] Section-table physical order does not change the indexed semantic identity.
- [x] Unknown, duplicate, missing, overlapping, gapped, trailing, corrupt, oversized,
  cross-profile and encoded-overflow artifacts fail closed.
- [x] Per-section and whole-artifact integrity are independently exercised.
- [x] Exact cap-sized inputs reach header validation, while cap + 1 rejects before decode or
  input-sized allocation.
- [x] Reference stage leaves `ContentActivationController` empty/not-ready and is a compile-time
  type mismatch for `stage_primary`.
- [x] Existing FIRST_PRODUCTION tests and public constants remain green.
- [ ] Exact published head passes hosted repository gates.

## Excluded scope

No production activation, fallback, rollback, controller-authority mutation, fixture compiler
route, alternate loader subsystem, compression, spatial chunks, Cargo/dependency/lock changes,
B5/B6, runtime/client/protocol work, architecture-document mutation, arbitrary author metadata
gameplay semantics, full-world claim or external-repository mutation.

## Implementation / findings

The compiler and production modules provide the profile entrypoints; `reference_artifact.rs`
contains only the profile-specific carrier encoding/decoding and non-authoritative stage types.
Three existing production primitives (`ProductionAtom::from_artifact`, `encode_world_id`,
`parse_world_id`) became crate-visible so both production profiles use the same validated atom and
typed WorldId lineage. Their code and FIRST_PRODUCTION call sites are unchanged.

No P0/P1/P2 finding remains open from implementing-agent self-review at this checkpoint.

## Validation

Pinned local toolchain:
`RUSTUP_HOME=/workspace/scratch/fa7e96b1d063/.rustup-oteryn`,
`CARGO_HOME=/workspace/scratch/fa7e96b1d063/.cargo-oteryn`, Rust/Cargo 1.94.0.

### Focused

- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_reference_artifact` — PASS,
  7/7.
- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_first_production --test content_reference_playable`
  — PASS, 4/4 and 31/31.

### Component/integration

- `cargo +1.94.0 test --locked -p oteryn-game-server --lib` — PASS, 456/456.
- `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings` — PASS.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings` — PASS.
- `cargo +1.94.0 fmt --all -- --check` — PASS.
- `cargo +1.94.0 test --locked -p oteryn-game-server --doc` — PASS, including the
  non-authoritative Reference-stage/controller compile-fail boundary.
- `python -B tools/agents/validate_governance.py` — PASS.
- `python -B tools/repository/validate_repository_policy.py` — PASS.
- `git diff --check` — PASS.

### E2E

`NOT_APPLICABLE`: there is no runtime/client/activation composition in this bounded compiler and
loader slice. The focused public integration test is the required source-to-stage journey.

### Exact-head CI

- final head: pending
- trigger source: normal pull request publication
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending final commit
- method/reviewer: implementing agent, whole diff plus adversarial carrier mutation matrix
- material findings: none open
- verdict: READY_FOR_PUBLICATION

## Independent review

- required: parent control plane owns current policy disposition; this worker does not request
  expensive review
- exact head: pending
- method/auditor: pending parent disposition
- material findings: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## PR and closeout

- changed-file review: exact six changed paths, all within allocated custody; no
  `reference_playable.rs` or fixture `artifact.rs` mutation required
- unresolved review threads: pending PR creation
- related/superseded PRs: none
- protected auto-merge: forbidden in this worker task
- merge commit/result: not requested
- ownership release: pending parent handoff

## Context checkpoint

```yaml
last_progress: all required local Rust, format, governance, policy and diff gates pass
status: ready
branch: agent/content-world-cw3-d3-reference-artifact-504
head_sha: pending
pr: pending
final_head_sha: pending
final_head_frozen_at: pending
ci_trigger_source: pull_request
ci_check_generation: pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: run final local gates, freeze, publish normally, create PR, and verify exact-head CI
```
