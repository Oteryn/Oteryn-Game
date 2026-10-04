# OTV2-20261004-attack-parity-1-packet

```yaml
task_id: OTV2-20261004-attack-parity-1-packet
title: "ATTACK-PARITY-1-PACKET-1: packet auto-attack parity scope"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/attack-parity-1-packet-20261004
issue: 162
pr: 1768
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ATTACK_PARITY1_AUTO_ATTACK_PARITY_SCOPE_PACKET_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-attack-parity-1-packet.md
public_contracts: []
depends_on: [ATTACK-0, ATTACK-1a]
blocks: [ATTACK-PARITY-1a, ATTACK-PARITY-1b]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- The TibiaPal calculator is a page over the TibiaTools JSON API. The API checks only the player
  attack range: it has no fight-mode, defence, armor, block or interval output. Those values stay
  `PARITY_PENDING` (packet §1.1).
- Every assertion is bound to the Oteryn Offensive mode (`attack_factor` 1.0). The Balanced and
  Defensive factors stay pending. Monk is excluded, because the calculator hardcodes VoH (§1.1;
  #1768 P1 4177982266 and 4177982263).
- ATTACK-PARITY-1a captures the ATTACK-0 §6 grid into a checked-in fixture, now and in parallel
  with ATTACK-1b. ATTACK-PARITY-1b asserts it and corrects the formulas after ATTACK-1b
  (§1.2, §2).
- Five probes show a nonzero TibiaPal minimum and a low-skill maximum gap (§1.3). Fixtures that
  disagree with the pinned source go to the control plane as a QUESTION (§1.4).
- No code, contract or wire change.
- #1768 P1 4178014589: the gate asserts the mean as well as min and max. A uniform draw cannot
  match TibiaPal's average, so those rows stay `PARITY_PENDING` with a checked-in residual list,
  and the distribution goes to ATTACK-DIST-0.
- #1768 P1 4178032396: each weapon row's TibiaTools attack must equal the Oteryn KNOWN attack
  before any check. Mismatches go to a checked-in list.
- #1768 P1 4178079948: the TibiaTools source expression is pinned at commit `a1d36890`, including
  the `floor(6 * attack / 5)` step. It reproduces all five probes exactly. 1b implements it instead
  of fitting, and checks it offline over every attack value. The grid covers each attack residue
  mod 5, and min and max must match exactly.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
