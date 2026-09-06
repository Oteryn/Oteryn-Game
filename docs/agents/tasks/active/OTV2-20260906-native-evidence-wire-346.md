# OTV2-20260906-native-evidence-wire-346

```yaml
task_id: OTV2-20260906-native-evidence-wire-346
title: Qualify bounded native evidence wire codec
mode: IMPLEMENT
status: validating
admission_state: ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-evidence-wire-346
issue: 346
pr: null
allocation_source_main_sha: 9ceeb231e2bb92c70eae83369c84f0f3fa6fccb2
admission_main_sha: ad7273e3e91a4e4254abb9aa2710c7e0c9754afe
base_sha: ad7273e3e91a4e4254abb9aa2710c7e0c9754afe
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: resource_integration_audit
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

Work5559516855 admitted immutablead7273e3e91a4e4254abb9aa2710c7e0c9754afe in window1 13:27–14:27UTC. Implemented four request/response families with independently supplied V1 expected literals and exact V2 constants. Fixed inline parser/output storage prevents peer-sized heap allocations; raw8192, key64, decoded256, field16 and request1024 bounds precede growth. Output is inert wire facts only. Actual compiler RED unresolved module preceded GREEN. Independent per-field and cross-purpose corpus covers missing/null/duplicate, scalar/binding/substitution, canonical numbers/UUID/key/base64, UTF8/surrogates, truncation and caps. No source capability/transport/schema/Cargo was added. Exact-head CI and independent review pending.

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
last_progress: bounded four-operation codec and independent negative corpus implemented
status: validating
admission_state: ADMITTED
execution_window_number: 1
execution_windows_completed: 0
worker_rotations: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
owner_action_required: null
blocker: independent_review_and_exact_head_CI
next_action: Work independently reviews staged material candidate and publishes for exact-head CI
```

## Window1 validation evidence

Compiler RED: wire346-red.log unresolved admission_evidence module. Initial five integration cases GREEN; expanded matrix and strict all-target Clippy qualification recorded at handoff. Fixed inline representation intentionally retains bounded stack storage instead of boxing; local large-enum lint explanations preserve this allocation property. The sole UTF8 invariant assertion operates on private parser-validated storage. No unsafe code or global allocator hook is used. Decoder has no heap allocation path; request String is created only after complete1024-byte bounded encoding. This source-level allocation argument is not a measurement of TLS or whole-executor memory.

Final local candidate checks: library291/291 PASS (unconfigured legacy SQL returns are not DB proof), dedicated wire integration8/8 PASS, strict all-target Clippy PASS, fmt and whitespace PASS, governance26/9 PASS. Self-review inspected the full codec, fixed array growth, field masks, scalar/Unicode/number parsing, exact bindings and all eight independent corpus tests. No unresolved self-review finding; independent reviewer must qualify the exact staged tree.

## Independent numeric test repair — cycle1

Accepted P2: malformed source_revision tests retained decision_identity1, allowing secondary identity mismatch to mask numeric validation. Numeric negatives now carry the identical malformed revision/decision text; separate otherwise-valid decision mismatch stays covered. Both Account families test minimum generation0/max/max+1; all four families reject zero source revision while timestamp/uncertainty0 pass. No runtime change was needed. Window1/admission retained, repair_cycles_for_current_gate:1. Focused qualification and repaired-tree independent readback required.
