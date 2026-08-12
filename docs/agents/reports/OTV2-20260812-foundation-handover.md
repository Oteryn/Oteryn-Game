# Oteryn v2 Foundation Programme — Successor Handover

- Handover ID: `OTV2-20260812-foundation-handover`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refreshed: 2026-08-12 13:35 +02:00
- Repository: `blakinio/Oteryn-v2`
- Trusted refresh base: `main@5c502d24557621efc798def87b68f137ba23fad8`
- GAME-ITEM delivery: PR #205 / exact final head `53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a` / squash merge `5c502d24557621efc798def87b68f137ba23fad8`
- GAME-ITEM lifecycle closeout: PR #206
- Terminal successor result after closeout: **`ROTATE`**
- Runtime implementation authority: **NOT GRANTED**
- PostgreSQL DDL/migration execution authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This is the durable successor handoff for Oteryn-v2 architecture continuation. It replaces chat history. The successor must read trusted-base governance and live GitHub state first, verify drift/ownership, then execute the one `next_action` recorded below.

The refresh in PR #206 is lifecycle/status reconciliation only. It does not create new GAME-ITEM semantics and becomes canonical when that closeout merges.

## 2. Canonical current state after this closeout

```text
GAME-VISION-01        ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-CHAR-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-02                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
ANL-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-ITEM-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-03                PROPOSED / PLANNED / NOT_STARTED
PROD-ENTITLEMENTS-01  PROPOSED / PLANNED / NOT_STARTED
```

The canonical detailed status is `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

Architecture acceptance does **not** imply runtime implementation, PostgreSQL DDL/migrations, gameplay traffic or production readiness.

## 3. Accepted/lifecycle-closed architecture — consume rather than rediscover

The successor must consume these accepted boundaries unless a later explicit superseding decision exists:

- one native Rust client/server stack and one project-owned `protocol-oteryn`;
- `protocol-canary` reference-only and excluded from production runtime/fallback/translation;
- ADR-0009 one-process GameNode identity, multichannel-first single-writer ownership and evidence-driven internal decomposition;
- Platform Identity/Gateway versus native-game authority separation;
- PostgreSQL native-game target with separate Platform/game ownership;
- FND-ID-01, FND-02, FND-03 and FND-04 identity/protocol/runtime/admission/fencing/recovery semantics;
- DUR-01 durable identifier representation, including `ItemInstanceId` UUIDv7 identity;
- ANL-01 event/audit identity, durability, privacy and read-only Game Intelligence boundaries;
- GAME-VISION minimum product direction and the immutable first Reference target: Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary;
- GAME-CHAR Stage A + Stage B ownership/lifecycle/progression semantic envelope and fail-closed per-behavior parity discipline;
- DUR-02 profile-neutral Character persistence sub-baseline and whole Persistence-v1 migration/transaction/outbox/durable-ack/PITR/schema-evolution architecture;
- GAME-ITEM-01 typed item semantic envelope from PR #205.

Do not restart FND-01, VSL-02, GAME-CHAR, DUR-02 or GAME-ITEM architecture merely because older backlog prose still describes their pre-acceptance state.

## 4. Accepted GAME-ITEM-01 — binding result

Canonical sources:

- `docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_ANALYSIS.md`;
- `docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`.

Delivery evidence:

- PR #205;
- frozen final head `53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a`;
- implementing-agent self-review `4915880173` — PASS, zero material findings;
- independent Codex exact-head no-suggestion review request `5266011485`, PR reaction evidence `450215687`;
- Agent Governance `31591336315` — PASS;
- Dependency Review `31591336312` — PASS;
- CodeQL `31591336340` — PASS;
- squash merge `5c502d24557621efc798def87b68f137ba23fad8`.

Binding semantic closure:

```text
ItemType
-> stable namespaced immutable/versioned authored semantic definition

ItemInstance
-> concrete mutable lifecycle using DUR-01 ItemInstanceId

StaticItemPlacement
-> authored world/content placement; not automatically durable ItemInstance
```

Accepted rules also include:

- typed bounded stack/charge/durability/temporal/equipment/container/binding/upgrade-modifier capability state;
- no arbitrary authoritative JSON/EAV/free-form script state;
- server-authoritative atomic multi-slot/exclusion equipment occupancy;
- bounded acyclic container legality and mandatory absolute resource/security ceilings before implementation acceptance;
- deterministic item modifier contribution ordering requirement without taking SIM arithmetic ownership;
- explicit definition revision compatibility/migration, no silent reinterpretation;
- world scope, binding, current location, authorization and display ownership remain distinct;
- exact target-sensitive Reference mechanics that are not evidenced remain `PARITY_PENDING_EVIDENCE` and fail closed;
- `PROD-ENTITLEMENTS-01` remains separately unaccepted; GAME-ITEM does not activate entitlement consumption.

## 5. DUR-03 ownership — next gate, not yet accepted

GAME-ITEM does **not** own the transaction/conservation mechanics that make durable item/value mutation safe.

`DUR-03` must own and freeze at architecture level:

- create/destroy/split/merge/transform ItemInstanceId transition rules under DUR-01;
- one authoritative item location and atomic old/new location transitions across inventory/equipment/container/ground and later transfer surfaces;
- idempotency and duplicate-command outcomes;
- stale session/writer rejection;
- retry, crash and partial-failure behavior;
- item/currency/value conservation and provenance;
- atomic durable evidence required to independently reconcile anti-duplication invariants through ANL/Game Intelligence;
- pickup/drop/loot/trade/market/mail/depot/reward/bank/currency transfer semantics without absorbing their separate product/domain policy owners.

`DUR-03` is currently only:

```text
DecisionStatus       = PROPOSED
DeliveryStatus       = PLANNED
ImplementationStatus = NOT_STARTED
runtime authority    = NONE
```

A successor may open one bounded **paper-only** DUR-03 architecture task. That does not authorize runtime item mutation or SQL DDL.

## 6. Binding whole-DUR-02 rules — do not redesign generic persistence

Consume `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md`:

1. one ordered game-owned migration history for current native-game DB boundary, immutable explicit migration artifacts, dedicated least-privilege migrator, no production runtime schema auto-sync;
2. `READ COMMITTED` only with explicit anomaly-closing proof, otherwise bounded `SERIALIZABLE` or stricter accepted mechanism, preserving semantic operation identity across retries;
3. one ANL-compatible durable journal plus crash-safe publication state, atomically committed where owning mutation requires durable evidence;
4. acknowledged durable success means committed recoverable state; FND-03 runtime checkpoint and disaster RPO are separate;
5. PITR-capable, restore-tested fail-closed recovery with non-rollback authority fencing;
6. `EXPAND -> MIGRATE/BACKFILL -> VALIDATE -> CUT OVER -> CONTRACT` schema evolution.

DUR-03 may specialize transaction correctness for item/value conservation but may not silently weaken these common rules.

## 7. Product/domain ownership preserved

- `EXP-ECONOMY-01` — market/economy policy;
- `EXP-SOCIAL-01` — guild/social policy;
- `EXP-HOUSES-01` — houses;
- `GAME-META-01` — recurring/meta rewards;
- `EXP-EVENTS-01` — encounter/event rewards;
- `ANL-01` — event/audit semantic authority;
- `SIM-DETERMINISM-01` + gameplay/ruleset owners — exact deterministic arithmetic/rounding;
- `DUR-04` — concrete content/world schemas, compiler/bundle and scripting runtime;
- `PROD-ENTITLEMENTS-01` — Oteryn-v2 entitlement consumer/enforcement contract, still unaccepted.

DUR-03 owns conservation where these domains move item/currency/value; it does not absorb their business policy.

## 8. Implementation boundary

A future owner may separately authorize a bounded server/persistence foundation implementation programme for already accepted common scopes. A safe decomposition hypothesis remains:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content slices
```

**This handoff grants no implementation authority.** Do not create Rust server runtime, DDL, migration files, item transaction runtime, listeners or production configuration merely because architecture prerequisites exist.

## 9. Parallel safe architecture work

After GAME-ITEM closeout, one bounded DUR-03 paper-only task is the ordered item/value successor. Separate owners may independently continue:

- `GAME-CHANNEL-01`;
- Reference evidence/parity manifest/tooling;
- `DUR-04` minimum headless content path;
- `SIM-DETERMINISM-01`;
- existing lag/disconnect architecture discussions.

Do not absorb their paths or semantics into DUR-03.

## 10. Repository, production and external-source authority

Routine writes remain limited to `blakinio/Oteryn-v2` unless the owner explicitly grants another exact write task.

Without such authority, `blakinio/Oteryn-Platform`, `blakinio/Otheryn`, `blakinio/otclient`, Canary repositories and other external sources are read-only evidence inputs.

No production deployment, protected environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying is authorized.

## 11. Successor bootstrap

Before mutation, the successor must read and follow at minimum:

1. root `AGENTS.md` and `AGENTS.override.md`;
2. `docs/agents/AGENTS.md`;
3. `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
4. `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
5. `docs/agents/PROMPTING_HANDOVER.md` and `CONTEXT_HANDOFF.md`;
6. `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`;
7. this handoff report;
8. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
9. accepted GAME-ITEM, DUR-01, DUR-02 and ANL-01 contracts for DUR-03;
10. live main, open PRs, active tasks/owned paths, exact reviews and CI.

Live merged repository state overrides this report if state has legitimately advanced.

## 12. Context checkpoint

```yaml
status: ready
terminal_invocation_result: ROTATE
repository: blakinio/Oteryn-v2
trusted_base_sha: 5c502d24557621efc798def87b68f137ba23fad8
closeout_pr: 206
owned_paths: []
public_contracts: []
last_progress: GAME-ITEM-01 delivery PR #205 passed exact-head self-review, independent no-suggestion Codex review and all required CI and was squash-merged as 5c502d24557621efc798def87b68f137ba23fad8; lifecycle closeout #206 reconciles canonical status/handoff and releases GAME-ITEM ownership.
validation_state: PR #205 delivery PASS; closeout #206 must pass its own exact-head review/CI before this refreshed handoff becomes canonical
e2e_state: NOT_APPLICABLE documentation-only architecture/closeout
blocker: null
owner_action_required: false
next_action: From live main after GAME-ITEM lifecycle closeout, create one bounded paper-only `DUR-03` architecture task consuming accepted GAME-ITEM, DUR-01, DUR-02 and ANL-01; preserve `DUR-03 = PROPOSED / PLANNED / NOT_STARTED` until its own acceptance and do not implement runtime/DDL/production behavior.
```
