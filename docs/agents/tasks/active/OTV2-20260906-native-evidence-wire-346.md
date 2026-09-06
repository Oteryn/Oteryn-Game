# OTV2-20260906-native-evidence-wire-346

```yaml
task_id: OTV2-20260906-native-evidence-wire-346
title: Qualify bounded native evidence wire codec
mode: IMPLEMENT
status: waiting
admission_state: NOT_ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-evidence-wire-346
issue: 346
pr: null
allocation_source_main_sha: 9ceeb231e2bb92c70eae83369c84f0f3fa6fccb2
admission_main_sha: NOT_ADMITTED
base_sha: NOT_ADMITTED
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated wire codec worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/admission_evidence.rs
  - apps/game-server/tests/admission_evidence_wire.rs
  - apps/game-server/src/lib.rs
  - docs/agents/tasks/active/OTV2-20260906-native-evidence-wire-346.md
  - docs/superpowers/plans/2026-09-06-native-evidence-wire.md
public_contracts: [FND-NATIVE-SOURCE-EVIDENCE-V1, NATIVE-SOURCE-RESOURCE-ENVELOPE-V1]
depends_on: [330, 339, protected_346_allocation, protected_registry_342]
blocks: [native_source_wire_qualification]
cross_repository_coordination_id: OTV2-NATIVE-SOURCE-EVIDENCE
external_repositories: []
```

## Outcome

Encode exact bounded native evidence requests and decode closed responses into unprivileged wire data for all four accepted operations. This independently testable prerequisite is useful before an actual producer is available, but it does not establish authenticated source or Server Seam readiness.

## Architecture and source of truth

PROVEN: source330 accepted at5412215718d66c743fb78eadc561e6a23b5e2b5f, source339 atc4099a5a626c5fb17cfe40c11cf8dd813b4550e7 and configuration addendum344 at9ceeb231e2bb92c70eae83369c84f0f3fa6fccb2. Read their full native source/resource decisions and Foundation FND04 contracts. Registry342/PR345 must be protected and read back before admission to the fixed-bound implementation. DERIVED: pure codec can qualify without installing producer configuration. UNKNOWN: actual counterpart, PKI/bootstrap and interoperable fresh evidence literal bindings. Source helper is an implementation adviser, not a second architecture authority.

Fresh evidence purpose/scope/key_purpose literals are not spelled by330. Use bounded independently supplied expected V1 bindings, never response/token-derived expectations or invented production literals. Explicit V2 constants and established fresh issuer/profile remain exact. Do not equate credential purpose with evidence key_purpose.

## High-risk authority/recovery qualification

No production mutation, PREPARE/COMMIT, controller installation or persisted recovery interpretation occurs: those mutation boundaries are NOT_APPLICABLE. Parser security and provenance binding are applicable. Test one independent input mutation per negative across every operation and fresh/recovery substitution direction. Expected bindings originate independently of hostile responses. Wire observations remain inert; no sealed owning/current source implementation, accepted-source floor, local publication revision or re-aged timestamp may be created.

## Acceptance criteria

- [ ] Exact four request families, Account13/trust14-field observed responses, and exactly3-field failures; all required fields and no extras.
- [ ] Closed bounded flat parsing rejects unknown, duplicate, nested, malformed, oversized and trailing input before peer-sized decoded allocation; no generic DOM-first path.
- [ ] Exact operation/version/source/account/trust-scope binding; canonical UUID, decimal u64/i64, booleans, key-ID grammar and Ed25519 base64 bytes preserved.
- [ ] Every applicable max/max+1/overflow/escape/encoding boundary and both fresh/recovery substitution directions have independent tests and positive controls.
- [ ] Denial/untrust stays observation; failure carries no fabricated authority facts; unprivileged output cannot bypass Foundation seals.
- [ ] Focused RED/GREEN, full affected validation and whole-diff self-review; independent exact-head parser/security review with no unresolved material findings.
- [ ] Protected registry readback, exact-head canonical CI and normal Merge Queue; Work protected readback/archive/release.

## Excluded scope

No HTTP/TLS client, endpoint/method/status mapping, deployment, producer descriptor installation, Foundation/Durability/Cargo/SQL/registry/workflow edits, source registration, live-account/secret/Platform/Atlas/META writes. lib.rs permits only the single new module export under the explicit247 lease amendment. No fixture proves actual-source availability or closes247.

## Implementation / findings

NOT_STARTED. Preserve existing shared247 branch and work; no new runtime write before protected allocation/admission. Consult the exact plan for field matrix and boundary coverage.

## Validation

Focused: actual codec RED/GREEN and independent golden/negative/limits matrix. Component: Rust1.94 locked affected library/test targets, strict all-target Clippy, fmt, applicable architecture/governance. E2E: live transport/PostgreSQL NOT_APPLICABLE to pure codec; no dependency failure may be relabeled passing. Exact-head canonical CI and protected Merge Queue mandatory.

## Self-review

NOT_STARTED; worker must review full changed code/tests, field/limit matrix and all acceptance criteria.

## Independent review

Required YES for hostile-input parser and cross-repository provenance binding; separate non-author exact-head review.

## PR and closeout

One branch/PR after admission, Work publication/integration control. No force/rebase/reset/no-op retrigger. Preserve counters across bounded windows. Task metadata cannot contain its own future SHA; PR/check evidence records final head.

## Context checkpoint

```yaml
last_progress: exact prospective pure wire prerequisite and247 export lease amendment prepared
status: waiting
admission_state: NOT_ADMITTED
execution_window_number: 0
execution_windows_completed: 0
worker_rotations: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
owner_action_required: null
blocker: protected_allocation_and_registry_readback
next_action: Work verifies protected allocation and registry then admits one bounded exclusive wire worker
```
