# PROD-ENTITLEMENTS-01 — Game Consumer / Enforcement Contract Acceptance

Status: **ACCEPTED ARCHITECTURE — RUNTIME IMPLEMENTATION AND ACTIVATION NOT AUTHORIZED**

## Canonical accepted content

The canonical Game-side consumer/enforcement semantics for `PROD-ENTITLEMENTS-01` are the exact migrated contract content retained at:

- repository: `Oteryn/Oteryn-Game`;
- accepted merge: `d40a225e5fedca0396f34b4f2b6c1e343161e6ff`;
- historical migrated path: `docs/architecture/PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md`;
- exact accepted contract blob: `1cb0ab9f1c774746831d1676da415ad39c9cb399`;
- target PR: #20;
- target Issue: #19;
- source provenance: `blakinio/Oteryn-v2#115` / source PR #317;
- exact producer baseline: `Oteryn/Oteryn-Platform@afaa6d1d8340e44b1152b62d6d27e5fd1649804a`;
- producer contract: `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md` at that producer revision.

The migrated file keeps its historical `CANDIDATE` header and filename intentionally so the target does not rewrite source-side provenance after acceptance. This acceptance record is the target-side lifecycle authority that promotes that exact immutable content to accepted architecture.

## Accepted authority boundary

- Platform remains commercial entitlement lifecycle/truth authority.
- Game remains authoritative gameplay enforcement/mutation/result authority.
- Profile-B authority is finite and producer-grounded; stale use is bounded and explicit.
- Lifecycle/authority high-water fencing and same-ordered-revision equivocation fail closed.
- Producer-consumption progress cannot outrun the durable consumer fence.
- Trusted-time uncertainty only narrows entitlement authority; reconnect/session continuity is never entitlement grace.
- Gameplay-mutating delivery uses stable operation identity, idempotency and reconciliation rather than blind replacement retries.
- Mixed-version rollout and rollback require explicit compatibility and may not weaken the authority fence.

## Explicitly not authorized

Acceptance of this paper contract does **not** authorize:

- runtime/server/client implementation;
- persistence schema or migration;
- transport/IDL/broker selection;
- cryptographic/service-auth implementation;
- Premium/VIP or other product activation;
- numeric lease/refresh/skew values;
- payment/provider operations;
- production/protected-environment rollout;
- live account/session/game-state mutation.

Those remain separate evidence-backed tasks and gates.

## Evidence

Exact target head reviewed before merge: `0dfa0c5cdcd811c63d6926da166550712dfb59fc`.

- Agent governance: PASS (`32306743487` after Ready-state metadata repair; earlier exact-head generation `32306130413` also PASS).
- Architecture semantic audit: PASS (`32306433375`; earlier `32306130260` PASS).
- Merge gate: PASS (`32306743528`; earlier `32306130246` PASS).
- Merge authority audit: PASS (`32306130278`).
- Independent exact-head review: PASS, review `4977102554`, zero unresolved HIGH/CRITICAL/material findings.
- Unresolved review threads before merge: 0.
- Squash merge: `d40a225e5fedca0396f34b4f2b6c1e343161e6ff`.
