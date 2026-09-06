# OTV2-20260906-control-loss-reconnect-bridge-353

## Current owner stop checkpoint — 2026-09-06T17:40:25Z

The owner explicitly requested saving current work to the repository and stopping. All agents are stopped. This override supersedes older in-progress/qualification prose below. The partial repair is **UNVERIFIED WIP**, not a resolved P1 or permission to integrate. No further implementation, test execution, Merge Queue entry, consumer activation or lease release occurs in this run.

PR358 merged as protected `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3` at17:31:29 despite conversion to draft at17:28:25; that operation did not remove the existing queue entry. Its earlier clean review is superseded by the P1 recorded in353 comment5560950482. Early-terminal replacement must establish the successor RECONNECTABLE anchor and claims at PREPARE, retaining predecessor connection generation and no controller. COMMIT alone activates the successor generation/transport/proof/protection. The original `FreshAdmissionCommit` must remain immutable; a distinct complete anchor/current projection must carry replacement state. Exact predecessor PREPARED-attempt fencing must preserve slots and require independently sourced session/attempt/transport binding. The currently saved implementation/test edits are incomplete and must be reviewed and qualified before use.

Same canonical branch and admission are preserved. Local repair HEAD is native `4b0f55db36a3bfdb35cbe1af78f3ac1482b8bc8d` with normal merge parent `7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3`; baseline357 documentation changes are merge-up, not additional authored scope. Window1 closed with50m51s charged/9m09s discarded. Window2 was granted in353 comment5560958184, started17:36:22 and paused17:40:25: conservatively4m03s charged,55m57s remaining, completed windows1/repair cycles3/rotations0. No counter, history or admission reset. A future explicit resume must first read canonical refs, this draft checkpoint, current allocations and these preserved budgets.

B329 remains frozen at `834db1d7118d751e31287715d3eaac7780a0c7b9` (PR335, actual PostgreSQL17.6 366/0, full acceptance open). Driver351 remains frozen at `2aecb63f03e01c5e2c3eb8933dbb51d6f8b8c59c` (draft PR356, full checkpoint CI passed, full TLS/PG gate open). Its include-only test lease was activated by353-independent351 comment5560895137 without a new worker window. Child C319 still lacks separately authorized Platform native UUID/security/signing producers. Server Seam247 stays held at `9370b254c6ac4f6529e069c1968ae6bfa1e1750e`; no production readiness is claimed.

```yaml
task_id: OTV2-20260906-control-loss-reconnect-bridge-353
title: Bridge complete owning-loss continuity into reconnect
mode: IMPLEMENT
status: in_progress
admission_state: ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/control-loss-reconnect-bridge-353
issue: 353
pr: null
allocation_source_main_sha: 53c6bdf06a2282d893035a995c46052c88f935b4
admission_main_sha: b61f9d8cc1c0a7289ffdaf1bf4e42b851d2c0f9a
base_sha: b61f9d8cc1c0a7289ffdaf1bf4e42b851d2c0f9a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: foundation_audit sole Foundation bridge worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/foundation/admission_authority_publication.rs
  - apps/game-server/src/foundation/control_loss_reconnect_bridge_tests.rs
  - docs/agents/tasks/active/OTV2-20260906-control-loss-reconnect-bridge-353.md
  - docs/superpowers/plans/2026-09-06-control-loss-reconnect-bridge.md
public_contracts: [FND-04B, DUR-FRESH-RESOURCE-ENVELOPE-V1]
depends_on: [338, protected_353_allocation, released_338_ownership]
blocks: [B329_full_owning_loss_reconnect]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Implement the concrete caller bridge explicitly excluded from protected348 pending separate exact allocation. Foundation338 remains completed semantic delivery, with original admission/history retained; this child is not a restart or replacement338 worker. Accepted FND-04B, protected owning-loss addendum and existing complete recovery/protection contracts govern; no new architecture or numeric policy is selected. Canonical plan: `docs/superpowers/plans/2026-09-06-control-loss-reconnect-bridge.md`.

PROVEN: protected343 atedef416745f92b79371f98739272c840b0a9b357 provides sealed complete owning loss. `RecoveryProtectionContinuityV1` includes NotEntitled and independent entitlement/rearm provenance. Legacy `ReconnectContinuityV1` requires closed Unused/Fenced and derives activation fencing from candidate connection generation. Mapping either an absent entitlement or independent generation into that legacy field can invent protection/history. DERIVED: an additive versioned durability continuity/operation envelope is required to preserve complete semantics without breaking existing V1/V2 representations. Exact implementation shape must be derived from accepted contracts and complete consumer source, never from a convenient flag/downcast.

## Acceptance criteria

- [ ] Additive closed continuity/operation and split-phase consumer preserve complete original owning-loss operation, epoch/grace, protection/rearm, retained budget/attempts and owner-generation namespaces.
- [ ] Cover both same-session recovery and V2 early-terminal replacement within original grace, with complete continuity and exact current claim binding through additive publication predicates.
- [ ] Existing verified V1/V2 credential semantics, public signatures, exhaustive enums and wire protocol stay unchanged. Never manufacture a legacy record for an unrepresentable state.
- [ ] PREPARE and final predicates independently validate current owner, canonical reconnectable session, account/character/world/lease/runtime, claims, transport/generation and immutable timing. History/receipt alone cannot recreate live authorization.
- [ ] NotEntitled never gains protection; unused entitlement activates only by its accepted rule; existing activation/deadline/consumption/rearm history is neither reset nor relabeled. No invented fence or policy duration.
- [ ] Immutable exact retry, conflicting operation/time, final outcome, completion and reconciliation remain distinct; restart retains original identity and requires fresh current authority before any new effect/adoption.
- [ ] Meaningful independent positive controls and one-invariant negatives cover all protection forms, V1/V2 boundaries, lost response, stale completion/current owner/claims, eight-attempt history and queue/deadline changes. Existing regressions and anti-forgery compile-fail checks remain enforced.
- [ ] Full package/doctests, strict all-target Clippy/fmt, source-included actual B compile, governance and complete author review pass; genuinely independent final material review resolves P0/P1/P2.
- [ ] Exact-head canonical CI, normal protected Merge Queue/readback and Work archive/release before B activation. SQL/actual-source/E2E readiness remains separately required.

## Scope, parallelism and exclusions

Exactly five paths activate only after this package is protected, Work reads back its allocation and338 release, verifies overlaps and grants immutable admission. One new branch and one sole writable worktree; no second Foundation writer. Tests may be included from the already-owned inner module; no demonstrated facade/export need. The publication module is required for an additive owner-sealed V2 early-terminal replacement claim transition carrying complete continuity: existing replacement authorization/claim APIs require a legacy reconnect record and cannot faithfully represent all protection histories. Preserve those existing APIs and closed enums; do not manufacture a legacy record. Report any additional concrete file before mutation.

B owns its fourteen SQL/codec/harness paths, driver351 owns its protected vendor/Cargo lease, and this worker owns only the semantic bridge. Work162 comment5560287018 explicitly permits this bounded path-disjoint third lane because both missing prerequisites block complete B. No shared writable worktree/control plane or Cargo/lib/registry overlap follows. Same programme budget; task counters begin only on actual admission and persist across later windows.

No SQL/B, Cargo/lock, lib/facade/verifier, workflow/registry, old migration, actual producer/bootstrap/transport/Platform/other repository write. No new wire version or unrelated post-grace SQL implementation. Missing producer/runtime registration remains ChildC. B must separately consume the new complete envelope after protected delivery; partial unsafe legacy projection is not integration.

## Validation and review

TDD begins with the representability/authority failure: NotEntitled cannot be converted into unused protection or fabricated fenced history. Focused semantic tests use independently sealed fixtures, never a production registration bypass. Full current package/source-included B compile and exact-head CI qualify compatibility; unconfigured SQL is not PostgreSQL evidence. Author reviews all relevant existing consumer/source code and complete changed diff, then separate non-author reviews the final material candidate. No local-only completion credit, force/reset/replacement/no-op retrigger or test suppression.

## Context checkpoint

```yaml
last_progress: verified clean canonical cache bound at 2026-09-06T16:30:51Z under Work353 comment5560585146
status: in_progress
admission_state: ADMITTED
execution_window_number: 1
execution_windows_completed: 0
worker_rotations: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
owner_action_required: null
blocker: null
next_action: independently review exact repaired five-path candidate then publish through Work
```

## Window 1 material checkpoint

Immutable admission and canonical cache bind remain b61f9d8cc1c0a7289ffdaf1bf4e42b851d2c0f9a /2026-09-06T16:30:51Z under Work353 comment5560585146. Implementation covers complete Fast, recoveryV1/recoveryV2 same-session and recoveryV2 early-terminal replacement. Full original loss history/protection/rearm/budget/FND02 and separate loss/reconnect decision times are preserved. V1 captures only actual successful verifier key/floor lookups; V2 retains genuine source audit. Every proof class has owner-issued inactive successor metadata and requires independently current active proof before adoption. Actual source registration, secret mechanics, B codec/SQL and driver accounting remain separate acceptance.

Author and independent review findings repaired test-first: original loss commit substitution; restored V2 provenance rollback; signed-audit/deadline inconsistency; canonical eligibility/shared-floor and claim-history validation; impossible PREPARE-after-COMMIT history; unresolved other Prepared budget; missing Recovery proof rotation; incomplete Fast current security/proof freshness; equal-revision inactive-to-active proof substitution. Two material candidate repair cycles, completed windows0/rotations0, no retry reset. Full local package, Clippy, fmt, governance and source-included actualB6036 were green on the first candidate; all affected layers are rerun for the repaired final tree. Independent exact-material review and remote canonical CI/Merge Queue/readback remain required before completion.

### Final local qualification, 2026-09-06T17:13:42Z

Runtime material frozen in tree cf660986f176d47d793478ca0f7bd8f37ec6236a. Full `cargo test -p oteryn-game-server --offline` PASS:364 library tests (24 complete-bridge tests), all remaining package targets,29 doctests including new anti-forgery controls. Strict all-target Clippy, workspace fmt check, whitespace and governance PASS. Exact three owned runtime/test blobs composed with canonical B6036bee65dd798609dcfa63d7189d2ea12abfee3 in isolated qualification cache: `cargo test -p oteryn-game-server --test durability_postgres --no-run --offline` PASS. This is source compatibility compile, not configured PostgreSQL execution or complete B consumer activation.

Independent non-author allocation_review rebound the whole five-path cf660 material and reported no remaining P0/P1/P2 source findings; all accepted material repairs are resolved. This evidence-only checkpoint preserves the runtime freeze. Window1 remains rooted at16:30:51Z (42m51s elapsed,17m09s remaining at qualification); completed0/rotation0/repair2 unchanged pending Work publication/integration. Current local branch HEAD still immutable admission b61f9d8; Work must publish the exact reviewed tree, verify remote head, run canonical exact-head CI and normal Merge Queue/readback. No local-only completion or Server Seam release is claimed.
