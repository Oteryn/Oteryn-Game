> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #697 is merged on protected main as `fd6b0400186cc546c8b3388398d96e269e4e6089`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260920-content-world-cw4-d4-reference-item-runtime-consumption-504

```yaml
task_id: OTV2-20260920-content-world-cw4-d4-reference-item-runtime-consumption-504
title: CW4 non-authoritative Reference Item runtime consumption
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw4-d4-reference-item-runtime-consumption-504
issue: 162
pr: pending
base_sha: 2b667d98afaacfd0985d1941787c1c8c362d6231
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world build"
created_at: 2026-09-20T08:35:07Z
updated_at: 2026-09-20T08:37:34Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_runtime.rs
  - docs/agents/tasks/active/OTV2-20260920-content-world-cw4-d4-reference-item-runtime-consumption-504.md
public_contracts: []
depends_on:
  - "protected D3 Reference artifact/compiler/stage #694"
  - "protected CW4 world runtime #669"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Add one crate-private immutable `NonAuthoritativeReferenceItemRuntimeView`. It consumes the
existing `NonAuthoritativeReferenceStage` and a `RuntimeScopeRefV1`, rejects a different
`WorldId`, retains the exact `ReferencePlayableGenerationIdentity`, typed Item identity and
server/client artifact digests, and resolves authoritative Item semantics only through
`stage.server_artifact().lookup_server_item(exact_identity)`.

The view grants no activation, controller, instance, materialization, placement, location,
inventory, trade, durability, client or protocol capability.

## Architecture and source of truth

- **PROVEN:** protected base `2b667d98afaacfd0985d1941787c1c8c362d6231` includes D3 #694's
  bounded one-Item compiler, indexed loader and non-authoritative stage, plus protected
  `world_runtime.rs` #669.
- **PROVEN:** the canonical protected B1 vase project path produces exact typed identity
  `Item / oteryn:item.decor.vase / definition-r1` with `Physical`, materializable,
  `NonStackable` and `CharacterInventory` semantics.
- **PROVEN:** `RuntimeScopeRefV1::world_id()` is the existing typed world binding. This task adds
  no alternate scope or world identity.
- **DERIVED:** retaining the already validated staged generation identity avoids a second
  registry, loader, compiler or source-read path; one exact indexed server lookup supplies the
  owned immutable runtime semantic value.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this change creates an immutable non-authoritative read view. It performs no
production mutation, PREPARE/COMMIT, session/lease/generation grant, controller
installation/restoration, durable write or recovery interpretation. A wrong world fails before
the view exists, and the existing activation controller remains empty and not ready.

## Acceptance criteria

- [x] Canonical protected B1 project -> existing linker -> existing compiler -> existing stage ->
  runtime view retains exact same-world generation identity, typed Item ref and artifact digests.
- [x] Authoritative Item semantics are resolved only by the staged server artifact's exact typed
  lookup and remain `Physical`, materializable, `NonStackable`, `CharacterInventory`.
- [x] An unknown exact typed identity returns `None` from the immutable one-Item view.
- [x] A different `WorldId` fails closed before runtime-view construction.
- [x] `ContentActivationController` remains empty, inactive and not ready.
- [x] Required unchanged focused suites, game-server library, formatting, strict all-target
  Clippy, governance, repository policy and diff checks pass on the final local candidate.
- [ ] The exact published head passes hosted repository gates.

## Excluded scope

No D3 format/loader/stage mutation; no `lib.rs` or `content/mod.rs`; no direct project/source read
in production; no second registry/loader; no `LocalObject` fabrication; no placement, transition,
instance, materialization, location, inventory, trade, durability, activation, controller, fence,
client or protocol construction; no B5; no Cargo, lock, workflow, architecture or contract change;
no Merge Queue submission or protected integration.

## Implementation / findings

The runtime view owns only the supplied scope, an exact clone of the stage's generation identity
and the `ReferenceServerItem` decoded through the stage server index. All fields are private and
there are no mutating methods. Exact lookup returns the retained server semantic record only for
the full typed identity; every other identity returns `None`.

No P0/P1/P2 finding is open from implementation self-review so far.

## Validation

Pinned toolchain: Rust/Cargo 1.94.0 via
`RUSTUP_HOME=/workspace/scratch/fa7e96b1d063/.rustup-oteryn` and
`CARGO_HOME=/workspace/scratch/fa7e96b1d063/.cargo-oteryn`.

### Focused

- `cargo +1.94.0 test --locked -p oteryn-game-server --lib world_runtime::tests::protected_b1_vase_stage_binds_exact_runtime_item_view_without_controller_authority -- --exact`
  — PASS, 1/1.
- `cargo +1.94.0 test --locked -p oteryn-game-server --lib world_runtime::tests::reference_item_runtime_view_rejects_a_different_world -- --exact`
  — PASS, 1/1.
- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_reference_artifact --test content_reference_playable --test content_first_production --test content_world_cw2_b1_import`
  — PASS, 7/7, 31/31, 4/4 and 6/6.

### Component/integration

- `cargo +1.94.0 test --locked -p oteryn-game-server --lib` — PASS, 458/458.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings` — PASS.
- `cargo +1.94.0 fmt --all -- --check` — PASS.
- `python -B tools/agents/validate_governance.py` — PASS.
- `python -B tools/repository/validate_repository_policy.py` — PASS.
- `git diff --check` — PASS.

### E2E

Canonical B1 project-to-runtime-view journey is covered by the first focused in-process test.
No executable gameplay, activation or client composition is authorized by this increment.

### Exact-head CI

- final head: pending
- trigger source: normal ready pull request publication
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending final commit
- method/reviewer: implementing agent, whole diff and adversarial exact-identity/world-boundary sweep
- material findings: none open after the exact-identity/world-boundary sweep
- verdict: READY_FOR_PUBLICATION

## Independent review

- required: parent control plane owns current policy disposition; this worker does not request review
- exact head: pending or `NOT_APPLICABLE`
- method/auditor: pending parent disposition
- material findings: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending final candidate
- unresolved review threads: pending PR creation
- related/superseded PRs: none
- protected auto-merge: forbidden for this task
- merge commit/result: not requested
- ownership release: pending parent handoff

## Context checkpoint

```yaml
last_progress: all required local Rust, format, governance, repository-policy and diff gates pass
status: ready
branch: agent/content-world-cw4-d4-reference-item-runtime-consumption-504
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
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
next_action: freeze the exact candidate, publish normally, create a ready PR and verify hosted CI
```
