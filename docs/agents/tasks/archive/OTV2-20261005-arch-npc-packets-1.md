# OTV2-20261005-arch-npc-packets-1

```yaml
task_id: OTV2-20261005-arch-npc-packets-1
title: "ARCH-NPC-PACKETS-1: NPC-CONTENT-1 and NPC-WIRE-1 packets (ARCH-NPC-PACKETS-V1)"
mode: ARCHITECTURE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-npc-packets-20261005
issue: 162
pr: 1846
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_NPC_PACKETS_2026-10-05.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-npc-packets-1.md
depends_on: []
blocks: [OTV2-20261005-npc-content-1, OTV2-20261005-npc-wire-1]
```

## Outcome

- Answers the control plane request (#162, 2026-10-05) to packet the NPC track's first children
  that can start now. Executes NPC-0 with the NPC-BEHAVIOUR-0, QUEST-GATE-0 and TIMED-ITEM-0
  amendments on `main`. No contract changes.
- NPC-CONTENT-1 (impl worker): a pure runtime content model over the merged
  `NpcDataCatalogue`. It covers offer and route rules, item resolution through a narrow
  item-facts trait, and the generated minimal replies. No wire, runtime state or persistence.
- NPC-WIRE-1 (impl worker, protocol review): the NPC-0 §4 registry and proto rows, fail-closed
  codecs, server capability rows and limit rows `NPC0-RL-01..07`, with no send path. It runs
  after NPC-CONTENT-1 and #1824.
- NPC-PLACE-1, NPC-TALK-1 and the later children are not packeted.
- Codex round on b20b5371 is fixed:
  - NPC-WIRE-1 acceptance requires the FND-02 §22 independent wire evidence: golden bytes, a
    raw-byte verifier that does not use the production codec, a malformed corpus, table round
    trips and additive evolution. It names the inapplicable items (#1846 review 4186908067);
  - §4 answers the five ARCHITECTURE_DECISION_DISCIPLINE questions (#1846 review 4186908078).
- Owner questions: none.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
