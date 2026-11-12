# ITEMS-1533-EVIDENCE-20261112

~~~yaml
task_id: ITEMS-1533-EVIDENCE-20261112
title: Publish bounded 15.33 Item evidence and source census
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/items-1533-evidence-20261112
pr: 3148
base_sha: 83be72ad403ebc221dadba4b161c85fa1d47219a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/oteryn_spell_comparison
created_at: 2026-11-12T23:02:14.113659+00:00
updated_at: 2026-11-12T23:02:45.521987+00:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/items-15-33/**
  - docs/agents/tasks/archive/ITEMS-1533-EVIDENCE-20261112.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
~~~

## Outcome

Publish one draft evidence PR with the full structural census and frozen
Basic110 card observations. This task completes the evidence-authoring slice;
it does not claim merge, implementation, or production readiness.

## Architecture and source of truth

- PROVEN: source commit 83be72ad403ebc221dadba4b161c85fa1d47219a;
  69 canonical shards and every selected profile input traversed.
- PROVEN: Test Server Client 15.33.dd0955 retained lossless card evidence.
- DERIVED: candidate repository keys matched to observed full names.
- UNKNOWN: actual client numeric IDs and unique Item types/instances.
  A collector name signature can collapse separate same-name UI rows.
- CONFLICT: recorded goo shell weight 58.00 oz versus canonical 48.00 oz;
  merged agrees at 58.00 oz. Death oyoroi Mantra 16 versus canonical 15.
- Authority: owner-directed #1622 allocation comment 6837923337.
  No authority to mutate content, runtime, workflow, merge or deployment.

## High-risk authority/recovery qualification

NOT_APPLICABLE: documentation, factual reference data and a read-only evidence
validator. No production mutation, credential change, protocol or durable-value
implementation is performed.

## Acceptance criteria

- [x] Compact source census, field definitions and source/consumer boundaries.
- [x] Frozen 33 full-pass cards plus Basic24–110:87 Basic records,
  77 new manual reads and 10 exact-name-signature reuses.
- [x] 110 distinct name-signature observation groups, without unique-Item or
  complete-inventory claims. Basic111+ excluded.
- [x] 868 owner-retained file receipts;168 immutable source blob receipts.
- [x] Original assets, narrative text and large intermediate data stay local.
- [x] Draft PR 3148 published, task record included before final freeze.

## Reference parity

NOT_APPLICABLE as a mechanic implementation gate: this task retains evidence.
Its qualified source conflicts and unknowns are in
[qualified-differences.json](../../evidence/items-15-33/qualified-differences.json).
No claim of complete retail, graphical, animation, Look or gameplay parity.

## Excluded scope

No content definitions, pins, Rust, Cargo, protocol, governance, workflows or
runtime behaviour changed. No game/UI action, NAS/production access, merge,
deployment, external-review trigger or implementation-worktree mutation.

## Implementation / findings

The corpus covers 20 enumerated collections and 1770 normalized field paths.
The merged source membership is 6090 after 73 replacements, 3 additions and 10
starters, rather than the naive sum 6077+76+10. Materializable flags do not prove
admission. Static description, generated Look, type inspection, dynamic instance
fields, trade, identity and appearance are separate views.

Name-signature deduplication is not an Item identity guarantee. A later Amulets
search showed two same-name wolf tooth chain rows. Frozen values remain evidence
of the observed cards; source keys are candidate correspondences, not client IDs.
The Fireheart cuirass case variation has a unique casefold source match;
the earlier case-sensitive unmatched candidate is withdrawn.

## Validation

- python docs/agents/evidence/items-15-33/validate.py --source-blobs: pass; 168 source blobs, 110 name-signature groups, 1770 field paths.
- python docs/agents/evidence/items-15-33/validate.py --evidence-root OWNER_WORKSPACE: pass; 868 proof bytes and frozen reader subset hash.
- python tools/agents/validate_governance.py: pass.
- python -m unittest discover -s tools/agents/tests: pass; 129 tests.
- git diff --check: pass.

Component/integration and game E2E: NOT_APPLICABLE to this evidence-only delta;
not run. Exact-head CI is pending on the final head; local receipts are not CI.

## Self-review

The sole writer reviewed all changed paths and read-only validator logic,
validated every structured record, confirmed the allocation boundary and checked
all source/proof receipts. No material finding remains in this evidence slice.
No runtime correctness or mergeability verdict is made.

## Independent review

Required by repository policy before merge. Not triggered by this worker:
the owner allocation forbids an external review trigger. The control plane
owns the independent review and exact-head CI decision.

## PR and closeout

- PR: https://github.com/Oteryn/Oteryn-Game/pull/3148 (draft).
- Changed paths: only the allocated evidence prefix and this task record.
- Review/CI: pending; no green or mergeable claim.
- Merge commit/result: squash merge of #3148 if authorized and accepted later.
- Ownership: released to the control plane after final FREEZE_SHA readback.
- The final SHA cannot be embedded in its own commit; immutable PR/head readback
  records the final candidate after the last authoring write.

## Context checkpoint

~~~yaml
last_progress: evidence slice published as draft PR 3148; final task record prepared
status: completed
branch: codex/items-1533-evidence-20261112
pr: 3148
final_head_sha: null
final_head_frozen_at: null
owner_action_required: null
blocker: null
next_action: control plane qualifies the frozen final PR head; no worker writes after freeze
~~~
