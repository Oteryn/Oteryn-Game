# Oteryn v2 Global Architecture Decision Register

- Status: **Active coordination register**
- Date: 2026-08-17
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Current execution status: `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`
- Detailed gameplay/product horizon: `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`
- Stable foundation backlog: `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`

## 1. Purpose and precedence

This register preserves stable architecture gate IDs, current decision state and the boundary between accepted architecture and later implementation/proof work.

```text
DecisionStatus != DeliveryStatus != ImplementationStatus
```

Accepted ADRs/contracts/owner baselines are semantic authority. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` is current execution-status authority. Historical proposal/candidate/backlog prose remains history when superseded by later owner acceptance.

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
| `SIM-DETERMINISM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
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

First-wave acceptance is canonical through PR #309 / `bf2a2ae279516f62626a5d8f4dc1aeb587535c62`; its terminal task archive is consolidated into #314. Stage-C acceptance is canonical through PR #311 / `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c`; a fresh separate-session independent review reported zero material findings on exact head `c5d9f839abd8998d42f4f37b203882f03bb51ce0`; lifecycle/status closeout is PR #318 / `a6a5180d98cf7791e40d9e1d08b25a5c8b4eff96`.

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

No architecture acceptance or prompt release silently promotes parity.

## 5. Registered product/alpha horizon

Stable IDs remain registered even when they do not block the first technical slice:

- `VSL-01`, `VSL-MOVE-01`, `VSL-COMBAT-01`, `VSL-CONTENT-01`, `VSL-02-NATIVE-CLIENT`;
- `ALPHA-RULESET-01`, `ALPHA-CONTENT-01`, `ALPHA-CLIENT-01`, `ALPHA-GM-01`, `ALPHA-QUALITY-01`, `ALPHA-OPS-01`;
- `LIVE-OPS-01`, `ALPHA-COMPAT-01`, `ALPHA-PRIVACY-01`, `ALPHA-CLIENT-SEC-01`, `GM-01`;
- `PERF-01`, `OPS-CHANNEL-01`, `ANL-02`, `ANL-03`;
- `PROD-ENTITLEMENTS-01` — `PROPOSED / PLANNED / NOT_STARTED`; Oteryn-v2 consumer/enforcement not accepted.

Detailed expansion/deferred horizon remains in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`, including `GAME-META-01`, `GAME-INSTANCES-01`, `GAME-WORLD-LIFECYCLE-01`, `INTEGRATION-API-01`, `MOD-ECOSYSTEM-01`, `EXP-EVENTS-01`, `EXP-HOUSES-01`, `EXP-SOCIAL-01`, `EXP-ECONOMY-01`, `EXP-SECURITY-01`, `EXP-UPDATE-01`, `EXP-OPS-01`, `EXP-OBS-01` and `EXP-SCALE-01`.

Registration prevents omission; it does not authorize implementation.

## 6. Progressive execution policy

1. Do not re-open accepted architecture without named superseding evidence.
2. Proposal/candidate delivery is not owner acceptance.
3. Implementation may not choose unresolved authority, idempotency, durability, public protocol or persistent-value semantics.
4. Reversible technology/library choices stay deferred where architecture intentionally leaves them open.
5. Resource values come from accepted registries/PERF/OPS evidence, not arbitrary constants.
6. Reference parity remains an evidence claim.
7. Permanent World Project/Bundle encoding still requires the DUR-04 format spike and later owner decision.
8. High-risk protocol/session/admission/persistence/item/loot/value/multichannel/fencing changes require genuinely independent exact-head review.
9. Entitlement implementation remains excluded until `PROD-ENTITLEMENTS-01` is separately accepted.

## 7. Released implementation handoff

PR #314 merged as `88f4fb754b5ae11243afd38a9e0b6a8e3b0a5815`; the formally evaluated implementation coordinator programme is released. The merge itself started no worker.

```text
EXECUTOR_PROGRAMME: RELEASED
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
IMPLEMENTATION_WORKERS_STARTED: NO
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
```

After explicit coordinator invocation, the live coordinator may create bounded implementation allocations in `Oteryn/Oteryn-Game` under repository governance. Direct workers remain read-only until allocated. Production/protected-environment/live data/session/account, Platform, external-repository, entitlement, Reference-parity and owner-funded-AI authority remain separately governed.

`PRODUCTION_AUTHORITY: NONE`
