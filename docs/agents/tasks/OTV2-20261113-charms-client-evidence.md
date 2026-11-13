# OTV2-20261113-charms-client-evidence

```yaml
task_id: OTV2-20261113-charms-client-evidence
title: Retain 25 static charm descriptions and qualified source comparisons
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charms-client-evidence-20261113
pr: null
base_sha: 3e168ce6afe374485da20ee0ac13d3749558e2f8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-charms-evidence-publication
created_at: 2026-11-13T10:44:41Z
updated_at: 2026-11-13T10:44:41Z
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

Preserve the completed first static pass, source comparisons and private proof
receipts. All 25 names have a visible current/locked-preview description and
one unlock/next-upgrade cost. This is not all-stage or combat qualification.
The lead alone operates the test client; the publication worker only analyzes
saved files. Later active tests have separate evidence and are outside this PR.

## Architecture and source of truth

- PROVEN: 25 catalogue records, 14 Major and 11 Minor, at the pinned base SHA.
- PROVEN: visible values/costs match their source stages; exact source/image
  bytes are verified using hashes. Stage numbers are derived candidates.
- PROVEN: four simplified description projections omit UI details already
  retained in fuller source mechanics. No runtime defect is inferred.
- DERIVED: key-derived Void's Call name loses the apostrophe in Card.name.
- UNKNOWN: other stages, combat/procs, area geometry, exact leech/Gut
  arithmetic, reset/clear formulas and icon/animation parity.

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

Game input by the worker, active test mutations, assignments/purchases,
anti-logout, runtime or UI fixes, Bestiary PR #3171, Items/Cloud branches,
original image/asset redistribution, deployment and merge.

## Implementation / findings

The publication branch starts at fresh main and has one writer. The prior
Bestiary head remains unchanged. The final immutable static reader checkpoint
was consumed before publication; later active-test results are excluded.

## Validation

- Focused validator with private-image and source-blob verification: PASS.
- Nine evidence control/negative tests: PASS.
- `python tools/agents/validate_governance.py`: PASS.
- `python -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `git diff --check`: PASS.
- Component/integration/E2E: NOT_APPLICABLE, no game/runtime/content input change.
- Exact-head CI: pending first publication; no green/mergeability claim.
