# Oteryn v2 Foundation Programme — Successor Handover

- Handover ID: `OTV2-20260812-foundation-handover`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refreshed: 2026-08-12 16:16 +02:00
- Repository: `blakinio/Oteryn-v2`
- Trusted refresh base: `main@63380bcba469027e90677aaf4db571fa941be2f4`
- DUR-03 delivery: PR #207 / exact final head `a1d949362e219373a5d314c0e9ddf8de110362dd` / squash merge `63380bcba469027e90677aaf4db571fa941be2f4`
- DUR-03 lifecycle closeout: PR #208
- Terminal successor result after closeout: **`ROTATE`**
- Runtime implementation authority: **NOT GRANTED**
- PostgreSQL DDL/migration execution authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This is the durable successor handoff for Oteryn-v2 architecture continuation. It replaces chat history. A successor must read trusted-base governance and live GitHub state first, verify drift/ownership, then execute the one `next_action` below.

PR #208 is lifecycle/status reconciliation only. It does not create new DUR-03 semantics and becomes canonical only when the closeout merges.

## 2. Canonical current state after DUR-03 closeout

```text
GAME-VISION-01        ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-CHANNEL-01       PROPOSED / PLANNED / NOT_STARTED
GAME-CHAR-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-ITEM-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-02                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-03                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-04                PROPOSED / PLANNED / NOT_STARTED
ANL-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
SIM-DETERMINISM-01    PROPOSED / PLANNED / NOT_STARTED
PROD-ENTITLEMENTS-01  PROPOSED / PLANNED / NOT_STARTED
```

The canonical detailed status is `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

Architecture acceptance does **not** imply runtime implementation, PostgreSQL DDL/migrations, gameplay traffic or production readiness.

## 3. Accepted/lifecycle-closed architecture — consume rather than rediscover

Unless a later explicit superseding decision exists, preserve:

- one native Rust client/server stack and one project-owned `protocol-oteryn`;
- `protocol-canary` reference-only and excluded from production runtime/fallback/translation;
- ADR-0009 one-process GameNode identity, multichannel-first single-writer ownership and evidence-driven internal decomposition;
- Platform Identity/Gateway versus native-game authority separation;
- PostgreSQL native-game target with separate Platform/game ownership;
- FND-ID-01, FND-02, FND-03 and FND-04 identity/protocol/runtime/admission/fencing/recovery semantics;
- DUR-01 durable identifier representation including non-reused UUIDv7 `ItemInstanceId`;
- DUR-02 Persistence-v1 migration/transaction/outbox/durable-ack/PITR/schema-evolution architecture;
- ANL-01 event/audit identity, durability, privacy and read-only Game Intelligence boundaries;
- GAME-VISION minimum product direction and immutable first Reference target: Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary;
- GAME-CHAR Stage A + Stage B semantic envelope and fail-closed per-behavior parity discipline;
- GAME-ITEM typed item semantic envelope;
- DUR-03 item/currency/value transaction, conservation and anti-duplication envelope.

Do not restart accepted FND/DUR/GAME-CHAR/GAME-ITEM architecture merely because older backlog prose describes pre-acceptance state.

## 4. Accepted DUR-03 — binding result

Canonical sources:

- `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_ANALYSIS.md`;
- `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`.

Delivery evidence:

- PR #207;
- frozen final head `a1d949362e219373a5d314c0e9ddf8de110362dd`;
- implementing-agent self-review `4916797999` — PASS, material findings `0`;
- independent Codex exact-head no-suggestion review request `5267211845`, PR reaction `450358534`;
- Agent Governance `31599369738` — PASS;
- Dependency Review `31599369737` — PASS;
- CodeQL `31599369780` — PASS;
- unresolved material review threads before merge: `0`;
- squash merge `63380bcba469027e90677aaf4db571fa941be2f4`.

Binding semantic closure includes:

### One durable semantic location

Every live durable `ItemInstance` has exactly one typed immediate semantic location. Runtime ground/checkpoint projection and durable recovery state do not become peer authorities; they represent/reconstruct the same semantic item location under their accepted runtime/durability owners.

### Runtime ↔ durable value handoff

For pickup/drop and other mixed runtime-ground/instance versus durable Character/value operations:

```text
current runtime owner reserves under current ownership generation
-> bounded asynchronous persistence request
-> one game-owned PostgreSQL transaction is durable value linearization point
-> completion returns as a normalized runtime input
-> current valid owner reconciles projection
```

The runtime writer does not block synchronously on DB/remote work. A stale ownership generation cannot commit or apply completion. A committed durable pickup/drop survives GameNode crash and stale checkpoint/ground ghosts cannot authorize a second mutation.

### Identity transitions

- same concrete item lifecycle preserves `ItemInstanceId`;
- every newly independently locatable concrete lifecycle gets a fresh transaction-scoped `ItemInstanceId`;
- split retains source ID and gives the new stack a fresh output ID;
- merge/quantity transfer retains receiver ID and retires a source that reaches zero;
- planned output IDs remain stable across physical retry and are never reassigned to another logical transaction;
- type-changing transform explicitly selects internal Oteryn `PRESERVE_INSTANCE` or `REPLACE_INSTANCE` policy;
- external Reference behavior does not expose Oteryn UUID identity, so Oteryn lifecycle identity policy is not guessed from Global behavior.

### Conservation and provenance

Every authoritative value mutation is explicitly classified as one of:

```text
TRANSFER
SPLIT_MERGE_QUANTITY
STATE_MUTATION
MINT
BURN
TRANSFORM
CONVERSION
```

Conservation uses exact item/asset quantities plus complete input/output/source/sink lineage, never market-price equality. Non-item fungible value uses exact bounded arithmetic; binary floating point is not authoritative conservation arithmetic.

### Retry/idempotency

- FND-02 `CommandRef=(GameSessionId,CommandId)` remains player-command ingress/order identity;
- every logical atomic durable value mutation has one ANL `TransactionId`;
- `OperationId` is used where a workflow spans multiple durable transactions or durable asynchronous continuation;
- a proven non-commit may reread/rematerialize the same logical intent under the same TransactionId and planned output identities;
- an ambiguous commit freezes the exact materialized candidate until durable classification;
- lost response never justifies a guessed second transaction;
- durable receipts/reconciliation preserve no-double-effect safety where CommandRef/current state alone is insufficient.

### Authority and custody

Transactions consume current accepted GameSession/CharacterLease/runtime ownership fences as applicable. `ItemInstanceId`, binding, location, `NodeId` and old transport generation are not credentials.

Multi-transaction workflows use explicit typed custody, stable OperationId where needed, independently conservation-safe steps and new compensating transactions rather than history rewrite. Current v1 durable value linearization remains one game-owned `oteryn_game` PostgreSQL transaction; no Platform/game distributed 2PC or mirrored dual value authority is accepted.

### Evidence and recovery

Security/value mutation classes requiring durable audit use bounded ANL-compatible evidence. Concrete event IDs/payloads and numeric DUR-03 resource ceilings remain required pre-implementation work rather than invented architecture values.

Restore/integrity recovery fails closed until item identities, exactly-one-location, container/custody graph, receipts/source-cause uniqueness, retained mandatory audit sets, non-item asset invariants and newer authority fences reconcile. Game Intelligence may investigate but cannot mutate or auto-repair authoritative value.

## 5. Product/domain ownership preserved

DUR-03 owns transaction/conservation mechanics where value moves. It does not accept downstream business policy:

- `GAME-CHANNEL-01` — channel product/policy behavior;
- `EXP-ECONOMY-01` — market/economy policy;
- `EXP-SOCIAL-01` — guild/social policy;
- `EXP-HOUSES-01` — houses;
- `GAME-META-01` — recurring/meta rewards;
- `EXP-EVENTS-01` — encounter/event rewards;
- owning combat/content gates — loot generation/materialization timing;
- `SIM-DETERMINISM-01` + ruleset/gameplay gates — exact deterministic arithmetic/rounding;
- `DUR-04` — concrete content/world schemas, compiler/bundle and scripting runtime;
- `PROD-ENTITLEMENTS-01` — game entitlement consumer/enforcement, still unaccepted.

Direct cross-world gameplay-value transfer remains forbidden until a dedicated accepted transfer contract proves safety; burn in world A plus mint in world B cannot bypass that rule.

## 6. Implementation boundary

A future owner may separately authorize bounded implementation using already accepted architecture. A safe decomposition hypothesis remains:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content slices
```

**This handoff grants no implementation authority.** Do not create Rust server/item transaction runtime, DDL/migration files, listeners, live item/currency mutation or production configuration merely because the architecture gates are accepted.

## 7. Next ordered paper-only architecture gate

The owner-accepted programme refinement orders `GAME-VISION-01 minimum + GAME-CHANNEL-01` before multichannel becomes a product feature. GAME-VISION minimum is already accepted; GAME-CHANNEL remains `PROPOSED / PLANNED / NOT_STARTED` and no competing open GAME-CHANNEL PR was found during DUR-03 closeout.

Therefore the one successor action after PR #208 merges is:

```text
GAME-CHANNEL-01
```

A bounded GAME-CHANNEL contract should consume accepted multichannel/runtime/product/value boundaries and decide only the channel product/policy questions that block safe multichannel product behavior, including:

- channel creation/removal/capacity-policy semantics without freezing deployment technology;
- player choice versus automatic assignment;
- party/friend co-location;
- queues and channel visibility;
- channel switching, cooldowns and anti-hopping;
- spawn/loot/resource multiplication policy;
- world-global boss/event/reward eligibility implications;
- PvP/channel implications;
- social/community fragmentation safeguards;
- failure/recovery rule preserving same-channel combat/spawn/loot state rather than silently moving a player to a different simulation.

It must not absorb `OPS-CHANNEL-01` deployment/orchestration, DUR-03 value conservation, social/economy business domains, exact Reference PvP formulas, Rust runtime implementation or production configuration.

## 8. Parallel safe paper-only work

Separate owners may independently continue:

- Reference evidence/parity manifest/tooling;
- `DUR-04` minimum headless content path;
- `SIM-DETERMINISM-01`;
- existing lag/disconnect architecture discussions.

Do not absorb their paths or semantics into GAME-CHANNEL.

## 9. Repository, production and external-source authority

Routine writes remain limited to `blakinio/Oteryn-v2` unless the owner explicitly grants another exact write task.

`blakinio/Oteryn-Platform`, `blakinio/Otheryn`, `blakinio/otclient`, Canary repositories and external sources remain read-only evidence inputs unless separately authorized.

No production deployment, protected environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying is authorized.

## 10. Successor bootstrap

Before mutation, the successor must read and follow at minimum:

1. root `AGENTS.md` and `AGENTS.override.md`;
2. `docs/agents/AGENTS.md`;
3. `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
4. `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
5. `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md`;
6. `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`;
7. this handoff report;
8. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
9. accepted ADR-0001/ADR-0009/GAME-VISION/FND-03/FND-04/DUR-03 and multichannel scope sources relevant to GAME-CHANNEL;
10. live main, open PRs, active tasks/owned paths, exact reviews and CI.

Live merged repository state overrides this report if state has legitimately advanced.

## 11. Context checkpoint

```yaml
status: ready
terminal_invocation_result: ROTATE
repository: blakinio/Oteryn-v2
trusted_base_sha: 63380bcba469027e90677aaf4db571fa941be2f4
closeout_pr: 208
owned_paths: []
public_contracts: []
last_progress: DUR-03 delivery PR #207 passed exact-head self-review, independent no-suggestion Codex review and Agent Governance/Dependency Review/CodeQL, then squash-merged unchanged as 63380bcba469027e90677aaf4db571fa941be2f4; lifecycle closeout #208 promotes canonical status and releases DUR-03 ownership.
validation_state: PR #207 delivery PASS; closeout #208 must pass its own exact-head documentation/governance validation before this refreshed handoff becomes canonical.
e2e_state: NOT_APPLICABLE documentation-only architecture/closeout
blocker: null
owner_action_required: false
next_action: From live main after DUR-03 lifecycle closeout, create one bounded paper-only `GAME-CHANNEL-01` architecture task consuming accepted multichannel/runtime/product/value boundaries; do not implement runtime/DDL/production behavior.
```
