# Oteryn v2 Foundation Programme — Successor Handover

- Handover ID: `OTV2-20260812-foundation-handover`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Prepared: 2026-08-12 10:20 +02:00
- Repository: `blakinio/Oteryn-v2`
- Trusted handover base: `main@22b64e1b20cf2220828f5a3d47b30df29f9a60b6`
- Terminal invocation result: **`ROTATE`**
- Runtime implementation authority: **NOT GRANTED**
- PostgreSQL DDL/migration execution authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This document is the durable handoff for the next architecture agent. It replaces chat history for continuation of the Oteryn-v2 foundation programme.

The successor must read trusted-base governance and live GitHub state first, verify drift, then execute the one `next_action` recorded at the end. Do not reconstruct old reasoning from chat when canonical architecture, task archives and merged PR evidence exist.

## 2. PROVEN current state

At the handoff base:

```text
main = 22b64e1b20cf2220828f5a3d47b30df29f9a60b6
```

Whole `DUR-02 — Persistence v1` is terminally:

```text
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
DDL/migrations       = NOT_AUTHORIZED
```

The canonical current status is `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

### Accepted/lifecycle-closed architecture that must be consumed rather than rediscovered

- `FND-ID-01` — foundation identifier semantics;
- `FND-02` — `protocol-oteryn` v1 architecture;
- `FND-03` — authoritative runtime execution architecture;
- `FND-04` — admission/GameSession/CharacterLease/reconnect architecture;
- `DUR-01` — durable identifier representation;
- `ANL-01` — game event/audit foundation;
- `NET-TRANSPORT-01` / ADR-0014..0016 — TCP-default/future-QUIC architecture direction, with runtime transport still unavailable;
- `GAME-VISION-01` — minimum product direction and immutable first Reference target;
- `GAME-CHAR-01` Stage A + Stage B — character semantic architecture;
- `DUR-02` profile-neutral Character persistence partial baseline;
- whole `DUR-02 — Persistence v1` common architecture.

The canonical Rust workspace/client migration is already delivered. Do **not** restart FND-01, VSL-02 or root-workspace bootstrap work.

## 3. Recent binding delivery evidence

### GAME-CHAR semantic closure

- delivery PR #193;
- merge `08775e378db8c1fd6bb97bedf66bf08b3541f35f`;
- lifecycle closeout PR #194.

### DUR-02 Character persistence partial baseline

- delivery PR #197;
- merge `a88c15e6bf77fe4b775005011ec3cc38837f2a0a`;
- lifecycle closeout PR #198.

This accepted normalized current-state Character persistence, `CharacterRevision`, account portfolio guards, domain-canonical Character name registry, typed child/profile extensions, separate FND-04 authority relations, atomic admission/recovery persistence, durable receipts, audit/publication atomicity and fail-closed restore fencing. Generic JSON/KV/EAV miscellaneous state remains rejected.

### Whole-DUR-02 reconciliation

- decision packet delivery PR #199;
- merge `b37a4071787fb0a3af13608670c44fc07adcc78d`;
- lifecycle closeout PR #200.

The historical fourteen-subject Persistence-v1 catch-all was reduced to six genuinely common persistence rules and every historical subject received a named disposition/owner.

### Whole-DUR-02 owner acceptance

- delivery PR #201;
- exact final head `900be9f499981e638a6f8089fb46331b43ba321c`;
- self-review `4914253621` — PASS;
- independent Codex review `5264011166` — PASS, no major issues;
- Agent Governance `31576235871` generation #907 — PASS;
- Dependency Review `31576235909` generation #651 — PASS;
- CodeQL `31576235921` generation #795 — PASS;
- squash merge `ec4b840b0742967370a4235d87094b29a802fe28`;
- lifecycle closeout PR #202;
- closeout merge / handoff-base main `22b64e1b20cf2220828f5a3d47b30df29f9a60b6`.

## 4. Binding whole-DUR-02 rules

The successor must consume `docs/architecture/DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` rather than redesigning generic persistence.

Accepted common rules:

1. one ordered game-owned migration history for the current native game database boundary, immutable explicit migration artifacts, dedicated least-privilege migrator, and no production runtime schema auto-sync;
2. `READ COMMITTED` only with explicit anomaly-closing locks/constraints; otherwise bounded `SERIALIZABLE` or a stricter domain mechanism, preserving semantic operation identity across retry;
3. one ANL-compatible durable journal plus crash-safe mutable publication claim/checkpoint state, atomically committed with mandatory owning mutations;
4. acknowledged durable success means committed durable state recoverable across ordinary process/node restart; FND-03 runtime checkpoint/replay and disaster RPO are separate concepts;
5. production persistence must be PITR-capable and restore-tested, with restored authority fail-closed and a newer non-rollback authority/recovery fence before admission resumes;
6. game-wide `EXPAND -> MIGRATE/BACKFILL -> VALIDATE -> CUT OVER -> CONTRACT` schema evolution with writer fencing and evidence-based rollback/recovery.

Exact Rust DB/migration library, table/index names, partitioning, numeric retry/RPO/RTO/backup cadence and production topology remain downstream/evidence-driven.

## 5. Historical DUR-02 ownership split — preserve exactly

- Character state/revision and Character/FND-04 persistence: already satisfied by accepted Character/FND-04 architecture;
- inventory/equipment/ground/item/currency/value semantics and conservation: `GAME-ITEM-01 -> DUR-03`;
- event/audit semantics: `ANL-01`;
- market/economy semantics: `EXP-ECONOMY-01`;
- guild/social semantics: `EXP-SOCIAL-01`;
- houses: `EXP-HOUSES-01`;
- recurring/meta rewards: `GAME-META-01`;
- encounter/event rewards: `EXP-EVENTS-01`;
- partitioning/sharding and exact Rust database/migration library: implementation/PERF evidence unless a later correctness constraint appears.

`MOVED` means ownership moved. It does **not** mean the destination gate is accepted.

## 6. Implementation boundary at handoff

The architecture is now sufficiently complete that a later owner can explicitly authorize a bounded real server/persistence implementation programme.

Accepted safe decomposition for such a future authorization:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content slices
```

However, **this handoff does not grant that authorization**.

The next agent must not create Rust server code, DDL, migration files, persistence adapters, listeners or production configuration merely because the architecture prerequisite is ready. A new explicit owner instruction authorizing implementation overrides this handoff and requires its own bounded implementation task/branch/PR/evidence.

## 7. Current safe architecture work

Without new implementation authority, the next safe architecture progression is:

```text
GAME-ITEM-01
-> DUR-03
```

`GAME-ITEM-01` must define item model/equipment/container/transform semantics against the accepted first Reference target where parity applies. `DUR-03` then owns transfer, conservation, single-location, retry/crash and anti-duplication invariants for item/currency/value mutation.

The following may proceed in parallel only under separate path/task ownership:

- `GAME-CHANNEL-01`;
- Reference evidence/parity manifest/tooling;
- `DUR-04` minimum headless content path;
- `SIM-DETERMINISM-01`;
- existing lag/disconnect architecture discussions.

## 8. Existing concurrent state

At handoff preparation, open unrelated PRs were:

- PR #191 — GAME-CHAR factual provenance correction (`2001 -> 2002`), no substantive architecture change;
- PR #162 — CI/governance aggregate merge-gate work.

Do not edit, close, rebase or merge them from a successor gameplay/architecture task unless live ownership explicitly requires coordination.

Active long-lived architecture checkpoints also include lag/disconnect analysis. Their records explicitly state that runtime implementation is not authorized. Do not delete or absorb them into unrelated work.

Always re-check live state because these PR/task facts may change after this handoff merge.

## 9. Authority and repository boundaries

Routine writes remain limited to:

- `blakinio/Oteryn-v2`.

Unless the owner explicitly grants a new exact write task, these remain read-only evidence sources:

- `blakinio/Oteryn-Platform`;
- `blakinio/Otheryn`;
- `blakinio/otclient`;
- Canary repositories and other external sources.

No production deployment, protected environment approval, production secrets, live account/session/data/database mutation or proprietary asset copying is authorized.

## 10. Successor bootstrap

Before mutation, the successor must read and follow at minimum:

1. root `AGENTS.md` and `AGENTS.override.md`;
2. `docs/agents/AGENTS.md`;
3. `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
4. `docs/agents/PROMPTING_HANDOVER.md` and `CONTEXT_HANDOFF.md`;
5. `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`;
6. this handoff report;
7. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
8. applicable accepted owner baselines/contracts for the selected gate;
9. live main, open PRs, active tasks/owned paths, exact reviews and CI.

Live merged repository state overrides this report if state has legitimately advanced.

## 11. Resume prompt for the next agent

Use the repository continuation prompt `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`. It is refreshed in the same handoff delivery and must be treated as subordinate to trusted-base governance and live main.

Do not use the previous chat as a required dependency.

## 12. Context checkpoint

```yaml
status: ready
terminal_invocation_result: ROTATE
repository: blakinio/Oteryn-v2
trusted_base_sha: 22b64e1b20cf2220828f5a3d47b30df29f9a60b6
branch: null
head_sha: null
pr: null
owned_paths: []
public_contracts: []
last_progress: GAME-CHAR, profile-neutral Character persistence and whole DUR-02 Persistence-v1 architecture are owner-accepted and lifecycle-closed; common server/persistence architecture is ready for a separately authorized implementation programme, but no implementation authority was granted before handoff.
validation_state: handoff refresh delivery must pass exact-head documentation/governance CI before becoming canonical
audit_state: handoff refresh requires self-review; independent review not required unless authority/semantic scope changes
e2e_state: NOT_APPLICABLE documentation-only handoff
ci_generation: null
run_ids: []
counters:
  repair_cycles: 0
  unchanged_state_checks: 0
blocker: null
next_action: From live main, create one bounded paper-only `GAME-ITEM-01` architecture task that consumes the accepted Reference target and preserves `DUR-03` as the item/currency/value conservation authority.
```
