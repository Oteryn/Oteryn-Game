# FND-04 — Identity, Game Session, Admission and Character Lease Contract

- Status: Candidate final FND-04 integration index; canonical when owning FND-04C delivery merges
- Programme: Issue #112
- Final integration gate: Issue #130 (`FND-04C`)
- Repository: `blakinio/Oteryn-v2`
- This file is intentionally thin and does not duplicate component contracts.

## 1. Purpose

FND-04 defines the semantic authority model for account/character gameplay admission, CharacterLease fencing, GameSession/TransportBinding lifecycle, reconnect/recovery continuity and their cross-component failure/compatibility contract.

Central invariant:

```text
Platform may authenticate and authorize a bounded attempt.
Only current Oteryn-v2 game-domain authority may create/replace gameplay control.
One authoritative actor/control transition has one linearized winner.
```

## 2. Normative component contracts

### FND-04A — fresh admission

- `docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`
- `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`

Owns:

- Platform attempt vs Oteryn-v2 final admission authority;
- AccountPresenceClaim / CharacterLease / GameSession / initial TransportBinding / RuntimeScopeAuthority separation;
- AccountId->CharacterId ownership before CharacterId->WorldId eligibility;
- one atomic fresh-admission final revalidation/commit;
- strict purpose-separated fresh-entry credential;
- independent protocol/transport/ruleset/content/map/world-policy/offer revision bindings;
- GrantNonce/AdmissionAttemptRef semantics;
- current security/trust source-age <=5s + anti-rollback evidence;
- fresh-admission error semantics.

### FND-04B — reconnect, recovery and continuity

- `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md`
- `docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md`

Owns:

- one current playable `connection_generation` per GameSession;
- healthy-binding non-preemption;
- PREPARE/COMMIT rebind and lost-response reconciliation;
- reconnect proof rotation/fencing;
- server-authoritative `ControlLossEpoch` and same-session grace origin;
- exact 4-second defensive PvE re-entry protection and stable-control re-arm anti-loop rule;
- same-GameSession reconnect/recovery and post-grace existing-actor recovery;
- GameNode/current-owner failover continuity without guessed authority;
- purpose-separated recovery credential and independent recovery revisions;
- reconnect/recovery error semantics.

### FND-04C — integration

- `docs/architecture/FND-04C_ERROR_DIAGNOSTICS_FAILURE_COMPATIBILITY_CONTRACT.md`
- `docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md`
- `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`

Owns:

- complete cross-component FND-04 error/diagnostic/correlation catalogue;
- historical alias disposition;
- Foundation failure-scenario integration;
- cross-repository compatibility/rollout/rollback obligations;
- implementation evidence gates;
- security/privacy integration;
- current programme completion status.

## 3. Authority layers

FND-04 keeps these concepts separate:

```text
AccountPresenceClaim
CharacterLease + character_lease_generation
GameSessionId
TransportBinding = GameSessionId + connection_generation
RuntimeScopeAuthority = semantic runtime scope + ownership generation
```

`NodeId` is process-incarnation/placement identity, not authority.

`GameSessionId` is identity, never bearer proof.

`HandoffId` remains conditional and is not used for ordinary reconnect/recovery.

## 4. Actor/control lifecycle summary

Actor presence:

```text
ABSENT
PRESENT_CONTROLLED
PRESENT_UNCONTROLLED
```

Fresh admission may create the first GameSession only after A's atomic admission boundary.

Eligible same-session reconnect preserves GameSessionId and advances `connection_generation` only through B's atomic COMMIT.

Once a GameSession is terminal, it never revives. If the same authoritative actor remains `PRESENT_UNCONTROLLED`, eligible post-grace recovery creates a **new** GameSession without resetting actor/gameplay state.

Actor becoming `ABSENT` permits ordinary lifecycle release and retires old reconnect protection continuity state; later fresh admission is a new authority lifecycle.

## 5. Timing ownership

The following exact security/profile ceilings are accepted:

- fresh/recovery grant maximum lifetime: 30 seconds;
- fresh/recovery verifier skew: 5 seconds;
- Platform-security and grant-signing key/profile authenticated source-age ceiling: <=5 seconds;
- defensive PvE protection after eligible valid re-entry: exactly 4 seconds.

FND-04 intentionally does **not** freeze numeric values for:

- liveness probe cadence/hysteresis/control-loss detection;
- stale transport cleanup;
- same-session grace duration;
- stable-control protection re-arm threshold;
- CharacterLease TTL/renew/safety deadlines;
- prepared candidate/resource/rate limits.

Historical superseded candidate values `2s/5s/15s` for reconnect/liveness/grace are non-canonical. Deferred numbers require measured registry/OPS/PERF/DUR evidence before implementation activation.

## 6. Compatibility summary

No opaque FND-04 `compatibility_revision` exists.

Fresh admission and recovery use separate credential profiles and independent mandatory revision dimensions. FND-02 `schema_revision` remains diagnostic/build evidence rather than exact gameplay admission/recovery equality.

There is no fresh/recovery/Canary credential reinterpretation or silent downgrade.

## 7. Security/privacy summary

- server game/liveness/runtime evidence decides gameplay control/loss/protection;
- client/OS/Launcher/Guardian evidence is corroborative only;
- diagnostics opt-out remains respected and missing client evidence is not adverse;
- no broad Windows Event Log ingestion, kernel driver, invasive anti-cheat or mandatory device fingerprint is required by FND-04;
- Game Intelligence may investigate bounded/audited patterns but never decides runtime authority or autonomously sanctions/mutates gameplay;
- credentials/proofs/nonces/private keys/private fence values never enter ordinary telemetry.

## 8. Completion and implementation boundary

After accepted FND-04C delivery and lifecycle closeout, FND-04 is **architecture complete**.

That does not mean the runtime is implemented.

Runtime/protocol/persistence/Platform/KMS/deployment implementation requires separately explicit authority and all implementation evidence defined by FND-04C plus downstream DUR/OPS/PERF decisions for deferred physical/numeric values.

Historical superseded FND-04 documents/PRs remain provenance only and cannot override the accepted A/B/C contract set.
