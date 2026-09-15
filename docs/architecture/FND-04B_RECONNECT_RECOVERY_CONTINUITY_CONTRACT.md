# FND-04B — Reconnect, Recovery and Continuity Contract

- Status: Candidate bounded architecture contract; canonical only when the owning FND-04B delivery merges
- Gate: `FND-04B`
- Programme: Issue #112
- Owning delivery: Issue #127 / PR #128
- Repository: `blakinio/Oteryn-v2`
- Trusted base: `main@2fd7bac4879f381d5b97230732076df2e9c61f95`
- Historical reviewed evidence only: superseded PR #109 exact head `bf82e392d6ef8b1e627849cdc7383af9a7c987ae`
- Normative recovery profile: `docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md`
- Consumes: accepted FND-04A; FND-02; accepted FND-03; ADR-0003; ADR-0012; FND-ID-01; disconnect/re-entry owner decisions; Foundation Error Vocabulary
- Does not authorize: runtime/protocol/persistence/Platform/KMS/deployment/production implementation or final FND-04C integration

## 1. Purpose and central invariant

FND-04B freezes game-domain rules for restoring player control after transport/process loss without split control, stale-writer resurrection, replayable authority, actor reset or security downgrade.

```text
one GameSession
-> at most one current playable TransportBinding generation
-> replacement authority changes only at one revalidated atomic boundary
-> stale/prepared/old transports never regain command/liveness/fencing authority
```

FND-04A fresh-admission authority remains unchanged. Fresh-entry grants are never reconnect/recovery credentials.

## 2. Authority layers remain distinct

- `AccountPresenceClaim` — account-global mandatory-presence ownership;
- `CharacterLease` + non-reused generation — character writer/control fence;
- `GameSessionId` — logical control lifecycle identity, never bearer proof;
- `TransportBinding = GameSessionId + connection_generation` — concrete playable transport fence;
- `RuntimeScopeAuthority` — current ChannelRuntime/InstanceRuntime owner + ownership generation.

`NodeId` is placement/process-incarnation evidence, not authority. FND-04B adds no foundation identity. `HandoffId` is not used for ordinary reconnect/recovery.

## 3. Actor and GameSession continuity states

Actor presence:

```text
ABSENT
PRESENT_CONTROLLED
PRESENT_UNCONTROLLED
```

Session continuity MUST preserve semantics equivalent to:

```text
ACTIVE
CONTROL_SUSPECTED
RECONNECTABLE
TERMINATING
TERMINAL
```

`CONTROL_SUSPECTED` is not loss authority: it permits no bearer-proof preemption, no `ControlLossEpoch`, no same-session grace and no protection.

`RECONNECTABLE` begins only after authoritative unexpected playable-control loss. `TERMINAL` is irreversible for that GameSessionId; later control requires a new GameSessionId.

## 4. Server-authoritative liveness

FND-02 authenticated probe/ack is the primitive. Socket-open, TCP keepalive, gameplay silence, client self-report or OS/Launcher/Guardian evidence alone never decides playable-control authority.

Client/OS/Launcher/Guardian evidence remains corroborative only under accepted privacy/forensic policy.

The following values must be finite before implementation but are deliberately not guessed here:

- probe cadence;
- missed-probe/hysteresis thresholds;
- control-loss detection delay;
- stale transport cleanup delay;
- same-session grace duration;
- protection re-arm stability threshold;
- scheduler/clock margins.

Historical `2s/5s/15s` values from superseded #109 are non-canonical.

## 5. ControlLossEpoch

`ControlLossEpoch` is logical server-authoritative continuity state, not a new public foundation identity.

It begins only when the current game authority proves unexpected playable-control loss for a previously controlled actor/session.

It does not begin from graceful logout, fresh login, healthy migration, socket close alone, PREPARE, failed reconnect/recovery, client crash report alone or process restart alone.

One continuous loss episode has one logical epoch. Retries, multiple candidates, failed COMMITs, recovery-grant retries and runtime-owner relocation do not create additional epochs.

Implementation MUST preserve a non-reused internal epoch discriminator/ordinal or equivalently strong evidence plus:

- epoch origin/deadline evidence;
- whether protection was activated;
- protection expiry if activated;
- protection re-arm eligibility state.

This continuity state survives GameNode/runtime-owner replacement and, while the same authoritative actor remains `PRESENT_UNCONTROLLED`, may outlive terminality of the old GameSession. It is retired when the actor legally becomes `ABSENT` or when no longer required for bounded audit/protection semantics. A later fresh actor admission must not inherit an old epoch.

## 6. Same-session grace

The grace duration is finite but its numeric value is deferred.

Its origin is exact:

```text
same-session grace begins at the authoritative ControlLossEpoch boundary
```

It never begins at socket close, first missed probe, cleanup or reconnect attempt.

The original deadline/remaining eligibility is server-authoritative continuity state and MUST survive failover/restart without restart or extension. If current authority cannot prove the original epoch/deadline, it MUST NOT guess a new same-session window.

Stale-transport cleanup is a separate resource lifecycle and does not redefine grace.

## 7. Exact 4-second defensive PvE protection

Accepted owner semantics:

```text
eligible valid re-entry after unexpected playable-control loss
-> exactly 4 seconds defensive PvE protection
```

Protection begins when authoritative playable control is successfully restored for an eligible loss epoch.

Per eligible protection entitlement:

- activation occurs at most once;
- retries/rebinds/recovery within the same entitlement do not reset/extend it;
- failed PREPARE/COMMIT never activates it;
- graceful logout/login and healthy migration do not receive it;
- protection deadline survives failover/restart without restarting the 4-second interval.

Gameplay semantics remain exactly as accepted: movement/self-heal/resource potions allowed under ordinary rules; offensive PvE and healing another player prohibited and not buffered; receiving legal healing allowed; no HP/resource/position/condition/cooldown/combat/PZ/logout/threat/aggro/encounter/committed-effect reset.

## 8. Protection re-arm and disconnect-loop abuse prevention

Successful control restoration closes the current loss episode, but **does not automatically re-arm another 4-second protection entitlement**.

A later unexpected loss may receive a new protection entitlement only after the current generation has first satisfied a server-authoritative **stable-control re-arm condition** proving meaningful resumed control beyond reconnect/retry churn.

Requirements:

- re-arm depends only on server gameplay/liveness/runtime evidence;
- client disconnect/reconnect timing cannot self-declare re-arm;
- exact numeric stability/hysteresis threshold is deferred to measured OPS/PERF/gameplay evidence, but MUST be finite and registered before implementation;
- a loss occurring before re-arm may still be handled as a liveness/reconnect event, but it does **not** create another fresh 4-second protection window;
- an already active protection window continues to its original expiry and is never extended by another loss/re-entry;
- re-arm state/deadline evidence survives GameNode/runtime-owner replacement and cannot reset on restart.

This prevents deliberate network toggling from producing effectively indefinite protection while preserving valid later protection after genuinely resumed stable play and a new unexpected loss.

## 9. Fast reconnect proof

Fast reconnect proof is high-entropy bearer-sensitive game-domain secret material distinct from GameSessionId.

It MUST be:

- issued only after a successful authoritative session/control boundary;
- scoped to one GameSession continuity chain;
- accepted only over authenticated FND-02/TLS transport;
- secret-safe and never ordinary logs/metrics/analytics;
- rotated on each successful same-GameSession replacement;
- predecessor-fenced so old proof cannot authorize later generation.

Exact hash/storage/KMS mechanics and future sender-constrained/PoP form are deferred.

Possession alone cannot evict a healthy current binding.

## 10. Healthy current binding non-preemption

If current `connection_generation` still has sufficient server-authoritative playable-control evidence, unsolicited reconnect/recovery fails without authority change even with current reconnect proof or a valid recovery JWT.

The contender receives no command/liveness authority, does not advance/fence generation, does not activate/rotate proof, does not create loss/protection state and consumes no recovery nonce as success.

Healthy-session migration is a distinct future transition requiring authorization rooted in current authoritative generation.

## 11. ReconnectAttemptRef and PREPARE

`ReconnectAttemptRef` is operation idempotency/correlation identity, not bearer authority or foundation entity identity.

PREPARE binds a finite candidate to at least:

```text
GameSessionId
ReconnectAttemptRef
predecessor connection_generation
strict-successor candidate connection_generation
exact candidate authenticated transport
proof class used
finite prepared deadline
current authority/reconciliation observations
```

PREPARE grants no gameplay/liveness/fencing authority and consumes no RecoveryGrantNonce as success.

### Candidate successor reconnect proof

PREPARE MAY generate and deliver one **inactive** candidate successor reconnect proof to the exact prepared candidate transport.

- inactive proof has zero authority before COMMIT;
- it is bound to exact attempt/session/candidate generation/transport;
- same eligible PREPARE retry returns the same logical candidate/proof, not another independently usable secret;
- abort/expiry/supersession permanently invalidates it;
- COMMIT atomically activates it while invalidating/fencing predecessor proof.

An equivalent secret-delivery construction is acceptable only with proof of identical lost-response and fencing invariants.

Pending candidate resources MUST be bounded; exact numeric limits are later registry evidence.

## 12. PREPARE eligibility

Fast reconnect PREPARE requires:

- session `RECONNECTABLE`, not terminal/superseded;
- original same-session grace still valid;
- no healthy current controller;
- current reconnect proof for that continuity chain;
- predecessor generation/session match;
- compatible AccountPresenceClaim and CharacterLease;
- current runtime owner/placement resolves same actor/session;
- safe FND-02 reconciliation state.

Reauthenticated same-session recovery uses the dedicated recovery profile in place of missing reconnect proof.

PREPARE is never authorization escrow.

## 13. COMMIT atomic revalidation

Immediately before/atomically with same-session authority switch COMMIT revalidates:

1. exact candidate exists, unexpired and bound to candidate transport/session/attempt;
2. predecessor generation still exactly matches preparation source;
3. session remains reconnect-eligible and original grace deadline valid;
4. no healthy current controller regained authority;
5. AccountPresenceClaim still denotes same CharacterId;
6. CharacterLease remains compatible/current;
7. RuntimeScopeAuthority/placement/ownership generation remains current;
8. FND-02 command/server-sequence/domain-revision reconciliation remains safe;
9. no newer handoff/takeover/fence/terminal transition supersedes candidate;
10. proof-specific mutable security/trust/revision/nonce facts remain current.

Only success atomically:

```text
fences predecessor transport authority
+ makes candidate connection_generation current
+ binds command/liveness/reconciliation authority to candidate transport
+ activates candidate successor reconnect proof
+ invalidates predecessor proof
+ preserves GameSessionId
+ preserves CommandId/server_sequence/domain revisions
+ preserves actor/gameplay state
+ marks re-entry for current loss epoch
+ activates exact 4s protection only if this epoch has an eligible not-yet-used entitlement
```

Success is externally visible only after commit.

## 14. Failed/stale COMMIT

Failed candidate never advances generation, revives predecessor, activates candidate proof, invalidates a current proof merely due to failure, consumes RecoveryGrantNonce as success, manufactures protection or rolls authority back to PREPARE-time state.

Candidate proof becomes permanently unusable after abort/expiry/supersession. Whatever authority is actually current at revalidation remains current.

Prepared expiry is distinct from grace expiry; a new PREPARE is possible only if original grace/current state independently still permit it.

## 15. Lost response and reconciliation

Same `ReconnectAttemptRef` returns stable state: committed/already committed, prepared/pending, aborted, prepared-expired or terminally superseded.

A lost COMMIT response cannot produce a second switch.

If COMMIT succeeded, predecessor generation/proof remain fenced and candidate generation/proof remain current. Result replay may occur only on the exact current authenticated candidate transport or equivalently proven current-generation channel; current proof is never leaked to an unproven transport.

Inactive proof delivery during PREPARE allows a client to possess the successor proof before COMMIT response. If both committed transport and usable successor proof are lost, predecessor proof never revives; reauthenticated recovery is required.

If process recovery cannot prove whether candidate committed, current generation/proof/attempt state must be reconstructed from fenced authority evidence before further same-session authority. If not provable, fail closed with no guessed winner.

## 16. Same-GameSession continuity

Successful same-session reconnect/recovery:

- preserves GameSessionId;
- establishes strictly newer connection_generation;
- preserves actor state;
- preserves FND-02 CommandId order, server_sequence and typed domain revisions;
- resumes only from a provable reconciliation boundary;
- uses explicit resync/snapshot when continuity cannot be proven;
- never guesses through gaps.

A stale physically open socket has zero command/liveness/fencing authority after the winner commits.

## 17. Reauthenticated recovery authority

When reconnect proof is unavailable, Platform may issue only `oteryn-reauth-recovery-v1`.

It proves only fresh Platform authentication and permission to attempt recovery of CharacterId in WorldId. It does not prove GameSession existence, actor presence, current placement, lease/runtime ownership, controller health or permission to recreate/reset actor.

Before world/actor/controller classification Oteryn-v2 MUST prove current `AccountId -> CharacterId` ownership first, then current `CharacterId -> WorldId`/eligibility.

Recovery carries no ChannelId, InstanceId, NodeId or runtime ownership generation as authority.

## 18. Recovery revisions and security evidence

Recovery independently binds:

```text
protocol_major
transport_profile
ruleset_revision
content_revision
map_revision
world_policy_revision
```

No opaque `compatibility_revision` exists. FND-02 `schema_revision` remains diagnostic/build evidence.

Platform-security and recovery key/profile trust evidence preserve FND-04A semantics:

- authenticated source authority/purpose/scope;
- authenticated source-observation provenance;
- conservative `upper_bound_source_age <= 5s` including clock uncertainty;
- cache operations never re-age evidence;
- monotonic/comparable source revision/fence;
- older allow/trust cannot roll back newer deny/revoke;
- equal revision contradictory content fails closed;
- restart reconstructs current non-rollback floor before authorization.

PREPARE/earlier validation is not trust escrow; mutable proof-specific facts revalidate at COMMIT.

## 19. Ordered recovery dispatch

After recovery credential/security plus ownership-safe account/character/world validation:

1. healthy current playable controller -> dedicated conflict;
2. same GameSession reconnect-eligible and original grace valid -> same-session recovery via PREPARE/COMMIT;
3. prior GameSession terminal but same actor remains `PRESENT_UNCONTROLLED` -> post-grace recovery with new GameSession;
4. otherwise -> target not eligible.

Healthy-controller conflict cannot be hidden as generic no-target.

## 20. Same-session reauthenticated recovery

Uses same PREPARE/COMMIT state machine with recovery grant replacing missing reconnect proof.

COMMIT additionally revalidates recovery JWT time, exact credential/trust state, key/profile evidence source age/order/current decision, Platform-security source age/order/generation/state, RecoveryGrantNonce, each independent revision, AccountId->CharacterId first and CharacterId->WorldId second.

RecoveryGrantNonce is consumed only with successful authority switch.

## 21. Grace expiry and post-grace existing-actor recovery

Once original same-session grace expires:

- old GameSession proceeds to terminality;
- GameSessionId cannot revive;
- reconnect proof cannot resurrect it;
- old prepared candidates cannot commit;
- actor may remain `PRESENT_UNCONTROLLED` under gameplay/combat/logout rules.

Later eligible control uses reauthenticated recovery with a **new GameSessionId**.

Post-grace recovery requires valid account/character/world relation, old session terminal, same actor still `PRESENT_UNCONTROLLED`, no current controller, same AccountPresenceClaim, current CharacterLease/runtime owner/placement and current independent revisions.

One atomic boundary revalidates all mutable credential/security/nonce/ownership/actor facts.

Success:

```text
preserves existing actor exactly
+ creates new canonical GameSessionId
+ connection_generation = 1
+ new current reconnect proof
+ fresh authoritative snapshot/reconciliation boundary
+ consumes RecoveryGrantNonce
+ restores playable control
```

No respawn, teleport, heal, refill, condition/cooldown/combat/PZ/logout/threat/aggro/encounter reset or committed-effect rollback.

If an old loss epoch remains provably associated with this still-present actor and has an eligible unused protection entitlement, re-entry may activate that entitlement; post-grace status alone does not create/re-arm one. If actor became `ABSENT`, old epoch/protection eligibility is retired and a later fresh admission cannot inherit it.

## 22. Recovery locator and GameNode/process failure

Platform grant does not authorize concrete placement. Game domain resolves current actor/session placement and current runtime owner before commit.

Locator is routing evidence, not authority. Stale locator cannot attach control to former owner; current RuntimeScopeAuthority generation fences old owners.

After GameNode/process replacement, same-session continuation is allowed only if fenced recoverable evidence proves at minimum:

- GameSessionId and terminality;
- current/last connection generation;
- reconnect proof current/candidate/rotation state;
- ReconnectAttempt disposition needed for ambiguity resolution;
- current ControlLossEpoch discriminator;
- original grace deadline;
- protection activation/expiry/re-arm state;
- actor/lease state;
- FND-02 reconciliation boundary.

If this cannot be proven, same GameSession is not guessed/recreated and grace/protection deadlines are not restarted. If actor remains authoritative, only accepted post-grace path may later restore control once old session is terminal/fenced. If actor presence cannot be proven, fail closed rather than recreate/duplicate.

Process restart itself does not create a loss epoch.

## 23. Channel/Instance handoff boundary

Ordinary reconnect/recovery follows current authoritative placement and never chooses a new Channel/Instance from stale credential.

Concurrent handoff/fencing participates in COMMIT revalidation. Stale reconnect cannot resurrect source owner. `HandoffId` is used only by a separately accepted actual handoff contract, never ordinary reconnect.

## 24. Security/privacy

Never log/export raw reconnect proof, candidate successor proof, recovery JWT, RecoveryGrantNonce, OAuth/Game Login Ticket/private key/PoP material.

Before successful recovery authentication, diagnostics do not reveal semantic issuer/audience/profile/purpose/typ/schema/world/actor/controller match state.

AccountId/CharacterId remain privacy-controlled. Game Intelligence may investigate bounded/audited patterns but cannot sanction, mutate gameplay or decide runtime reconnect authority. No kernel driver, invasive anti-cheat or mandatory device fingerprint is authorized.

## 25. FND-04B error subset

FND-04C integrates the complete catalogue. B freezes its transition semantics.

| Code | Category | Progression | Retry / next authority | Mutation outcome | Public class | Redacted diagnostic |
|---|---|---|---|---|---|---|
| `RECONNECT_PROOF_INVALID` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | current proof or reauthenticated recovery | none | `AUTHENTICATION_REQUIRED` | `reconnect proof authentication failed` |
| `RECONNECT_HEALTHY_BINDING_PRESENT` | `CONFLICT` | `TERMINAL` | incumbent remains authority | none | `CHARACTER_ALREADY_ACTIVE` | `current playable controller remains authoritative` |
| `RECONNECT_NOT_ELIGIBLE` | `SESSION_REJECTED` | `TERMINAL` for same-session path | resolve recovery path | none | `SESSION_UNAVAILABLE` | `session is not eligible for same-session reconnect` |
| `RECONNECT_PREPARED_EXPIRED` | `TIMEOUT` | `TERMINAL` for candidate | new PREPARE only if original grace/current facts permit | candidate aborted | `TEMPORARILY_UNAVAILABLE` | `prepared reconnect candidate expired` |
| `RECONNECT_PREPARED_STALE` | `STALE_GENERATION` | `TERMINAL` | reconcile current authority | none | `SESSION_UNAVAILABLE` | `prepared reconnect candidate no longer current` |
| `RECONNECT_RECONCILIATION_UNAVAILABLE` | `INTERNAL_UNAVAILABLE` | bounded `RETRYABLE` | same-attempt/current-authority reconciliation only | none | `TEMPORARILY_UNAVAILABLE` | `reconnect authority outcome requires reconciliation` |
| `RECONNECT_GRACE_EXPIRED` | `SESSION_REJECTED` | `TERMINAL` for old GameSession | post-grace recovery/fresh path as state permits | old GameSession cannot revive | `SESSION_UNAVAILABLE` | `same-session reconnect window expired` |
| `RECOVERY_GRANT_AUTHENTICATION_FAILED` | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | fresh authenticated recovery | none | `AUTHENTICATION_REQUIRED` | `recovery credential authentication failed` |
| `RECOVERY_GRANT_REVISION_UNSUPPORTED` | `UNSUPPORTED_REVISION` | `TERMINAL` | compatible revision; no downgrade | none | `CLIENT_UPDATE_REQUIRED` | `recovery authoritative revision unsupported` |
| `RECOVERY_GRANT_SECURITY_EVIDENCE_STALE` | `DEPENDENCY_UNAVAILABLE` | bounded `RETRYABLE` while grant/target valid | fresh authenticated non-rollback evidence | no nonce/authority mutation | `TEMPORARILY_UNAVAILABLE` | `recovery security evidence unavailable, stale or superseded` |
| `RECOVERY_HEALTHY_CONTROLLER_PRESENT` | `CONFLICT` | `TERMINAL` | incumbent remains authority | none | `CHARACTER_ALREADY_ACTIVE` | `recovery blocked by current playable controller` |
| `RECOVERY_TARGET_NOT_ELIGIBLE` | `SESSION_REJECTED` | `TERMINAL` | resolve current actor/session flow | none | `SESSION_UNAVAILABLE` | `recovery target is not eligible` |

Recovery-profile malformed/binding/time/replay/security/world outcomes remain normative in the companion profile and are integrated by C.

Correlation fields are credential-free; never raw proofs/nonces/private fencing values.

## 26. Required implementation evidence

Future implementation acceptance MUST independently prove at least:

### Reconnect/proof

- stale/predecessor proof replay rejected;
- two contenders after eligible loss -> one commit winner;
- healthy binding + stolen proof -> incumbent not preempted;
- PREPARE then incumbent regains control -> COMMIT rejected;
- PREPARE then generation/session/lease/runtime owner changes -> rejected;
- PREPARE expiry while original grace valid -> fresh evaluation required;
- grace expiry -> old GameSession cannot revive;
- PREPARE response loss -> same attempt does not mint second candidate proof;
- COMMIT success + lost response -> same attempt reconciles winner;
- loss of committed transport/proof -> predecessor proof never revives; reauthenticated recovery required;
- physically open stale socket -> commands/liveness rejected.

### Loss/grace/protection

- socket close without authoritative loss -> no epoch/grace/protection;
- grace origin equals loss epoch, not socket/cleanup;
- graceful logout/login -> no protection;
- one entitlement + repeated retries -> one exact 4s activation;
- repeated disconnect before stable-control re-arm -> no new 4s entitlement;
- stable-control re-arm then later independent unexpected loss -> new entitlement may be created;
- process/runtime-owner replacement during grace/protection/re-arm -> deadlines/state preserved, not reset;
- actor becomes ABSENT -> old epoch/protection eligibility retired;
- client/OS diagnostics cannot override liveness/protection authority.

### Recovery security

- companion profile crypto/schema/binding/revision fixtures;
- older allow/trust cannot roll back newer deny/revoke while still <5s;
- cache refresh cannot re-age;
- restart without anti-rollback floor cannot authorize;
- key/profile revoke or Platform security change after PREPARE before COMMIT -> fail before nonce/authority mutation.

### Ownership/actor/failover

- non-owned CharacterId fails before world/actor/controller classification;
- valid ownership + stale world -> no retarget;
- healthy controller not hidden as no-target;
- post-grace actor present -> new GameSession without reset;
- actor becomes ABSENT before commit -> reject, no recreation;
- old runtime owner rejected after ownership-generation change;
- missing continuity/loss/grace/protection evidence after failover -> same session not guessed and timers not reset;
- stale locator and handoff/reconnect races obey current owner fence.

## 27. Decision timing

| Decision | Now? | Blocks | Risk if wrong later | Superseding evidence | Deferred |
|---|---|---|---|---|---|
| one current transport generation | `YES` | reconnect/session/runtime | split authority | formal/fault proof | physical structures |
| healthy binding non-preemption | `YES` | security | stolen proof kicks healthy player | current-generation migration contract | migration UX/protocol |
| PREPARE/COMMIT + revalidation | `YES` | reconnect wire/runtime | stale authorization/lost response | alternative linearizability proof | storage encoding |
| inactive successor proof or equivalent | `YES` | safe proof rotation | commit-response loss strands/resurrects proof | equivalent secret-delivery proof | secret representation |
| grace origin at loss epoch; failover preserves deadline | `YES` | reconnect eligibility | restart/socket events change authority window | explicit semantic supersession | numeric duration |
| protection stable-control re-arm | `YES` | anti-abuse | disconnect loop renews safety indefinitely | gameplay/liveness evidence preserving anti-loop invariant | numeric re-arm threshold |
| exact 4s protection | `YES`, accepted | gameplay | fairness contract | owner gameplay supersession | timer implementation |
| exact liveness/grace/cleanup/re-arm numbers | `NO` | implementation acceptance | guessed timing | measured evidence | numeric values |
| post-grace existing actor -> new GameSession | `YES` | recovery | reset/respawn exploit | proof preserving actor state | physical storage |
| independent recovery revisions | `YES` | profile/rollout | opaque mixed-state compatibility | coordinated supersession | physical registry |
| failover requires fenced complete continuity evidence | `YES` | recovery/failover | guessed session resurrection | durable fault proof | checkpoint mechanics |

## 28. Acceptance boundary

FND-04B acceptance freezes reconnect/recovery/continuity semantics only. FND-04C still integrates complete errors/diagnostics, failure scenarios, compatibility/rollout evidence, programme status and thin final FND-04 index.

No runtime implementation is authorized.
