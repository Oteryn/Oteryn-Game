# OTV2-20260907-remediation-wp6a-native-client

```yaml
task_id: OTV2-20260907-remediation-wp6a-native-client
title: Repair native client local lifecycle and PKCE bounds
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/remediation-wp6a-native-client-364
issue: 364
pr: null
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: WP6A_NATIVE_CLIENT_REPAIR
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths:
  - apps/client/src/windows_shell.rs
  - crates/identity/src/lib.rs
  - docs/agents/tasks/active/OTV2-20260907-remediation-wp6a-native-client.md
public_contracts: []
depends_on: []
blocks: [WP6_local_correctness]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Repair the current Windows executable lifecycle so local renderer/window failures cannot be reported as successful process completion, suspended/zero-size state cannot trigger an invalid redraw/render loop, and an existing renderer is explicitly resumed. Enforce the accepted PKCE verifier boundary by accepting only 32..=96 entropy bytes. This is a bounded local WP6 slice; it does not make gameplay/network entry available or close WP6/G1.

## Architecture and source of truth

PROVEN: protected admission main is `b3e637dc43a0a31ff2caf24a6450f7df56b43777`. `apps/client/src/windows_shell.rs` currently exits the event loop on several renderer/window failures without preserving a process error, ignores suspend/close errors, returns early from `resumed()` when a Window already exists, and requests redraw whenever a Window exists. `crates/renderer` already maps zero-size Resize to Suspended, so no renderer path is allocated. `crates/identity/src/lib.rs` currently has only a 32-byte minimum; 96 bytes encodes to a 128-character unpadded verifier and 97 exceeds that accepted bound.

PROVEN: #162 comment `5566870660` records this exact three-path allocation. Fresh open-PR search found no overlapping runtime client/renderer/PKCE mutation lineage. WP2/#353 and WP3/#351 are on disjoint Foundation/vendor leases; #308 frozen workflow paths are excluded.

DERIVED: the smallest safe repair is shell-local fatal-error retention/resume/redraw gating plus identity entropy upper-bound enforcement. No Cargo, renderer, runtime-controller or network mutation is necessary for this slice.

UNKNOWN: actual input-to-network/projection controller and Server Seam availability remain future WP6/WP8 acceptance dependencies.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this task changes no gameplay/session authority, PREPARE/COMMIT, durable schema/recovery, production state or protected control plane.

## Acceptance criteria

- [ ] Window/renderer initialization, suspend, resize, render and close failures cannot exit the event loop and then return successful `run()`.
- [ ] An existing Window+renderer resumes through the renderer lifecycle; suspended or zero-size state does not request/render a frame.
- [ ] Normal smoke/close remains successful and no new retry loop is introduced.
- [ ] PKCE rejects 31 and 97 entropy bytes, accepts 32 and 96, and produces verifier lengths 43 and 128 respectively without logging/exposing verifier material.
- [ ] Focused tests, Rust 1.94 fmt/Clippy and applicable Windows exact-head CI pass.
- [ ] Complete three-file diff is independently reviewed only if current policy/risk warrants it; no gameplay/G1 claim is made.

## Excluded scope

No `apps/client/src/lib.rs`, client-runtime, renderer, input, network/projection, protocol, Cargo/lock, server/Foundation/Durability, schema, workflow, ruleset/MQ, production/live-data or external-repository mutation. Do not enable gameplay entry or treat local lifecycle success as native client/server E2E.

## Implementation / findings

Initial allocation only. The worker must keep one canonical branch/PR and may not expand paths. Preserve the existing DX12 renderer and fail-closed pre-native gameplay state. Fatal errors must remain observable to the process rather than being converted to a clean event-loop exit.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending
- result: pending

### E2E

- scenario: NOT_APPLICABLE to gameplay E2E; this slice is local executable lifecycle/identity correctness
- result: pending Windows lifecycle qualification

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- classification: expected SHARED/client paths
- result: pending

## Self-review

- exact head: pending
- method/reviewer: remediation lead after worker publication
- material findings: pending
- verdict: pending

## Independent review

- required: pending under current META AI review policy
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- protected auto-merge: no integration before acceptance
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: exact WP6A three-path allocation created from protected main
status: implementing
branch: fix/remediation-wp6a-native-client-364
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
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
next_action: implement shell fatal-error/resume/redraw gating and PKCE 32..=96 bound within the three-path lease
```
