# OTV2-20260929-n4p-contract-game

```yaml
task_id: OTV2-20260929-n4p-contract-game
title: N4-P native login contract - Game lock entry and Game-side contract candidates
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/n4p-contract-game
pr: 1291
base_sha: 50d75c6573344be02984b38ea79e32fc9a9caf1f
head_sha: 1f782b4b4dac857afa19babe0dcc360029ea3704
final_head_sha: null
final_head_frozen_at: null
owner: "N4P-CONTRACT design writer (Claude Code)"
created_at: 2026-09-29T23:00:00Z
updated_at: 2026-09-30T00:45:00Z
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
    frozen_head: 892d640d7dbf5497811901d250d384b19e639556
```

## Outcome

Design-only contract candidates for native login (ADR-0020 §7 N4-P and N4), acceptance by the architect and the owner (#162 Q15a):

- Platform `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` (Oteryn/Oteryn-Platform PR #1420): native `/v1/login`, native ticket redemption to canonical AccountId, Character selection over the Game projection, route selection over NativeTopologyRegistry and reported runtime status, FND-04 grant issuance, signing-key custody, `attempt_ref` idempotency, error mapping, rate limits, Canary-only list, unknowns U1–U14.
- Game `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` (Q16b): the node reports its committed readiness publication and revisions (no endpoint; the World Registry owns the route) with a heartbeat; `oteryn-game-ops` reports each assignment so Platform binds node reports to the ownership authority; `assignment_epoch` resets after a restore; a separate client certificate per purpose.
- Game `OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` (Q17a): per-account snapshot push from Character Authority hosts with `(epoch, revision)` ordering, a transactional outbox, a liveness watermark (issuance fails closed when stale) and a full resync on epoch raise, consistent with `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`; push accepted for Q18a internal builds, release choice push-with-bound vs pull.
- Lock entry `OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN` in `PENDING_CANONICAL_MERGE`, pinning the Platform path, PR #1420, frozen head and file digest as pending evidence only.
- ADR-0020 factual notes: the "#1083" batch-partner reference is wrong (#1083 is the merged capture tool; #1275 carried N5 and the #1249 CodeQL bump); pointer from N4-P to the Platform candidate.

Q18a is recorded only as an internal-build allowance in the scope of the Platform contract and in the lock entry note.

## Architecture and source of truth

- PROVEN: FND-04 profile claims and error subset (`FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`, `FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md` §11); node readiness publication fields (`apps/game-server/src/node/serve.rs`, node-boot decision D3/D5); mTLS client shape (`native_admission_source`).
- PROVEN (Platform `main@db9ce970`): ticket issuance requires a Canary binding today; `identities.account_id` UUIDv7 and `native_security_generation` exist; `NativeTopologyRegistry` is testing/preproduction only; `NativeSigningTrustRegistry` holds public keys only.
- DERIVED: deterministic Ed25519 re-signing gives byte-identical retries without storing tokens.
- UNKNOWN: Platform U1–U18; Game U-RS1–U-RS5 and U-LC1–U-LC6.

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
- Review: independent security/architecture review round 1 on `1f782b4b` (Game) and `050df749` (Platform) returned FIX. The Material findings (route owned by the World Registry; identity per purpose) and all evidence gaps and hardening items were fixed in one push per repository. Re-review is returned to the lead.
- Merge commit/result: squash merge of #1291.
- Follow-ups: after Platform PR #1420 merges, set the lock entry to `LOCKED` with the merge commit and digest; resource-limit registry entries; implementation lanes per #162 plan step 2.
