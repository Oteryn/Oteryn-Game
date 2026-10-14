# OTV2-20261014-accept-spell-npc-map-0-flip

```yaml
task_id: OTV2-20261014-accept-spell-npc-map-0-flip
title: "ACCEPT-SPELL-NPC-MAP-0: flip the mapping decision to ACCEPTED (status line and M3 references)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/magic-cast-1-spell-npc-map0-flip-20261014
pr: 2098
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ACCEPT_SPELL_NPC_MAP0_ACCEPTANCE_DECISION_2026-10-13.md
  - docs/architecture/reviews/OTERYN_GAME_SPELL_NPC_MAP0_SPELL_AND_NPC_HANDOFF_MAPPING_DECISION_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261013-accept-spell-npc-map-0-flip.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- The mapping decision status line reads ACCEPTED (conditional on this PR passing review, CI and protected integration), the two M3 references are corrected, and the packet status is conditional. No other rule text, code, wire or content change.

## Architecture and source of truth

- PROVEN: the decision is CANDIDATE on main until this PR is integrated; the packet (merged in #2084) lists what acceptance lifts.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation only. No mutation, no code, nothing enabled.

## Acceptance criteria

- [x] Only the status line, the two M3 references and the packet status change.
- [x] The packet status is conditional ACCEPTED and the decision points at it.

## Excluded scope

- No code, content, migration or protocol change. SPELL-PRESENT-0 has its own flip PR.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only.

### Exact-head CI

- final head: the PR head
- result: the PR checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: task writer, followed the ACCEPT-CHAR-POSITION-0 pattern.
- material findings: none open
- verdict: ready for review

## Independent review

- required: spell and content review per the decision's own gate; the control plane owns the call.
- verdict: pending.

## PR and closeout

- changed-file review: documentation paths only
- unresolved review threads: none
- related/superseded PRs: none
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: status flip written
status: completed
branch: claude/magic-cast-1-spell-npc-map0-flip-20261014
pr: 2098
blocker: none
next_action: control plane sends the PR to review
```
