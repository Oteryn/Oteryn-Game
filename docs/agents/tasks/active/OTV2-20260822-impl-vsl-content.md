# OTV2-20260822-impl-vsl-content

```yaml
task_id: OTV2-20260822-impl-vsl-content
title: Implement minimal native VSL content compiler loader seam
mode: BLOCKED_OWNER_DECISION
status: evidence_delivery_merged_repair_complete_production_blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 54
issue_state: open_blocked
pr: 58
final_head_sha: ab0b4241c107bfb2c6052e58aec241da130774c7
evidence_delivery_merge_sha: 8f99f25d0b1b3472d40504cd54b463cf752ebe7a
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-24T15:40:00+02:00
owned_paths: []
shared_lease: released
repair_issue: 85
repair_issue_state: completed
repair_pr: 87
repair_final_head_sha: c9d3570f528acc8e22e3055e4f8de712e9057abd
repair_merge_sha: db95bc720529b643531c79f708086f69dd612d22
future_write_authority: requires_new_coordinator_allocation
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

## Production acceptance blocker

Production CONTENT is intentionally **not accepted or activated** by the evidence delivery. `RESOURCE_LIMITS_REGISTRY.json` still has no accepted DUR-04/VSL production loader/compiler hard maxima, and DUR-04 production activation authority remains unavailable.

No numeric product limit, permanent `.omap`/`.owb`/World Project/Bundle representation, broad content import, Reference parity or production activation may be inferred from the evidence seam.

Future production work requires an owner-accepted architecture/registry decision plus a fresh coordinator write allocation. Until then this task is retained as a blocked lifecycle record with no active branch, owned paths or shared lease.

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
- [ ] production acceptance/activation — blocked by missing accepted DUR-04/VSL hard maxima and production authority.

## Context checkpoint

```yaml
last_progress: PR #58 evidence seam is merged, but a later independent exact-tree review found a reproducible P0 in the public activation boundary; Issue #85 created and fresh repair allocation is pending merge
status: evidence_delivery_merged_repair_complete_production_blocked
branch: null
head_sha: ab0b4241c107bfb2c6052e58aec241da130774c7
pr: 58
repair_issue: 85
blocker: repair allocation must merge before code mutation; production acceptance remains separately blocked by missing DUR-04/VSL maxima and production activation authority
owner_action_required: production CONTENT limits/authority only; no owner action is required for the bounded fail-closed repair
next_action: after repair allocation merges, implement #85 via TDD and exact-head independent review, then correct terminal evidence state
```
