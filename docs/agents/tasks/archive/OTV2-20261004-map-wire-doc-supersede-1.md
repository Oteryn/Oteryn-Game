# OTV2-20261004-map-wire-doc-supersede-1

```yaml
task_id: OTV2-20261004-map-wire-doc-supersede-1
title: Mark 2026-09-30 MAP-WIRE-1 candidate as SUPERSEDED
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-wire-doc-supersede-1-20261004
issue: 1793
pr: pending
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-haiku-4-5-20251001
created_at: 2026-10-04T00:00:00Z
updated_at: 2026-10-04T00:00:00Z
execution_policy: deferred_codex
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_MAP_WIRE1_MAP_STATE_WIRE_CONTRACT_CANDIDATE_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261004-map-wire-doc-supersede-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Documentation-only task: the 2026-09-30 MAP-WIRE-1 candidate document is updated to mark its status as SUPERSEDED and link to the accepted candidate and architectural packet review. The document is retained as evidence of the original design exploration.

## Architecture and source of truth

- Allocation: Codex P2 deferred from #1793 (owner-approved MAP-WIRE-1 acceptance).
- Bound META policy: 3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- Accepted contract: `docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md`.
- Architectural decisions: `docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md`.
- Key changes in accepted candidate: viewport bounds (18x14, visible floors rule), ground/corpse carriage model, house tile handling, targeting mechanics, and handle table budgets.

## Changes made

Updated the header of `OTERYN_GAME_MAP_WIRE1_MAP_STATE_WIRE_CONTRACT_CANDIDATE_2026-09-30.md`:
- Changed status from CONTRACT CANDIDATE to SUPERSEDED
- Added links to the accepted candidate and architectural review
- Added a Supersession Summary section listing which document sections are replaced vs. retained as evidence

No other content was modified. The document serves as reference evidence for the original design exploration.

## Pre-freeze verification

- No documents currently cite the old file as current contract authority (greps returned only archived task records).
- No changes needed to other files; the new candidate and architecture review are already in place.
- Validation: `python3 tools/agents/validate_governance.py` and `git diff --check` run before push.
