# Oteryn v2 Foundation Programme — Successor Handover

- Handover ID: `OTV2-20260812-foundation-handover`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refreshed: 2026-08-13 09:24 +02:00
- Repository: `blakinio/Oteryn-v2`
- Trusted refresh base: `main@1e16b32069868f14aa1761a512b6cd8b1024e277`
- SIM-DETERMINISM delivery: PR #214 / exact final head `4c6684328123aebd657696808372a5855980d34e` / squash merge `1e16b32069868f14aa1761a512b6cd8b1024e277`
- SIM-DETERMINISM lifecycle closeout: PR #215
- Terminal successor result after closeout: **`ROTATE`**
- Runtime/client/combat/AI/script implementation authority: **NOT GRANTED**
- PostgreSQL DDL/migration execution authority: **NONE**
- Platform write authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This is the durable successor handoff for Oteryn-v2 architecture continuation. Chat history is non-authoritative. A successor must read trusted-base governance and live GitHub state first, verify drift/ownership, and then execute the one `next_action` below.

PR #215 is lifecycle/status/handoff reconciliation only. It does not create or change SIM-DETERMINISM semantics and becomes canonical only when the closeout merges.

## 2. Canonical current state after SIM closeout

```text
GAME-VISION-01        ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-CHANNEL-01       ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-CHAR-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-ITEM-01          ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-02                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-03                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
DUR-04                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
ANL-01                ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
SIM-DETERMINISM-01    ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
PROD-ENTITLEMENTS-01  PROPOSED / PLANNED / NOT_STARTED
```

Architecture acceptance does **not** imply runtime/client/server implementation, compiler/loader/Studio/WIT-host implementation, PostgreSQL DDL/migrations, Platform implementation, broad content import, gameplay traffic, exact gameplay formulas or production readiness.

## 3. Accepted architecture — consume rather than rediscover

Unless explicitly superseded, preserve:

- one native Rust client/server stack and one project-owned `protocol-oteryn`;
- `protocol-canary` reference-only and excluded from target runtime/fallback/translation;
- FND-ID/FND-02/FND-03/FND-04 identity/protocol/runtime/session/admission/fencing/recovery semantics;
- DUR-01/DUR-02 durable identity/persistence transaction/migration/restore architecture;
- ANL-01 event/audit/privacy/read-only investigation boundary;
- GAME-VISION minimum product direction and immutable first Reference target after the 2026-07-28 Global Tibia server-save/maintenance boundary;
- GAME-CHAR formula-neutral authoritative progression facts with unresolved Reference arithmetic remaining fail-closed;
- GAME-ITEM typed item semantics;
- DUR-03 durable item/currency/value conservation/idempotency/anti-duplication;
- GAME-CHANNEL selection/queue/co-location/anti-hopping/multiplicity/qualitative lifecycle and one-World community/economy policy;
- DUR-04 typed semantic content graph, exact package lock, deterministic compilation, immutable bundle activation/migration, bounded loading/provenance and capability-bounded deterministic scripting;
- SIM-DETERMINISM deterministic arithmetic/RNG/order/replay/state-hash architecture described below.

Do not restart accepted gates merely because older backlog/predecision prose reflects an earlier state.

## 4. Accepted SIM-DETERMINISM-01 — binding result

Canonical sources:

- `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_ANALYSIS.md`;
- `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`.

Delivery evidence:

- PR #214;
- final repaired head `4c6684328123aebd657696808372a5855980d34e`;
- independent review `4924203877` on superseded head `5dc628f32ca4573725bcb4a42c3a7702536d7f35` found one material replay-provenance P1;
- owner-authorized repair cycle 4 restored exact server/build, `protocol-oteryn` and World Bundle provenance while keeping NodeId/process placement forensic-only;
- repaired P1 thread resolved;
- terminal full-diff self-review `4924321455` — PASS, material findings `0`;
- repeat exact-head self-review `4924423397` — PASS, material findings `0`;
- Agent Governance `31676250271` — PASS;
- Dependency Review `31676250273` — PASS;
- CodeQL `31676250272` — PASS;
- unresolved review threads before merge: `0`;
- the owner explicitly instructed the coordinator to finish PR #214 at exact head `4c668432...`, overriding the otherwise-required fresh independent-review-after-repair mechanism without relabeling self-review as independent and without authorizing any new owner-funded AI use;
- squash merge `1e16b32069868f14aa1761a512b6cd8b1024e277`.

Binding SIM closure includes:

- same canonical future-determining state + same exact owner-local normalized input order + same semantic revision/profile set + same normalized external facts => same normalized authoritative result sequence;
- no global total order and no second runtime commit ordinal; FND-03 RuntimeExecutionOrdinal remains owner-local evidence;
- `SimulationDeterminismProfileRevision` versions numeric/RNG/tie-break/hash/supported-target semantics without replacing content/ruleset/world-policy/script execution revisions;
- retryable/delayed logical occurrences bind exact behavior-affecting revisions and do not silently recalculate under newer incompatible semantics;
- exact discrete and DUR-03 conservation arithmetic remains exact; formula fixed-scale/rational semantics are preferred where appropriate; authoritative floating requires explicit deterministic profile and cross-target proof;
- formula descriptors define units, representation, operation order, explicit rounding boundaries/modes, invalid-state behavior and Reference evidence state;
- one process-global mutable gameplay RNG is forbidden; random decisions use stable semantic purpose identities and isolated keyed/counter decisions or isolated checkpointed substreams;
- retry/failover preserves the same logical random decision; speculative/rejected/aborted work does not independently advance committed RNG state;
- exploit-sensitive deterministic RNG may remain confidential and may not be derived solely from public facts unless predictability is explicitly accepted;
- wall clock, process-local monotonic time and authoritative execution order remain distinct; no global fixed tick is introduced;
- simultaneous/conflicting inputs use commutative semantics, stable semantic tie-breaks or exact retained FND-03 owner order;
- external nondeterminism crosses a typed normalization boundary before affecting authority;
- replay retains exact server/build executable identity, protocol profile/revision, World Bundle artifact/digest, semantic revisions, inputs/order, formula/script profiles, RNG evidence and normalized time/external facts;
- original NodeId/process-incarnation may be retained for forensics but replay correctness does not require recreating placement;
- canonical deterministic state/hash includes active revisions, gameplay state, RNG state/cursors, pending accepted timers/operations/continuations, occurrence identities and semantically relevant fences/revisions;
- state hashes and replay/divergence tooling are evidence only and cannot repair/mutate live authority;
- supported authoritative server targets must produce identical normalized authoritative outcomes; incompatible targets fail closed;
- missing replay/hash/RNG/formula/pending-state limits block implementation rather than becoming unlimited.

## 5. One next paper-only action

After SIM closeout, the remaining named pre-VSL paper-only programme action is:

```text
Build the versioned Reference evidence/parity manifest under its owning contract.
```

Do not invent a new stable gate ID unless the owner/repository explicitly creates one. The manifest must preserve the accepted first Reference target and evidence discipline:

- first Reference target remains Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary;
- each exercised Reference-sensitive behavior stays `PROVEN`, `OBSERVED`, `DERIVED`, `UNKNOWN`, `CONFLICT` or explicit `DECLARED_DIFFERENCE` until evidence supports stronger status;
- official public evidence is primary but not assumed exhaustive;
- controlled black-box observation may supply provenance-bound behavior evidence;
- community sources are corroborative/discovery inputs;
- Canary/crystalserver/other OTS code is hypothesis/inventory material, not proof of Global behavior;
- patch-note/search absence is not proof of no change;
- unresolved Reference-sensitive behavior remains fail-closed and cannot be `PARITY_CONFIRMED` by implementation convenience.

This next action is paper-only. It does not authorize runtime implementation, packet probing beyond separately authorized/legal evidence acquisition, proprietary asset/code copying, DDL, Platform writes or production changes.

## 6. Implementation boundary

A future executable server/persistence/content/Channel/SIM package still requires separate explicit owner implementation authorization and its own bounded evidence. The current safe high-level sequence remains real-boundary implementation slices only after owning contracts are ready.

`PROD-ENTITLEMENTS-01` remains independently unaccepted for Oteryn-v2 consumption; Premium/VIP/game-consumed entitlement activation remains unauthorized.

## 7. Successor bootstrap

Before mutation, a successor must read/follow at minimum:

1. root `AGENTS.md` and `AGENTS.override.md`;
2. `docs/agents/AGENTS.md`;
3. `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
4. `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
5. `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md`;
6. `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`;
7. this handoff;
8. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
9. accepted GAME-VISION and domain contracts relevant to the exact Reference claims being inventoried;
10. live `main`, open PRs, active tasks/owned paths, review threads and CI.

Live merged repository state overrides this report if state has legitimately advanced.

## 8. Context checkpoint

```yaml
status: ready
terminal_invocation_result: ROTATE
repository: blakinio/Oteryn-v2
trusted_base_sha: 1e16b32069868f14aa1761a512b6cd8b1024e277
closeout_pr: 215
owned_paths: []
public_contracts: []
last_progress: SIM-DETERMINISM-01 delivery PR #214 merged exact repaired head 4c6684328123aebd657696808372a5855980d34e as 1e16b32069868f14aa1761a512b6cd8b1024e277 after the explicit owner finalization override; lifecycle closeout #215 promotes canonical status and releases SIM ownership.
validation_state: delivery #214 merged after repaired P1, exact-head self-review and green Governance/Dependency/CodeQL; closeout #215 must pass its own exact-head documentation/governance validation.
e2e_state: NOT_APPLICABLE documentation-only architecture/closeout
blocker: null
owner_action_required: false
next_action: From live main after SIM-DETERMINISM lifecycle closeout, create one bounded paper-only task to build the versioned Reference evidence/parity manifest under its owning contract; do not invent a new stable gate ID or implement runtime/DDL/production behavior.
```
