# OTV2-20261005-arch-revoke-report-0

```yaml
task_id: OTV2-20261005-arch-revoke-report-0
title: "ARCH-REVOKE-REPORT-V1: scope revocation report and producer key separation, runtime-status amendment RS-A1 (D745)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-revoke-report-20261005
issue: 162
pr: 1832
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_REVOKE_REPORT_2026-10-05.md
  - docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-revoke-report-0.md
public_contracts:
  - docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md
depends_on: [OTV2-20261005-ops-assign-report-1]
blocks: [OTV2-20261005-ops-revoke-report-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Answers CP D745 and the two #1822 P1 findings at `1ce1363e`.
- Amendment RS-A1 (contract §16): the owner accepted it as written (option 1a) on 2026-10-05
  (CP D747). Platform accepted it on 2026-10-05 (CP D758, https://github.com/Oteryn/Oteryn-Platform/issues/1419#issuecomment-5995072150).
  The architect verified that record first-hand, and the owner confirmed both directly.
  - §16.1: `ReportScopeRevocationV1` on its own endpoint, with no node identity. Platform orders
    revocations and assignments in one per-scope sequence by `(epoch, generation)`. A latest
    revocation matches no node report. The body is derived from the durable row alone.
  - §16.2: Platform's identity registry is the complete per-purpose key check. `oteryn-game-ops`
    checks a closed set derived from `node_config_files`, which replaces
    `other_producer_certificate_files`.
  - §16.3: no shipped peer is affected, and the assignment wire is unchanged.
- Pointers in §3, §5, §12 and §13.
- Packet OPS-REVOKE-REPORT-1 (hard worker), after #1822 merges and after acceptance.
- Codex round on 5d09099d is fixed:
  - §16.1 defines the identical revocation replay as an idempotent `accepted` and corrects the
    response reference to §4;
  - §13 and the packet require negative fixtures for the revocation and its response decoder;
  - the decision test records "Must decide now? YES", the coupling and migration cost, the
    evidence that would justify superseding it, and what is not decided.
- Codex round on 3e800b9c is fixed:
  - listener keys join the fleet-wide check: Platform registers routed listener keys, which
    needs Platform's confirmation;
  - the Game check covers every node holding an assignment;
  - every malformed success-response shape has a test.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
