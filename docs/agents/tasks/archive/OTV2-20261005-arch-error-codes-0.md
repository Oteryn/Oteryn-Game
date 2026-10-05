# OTV2-20261005-arch-error-codes-0

```yaml
task_id: OTV2-20261005-arch-error-codes-0
title: "ARCH-ERROR-CODES-0: one error code space, one registry and one diagnostic line"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-error-codes-20261005
issue: 162
pr: 1840
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-error-codes-0.md
public_contracts: [docs/contracts/FOUNDATION_ERROR_VOCABULARY.md]
depends_on: []
blocks:
  - OTV2-20261005-err-registry-0
  - OTV2-20261005-err-node-1
  - OTV2-20261005-err-client-2
  - OTV2-20261005-err-tools-3
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Answers the owner's request for an error code system that lets the owner and the agents
  identify a problem quickly.
- Error codes:
  - One u32 code space, split into owner blocks. 1000–1999 are wire codes and stay in the
    protocol registry; the other blocks go in a new Game registry,
    `OTERYN_GAME_ERROR_CODE_REGISTRY.json`.
  - Each entry carries a category, a progression, an owner, a contract and a one-line hint.
    Numbers are append-only and never reused.
- Diagnosis and display:
  - An `explain.py` tool explains a code.
  - Each boundary error enum gets an exhaustive `code()`, with a test that checks it against the
    registry.
  - Every diagnostic line has the fixed key=value fields `code`, `name`, `cat` and a UUIDv7
    `trace`.
  - The client shows text keyed by code. Tools and CI print the code and a GitHub annotation.
- `FOUNDATION_ERROR_VOCABULARY.md` gains a "Code space" section.
- The decision defines four packets: ERR-REGISTRY-0, ERR-NODE-1, ERR-CLIENT-2 and ERR-TOOLS-3.
- §1.8 has four items that need owner acceptance: the display format, whether the player sees
  the code, Platform HTTP failure bodies, and priority.
- Codex round 1 is fixed:
  - only 1000–1999 codes go on the wire; a non-wire root code stays internal and is logged as
    `code` beside `wire`;
  - protocol codes are matched on the fields their registry defines, so 1001–1050 get no
    progression;
  - `detail` has an escaping and truncation rule.
- The owner ruled on §1.8 on 2026-10-05:
  - 1a: the `E1104` format;
  - 2a: the code is always shown to the player, and the release-build choice is deferred to a
    later security decision (§1.9);
  - 3a: the Platform body proposal goes after ERR-NODE-1 and is routed by the control plane;
  - 4b: all four packets are allocated now.
- Codex round 2 is fixed:
  - 1001–1050 derive their progression and retry requirement from `default_disposition` by a
    fixed table;
  - diagnostic lines carry no player-linked identifier;
  - the OS exit status keeps its coarse value;
  - one macro list is the single source of each boundary enum's code set.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
