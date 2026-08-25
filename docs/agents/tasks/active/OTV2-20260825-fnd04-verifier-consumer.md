# OTV2-20260825-fnd04-verifier-consumer

```yaml
task_id: OTV2-20260825-fnd04-verifier-consumer
title: Implement production FND-04 verifier consumer seam
mode: IMPLEMENT
status: ready_for_review
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/fnd04-verifier-consumer-115
issue: 115
pr: 151
base_sha: 12d1920b19d09dd3ce76e8910cc7bde401e63428
head_sha: pending_remote_freeze
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT security implementation worker for Issue #115
created_at: 2026-08-25T10:52:00+02:00
updated_at: 2026-08-25T10:52:00+02:00
execution_budget_minutes: 120
large_budget_reason: Security-sensitive JWS/Ed25519 verifier requires bounded parser, purpose separation, authoritative current-evidence checks, full TDD, supply-chain validation and independent exact-head review.
owned_paths:
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - docs/architecture/reviews/OTERYN_GAME_FND04_VERIFIER_CONSUMER_DELIVERY_2026-08-25.md
  - docs/agents/tasks/active/OTV2-20260825-fnd04-verifier-consumer.md
public_contracts:
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
depends_on:
  - issue:115
  - issue:128
  - issue:131
  - task:OTV2-20260825-fnd04-verifier-allocation
blocks:
  - issue:115
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Implement a Game-owned verifier/consumer seam that uses a verifier-fixed fresh/recovery trust context, validates bounded JWS Compact tokens with exact `Ed25519`, and combines authenticated claims with caller-provided current security/game evidence before returning trusted typed facts.

## Acceptance criteria

- [ ] Fresh and recovery profiles are purpose-separated with no reinterpretation/fallback.
- [ ] Token/header/payload/base64/JSON bounds and duplicate-member rejection match accepted profiles.
- [ ] Algorithm/key/trust/signature classification precedes semantic payload disclosure.
- [ ] Current security/trust evidence proves source-age <=5s and non-rollback floors.
- [ ] Fresh success returns existing `FreshAdmissionFacts`; recovery success returns non-authoritative typed facts only.
- [ ] Verification alone creates/revives/rebinds no GameSession and consumes no replay nonce.
- [ ] Only direct standards-conformant dependencies are added and pinned through workspace/app Cargo + lockfile.
- [ ] Focused tests, package/workspace tests, strict Clippy/rustfmt, governance/architecture and diff checks pass.
- [ ] Fresh non-authoring local `qwen2.5-coder:14b` review is bound to the final exact PR head with zero material findings.
- [ ] Exact-head repository CI including `game-gate` passes before squash merge.

## Excluded scope

No listener/socket bind, production port/TLS cert/private key/KMS selection, no durable journal implementation, no direct replay consumption, no GameSession creation/revival/rebind in the verifier, no gameplay/Durability/client implementation, no Platform/external-repository mutation.

## Validation

- TDD: RED/GREEN cycles complete; per-`kid` revocation and non-rollback-floor regressions are green
- package/workspace: green locally; exact-head repository CI required
- supply chain: locked workspace; no repair dependencies added
- independent review: REQUIRED, fresh non-authoring local model on exact final head
- E2E: verifier-only focused security integration; no live network/account/session path

## Context checkpoint

```yaml
last_progress: Separate fresh/recovery typed consumers now make trust decisions per kid and reject evidence below durable, source-owned non-rollback floors. The branch is awaiting independent exact-head security review and repository CI.
status: ready_for_review
branch: feat/fnd04-verifier-consumer-115
head_sha: pending_remote_freeze
pr: 151
final_head_sha: null
final_head_frozen_at: null
independent_review_required: true
owner_action_required: null
blocker: independent_exact_head_review_and_ci_pending
next_action: Freeze the remote PR head, complete non-authoring security review and exact-head repository CI, then merge only if all required gates pass.
```
