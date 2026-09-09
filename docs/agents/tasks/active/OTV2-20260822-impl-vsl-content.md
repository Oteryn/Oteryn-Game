# OTV2-20260822-impl-vsl-content

```yaml
task_id: OTV2-20260822-impl-vsl-content
title: Implement minimal native VSL content compiler loader seam
mode: IMPLEMENT
status: first_production_post_merge_p1_repair_local_qualified
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-first-production-54-20260909
issue: 54
issue_state: open_active
pr: 481
allocation_id: CONTENT54-FIRST-PRODUCTION-v1-20260909
allocation_comment: 5601519485
allocation_admission_sha: 9afb7cbb538674408bc7d2eaaaaa1e8917b04640
current_reconciled_base_sha: ece300c384aa1e53f208975f25e94635c2fcc7ce
registry_pr: 472
registry_merge_sha: 9afb7cbb538674408bc7d2eaaaaa1e8917b04640
evidence_delivery_pr: 58
evidence_delivery_merge_sha: 8f99f25d0b1b3472d40504cd54b463cf752ebe7a
repair_issue: 85
repair_pr: 87
repair_merge_sha: db95bc720529b643531c79f708086f69dd612d22
owner: content-first-production-coordinator
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-09-09T20:35:56+02:00
owned_paths:
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/model.rs
  - apps/game-server/src/content/compiler.rs
  - apps/game-server/src/content/artifact.rs
  - apps/game-server/src/content/production.rs
  - apps/game-server/src/content/activation.rs
  - apps/game-server/src/content/tests.rs
  - apps/game-server/tests/content_first_production.rs
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
shared_lease: not_required_for_allocated_paths
independent_review_required: true
live_deployment_authority: NONE
future_write_authority: bounded_first_production_content_profile_v1
```

## Delivered evidence seam

- typed stable content/package/world identities and exact revision/provenance binding;
- deterministic canonical VSL graph for cells/collision/relocation, creature/spawn, ability/effect, loot/XP/item and synthetic presentation;
- explicit GAME-CHANNEL multiplicity/eligibility for value-producing spawn evidence;
- deterministic server-authoritative and allowlisted client-safe projections;
- bounded `VSL_BUNDLE_EVIDENCE_PROFILE/v1/non-production` artifact with SHA-256 integrity;
- checked parser arithmetic and corruption/truncation/oversize/unknown-critical/incompatible rejection;
- staged all-or-nothing server/client activation preserving the prior active revision on failure;
- game-server composition through `pub mod content` while ordinary release and gameplay remain fail-closed.

## Delivery evidence

- PR #58 exact reviewed head: `ab0b4241c107bfb2c6052e58aec241da130774c7`;
- squash merge: `8f99f25d0b1b3472d40504cd54b463cf752ebe7a`;
- exact-head Merge Gate / `game-gate`: `SUCCESS`;
- whole-diff self-review: `PASS`;
- pre-merge genuinely independent exact-head review: historical PASS; later post-merge review found one P0 which is now repaired and separately evidenced below;
- Ready-state Architecture semantic audit: `SUCCESS`;
- source branch: absent after merge.

## Resolved post-merge P0 - evidence activation boundary

A later independent exact-tree review of merged PR #58 found that `content::ActivationSlot::stage_and_activate` was exported by the production public module even though its artifacts are explicitly non-production and DUR-04 production activation authority is `NONE`.

Issue #85 reproduced that defect with a compile-fail regression, allocation PR #86 granted the bounded repair, and PR #87 fixed the boundary by keeping `ActiveContent` / `ActivationSlot` and their impls under `#[cfg(test)]` only. Final repair head `c9d3570f528acc8e22e3055e4f8de712e9057abd` passed fresh independent review with P0=0/P1=0/P2=0 and exact-head `game-gate`, then squash-merged as `db95bc720529b643531c79f708086f69dd612d22`. Issue #85 is closed completed and its source branch is absent.

The repair changes only the public activation fence; it does not grant production VSL limits, permanent-format authority or production activation.

## Production blocker resolution and fresh allocation

The historical evidence-era blocker is resolved for repository implementation only. PR #462 protected `FIRST_PRODUCTION_CONTENT_PROFILE/v1` plus Amendments 01/02/03, and registry PR #472 serialized the final DUR-04 hard maxima through FULL Merge Queue and protected-main readback at `9afb7cbb538674408bc7d2eaaaaa1e8917b04640`.

Issue #54 comment `5601519485` is the fresh bounded coordinator allocation. The active implementation branch is `agent/content-first-production-54-20260909`; after path-disjoint protected-main movement it was reconciled without conflict to `main@0e8a358f134693871d5140309cf778a28da97277`.

This allocation grants bounded architecture implementation authority only. It grants **no** live deployment, production environment, publisher, credential, network-fetch or automatic content-switch authority.

## First-production implementation candidate

The current candidate keeps the historical evidence seam fail-closed and adds a separate production-only boundary:

- `production.rs`: fixed registry-backed v1 limits, typed production source graph, package manifest/provenance/Content Lock validation, deterministic ordinary-release compiler, distinct temporary bootstrap carrier, server/client projections and fail-closed staging;
- `activation.rs`: sealed external authorization interface, admission guard held across final authority checks and the atomic publication point, exact expected-current identity + activation sequence, one staged candidate, immutable active runtime state, restart-not-ready, explicit LKG fallback and monotonic rollback-as-new-activation;
- `content_first_production.rs`: external/public API proof that ordinary release compiles and stages from typed production input while non-production target promotion remains rejected.

The candidate does not select a permanent World Project/Bundle format, compression, CDN/signing topology, source parser, scripting runtime or live activation source.

## Pre-freeze findings and dispositions

- **F1 ACCEPTED/FIXED:** the initial production draft inherited raw evidence-era `max_hp`, loot `weight` and XP `amount` fields. Those would have introduced unallocated production value semantics. They were removed before candidate freeze and replaced by bounded policy/product-release references only.
- **F2 ACCEPTED/FIXED:** a stronger local activation revision added `AdmissionGuard`, current activation-sequence fencing and immutable runtime state but temporarily dropped exact expected-generation matching from staging. Exact `FirstProductionExpectation` matching was restored for server and client before `staged`, while retaining the stronger guard and sequence protections.

## Independent review findings and dispositions

Codex review of PR #481 exact head `23eeac5add144bfe255b74a1bcaa409d1c5779d2` completed with `P0=0`, `P1=5`, `P2=1`. All six findings were accepted and fixed on the same canonical branch before requalification:

- **F3 P1 FIXED:** source compilation and artifact staging now validate every semantic reference against its required definition family, not merely global key membership.
- **F4 P1 FIXED:** staged server/client pairs now prove semantic equivalence for every client-safe presentation/creature/ability/item projection against the authoritative server records.
- **F5 P1 FIXED:** artifact sections must cover the payload contiguously from section-table end through `payload_end`; unclassified gap bytes fail closed.
- **F6 P1 FIXED:** staging revalidates behavior policy revisions with production-atom rules, independently rejecting evidence/fixture/synthetic markers in self-consistent artifacts.
- **F7 P1 FIXED:** activated runtime lookup indexes loot entries by the production entry key in field 0.
- **F8 P2 FIXED:** exact-cardinality values above their hard maximum return `LimitExceeded` / capacity classification; below-minimum cardinality remains a semantic `InvalidArtifact` failure.

Focused post-fix regression evidence is `37/37` CONTENT unit tests, `4/4` public integration tests and strict game-server Clippy PASS after reconciliation with protected `main@4d06bad1c0d21f2237df290865be55d8f7ed4f02`. Fresh exact-head independent re-review and canonical CI remain mandatory.

Fresh post-fix whole-diff self-review: **PASS** with P0=0, P1=0, P2=0 open after reproducing and closing F3-F8. This remains self-review and does not replace fresh independent exact-head re-review.

## Later review generations and post-merge P1 repair

Subsequent exact-head independent review generations before protected integration found and repaired additional bounded issues on the same canonical branch: spawn-population overflow classification; overlong SHA-256 capacity classification; token-aware rejection of explicit `test` namespaces; non-cloneable active runtime generation; exact artifact-digest binding in staging expectations; typed UUIDv7 `WorldId`; and one-to-one presentation closure. The final delivery source head was `df3c4cf0ad5bc3153e3d34f7dc78698eb49ca59e`.

PR #481 then passed canonical exact-head CI and FULL Merge Queue and was protected as `main@ece300c384aa1e53f208975f25e94635c2fcc7ce`. A post-merge independent whole-head review of unchanged `df3c4cf...` found one additional **P1**, accepted and reproduced against the protected implementation: `MultiplicityClass::ExplicitEventPolicyRequired` was accepted by both compiler and staging even though `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` requires exact event-owner simulation/eligibility policy before activation and `FIRST_PRODUCTION_CONTENT_PROFILE/v1` carries no such policy.

- **POST-MERGE P1 ACCEPTED/FIXED (local repair candidate):** first-production v1 now rejects `ExplicitEventPolicyRequired` at typed-source validation and independently rejects the serialized `EXPLICIT_EVENT_POLICY_REQUIRED` sentinel during staging. Supported resolved classes remain `CHANNEL_LOCAL_REPEATABLE`, `CHANNEL_LOCAL_SHARED_ELIGIBILITY`, and `WORLD_SCOPED_UNIQUE`.
- RED evidence: focused regression failed on protected behavior before the repair because `compile_first_production` accepted the sentinel.
- GREEN evidence: `explicit_event_policy_required_fails_closed_without_event_owner_policy` passes for both compiler and a self-consistent crafted artifact; full production suite is `21/21 PASS`.
- This repair does not add event policy, runtime event authority, permanent content format, network/deployment behavior, or live activation authority.

Fresh independent exact-head review, canonical CI, FULL Merge Queue and protected-main readback are mandatory for the repair generation before #54 closeout.

Post-repair adversarial whole-diff self-review: **PASS, P0=0 / P1=0 / P2=0 open**. Finding-family sweep covers both applicable consumer boundaries (typed source compilation and independent artifact staging); authority/recovery/session/persistence/retry/concurrency operators are NOT_APPLICABLE because this repair only rejects an unresolved authored multiplicity sentinel before staging/activation and introduces no mutation authority.
## Whole-diff self-review

The pre-Codex self-review after F1/F2 repair was `P0=0/P1=0/P2=0`, but independent review later found F3-F8 above. After accepting and fixing all six independent findings, the candidate requires a fresh whole-diff self-review plus fresh independent exact-head re-review before integration.

## Current local validation

- production + evidence CONTENT unit tests after Codex fixes: `37/37 PASS`;
- public first-production integration tests: `4/4 PASS`;
- strict `cargo +1.94.0 clippy -p oteryn-game-server --all-targets -- -D warnings`: `PASS`;
- `python tools/agents/validate_governance.py`: `PASS`;
- `cargo +1.94.0 run -q -p oteryn-architecture-check -- workspace .`: `PASS`;
- game-server doctests: `30/30 PASS`;
- full game-server local Windows run: `394 PASS / 3 source-scan failures`; the same three durability tests pass `3/3` on a detached, byte-clean protected `main@0e8a358f...` checkout with `core.autocrlf=false`, proving the failures are Windows checkout newline artifacts outside this task's diff rather than CONTENT regressions;
- local Windows repository-policy validator is non-qualifying because baseline `core.autocrlf=true` changes canonical LICENSE bytes and its temp-path JSON test receives Windows backslashes; canonical Linux exact-head CI remains required and is not bypassed.

## Acceptance state

- [x] deterministic canonicalization independent of source enumeration order;
- [x] stable namespaced identities and exact revision/provenance binding;
- [x] duplicate/missing-reference/source-classification rejection;
- [x] server-authoritative vs allowlisted client-safe projections with leakage-negative proof;
- [x] deterministic non-production evidence bytes plus integrity checks;
- [x] corrupt/truncated/oversized/unknown-critical/incompatible artifacts rejected before activation;
- [x] evidence staging and test-only all-or-nothing activation semantics exist;
- [x] production public API excludes non-production activation publication — repaired by Issue #85 / PR #87;
- [x] exact-head focused/component/workspace validation and whole-diff review;
- [x] genuinely independent exact-head review for parser/item/loot/value semantics;
- [x] evidence-only composition through the production game-server crate;
- [ ] first-production repository integration — pending fresh exact-head independent re-review, canonical CI, FULL Merge Queue and protected-main readback; live deployment authority remains NONE.

## Context checkpoint

```yaml
last_progress: post-merge independent review P1 reproduced RED and fixed locally on the same canonical #54 branch after reconciliation with protected main@ece300c
status: first_production_post_merge_p1_repair_local_qualified
branch: agent/content-first-production-54-20260909
head_sha: null
current_reconciled_base_sha: ece300c384aa1e53f208975f25e94635c2fcc7ce
pr: 481
allocation_id: CONTENT54-FIRST-PRODUCTION-v1-20260909
registry_merge_sha: 9afb7cbb538674408bc7d2eaaaaa1e8917b04640
focused_content_tests: 37/37 PASS
public_integration_tests: 4/4 PASS
strict_clippy: PASS
governance_validation: pending_fresh_exact_head
architecture_semantic: pending_fresh_exact_head
review_findings: historical findings fixed; post-merge ExplicitEventPolicyRequired P1 accepted and fixed locally
whole_diff_self_review: PASS_POST_FIX_P0_0_P1_0_P2_0
blocker: repair generation requires fresh exact-head independent review, canonical CI, FULL Merge Queue and protected-main readback
owner_action_required: null
next_action: qualify and freeze the post-merge P1 repair, push same canonical branch, open bounded repair PR, then complete fresh review/CI/FULL Merge Queue/readback
```
