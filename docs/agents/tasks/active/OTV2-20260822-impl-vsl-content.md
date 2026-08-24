# OTV2-20260822-impl-vsl-content

```yaml
task_id: OTV2-20260822-impl-vsl-content
title: Implement minimal native VSL content compiler loader seam
mode: BLOCKED_OWNER_DECISION
status: evidence_delivery_merged_production_acceptance_blocked
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
updated_at: 2026-08-24T13:45:00+02:00
owned_paths: []
shared_lease: released
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
- genuinely independent exact-head review: `PASS`, terminal material findings `P0=0 / P1=0 / P2=0` after explicit finding adjudication;
- Ready-state Architecture semantic audit: `SUCCESS`;
- source branch: absent after merge.

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
- [x] staging separate from all-or-nothing activation;
- [x] exact-head focused/component/workspace validation and whole-diff review;
- [x] genuinely independent exact-head review for parser/item/loot/value semantics;
- [x] evidence-only composition through the production game-server crate;
- [ ] production acceptance/activation — blocked by missing accepted DUR-04/VSL hard maxima and production authority.

## Context checkpoint

```yaml
last_progress: bounded CONTENT evidence seam merged through PR #58 as 8f99f25d0b1b3472d40504cd54b463cf752ebe7a after exact-head game-gate, self-review and genuinely independent exact-head review; delivery branch deleted and shared lease released.
status: evidence_delivery_merged_production_acceptance_blocked
branch: null
head_sha: ab0b4241c107bfb2c6052e58aec241da130774c7
pr: 58
blocker: accepted DUR-04/VSL production hard maxima and production activation authority are absent
owner_action_required: accept production VSL limits/authority through architecture and registry process if production CONTENT is to continue
next_action: no implementation work until a fresh coordinator allocation follows that owner decision
```
