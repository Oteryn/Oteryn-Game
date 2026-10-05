# OTV2-20261005-arch-spell-lock-2-0

```yaml
task_id: OTV2-20261005-arch-spell-lock-2-0
title: "ARCH-SPELL-LOCK-2-V1: native spell casts without Channel guards across durable I/O"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-spell-lock-2-20261005
issue: 162
pr: 1836
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_SPELL_LOCK_2_2026-10-05.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-spell-lock-2-0.md
public_contracts: []
depends_on: []
blocks: [OTV2-20261005-spell-lock-2a, OTV2-20261005-spell-lock-2b]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Answers Codex finding 4178405815 on #1534 (Channel guards held across durable combat I/O) and
  the SPELL-LOCK-1 BLOCKER, which moved the finding to SPELL-LOCK-2 as a design proposal.
- Rules that a native cast linearizes at one stage section S, which holds `runtime`,
  `spell_states` and `door` and contains no await.
- Adds a per-Channel spell lane that mirrors advisory key 33 and is taken by every key-33 writer,
  with the lock order lane, runtime, spell_states, door, attack.
- Enforces the lane at the shared boundary: the functions that take key 33 require a lane permit
  for the same Channel, so the compiler lists every caller (#1836 review 4183320735).
- Adds the mandatory decision test (#1836 review 4183320746).
- Makes the spell slot reservation complete: every mutator of a reserved slot either checks it or
  is shown unable to reach one. The caster stays visibly pending for the whole pass.
- Rejects a runtime revision counter, committing under the guards, narrowing the install fence,
  and a single packet.
- Packets SPELL-LOCK-2a (commit side, hard worker) and SPELL-LOCK-2b (read-side prefetch, hard
  worker, after 2a).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
