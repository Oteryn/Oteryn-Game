# OTV2-20260810-fnd04b-reconnect-recovery-continuity

```yaml
task_id: OTV2-20260810-fnd04b-reconnect-recovery-continuity
title: FND-04B reconnect recovery and continuity bounded contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd04b-reconnect-recovery-continuity
issue: 127
programme_issue: 112
pr: 128
trusted_base_sha: 2fd7bac4879f381d5b97230732076df2e9c61f95
historical_reviewed_evidence_pr: 109
historical_reviewed_evidence_head: bf82e392d6ef8b1e627849cdc7383af9a7c987ae
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-10T12:50:00+02:00
updated_at: 2026-08-10T13:14:00+02:00
repair_cycles_for_current_gate: 3
max_repair_cycles_for_current_gate: 3
blocker: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260810-fnd04b-reconnect-recovery-continuity.md
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
public_contracts:
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
  - docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md
```

## Goal

Deliver bounded FND-04B from programme #112 after accepted FND-04A. Freeze reconnect, recovery and continuity authority/security semantics without runtime implementation or FND-04C integration.

## Repair history

### Cycle 1 — structural schema vs semantic binding/profile/revision

Recovery schema now owns only claim membership/type/canonical lexical shape. After authentication, wrong `iss`/`aud`/`typ`/`purpose` is binding mismatch; unsupported profile/protocol/transport/gameplay revision is revision unsupported. Invalid signature preempts otherwise well-formed semantic defects.

### Cycle 2 — grace origin, failover continuity and successor proof

- same-session grace begins exactly at server-authoritative `ControlLossEpoch`, never socket-close/cleanup/retry;
- original grace deadline and protection state survive failover without restart;
- internal non-reused loss-epoch evidence prevents duplicate protection;
- PREPARE may deliver one inactive candidate successor proof bound to exact candidate;
- COMMIT atomically activates successor proof and fences predecessor;
- aborted/expired candidate proof is permanently invalid;
- lost-response reconciliation never mints a second authority/proof;
- ambiguous post-crash commit state must be reconstructed from fenced evidence or fail closed.

### Cycle 3 — protection re-arm / disconnect-loop abuse

Review found that `control restored -> immediately lost again` must not automatically create another 4-second entitlement, otherwise deliberate network toggling could approximate indefinite safety.

Accepted final rule:

- one eligible loss episode may activate one exact 4-second protection window at most once;
- successful control restoration does **not** automatically re-arm a new protection entitlement;
- a later loss can receive new protection only after a server-authoritative stable-control re-arm condition has been satisfied;
- exact numeric re-arm hysteresis is deferred to measured gameplay/network/OPS evidence but must be finite/registered before implementation;
- loss before re-arm may still reconnect but creates no fresh protection window and never extends an active one;
- re-arm/epoch/protection state survives failover and cannot reset on process restart;
- epoch/protection continuity may remain attached to the same `PRESENT_UNCONTROLLED` actor after old GameSession terminality, but is retired when actor becomes `ABSENT`; fresh admission never inherits old protection state.

Repair budget is exhausted at `3/3`. Any new material finding is a hard blocker; no task-local fourth repair is authorized.

## Historical corrections retained

- historical `2s/5s/15s` values are non-canonical; only exact 4-second protection is frozen;
- no opaque `compatibility_revision`;
- verifier-anchored recovery trust + deterministic crypto/schema precedence;
- authenticated source-age <=5s + anti-rollback security/trust evidence;
- AccountId->CharacterId before world/actor/controller classification.

## Final validation plan

- exact three-path scope / clean ancestry;
- complete state-machine/race/security review;
- proof/grace/epoch/protection/re-arm failover continuity review;
- healthy-binding non-preemption;
- stale generation/proof/candidate fencing;
- recovery credential/profile security and independent revision review;
- GameNode no-guessed-continuity review;
- exact-head Agent Governance, Dependency Review and CodeQL PASS;
- zero unresolved material threads;
- terminal exact-head architecture/security review with zero material findings;
- squash merge unchanged accepted head only;
- separate lifecycle closeout before FND-04C.

Runtime/component/browser E2E: `NOT_APPLICABLE`.
