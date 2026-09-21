# CONTENT-QUEST-01 r7 repository retention

Status: PROPOSED_NONCANONICAL
Date: 2026-09-21
Tracking: #707 / PR #709
Base main: `e4da44a86d006df623b7b234d7b3a879516b86cd`

## Composition

This document advances the tracked r6 retention without rewriting it. Current CONTENT-QUEST-01 evidence is the composition of:

- `CONTENT-QUEST-01_R6_REPOSITORY_RETENTION_2026-09-21.md`;
- `CONTENT-QUEST-01_ANNIHILATOR_MAP_BINDING_WITNESS_2026-09-21.md`;
- Issue #707 retained r1-r6 contracts/evidence.

## r7 closure

Exact GitHub Actions run `35585892692`, job `106289013612`, on evidence head `0e3d83bce0ed5358b6ac9ad8f5b108105b740d66` checked out:

- `zimbadev/crystalserver@ac447fef0935e6df52dc6b6376ae4c1534ecd73f`;
- exact `world.otbm` blob `e95e8f7c7a95d1b634b49a5dea5a5dc76021406b`, SHA-256 `09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb`;
- existing parser `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.

The strict parse completed SUCCESS with `problems=[]` and physically confirmed the selected Annihilator lever, reward-access door, four reward chests, room exit and all six authored spawn coordinates. The inspected static OTBM records contain no stored action/unique IDs at those target tiles, confirming the source binding is an explicit map + startup-table + handler composition.

The six spawn tiles are physically heterogeneous: four are ground `410` / flags `0`; two are ground `10145` / flags `8`. Import must preserve that exact source projection and must not normalize them to a guessed shared arena tile.

## Validation horizon

Current required corpus: **75** cases. r7 adds T72-T75 for exact physical source binding, qualified map/startup/script composition, spawn-tile heterogeneity and same-map-blob/different-source-revision provenance.

Architecture area count remains 48/48 traced. Resource dimensions remain 26. Adversarial classes remain 14.

This source execution is evidence for the pinned Crystal physical map binding only. Runtime gameplay tests, Reference target parity and protected architecture acceptance remain separate.

## Current status

```text
architecture semantics: RESOLVABLE_SEMANTICS_CLOSED_IN_CANDIDATE
tracked retention: PRESENT_IN_PR_709
selected Crystal physical map binding: PROVEN_FOR_PINNED_SOURCE_BYTES
Reference target parity: NOT_QUALIFIED
runtime/DDL/production authority: NONE
protected architecture acceptance: PENDING
```

Next investigation should target remaining field-level Reference uncertainty or representative importer lowering; it must not reopen the resolved Quest/Encounter ownership model without new evidence.
