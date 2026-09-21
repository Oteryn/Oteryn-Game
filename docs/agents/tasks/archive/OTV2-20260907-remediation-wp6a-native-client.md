# OTV2-20260907-remediation-wp6a-native-client

```yaml
task_id: OTV2-20260907-remediation-wp6a-native-client
title: Repair native client local lifecycle and PKCE bounds
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 364
pr: 368
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: 4282af5f3475a3af70dff0b35904b005012d51c1
final_head_sha: 4282af5f3475a3af70dff0b35904b005012d51c1
final_head_frozen_at: null
owner: WP6A_NATIVE_CLIENT_REPAIR
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
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

- [x] Window/renderer initialization, suspend, resize, render and close failures are retained and returned by `run()` after event-loop exit.
- [x] An existing Window+renderer resumes through the renderer lifecycle; suspended or zero-size state does not request/render a frame.
- [x] Normal smoke/close remains successful and no new retry loop is introduced.
- [x] PKCE rejects 31 and 97 entropy bytes, accepts 32 and 96, and produces verifier lengths 43 and 128 respectively without logging/exposing verifier material.
- [x] Executed focused identity/client tests, Rust 1.94 fmt/Clippy and hosted exact-head Windows build/strict Clippy/visible pre-native smoke pass; direct OS minimize/restore callbacks and Windows-only unit tests remain NOT_EXECUTED.
- [x] Complete three-file diff is reviewed under current policy; no gameplay/G1 claim is made.

## Excluded scope

No `apps/client/src/lib.rs`, client-runtime, renderer, input, network/projection, protocol, Cargo/lock, server/Foundation/Durability, schema, workflow, ruleset/MQ, production/live-data or external-repository mutation. Do not enable gameplay entry or treat local lifecycle success as native client/server E2E.

## Implementation / findings

Implemented within the three-path allocation. `Application` retains only its first fatal lifecycle classification, exits, and returns that classification after `run_app()` completes. Window creation, renderer initialization, resume, suspend, resize, render and close have distinct non-secret classifications. Ordinary close and `--smoke` remain clean exits. Resume uses the existing Window's current inner size. Redraw request and render are gated on `SurfacePhase::Configured`; there is no retry loop and the renderer/DX12 state machine is unchanged.

PKCE now accepts only 32..=96 bytes and reports a non-material-bearing `EntropyLengthOutOfRange` classification. Existing `SecretString` Debug/Display redaction is unchanged. Four focused boundary tests cover 31/32/96/97 bytes and assert the exact accepted verifier lengths.

## Validation

### Focused

- command/run: `cargo +1.94.0 test --locked -p oteryn-identity`
- result: PASS — 6 unit tests, including all four PKCE boundaries, plus doc-tests
- command/run: `cargo +1.94.0 clippy --locked -p oteryn-identity --all-targets -- -D warnings`
- result: PASS

### Component/integration

- command/run: `cargo +1.94.0 fmt --all --check`
- result: PASS after applying rustfmt
- command/run: `cargo +1.94.0 clippy --locked -p oteryn-client --all-targets -- -D warnings`
- result: PASS on the Linux host; Windows-only shell code is excluded by target cfg
- command/run: `cargo +1.94.0 test --locked -p oteryn-client`
- result: PASS — 2 unit tests plus doc-tests; Windows-only shell helper tests are excluded by target cfg on this host

### E2E

- scenario: NOT_APPLICABLE to gameplay E2E; this slice is local executable lifecycle/identity correctness
- result: Windows OS callbacks could not be deterministically exercised on the Linux implementation host. Pure helper regressions cover first-fatal retention and configured-only redraw eligibility without allocating a renderer; hosted Windows build/Clippy/visible pre-native smoke later passed in exact-head CI. Direct OS minimize/restore callbacks and Windows-only unit tests remain NOT_EXECUTED, and no gameplay E2E claim is made.

### Exact-head CI

- final head: `4282af5f3475a3af70dff0b35904b005012d51c1`
- trigger source: pull_request
- workflow/run/job: Merge gate `34102705942`; hosted Windows job `101680834066`; Linux job `101680834072`
- classification: SHARED/client paths
- result: PASS — Windows build, strict Clippy and visible pre-native smoke passed; Linux validation passed. Direct OS minimize/restore callbacks and Windows-only unit tests were not executed and are not claimed.

## Self-review

- exact head: `4282af5f3475a3af70dff0b35904b005012d51c1`
- method/reviewer: remediation lead complete three-file effective-diff review
- material findings: zero open
- verdict: PASS

## Independent review

- required: completed on the stable risk-bearing source candidate; no repeat review required after source-neutral upstream reconciliation
- exact head: `8c716055469c6430851ea419c6b2cbc4708c24be`; final delivery head `4282af5f3475a3af70dff0b35904b005012d51c1` preserves the reviewed task-source blobs
- method/auditor: Codex independent review
- material findings: none
- verdict: PASS; final reconciliation changed only upstream ancestry, and final-head self-review/blob comparison found no risk-bearing source change requiring re-review

## PR and closeout

- changed-file review: PASS — exact three allocated files; protected-main blob readback matches
- unresolved review threads: none
- protected auto-merge: full Merge Queue `34103516674` SUCCESS
- merge commit/result: PR #368 squash-merged as `686cf85cac3e7f66a473527c198705466dc5c3bd` and read back from protected main
- ownership release: complete; broader F04/F05/WP6 and G1 remain open
- branch disposition: merged task branch deleted; live matching-ref readback returned no branch

## Context checkpoint

```yaml
last_progress: PR 368 integrated through successful Merge Queue and read back from protected main; bounded F06 task archived and ownership released
status: completed
branch: null
head_sha: 4282af5f3475a3af70dff0b35904b005012d51c1
pr: 368
final_head_sha: 4282af5f3475a3af70dff0b35904b005012d51c1
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
next_action: null
```
