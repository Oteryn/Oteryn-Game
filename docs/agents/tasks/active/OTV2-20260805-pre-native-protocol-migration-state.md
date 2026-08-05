# OTV2-20260805-pre-native-protocol-migration-state

```yaml
task_id: OTV2-20260805-pre-native-protocol-migration-state
title: Define the post-migration pre-native-protocol client state
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-01-pre-native-protocol-state
pr: 44
base_sha: 100f2d538fa1fc5d0f32d2aed491778bc60033a1
head_sha: 2543ead27846bf78ff6623d2e4d9cd028a1b24b3
owner: GPT-5.6-Thinking-architecture-coordinator
created_at: 2026-08-05T23:26:00+02:00
updated_at: 2026-08-05T23:31:00+02:00
execution_budget_minutes: 30
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md
  - docs/agents/tasks/active/OTV2-20260805-pre-native-protocol-migration-state.md
public_contracts:
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md
depends_on:
  - ADR-0001
  - ADR-0002
  - ADR-0008
  - FND-01
  - VSL-02
blocks:
  - the atomic destination migration until the no-adapter transition state is explicit
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient (read-only evidence)
```

## Outcome

Record the owner-accepted migration invariant that the atomic Rust-client destination migration may produce a compilable and launchable `pre-native-protocol` client state, while production gameplay entry remains explicitly unavailable until the native Oteryn protocol and the later runtime/admission gates are implemented and validated.

## Architecture and source of truth

### PROVEN

- ADR-0002 requires `FND-01`, then `VSL-02`, then one atomic destination migration/workspace PR before `FND-02`.
- ADR-0008 fixes `protocol-canary` as `REFERENCE_ONLY` and excludes it from the destination production dependency graph, negotiation, fallback and translation paths.
- The existing Rust client source workspace contains production dependencies on `protocol-canary` and does not contain an implemented `protocol-oteryn` crate.
- `FND-02` owns the native protocol contract; speculative protocol constants, placeholder codecs and false compatibility claims are forbidden before that gate.

### ACCEPTED

- The owner accepted on 2026-08-05 that the migrated client may compile and launch in a `pre-native-protocol` state.
- The transition state contains no production gameplay protocol adapter.
- Gameplay entry is fail-closed and visibly unavailable; no credentials are consumed and no false success is presented.
- `FND-02` is necessary to leave the no-native-protocol state, but production gameplay still requires the applicable identifier, runtime and admission gates, including `FND-ID-01`, `FND-03` and `FND-04`.

## Acceptance criteria

- [x] ADR-0011 defines the transition state and its exact security/product invariants.
- [x] The migration is not allowed to retain `protocol-canary` as a temporary production adapter.
- [x] The migration is not allowed to create an empty or speculative `protocol-oteryn` production crate.
- [x] The client may compile and launch, but every production gameplay-entry path fails closed before gameplay credential handoff or endpoint connection.
- [x] User-facing state cannot imply that gameplay is available.
- [x] `FND-02` is recorded as necessary but not sufficient for production gameplay; `FND-ID-01`, `FND-03` and `FND-04` remain mandatory.
- [x] No runtime code, workspace, protocol schema or external repository is modified.
- [ ] Exact-head documentation/governance validation passes.

## Excluded scope

- No client migration or Cargo workspace mutation.
- No `protocol-oteryn` schema, codec, transport or capability implementation.
- No Platform, Game Gateway, Game Session or game-server implementation.
- No changes to `blakinio/otclient`.
- No decision about the final launcher, login-screen or offline-tool UX beyond explicit non-deception and fail-closed behavior.

## Implementation / findings

- Added ADR-0011 as the canonical owner-accepted transition contract.
- Named `pre-native-protocol` as a programme state, not a player-selectable protocol mode, profile or release channel.
- Required a launchable client shell with no production gameplay adapter.
- Required failure before gameplay credential consumption, Game Session binding or gameplay endpoint connection.
- Required exact migration evidence proving that Canary, speculative native stubs and development fixtures cannot enter production artifacts.
- Preserved the later ownership of identifier, protocol, runtime and admission gates.

## Validation

### Focused

- command/run: complete PR #44 changed-file review against base `100f2d538fa1fc5d0f32d2aed491778bc60033a1`
- result: `PASS`; exactly two documentation files changed and both remain within declared ownership

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only task
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable product change
- result: `NOT_APPLICABLE`

### Exact-head CI

- head before this checkpoint: `2543ead27846bf78ff6623d2e4d9cd028a1b24b3`
- workflow/run: pending final task-record head
- result: pending

## Independent audit

- exact reviewed head: `2543ead27846bf78ff6623d2e4d9cd028a1b24b3`
- method/auditor: adversarial full-diff architecture review against ADR-0001, ADR-0002, ADR-0008, FND-01, VSL-02, FND-ID-01, FND-02, FND-03 and FND-04
- resolved material finding:
  - the initial owner confirmation named `FND-02` as the blocking protocol milestone, but `FND-02` alone cannot authorize production gameplay; ADR-0011 now states that it ends the no-native-protocol design state while identifier, runtime, admission and exact validation gates remain mandatory
- open material findings: none
- verdict: `PASS`

## Context checkpoint

```yaml
last_progress: ADR-0011 is complete, PR #44 is open, the exact diff is bounded and the independent architecture audit passed.
status: validating
branch: docs/fnd-01-pre-native-protocol-state
head_sha_before_checkpoint: 2543ead27846bf78ff6623d2e4d9cd028a1b24b3
pr: 44
blocker: Required exact-head GitHub workflows have not yet been inspected for the final task-record commit.
next_action: Inspect exact-head workflows, correct any failure, then mark PR #44 ready and merge only if required checks pass.
```
