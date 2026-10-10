# OTV2-20261010-house-1-contract

```yaml
task_id: OTV2-20261010-house-1-contract
title: "HOUSE-TENURE-0 house tenure contract candidate (HOUSE-1 step 1)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: house
base_branch: main
branch: claude/house-1-20261010
pr: 1956
base_sha: 348b2b76
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01C7Qrh8dkz2Gyy98AVraRGm (HOUSE-1 for control plane #1622)
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-10
updated_at: 2026-10-10
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_TENURE_CONTRACT_CANDIDATE_2026-10-10.md
  - docs/agents/tasks/archive/OTV2-20261010-house-1-contract.md
blocks:
  - OTV2-20261004-house-1a
high_risk_qualification: NOT_APPLICABLE (docs only)
```

## Outcome

Docs-only consolidation candidate. It binds EXP-HOUSES-01, HOUSE-OWN-0, HOUSE-CUSTODY-0, HOUSE-RUNTIME-0, SOCIAL-MAP-PACKETS-1 and ARCH-HOUSE-RT-INBOX-PACKETS-1 by reference. It re-decides one accepted rule: owner answer Q2a supersedes EXP-HOUSES-01 §17 and §25.21 (Aleta list edits). The candidate covers:

- the binding clause for each topic;
- what `main` has and lacks;
- the conflict between migration 0038 and the HOUSE-1a/HOUSE-ACL-1 plans;
- failure modes, conformance mapping and the slice plan;
- owner questions Q1-Q5 (§8).

Owner answers `1a 2a 3a 4a 5a` (D972) are recorded in §10. HOUSE-1a may be allocated once this PR merges and the ARCH-HOUSE-RT-INBOX-PACKETS-1 §1.5 exit gate holds (HOUSE-RUNTIME-1b and 1c accepted).

## Validation

- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
