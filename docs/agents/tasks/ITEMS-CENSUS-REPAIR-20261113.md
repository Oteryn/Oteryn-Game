# ITEMS-CENSUS-REPAIR-20261113

```yaml
task_id: ITEMS-CENSUS-REPAIR-20261113
title: Repair structural Item census and retain bounded Basic110 evidence
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/items-census-repair-cloud-20261113
pr: null
base_sha: 83be72ad403ebc221dadba4b161c85fa1d47219a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/evidence_comparison
created_at: 2026-11-13T00:00:00Z
updated_at: 2026-11-13T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/items-15-33/**
  - docs/agents/tasks/ITEMS-CENSUS-REPAIR-20261113.md
  - docs/agents/tasks/archive/ITEMS-CENSUS-REPAIR-20261113.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Retain the bounded Basic110 evidence from frozen PR3148 and repair its two
structural census findings in one independent candidate. The frozen branch is
unchanged. This draft does not claim complete client inventory or semantic parity.

## Architecture and source of truth

PROVEN: source pin 83be72ad403ebc221dadba4b161c85fa1d47219a;
frozen baseline 99738f5d262251241ac5c7803f12c4422adf547e.
DERIVED: structural counts reproduced from immutable receipt-verified blobs.
UNKNOWN: client numeric IDs, complete inventory and every-value currentness.
Existing qualified source conflicts and Basic110 scope remain unchanged.
Authority: owner-directed Cloud handoff through the active coordinator.

## High-risk authority/recovery qualification

NOT_APPLICABLE: evidence and read-only analysis only; no runtime mutation.

## Acceptance criteria

- [x] Domain UNKNOWN does not hide sibling facts; wrapper metadata retained.
- [x] Immediate and nested collection parents participate in array ancestry.
- [x] CSV matches a reproducible traversal of verified source blobs.
- [x] UNKNOWN/N/A/absence/empty/zero/null remain distinct.
- [x] Frozen client observations and private proof custody are preserved.

## Reference parity

NOT_APPLICABLE: no mechanic implementation. Structural source coverage is not
client semantic, graphical, animation, Look, instance or gameplay parity.

## Excluded scope

No content, pins, Rust, runtime, protocol, workflows, GUI actions, NAS, merge,
deploy or paid review trigger. No screenshots or proprietary assets in Git.

## Implementation / findings

Both P2 findings fixed together. The old 1770-path CSV fails exact reproduction;
the corrected output contains1878 paths (+106 held facts,+2 taxonomy metadata).
186 rows change, including108 new paths. Collection counts are unchanged.
The validator now compares every CSV cell to fresh traversal rather than trusting
a replacement hard-coded grand total. Four regressions cover domain state,
wrapper metadata, immediate/nested arrays and record-level state counting.
The nested-array regression was observed failing before the family fix.

## Validation

- python -m unittest discover -s docs/agents/evidence/items-15-33 -p test_census.py: pass;4 tests.
- python docs/agents/evidence/items-15-33/validate.py --source-blobs: pass;168 blobs,1878 paths,110 name-signature groups,87 Basic records.
- python tools/agents/validate_governance.py: pass.
- python -m unittest discover -s tools/agents/tests: pass;129 tests.
- git diff --check: pass.

Owner proof bytes were not rechecked in Cloud: private PNGs remain on Ubuntu;
868 inherited receipts remain unchanged. Historical custody validation is not
represented as a fresh Cloud proof-byte check. CI is pending, not claimed green.

## Self-review

Read the complete new reproducer, regressions, validator and generated delta.
Source SHA256/size/Git OID are checked before traversal; all output CSV cells are
reproduced. Remaining existing evidence fields retain exact baseline bytes.

## Independent review

Not triggered by worker. Review packet returned to the coordinator; no paid review.

## PR and closeout

Draft PR pending. No merge authorization; the coordinator owns freeze/readback
and subsequent independent review decisions. This is an evidence-authoring slice.
