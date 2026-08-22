# OTV2-20260822-impl-qa-e2e

```yaml
task_id: OTV2-20260822-impl-qa-e2e
title: Implement native QA E2E evidence platform shell
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-qa-e2e-01
pr: null
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owner: chat-github-20260822-qa-e2e
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-22T19:47:00+02:00
execution_budget_minutes: 60
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
public_contracts:
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
```depends_on:
  - Oteryn-Game#45
  - Oteryn-Game#46
blocks:
  - OTV2-IMPL-MOVE
  - OTV2-IMPL-COMBAT
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Deliver the smallest reusable evidence/test shell that classifies real-boundary journeys truthfully and cannot treat mocks, direct state mutation or environment startup as terminal E2E proof.

## Source facts

- `PROVEN`: this lane may evolve as production seams merge, but cannot invent missing Foundation/Domain/Content behavior.
- `PROVEN`: Tier 1 requires real production transport/server/persistence boundaries; Tier 2 requires instrumented native client evidence.
- `PROVEN`: missing prerequisites yield `BLOCKED` or `NOT_EVALUATED`, never fabricated PASS.

## Acceptance criteria

- [x] Deterministic scenario identity/config includes exact build/protocol/content/revision/seed/clock/topology/fault evidence fields.
- [x] Stable result classification distinguishes `PASS / UNSTABLE / FAIL / BLOCKED / NOT_EVALUATED`.
- [x] First-divergence and phase evidence are preserved without rewriting historical failed attempts.
- [x] Negative tests prove mock/direct-domain shortcuts cannot satisfy terminal Tier 1/Tier 2 evidence.
- [x] Cleanup status and diagnostic artifact references are first-class evidence.
- [x] Tests compile only against actually merged product seams; unavailable journeys remain explicit `NOT_EVALUATED` blockers.
## Implementation plan

1. RED: add integration-test-shell tests for scenario identity, evidence validation and result classification independent of missing product seams.
2. GREEN: implement only test-side evidence helpers inside `apps/game-server/tests/**`; no production adapter or direct authoritative mutation helper.
3. Add negative evidence-tier tests and deterministic cleanup/first-divergence accounting.
4. As FOUNDATION and later VSL seams merge, rebase/update this lane only through coordinator-approved integration and add real journeys without erasing prior outcomes.
5. Run full workspace CI, self-review and truthful closeout.

## Excluded scope

No production-default test adapter, no live account/data, no synthetic-client-as-Tier2 claim, no direct-domain-as-Tier1 shortcut, no product behavior invented to make scenarios green.

## Validation

### Focused
- command/run: `cargo test -p oteryn-game-server --test evidence_shell`
- result: PASS — 8 evidence-shell tests; 0 failed; Tier1/Tier2/Tier3 boundary rejection, exact comparison-cell classification, first-divergence, cleanup and physical-attempt retention covered

### Component/integration
- command/run: `cargo test -p oteryn-game-server --tests` and `cargo clippy -p oteryn-game-server --tests -- -D warnings`
- result: PASS — 3 existing package tests + 8 evidence-shell tests; Clippy `-D warnings` PASS; scoped `rustfmt --check` PASS on allocated test files

### E2E
- scenario: Foundation connect/admit/reconnect becomes countable only after real merged transport/admission seam
- result: `NOT_EVALUATED` until prerequisite exists

## Context checkpoint

```yaml
last_progress: TDD QA evidence shell is GREEN: canonical comparison-cell identity, all ADR phases, Tier boundary proof, anti-mock/direct-domain rejection, first-divergence validation, cleanup gating and PASS/UNSTABLE/FAIL/BLOCKED/NOT_EVALUATED population classification are implemented entirely under the allocated test path; 8/8 focused tests, package tests and Clippy -D warnings pass.
status: implementing
branch: agent/otv2-impl-qa-e2e-01
head_sha: 58d64130cc0526001bd1c9a00a179e1c39ad6e51
pr: null
blocker: real Tier 1/Tier 2 Foundation journeys remain `NOT_EVALUATED` until the required production transport/admission/persistence and native-client seams are merged
owner_action_required: null
next_action: commit and push the evidence shell; later add real journeys only after the coordinator proves their merged prerequisites without rewriting this historical synthetic-shell evidence.
```
