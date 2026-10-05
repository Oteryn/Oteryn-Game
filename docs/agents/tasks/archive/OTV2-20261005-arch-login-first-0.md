# OTV2-20261005-arch-login-first-0

```yaml
task_id: OTV2-20261005-arch-login-first-0
title: "ARCH-LOGIN-FIRST-PACKETS-V1: login-first packets, N8 admission-refusal wire, N4-P acceptance record"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-login-first-20261005
issue: 162
pr: 1815
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_LOGIN_FIRST_PACKETS_2026-10-05.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/architecture/ADR-0020-native-client-gameplay-entry.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-login-first-0.md
public_contracts:
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
depends_on: []
blocks: [N8-1, OPS-ASSIGN-REPORT-1, N4-1, N5-1, N2N3-1, RUNBOOK-1]
cross_repository_coordination_id: OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN
external_repositories:
  - Oteryn/Oteryn-Platform
```

## Outcome

- CP D721. Decision `ARCH-LOGIN-FIRST-PACKETS-V1` in
  `docs/architecture/reviews/OTERYN_GAME_ARCH_LOGIN_FIRST_PACKETS_2026-10-05.md`.
- N4-P acceptance (§1.1):
  - The lock entry is re-pinned from Platform `ed6c0d38` (#1420) to `f880cd74` (#1427), and the
    old pin is kept in `superseded_pins`.
  - The architect and the owner (D721) accept the contract for testing and preproduction only.
    `accepted_for_fnd02` stays false and the production gates are unchanged.
  - U16 ruling: raising an epoch is an operator-only ops action.
- N8 wire (§1.2):
  - A pre-admission ProtocolError is sent with codes 1100–1116 in FND-04A order, then the
    connection closes.
  - The doc maps each server refusal to its code and lists the refusals that stay frameless.
  - It defines the client `SessionError::AdmissionRefused{code}` and the retry rules.
  - Old peers stay safe and `foundation.proto` is unchanged. The wire waits for owner protocol
    acceptance.
- U1 recommendation (§1.3): a separate Passport public loopback client, any port, PKCE S256.
- Packets in dependency order (§2): N8-1, OPS-ASSIGN-REPORT-1, N4-1, N5-1, N2N3-1, RUNBOOK-1.
  Each lists owned_paths, validation, model class and its Platform pieces. Map-track paths are
  excluded.
- ADR-0020 gets a dated factual note. No code or protocol registry change.
- Owner questions: (1) N8 wire acceptance; (2) U1 loopback client choice.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
