# OTV2-20261113-charms-client-evidence

```yaml
task_id: OTV2-20261113-charms-client-evidence
title: Retain static charm descriptions and bounded reset/restore evidence
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charms-client-evidence-20261113
pr: https://github.com/Oteryn/Oteryn-Game/pull/3172
base_sha: 3e168ce6afe374485da20ee0ac13d3749558e2f8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-charms-evidence-publication
created_at: 2026-11-13T10:44:41Z
updated_at: 2026-11-13T11:13:17Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/charms-client-evidence-20261113/
  - docs/agents/tasks/OTV2-20261113-charms-client-evidence.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Preserve the first25-card static pass and the bounded active reset/restore
batch with source comparisons and private proof receipts. This is not all-stage
or combat qualification.
The lead alone operates the test client; the publication worker only analyzes
saved files. The authorized active pass adds one reset,18 level purchases and
nine restored assignments, without changing runtime code.

## Architecture and source of truth

- PROVEN: 25 catalogue records, 14 Major and 11 Minor, at the pinned base SHA.
- PROVEN: visible values/costs match their source stages; exact source/image
  bytes are verified using hashes. Stage numbers are derived candidates.
- PROVEN: four simplified description projections omit UI details already
  retained in fuller source mechanics. No runtime defect is inferred.
- DERIVED: key-derived Void's Call name loses the apostrophe in Card.name.
- UNKNOWN: other stages, combat/procs, exact area shape, exact leech/Gut
  arithmetic, first-free-reset/discount paths, clear fees and icon/animation parity.
- PROVEN: one paid reset5,495,250 and restoration of original6 Major/4 Minor
  levels and9 assignments; final balances114/6914,50/900.
- DERIVED: official reset-fee examples support the excess-over100 base rule;
  current full-level Game calculation is825,000 higher at757 with25% reduction.
  First-free-reset and actual entitlement identity remain untested.

## High-risk authority/recovery qualification

NOT_APPLICABLE: documentation and a standalone evidence validator only. No
runtime mutation, production data, identity, protocol or deployment change.

## Acceptance criteria

- [x] Retain 25 descriptions and observed values/costs with source pointers.
- [x] Distinguish source/projection findings from runtime claims.
- [x] Preserve 39 private image hashes; commit no original assets.
- [x] Prepare a separate draft candidate; no Bestiary or Items branch writes.
- [ ] Exact-head CI and coordinator review after publication.

## Reference parity

NOT_APPLICABLE as mechanic implementation. The evidence README contains the
comparison table; no mechanic ledger row moves to DONE.

## Excluded scope

Game input or further active test mutations/assignments/purchases by the worker,
anti-logout, runtime or UI fixes, Bestiary PR #3171, Items/Cloud branches,
original image/asset redistribution, deployment and merge.

## Implementation / findings

The publication branch starts at fresh main and has one writer. The prior
Bestiary head remains unchanged. The final immutable static reader checkpoint
was consumed before first publication. The active successor consumes a separate
qualified batch and retains the initial head as its predecessor.

AUTHORING resumed by the lead's explicit allocation after fresh live PR/head
readback at `2f91f9c906b9f87c8f1f1fa487bfafe03ff9f308`. This successor adds the
bounded active-test/restore evidence and official fee-source comparison in one
forward update. No runtime fix or independent review trigger is authorized
for this worker.

## Validation

- Focused validator with private-image and source-blob verification: PASS.
- Sixteen evidence control/negative tests: PASS on successor authoring tree.
- `python tools/agents/validate_governance.py`: PASS.
- `python -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `git diff --check`: PASS.
- Component/integration/E2E: NOT_APPLICABLE, no game/runtime/content input change.
- Exact-head CI: predecessor2f91f9c9 passed; successor checks pending. Independent
  review was not completed because the dispatched reviewer could not execute
  terminal reads (zero files reviewed). No review/mergeability claim.
