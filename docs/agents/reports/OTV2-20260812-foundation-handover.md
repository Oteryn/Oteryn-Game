# Oteryn v2 Foundation Programme — Successor Handover

- Handover ID: `OTV2-20260812-foundation-handover`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refreshed: 2026-08-12 17:26 +02:00
- Repository: `blakinio/Oteryn-v2`
- Trusted refresh base: `main@54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`
- GAME-CHANNEL delivery: PR #209 / exact final head `ca1112191ede7d316c874189f3053ad7f8247579` / squash merge `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`
- GAME-CHANNEL lifecycle closeout: PR #210
- Terminal successor result after closeout: **`ROTATE`**
- Runtime/client implementation authority: **NOT GRANTED**
- PostgreSQL DDL/migration execution authority: **NONE**
- Platform write authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This is the durable successor handoff for Oteryn-v2 architecture continuation. It replaces chat history. A successor must read trusted-base governance and live GitHub state first, verify drift/ownership, then execute the one `next_action` below.

PR #210 is lifecycle/status/handoff reconciliation only. It does not create or change GAME-CHANNEL semantics and becomes canonical only when the closeout merges.

## 2. Canonical current state after GAME-CHANNEL closeout

```text
GAME-VISION-01        ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-CHANNEL-01       ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
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

Architecture acceptance does **not** imply runtime/client implementation, PostgreSQL DDL/migrations, Platform implementation, gameplay traffic or production readiness.

## 3. Accepted architecture — consume rather than rediscover

Unless a later explicit superseding decision exists, preserve:

- one native Rust client/server stack and one project-owned `protocol-oteryn`;
- `protocol-canary` reference-only and excluded from production runtime/fallback/translation;
- one logical World may contain multiple Channels while WorldId/ChannelId/InstanceId/NodeId/GameSessionId remain distinct;
- ADR-0009 one-process GameNode identity, one logical mutation owner per Channel/Instance and same-semantic-Channel recovery;
- Platform Identity/Gateway/World Registry versus native-game authority separation;
- PostgreSQL native-game target with separate Platform/game ownership;
- FND-ID-01/FND-02/FND-03/FND-04 typed identity/protocol/runtime/session/admission/fencing/recovery semantics;
- DUR-01 durable representation including non-reused UUIDv7 ItemInstanceId;
- DUR-02 Persistence-v1 migration/transaction/outbox/durable-ack/PITR/schema-evolution architecture;
- ANL-01 event/audit durability/privacy/read-only Game Intelligence boundary;
- GAME-VISION minimum product direction and immutable first Reference target after 2026-07-28 Global Tibia server-save/maintenance;
- GAME-CHAR Stage A/B semantic closure and fail-closed per-behavior parity discipline;
- GAME-ITEM typed item semantic envelope;
- DUR-03 durable item/currency/value conservation/idempotency/anti-duplication envelope;
- GAME-CHANNEL player Channel selection/queue/co-location/anti-hopping/multiplicity/lifecycle product policy.

Do not restart these gates merely because older backlog or predecision prose describes pre-acceptance state.

## 4. Accepted GAME-CHANNEL-01 — binding result

Canonical sources:

- `docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_ANALYSIS.md`;
- `docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`.

Delivery evidence:

- PR #209;
- frozen final head `ca1112191ede7d316c874189f3053ad7f8247579`;
- implementing-agent exact-head self-review `4918161329` — PASS, material findings `0`;
- fresh independent Codex exact-head review request `5268790260` — no suggestions; PR 👍 reaction `450588928`;
- Agent Governance `31611424137` — PASS;
- Dependency Review `31611424147` — PASS;
- CodeQL `31611424261` — PASS;
- unresolved material review threads before merge: `0`;
- repair budget `3/3`, final head clean;
- squash merge `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`.

### Channel identity and entry

- `WorldId` remains one product/economy/community/profile boundary;
- canonical `ChannelRef=WorldId+ChannelId` identifies one public simulation;
- display labels/ordinals never become durable admission/queue/reward/lifecycle identity;
- current eligible directory may provide one non-authoritative recommendation plus explicit eligible player target;
- FND-04 remains final admission authority;
- stale/full/draining/recovering/unavailable/incompatible target fails closed and requires a fresh offer/grant; no silent retarget.

### Queue and co-location

- optional queue is bounded target-Channel **pre-admission** control state, never GameSession/CharacterLease/runtime/value authority;
- a long queue does not retain stale short-lived admission credentials;
- first generation does not reserve another Channel while the Character remains authoritative in source gameplay;
- party/friend co-location is a privacy-bounded recommendation/target hint;
- every Character admits independently; no party-owned Channel or atomic group admission.

### Channel switch and anti-hopping

- same-Channel reconnect/recovery is not a Channel switch;
- completed Channel switch uses the accepted safe source exit + fresh destination authorization/admission + fresh GameSessionId flow;
- combat/PvP, trade, unresolved DUR-03 value mutation, protected event/encounter, unsafe instance/house and stale authority remain switch blockers;
- GAME-CHANNEL owns durable World channel-policy state scoped `CharacterId+WorldId`, not automatically GAME-CHAR progression state;
- state retains enough prior successful Channel semantics to classify first admission, same-Channel fresh login and different-Channel switch after logout/relog/restart;
- first anti-hopping mechanism is time-based cooldown + hard blockers, but **no numeric duration is accepted yet**; it remains an implementation activation blocker pending evidence/owner decision;
- different-Channel destination playable authority and remembered Channel/guard advancement are one authoritative/recovery-safe semantic outcome;
- no mandatory new `ChannelSwitchId` was introduced;
- maintenance/incident exceptions must be typed, audited, client-unforgeable and consequence-preserving.

### Source/reward multiplicity

Channel-local runtime placement does not decide durable output multiplicity.

Every Channel-sensitive value-producing source/encounter family must explicitly compile/validate one class:

```text
CHANNEL_LOCAL_REPEATABLE
CHANNEL_LOCAL_SHARED_ELIGIBILITY
WORLD_SCOPED_UNIQUE
EXPLICIT_EVENT_POLICY_REQUIRED
```

Missing classification fails closed. Simulation scope and durable eligibility scope remain distinct. ChannelId cannot silently enter Character/Account/World reward reset/idempotency keys. No hidden inverse spawn/loot tuning based on active Channel count is accepted.

DUR-03 remains authoritative for actual value delivery conservation/idempotency.

### Dynamic public Channel lifecycle

GAME-CHANNEL owns qualitative product predicates; PERF/OPS retain all numeric capacity/service-objective windows, thresholds, headroom, hysteresis/timers and implementation/orchestration.

Accepted trigger vocabulary:

```text
DEMAND_PRESSURE
RECOVERY_PRESSURE
LOW_LOAD_CONSOLIDATION_CANDIDATE
CHANNEL_UNHEALTHY
```

New semantic public Channel capacity is legitimate only from sustained eligible demand or bounded recovery-capacity need, with insufficient current healthy eligible capacity/headroom, compatible World/event/reward/multiplicity policy, safe infrastructure and Ready-before-Selectable.

The following alone cannot create public Channel capacity:

- one party/friend group wanting a private copy;
- preference for an emptier Channel;
- rare-spawn/loot/resource farming;
- PvP avoidance;
- operator preference without demand/recovery/product justification.

`RECOVERY_PRESSURE` never allows affected Channel A actors to continue silently in new Channel B; same semantic Channel A preserves ChannelId when recoverable.

Low-load drain requires sustained low load, sufficient retained healthy capacity/headroom, no World/event/reward/recovery policy need, stopped incoming admissions/switches and safe incumbent exit.

Drain aborts/holds if demand returns, retained capacity degrades, recovery pressure appears, policy requires the Channel or continued drain threatens session/lease/item/value/encounter/checkpoint/recovery correctness.

Terminal semantic retirement requires zero authoritative sessions/placements, no dependent instance/event/transaction/checkpoint/recovery obligations, durable evidence finalization, fenced old owner/generation, invalidated stale routing/queue/admission references and explicit retirement rather than temporary inactivity. Temporary stop/reactivation preserves ChannelId; retired ChannelId is never reused.

### PvP/community/recovery

- direct PvP execution remains current Channel/Instance-local;
- World/Character-scoped PvP consequences survive Channel/GameSession transitions;
- one World remains one guild/economy/ranking/community boundary;
- local speech/combat/position remains Channel-local;
- failure recovery is same-ChannelId first;
- alternate Channel only after actor reaches a proven safe state and then obtains fresh admission;
- failure never clears combat/reward/value consequences.

### Policy/versioning

GAME-CHANNEL consumes existing `world_policy_revision`; it does not invent a new protocol major solely for Channel policy. Changing WorldId is not a Channel switch and cannot bypass world-scoped value/profile/Character lifecycle rules.

## 5. Historical GAME-CHANNEL predecision sources

`GAME-CHANNEL-01_PREDECISION_ANALYSIS.md` and `GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md` remain historical framework evidence.

The accepted contract consumes their qualitative lifecycle ideas only where explicitly restated in the final contract. Historical numeric utilization thresholds, player counts, time windows, headroom percentages, queue objectives, cooldown values and technology assumptions are **not accepted** by GAME-CHANNEL.

## 6. Implementation boundary

A future owner may separately authorize bounded implementation using accepted architecture. A safe server/persistence decomposition hypothesis remains:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content slices
```

Channel product implementation additionally needs concrete PERF/OPS numeric/orchestration decisions and any separately authorized Platform/Gateway/World Registry changes. Item/value implementation additionally consumes GAME-ITEM/DUR-03 and concrete audit/resource-limit evidence.

**This handoff grants no implementation authority.** Do not create Rust server/Channel/item/content runtime, DDL/migration files, Platform implementation, live Channel/value mutation or production configuration merely because architecture gates are accepted.

## 7. Next ordered paper-only architecture action

After GAME-CHANNEL closeout, remaining pre-VSL paper-only architecture includes independent:

- Reference evidence/parity tooling;
- `DUR-04` minimum headless content path;
- `SIM-DETERMINISM-01`.

To keep ownership singular, the successor action is:

```text
DUR-04 — Content, World Detail and Scripting Contract
```

A bounded DUR-04 architecture task should consume accepted ADR-0005 plus current identity/runtime/durability/security boundaries and freeze the **minimum headless content path** needed before VSL/content implementation:

```text
source schema
-> validator
-> deterministic compiler
-> versioned runtime World Bundle
-> bounded loader
```

It should also define, only to the extent required now:

- stable Content Registry package/version/dependency semantics;
- source-project versus compiled-bundle identity/revision/compatibility;
- deterministic build fingerprints and reproducibility;
- corruption/truncation/decompression/path/nesting/resource failure behavior;
- schema/bundle evolution and explicit migration policy;
- source/import provenance and migration classification boundaries;
- scripting capability model, determinism/context, resource sandbox, persistence/API authority and hot-reload/cutover semantics;
- asset/content provenance/rights boundary;
- minimum fixtures/evidence required before encoding/loader claims.

Do **not** build full Oteryn Studio, broad legacy importers, runtime gameplay/content implementation, SQL schemas, production content rollout or copy proprietary assets in that paper-only gate.

`SIM-DETERMINISM-01` and Reference evidence/parity tooling may continue independently under separate path ownership; selecting DUR-04 as `next_action` does not accept them.

## 8. Repository/production/external authority

Routine writes remain limited to `blakinio/Oteryn-v2` unless the owner explicitly grants another exact task.

`blakinio/Oteryn-Platform`, `blakinio/Otheryn`, `blakinio/otclient`, Canary repositories and other external sources remain read-only evidence inputs unless separately authorized.

No production deployment, protected environment approval, secrets, live account/session/data/database mutation, entitlement activation or proprietary asset copying is authorized.

## 9. Successor bootstrap

Before mutation, the successor must read and follow at minimum:

1. root `AGENTS.md` and `AGENTS.override.md`;
2. `docs/agents/AGENTS.md`;
3. `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
4. `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
5. `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md`;
6. `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`;
7. this handoff report;
8. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
9. ADR-0005 plus accepted identity/runtime/durability/item/channel/security sources relevant to DUR-04;
10. live main, open PRs, active tasks/owned paths, reviews and CI.

Live merged repository state overrides this report if state has legitimately advanced.

## 10. Context checkpoint

```yaml
status: ready
terminal_invocation_result: ROTATE
repository: blakinio/Oteryn-v2
trusted_base_sha: 54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1
closeout_pr: 210
owned_paths: []
public_contracts: []
last_progress: GAME-CHANNEL-01 delivery PR #209 passed terminal exact-head self-review, fresh independent no-suggestion Codex review and Agent Governance/Dependency Review/CodeQL after repair budget 3/3, then squash-merged unchanged as 54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1; lifecycle closeout #210 promotes canonical status and releases GAME-CHANNEL ownership.
validation_state: PR #209 delivery PASS; closeout #210 must pass its own exact-head documentation/governance validation before this refreshed handoff becomes canonical.
e2e_state: NOT_APPLICABLE documentation-only architecture/closeout
blocker: null
owner_action_required: false
next_action: From live main after GAME-CHANNEL lifecycle closeout, create one bounded paper-only `DUR-04` architecture task for the minimum headless content/world/scripting contract; do not implement runtime/Studio/DDL/production behavior.
```
