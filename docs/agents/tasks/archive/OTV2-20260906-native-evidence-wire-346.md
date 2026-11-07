# OTV2-20260906-native-evidence-wire-346

```yaml
task_id: OTV2-20260906-native-evidence-wire-346
title: Qualify bounded native evidence wire codec
mode: IMPLEMENT
status: completed
admission_state: RELEASED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-evidence-wire-346
issue: 346
pr: 349
allocation_source_main_sha: 9ceeb231e2bb92c70eae83369c84f0f3fa6fccb2
admission_main_sha: ad7273e3e91a4e4254abb9aa2710c7e0c9754afe
base_sha: ad7273e3e91a4e4254abb9aa2710c7e0c9754afe
head_sha: 7afec2da8bef81bc7ad03f274432c96e61e2bd09
final_head_sha: 7afec2da8bef81bc7ad03f274432c96e61e2bd09
final_head_frozen_at: recorded_in_PR_349_comment_5559913540
owner: child_c_audit
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

- [x] Exact four request families, Account13/trust14-field observed responses, and exactly3-field failures; all required fields and no extras.
- [x] Closed bounded flat parsing rejects unknown, duplicate, nested, malformed, oversized and trailing input before peer-sized decoded allocation; no generic DOM-first path.
- [x] Exact operation/version/source/account/trust-scope binding; canonical UUID, decimal u64/i64, booleans, key-ID grammar and Ed25519 base64 bytes preserved.
- [x] Every applicable max/max+1/overflow/escape/encoding boundary and both fresh/recovery substitution directions have independent tests and positive controls.
- [x] Denial/untrust stays observation; failure carries no fabricated authority facts; unprivileged output cannot bypass Foundation seals.
- [x] Focused RED/GREEN, full affected validation and whole-diff self-review; independent exact-head parser/security review with no unresolved material findings.
- [x] Protected registry readback, exact-head canonical CI and normal Merge Queue; Work protected readback/archive/release.

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
last_progress: repair2 adds independently asserted UUID rejection and exact decoded payloads
status: completed
admission_state: RELEASED
execution_window_number: 1
execution_windows_completed: 0
worker_rotations: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
owner_action_required: null
blocker: none
next_action: none — terminal codec delivery; actual source readiness remains319
```

## Window1 validation evidence

Compiler RED: wire346-red.log unresolved admission_evidence module. Initial five integration cases GREEN; expanded matrix and strict all-target Clippy qualification recorded at handoff. Fixed inline representation intentionally retains bounded stack storage instead of boxing; local large-enum lint explanations preserve this allocation property. The sole UTF8 invariant assertion operates on private parser-validated storage. No unsafe code or global allocator hook is used. Decoder has no heap allocation path; request String is created only after complete1024-byte bounded encoding. This source-level allocation argument is not a measurement of TLS or whole-executor memory.

Final local candidate checks: library291/291 PASS (unconfigured legacy SQL returns are not DB proof), dedicated wire integration8/8 PASS, strict all-target Clippy PASS, fmt and whitespace PASS, governance26/9 PASS. Self-review inspected the full codec, fixed array growth, field masks, scalar/Unicode/number parsing, exact bindings and all eight independent corpus tests. No unresolved self-review finding; independent reviewer must qualify the exact staged tree.

## Independent numeric test repair — cycle1

Accepted P2: malformed source_revision tests retained decision_identity1, allowing secondary identity mismatch to mask numeric validation. Numeric negatives now carry the identical malformed revision/decision text; separate otherwise-valid decision mismatch stays covered. Both Account families test minimum generation0/max/max+1; all four families reject zero source revision while timestamp/uncertainty0 pass. No runtime change was needed. Window1/admission retained, repair_cycles_for_current_gate:1. Focused qualification and repaired-tree independent readback required.

## Independent qualification repair — cycle2

Work rebind5559866827 assigns child_c_audit the sole bounded test-repair custody. Preserve immutable admissionad7273e3e91a4e4254abb9aa2710c7e0c9754afe, window1 13:27–14:27UTC and completed numeric repair1; current repair cycle2. Current published baseline843792e5b94baff4f6db4036bd303ed4e2f49302/PR349 remains historical review provenance. Normal reconciliation with protected maind9d1b566acb57b537ff901d9765c32a95110c259 preserves the five owned blobs and adds only protected348 coordinator documents; Work owns GitHub publication.

Both accepted P2 findings are fixed in tests only. The two Account families now reject matching malformed expected/returned UUIDs (uppercase, version, variant, nil, short/long, separator, nonhex and empty), with valid controls; unrelated identity mismatch cannot mask validation. Exact decoded assertions cover every observation field, both allow/trust booleans, independently specified nonzero32-byte key bytes and all four failure variants for each wire family. These are inert wire facts, not authenticated authority.

Regression sensitivity uses a separate temporary minimal crate containing copied codec/tests: UUID validation removed, allowed inverted, trusted inverted and NotFound substituted each independently produce a failing targeted test; original source is restored afterward. The owning runtime file is unchanged. Full repaired integration corpus11/11 PASS on Rust1.94 locked/offline. Mutant RED results are qualification experiments, not defects in the published runtime. This author cannot independently certify repair2; fresh non-author review and exact-head CI are required. No source/TLS/PostgreSQL/E2E readiness is claimed.

Repair2 final validation: dedicated integration11/11 PASS; strict locked/offline all-target Clippy PASS after removing an explicit test panic flagged by the repository lint (the existing test-only unwrap convention is retained); fmt/whitespace PASS; governance26/9 PASS. Full three-path repair self-review confirms no runtime/Cargo/export changes, original admission and window preserved, and both P2 dispositions fixed. Regression logs are in the isolated scratch wire346-repair2-mutations harness; restored source matches the unchanged owning codec. Fresh non-author review remains the next gate.

## Protected terminal closeout — Work162

PR349 integrated through normal protected Merge Queue at9be69b4e0a06f3978d5c5c5603ca3e5670a9f18a on2026-09-06T14:48:04Z; GitHub main readback matches. Final PR head7afec2da8bef81bc7ad03f274432c96e61e2bd09/tree8678db8cf0ffbb85a4a63fb64699778c971b867c preserves original branch history and normal d9d1 main reconciliation. Root whole-five-path review plus independent non-author b_checkpoint_review P0/P1/P2=0 binds the repaired11-case corpus; PR comment5559913540 records final review. Exact-head game-ci34039525638/game-gate and Merge Queue game-ci34039815144/game-gate succeeded. Issue346 closed by protected integration. Prior pending statements above remain historical checkpoints, superseded by this terminal evidence.

Work releases all five346 owned paths and its single admission_evidence module-export lease in lib.rs. Remaining247 composition ownership stays held with preserved branch9370b254c6ac4f6529e069c1968ae6bfa1e1750e; no concurrent writer is admitted by release. Archive authority is Work162 comment5560037097. Preserve immutable admissionad7273e3e91a4e4254abb9aa2710c7e0c9754afe, window1, rotations0 and repair2; no reset. The merged branch retains a documented continuing provenance role until programme closeout. No TLS/SQL/actual producer or Server Seam readiness follows.
