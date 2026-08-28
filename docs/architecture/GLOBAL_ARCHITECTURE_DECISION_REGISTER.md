# Oteryn v2 Global Architecture Decision Register

- Status: **Active coordination register**
- Date: 2026-08-22
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Current execution status: `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`
- Live implementation allocation: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`
- Detailed gameplay/product horizon: `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`
- Stable foundation backlog: `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`

## 1. Purpose and precedence

This register preserves stable architecture gate IDs, current decision state and the boundary between accepted architecture and implementation/proof work.

```text
DecisionStatus != DeliveryStatus != ImplementationStatus
```

Accepted ADRs/contracts/owner baselines are semantic authority. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` is current execution-status authority. Live implementation allocation + exact worker task/branch/PR/CI state own implementation execution truth. Historical proposal/candidate/backlog/checkpoint prose remains history when superseded by later accepted or current-status sources.

## 2. Accepted platform/foundation direction

Accepted named scope includes ADR-0001 through ADR-0016: native Rust client/server and one project `protocol-oteryn`; repository/client migration; Platform Identity/Gateway/final-game admission split; PostgreSQL game ownership; native world/content + Studio boundary; read-only analytics/audit; three-tier E2E; `protocol-canary` reference-only; GameNode one-writer runtime; Reference/Evolved profiles; fail-closed pre-native state; Character authority; Platform DB independence; TCP-default/future-QUIC; evidence-driven internal topology; and transport-mode vocabulary separated from runtime readiness.

## 3. Accepted foundation/durability/gameplay/client/analytics gates

| Gate | DecisionStatus | DeliveryStatus | ImplementationStatus |
|---|---|---|---|
| `FND-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` |
| `VSL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` |
| `FND-ID-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `FND-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `NET-TRANSPORT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `FND-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `FND-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-VISION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-CHANNEL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-CHAR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-ITEM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `SIM-DETERMINISM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED`* |
| `GAME-ABILITY-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-INTERACTION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ALPHA-CLIENT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-AI-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `QA-E2E-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED / EVIDENCE_REQUIRED` |
| `VSL-MOVE-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-COMBAT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-CONTENT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `PROD-ENTITLEMENTS-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |

\* The bounded executor-defined SIM core is implemented on merged `main` through PR #14 / `66619daf5837f31f7c54676e9f8351ed4ae220b0`. Whole-contract `PROVEN` remains withheld until named consumer/replay/VSL evidence exists.

First-wave architecture acceptance remains canonical through PR #309 / `bf2a2ae279516f62626a5d8f4dc1aeb587535c62`. Stage-C acceptance remains canonical through PR #311 / `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c` with lifecycle/status closeout through PR #318 / `a6a5180d98cf7791e40d9e1d08b25a5c8b4eff96`.

`PROD-ENTITLEMENTS-01` Game consumer/enforcement architecture was independently reviewed and accepted through PR #20 / merge `d40a225e5fedca0396f34b4f2b6c1e343161e6ff`; lifecycle closeout merged through PR #27 / `84f485089b97cfaba1b5c6628ed8e0ba6655dc51`. The retained `*_CANDIDATE.md` filename is historical provenance, not current DecisionStatus.

## 4. Reference evidence/parity registry

```yaml
target: Global Tibia after 2026-07-28 server-save boundary
registered_cases: 4
promoted_cases: 0
target_evidence: UNKNOWN
source_case_provenance: PENDING
legal_review: PENDING
oteryn_implementation: NOT_STARTED
parity: PARITY_PENDING_EVIDENCE
```

No architecture acceptance, implementation activity or fixture silently promotes parity.

## 5. Implementation execution state

Implementation coordinator execution has started and passed the first serial gates:

```text
BOOTSTRAP: COMPLETED
SIM: COMPLETED
WAVE1 EXACT-BASE BIND: MERGED (#46 / fd39c6aa026e82062a8b29af24811d467c115f19)
FOUNDATION: IMPLEMENTING (unmerged worker branch)
DOMAIN: IMPLEMENTING (unmerged worker branch)
CONTENT: IMPLEMENTING (unmerged worker branch)
QA: IMPLEMENTING (unmerged worker branch)
```

Unmerged worker code does not promote merged-main gate implementation status. Exact lane progress must be read from the live allocation plus worker branch/task/PR/CI state.

## 6. Registered product/alpha horizon

Stable IDs remain registered even when they do not block the first technical slice:

- `VSL-01`, `VSL-MOVE-01`, `VSL-COMBAT-01`, `VSL-CONTENT-01`, `VSL-02-NATIVE-CLIENT`;
- `ALPHA-RULESET-01`, `ALPHA-CONTENT-01`, `ALPHA-CLIENT-01`, `ALPHA-GM-01`, `ALPHA-QUALITY-01`, `ALPHA-OPS-01`;
- `LIVE-OPS-01`, `ALPHA-COMPAT-01`, `ALPHA-PRIVACY-01`, `ALPHA-CLIENT-SEC-01`, `GM-01`;
- `PERF-01`, `OPS-CHANNEL-01`, `ANL-02`, `ANL-03`;
- `PROD-ENTITLEMENTS-01` — architecture accepted/lifecycle-closed; runtime/product activation remains unimplemented and separately governed.

Detailed expansion/deferred horizon remains in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`, including `GAME-META-01`, `GAME-INSTANCES-01`, `GAME-WORLD-LIFECYCLE-01`, `INTEGRATION-API-01`, `MOD-ECOSYSTEM-01`, `EXP-EVENTS-01`, `EXP-HOUSES-01`, `EXP-SOCIAL-01`, `EXP-ECONOMY-01`, `EXP-SECURITY-01`, `EXP-UPDATE-01`, `EXP-OPS-01`, `EXP-OBS-01` and `EXP-SCALE-01`.

Registration prevents omission; it does not authorize implementation.

## 7. Progressive execution policy

1. Do not re-open accepted architecture without named superseding evidence.
2. Proposal/candidate delivery is not owner acceptance unless a later accepted baseline/merge explicitly says so.
3. Implementation may not choose unresolved authority, idempotency, durability, public protocol or persistent-value semantics.
4. Reversible technology/library choices stay deferred where architecture intentionally leaves them open.
5. Resource values come from accepted registries/PERF/OPS evidence, not arbitrary constants.
6. Reference parity remains an evidence claim.
7. Permanent World Project/Bundle encoding still requires the DUR-04 format spike and later owner decision.
8. High-risk protocol/session/admission/persistence/item/loot/value/multichannel/fencing changes require genuinely independent exact-head review.
9. Entitlement architecture acceptance does not authorize entitlement runtime activation, payments or production rollout.
10. Accepted FND-04 makes historical `2s/5s/15s` reconnect/liveness/grace timing values non-canonical/deferred; exact four-second defensive PvE re-entry protection remains accepted.

## 8. Released and active implementation handoff

The formally evaluated implementation coordinator programme is both released and active.

```text
EXECUTOR_PROGRAMME: RELEASED_AND_ACTIVE
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
IMPLEMENTATION_WORKERS_STARTED: YES
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
```

Production/protected-environment/live data/session/account, Platform, external-repository, entitlement activation, Reference-parity and owner-funded-AI authority remain separately governed.

`PRODUCTION_AUTHORITY: NONE`
