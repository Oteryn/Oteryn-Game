# OTV2-20260929-n4p-contract-game

```yaml
task_id: OTV2-20260929-n4p-contract-game
title: N4-P native login contract - Game lock entry and Game-side contract candidates
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/n4p-contract-game
pr: null
base_sha: 50d75c6573344be02984b38ea79e32fc9a9caf1f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "N4P-CONTRACT design writer (Claude Code)"
created_at: 2026-09-29T23:00:00Z
updated_at: 2026-09-29T23:50:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/architecture/ADR-0020-native-client-gameplay-entry.md
  - docs/agents/tasks/archive/OTV2-20260929-n4p-contract-game.md
public_contracts:
  - oteryn-native-gateway-login-v1 (Platform, pending lock entry)
  - oteryn-game-native-runtime-status-v1 (candidate)
  - oteryn-game-list-characters-for-account-v1 (candidate)
depends_on: []
blocks:
  - ADR-0020 N4 (client half of the Gateway chain)
cross_repository_coordination_id: OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN
external_repositories:
  - repository: Oteryn/Oteryn-Platform
    pr: 1420
    branch: claude/n4p-native-gateway-contract
    frozen_head: 050df7491852d40e9d19f23dad38fc5d6b1b4268
```

## Outcome

Design-only contract candidates for native login (ADR-0020 §7 N4-P and N4), acceptance by the architect and the owner (#162 Q15a):

- Platform `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` (Oteryn/Oteryn-Platform PR #1420): native `/v1/login`, native ticket redemption to canonical AccountId, Character selection over the Game projection, route selection over NativeTopologyRegistry and reported runtime status, FND-04 grant issuance, signing-key custody, `attempt_ref` idempotency, error mapping, rate limits, Canary-only list, unknowns U1–U14.
- Game `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` (Q16b): the node reports its committed readiness publication, revisions and gameplay endpoint over the existing `native_admission_source` mTLS client, with heartbeat.
- Game `OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` (Q17a): per-account snapshot push with `(epoch, revision)` ordering and a transactional outbox, consistent with `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`.
- Lock entry `OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN` in `PENDING_CANONICAL_MERGE`, pinning the Platform path, PR #1420, frozen head and file digest as pending evidence only.
- ADR-0020 factual notes: the "#1083" batch-partner reference is wrong; pointer from N4-P to the Platform candidate.

Q18a is recorded only as an internal-build allowance in the scope of the Platform contract and in the lock entry note.

## Architecture and source of truth

- PROVEN: FND-04 profile claims and error subset (`FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`, `FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md` §11); node readiness publication fields (`apps/game-server/src/node/serve.rs`, node-boot decision D3/D5); mTLS client shape (`native_admission_source`).
- PROVEN (Platform `main@db9ce970`): ticket issuance requires a Canary binding today; `identities.account_id` UUIDv7 and `native_security_generation` exist; `NativeTopologyRegistry` is testing/preproduction only; `NativeSigningTrustRegistry` holds public keys only.
- DERIVED: deterministic Ed25519 re-signing gives byte-identical retries without storing tokens.
- UNKNOWN: Platform U1–U14; Game U-RS1–U-RS4 and U-LC1–U-LC5.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation-only contract candidates; no mutation, authority, fence or recovery code is written.

## Acceptance criteria

- [x] Game candidates for the runtime-status producer and the projection exist and reference the Platform candidate.
- [x] Lock entry is `PENDING_CANONICAL_MERGE` with canonical fields unset (validator rule).
- [x] ADR-0020 note corrects "#1083" and points N4-P to the Platform candidate without changing a decision.
- [ ] Architect and owner acceptance (outside this task).

## Excluded scope

Code, migrations, workflows, `RESOURCE_LIMITS_REGISTRY.json` entries (proposed in the candidates as a follow-up), protocol registry, secrets, production, any accepted-contract semantics change.

## Closeout

- Validation: `python tools/agents/validate_governance.py` PASS; `python tools/repository/validate_repository_policy.py` PASS; `python -m unittest discover -s tools/agents/tests` PASS; `git diff --check` clean.
- Review: not requested by this worker; review packet returned to the lead.
- Merge commit/result: squash merge of this PR when integrated.
- Follow-ups: after Platform PR #1420 merges, set the lock entry to `LOCKED` with the merge commit and digest; resource-limit registry entries; implementation lanes per #162 plan step 2.
