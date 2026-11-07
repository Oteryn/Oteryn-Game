# Oteryn GameNode Process-Incarnation Bootstrap Authorization Decision

- Status: `OWNER_ACCEPTED ARCHITECTURE DECISION` after merge to protected `main`
- Date: 2026-09-22
- Repository: `Oteryn/Oteryn-Game`
- Parent control plane: Issue #162
- Source-readiness programme: Issue #319
- Governing owner decision: Issue #220 comment `5780141258`
- Consumed by: WP5 #415 runtime-scope assignment, S2 #757 custody fencing, later S3-B/G0
- Implementation authority granted by this decision: `NONE`
- Production authority: `NONE`

## 1. Decision timing

**Must decide now?** `YES`.

**Concrete downstream work blocked:** WP5 #415 cannot prove that a target `NodeId` represents a currently authenticated GameNode process incarnation, and S2 #757 cannot add the required database-visible stale-incarnation custody fence without a real current-incarnation authority.

**What becomes harder later if over-designed now:** selecting per-process PKI, heartbeat leases, orchestration, autoscaling or a vendor-specific workload identity would expand the first Reference slice and create avoidable deployment and credential lifecycle coupling.

**Evidence that may justify superseding this decision:** production deployment requirements that make launch-scoped authorization operationally unsafe, a mature workload-identity platform already required for other services, compromise/rotation evidence, multi-host orchestration needs, or measured operational burden showing a different authenticated-incarnation primitive is safer.

**Deliberately not decided:** heartbeat/failure-detector timing, autoscaling, Kubernetes/orchestrator choice, capacity policy, live migration, multi-region placement, general OPS control plane, or per-process mTLS for the first Reference slice.

## 2. Selected first-generation profile

The selected profile is:

`GAME-NODE-REGISTRATION-BOOTSTRAP-AUTH-V1`

A GameNode process:

1. generates one fresh strongly typed UUIDv7 `NodeId` locally for that process incarnation;
2. receives one launch-scoped bootstrap authorization from a trusted deployment/operator/control-plane context;
3. presents that authorization to the Game-owned registration boundary;
4. registration atomically consumes it and durably binds the authenticated launch/incarnation to the exact `NodeId`;
5. registration returns/maintains a sealed current-registration fact consumable by #415 and other explicitly authorized owners;
6. the bootstrap authorization is not reusable after successful registration.

The bootstrap authorization is independent of `NodeId`. `NodeId` is never a credential and never grants Channel or gameplay authority.

## 3. Required security and replay semantics

The first implementation must fail closed for:

- missing or malformed bootstrap authorization;
- replay of an already consumed launch authorization;
- same authorization with changed `NodeId` or changed launch binding;
- duplicate/colliding `NodeId`;
- nil, malformed or non-UUIDv7 `NodeId`;
- stale, revoked or superseded registration;
- wrong-incarnation use of a registration fact;
- unavailable or ambiguous current-registration authority.

The durable registration owner must preserve enough immutable operation/provenance identity to reconcile a lost response without retaining a reusable secret.

A successful registration proves process identity only. It grants zero `ChannelId`, `WorldId`, readiness, gameplay or persistence authority.

## 4. Supersession and currentness

Each process restart creates a new `NodeId`. A prior process incarnation never becomes current merely because it owns old state or presents an old receipt.

Explicit authorized revoke/supersession advances registration authority and makes the prior incarnation unusable for new #415 assignments and S2 custody.

No automatic lease expiry or heartbeat is required in V1. Existing assignment generation/fencing remains responsible for runtime-scope authority until explicit replace/revoke.

## 5. Composition requirements for #415 and S2

#415 may accept an initial or replacement target only after a serialized current-registration read proves the exact `NodeId` is a currently registered authenticated process incarnation at the assignment decision.

S2 #757 may use this owner to implement its required database-visible exclusive registration/custody fence. Every descriptor/observation/checkpoint/clear mutation that depends on current process custody must validate the same current incarnation atomically rather than trusting process-local state, a caller token, persisted evidence, or a stale receipt.

The first implementation should reuse the existing #415 dedicated PostgreSQL target where practical rather than inventing an additional CI target solely for registration.

## 6. Minimum evidence

Material acceptance must include real PostgreSQL/restart evidence for at least:

- fresh authorization + fresh UUIDv7 NodeId registration succeeds;
- reused authorization rejects;
- changed binding under the same authorization rejects;
- duplicate/wrong/stale/superseded NodeId rejects;
- restart obtains a new NodeId and prior incarnation is not current;
- current-registration read consumed by #415 succeeds only for the exact current incarnation;
- S2 stale process cannot mutate retained source state after replacement;
- lost registration response reconciles one result without consuming a second authorization;
- database outage/ambiguous currentness fails closed.

## 7. Non-decisions and authority

This decision does not choose a secret-delivery product, deployment vendor, certificate system, heartbeat, autoscaling policy, production endpoint, or live credential source.

Work must separately allocate exact runtime/SQL/migration/test paths before implementation. Normal independent review, canonical CI, Merge Queue and protected-main readback remain mandatory.

`IMPLEMENTATION_AUTHORITY: NONE_BY_THIS_DECISION`
`LIVE_DEPLOYMENT_AUTHORITY: NONE`
