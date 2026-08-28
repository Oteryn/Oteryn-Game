# OTV2-20260810-fnd04b-reconnect-recovery-continuity

```yaml
task_id: OTV2-20260810-fnd04b-reconnect-recovery-continuity
title: FND-04B reconnect recovery and continuity bounded contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
delivery_issue: 127
programme_issue: 112
delivery_pr: 128
trusted_base_sha: 2fd7bac4879f381d5b97230732076df2e9c61f95
final_delivery_head_sha: d32fd4d7a4c0edce97e0e25e3908885a2da41f4b
merge_sha: e6282b9c48713b2a2980f2598a81775f78725cff
repair_cycles_for_gate: 3
max_repair_cycles_for_gate: 3
terminal_review_id: 4896085320
agent_governance_run: 31381886243
dependency_review_run: 31381886237
codeql_run: 31381886238
runtime_e2e: NOT_APPLICABLE
ownership_released: true
owned_paths: []
canonical_public_contracts:
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
completed_at: 2026-08-10T13:10:00+02:00
```

## Result

FND-04B reconnect, recovery and continuity architecture is accepted on `main` through PR #128. No runtime implementation was authorized.

## Accepted continuity semantics

- one current playable `connection_generation` per GameSession;
- healthy current binding is non-preemptible by bearer reconnect proof or recovery JWT;
- PREPARE grants no gameplay/liveness/fencing authority;
- COMMIT atomically revalidates current session/generation/grace/controller/presence/lease/runtime/reconciliation/security facts and performs one authority switch;
- candidate successor reconnect proof is inactive before COMMIT, exact-candidate bound and atomically activated while predecessor proof is fenced;
- same-attempt reconciliation handles lost PREPARE/COMMIT response without another authority transition;
- same-session grace begins at server-authoritative `ControlLossEpoch`; restart/failover never resets or extends the original deadline;
- historical candidate `2s/5s/15s` reconnect/liveness values are non-canonical;
- exact 4-second defensive PvE protection is the only frozen reconnect/gameplay timing and is activated at most once per eligible entitlement;
- stable-control server evidence is required to re-arm a later protection entitlement, preventing disconnect-loop extension;
- old epoch/protection state is retired when actor becomes `ABSENT` and is never inherited by a fresh admission;
- same-session recovery preserves GameSessionId, CommandId/server_sequence/domain revisions and actor state;
- post-grace existing-actor recovery uses a new GameSessionId and preserves the same authoritative actor without respawn/reset/teleport/heal;
- GameNode replacement never derives authority from NodeId and same-session continuity requires complete fenced recoverable evidence;
- ordinary reconnect does not manufacture `HandoffId`.

## Recovery profile

The accepted `oteryn-reauth-recovery-v1` profile:

- is purpose-separated from fresh admission;
- uses JWS Compact JWT and fully specified `Ed25519`;
- uses verifier-configured recovery trust context; token semantics cannot select/broaden trust;
- follows deterministic algorithm/key/signature/schema/binding/profile/revision precedence consistent with FND-04A;
- binds AccountId/CharacterId/WorldId plus independent protocol/transport/ruleset/content/map/world-policy revision dimensions;
- contains no ChannelId/InstanceId/NodeId/runtime owner/HandoffId or opaque `compatibility_revision` authority;
- requires authenticated Platform-security and recovery key/profile source-age <=5s with monotonic anti-rollback semantics;
- proves AccountId->CharacterId before world/actor/controller classification;
- consumes RecoveryGrantNonce only with a successful recovery authority transition.

## Repair history

1. structural schema separated from semantic binding/profile/revision classification;
2. grace origin/failover deadline continuity and inactive successor reconnect-proof lost-response contract;
3. stable-control protection re-arm to prevent repeated disconnect safety loops.

Repair budget was exhausted at `3/3`; terminal review found zero further material findings.

## Final validation evidence

- delivery head `d32fd4d7a4c0edce97e0e25e3908885a2da41f4b`;
- Agent Governance `31381886243`: PASS;
- Dependency Review `31381886237`: PASS;
- CodeQL `31381886238`: PASS;
- terminal architecture/security review `4896085320`: PASS, zero material findings;
- unresolved review threads: 0;
- delivery squash merge `e6282b9c48713b2a2980f2598a81775f78725cff`;
- runtime/component/browser E2E: `NOT_APPLICABLE`.

## Next gate

Per programme #112, the only remaining FND-04 delivery gate is `FND-04C — Error/Diagnostics + Failure/Compatibility Integration`. This archive releases all FND-04B task ownership; the two public FND-04B contracts remain canonical until explicitly superseded.
