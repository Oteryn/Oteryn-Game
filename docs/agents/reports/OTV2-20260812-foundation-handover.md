# Oteryn v2 Foundation Programme — Successor Handover

- Handover ID: `OTV2-20260812-foundation-handover`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Refreshed: 2026-08-13 00:30 +02:00
- Repository: `blakinio/Oteryn-v2`
- Trusted refresh base: `main@568236c33cd23da017bca1dbd1ed98afc8da71f4`
- DUR-04 delivery: PR #212 / exact final head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23` / squash merge `568236c33cd23da017bca1dbd1ed98afc8da71f4`
- DUR-04 lifecycle closeout: PR #213
- Terminal successor result after closeout: **`ROTATE`**
- Runtime/client/compiler/loader/Studio implementation authority: **NOT GRANTED**
- PostgreSQL DDL/migration execution authority: **NONE**
- Platform write authority: **NONE**
- Broad content-import authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This is the durable successor handoff for Oteryn-v2 architecture continuation. Chat history is non-authoritative. A successor must read trusted-base governance and live GitHub state first, verify drift/ownership, then execute the one `next_action` below.

PR #213 is lifecycle/status/handoff reconciliation only. It does not create or change DUR-04 semantics and becomes canonical only when the closeout merges.

## 2. Canonical current state after DUR-04 closeout

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
SIM-DETERMINISM-01    PROPOSED / PLANNED / NOT_STARTED
PROD-ENTITLEMENTS-01  PROPOSED / PLANNED / NOT_STARTED
```

The canonical detailed status is `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.

Architecture acceptance does **not** imply runtime/client implementation, compiler/loader/Studio/scripting implementation, PostgreSQL DDL/migrations, Platform implementation, broad content import, gameplay traffic or production readiness.

## 3. Accepted architecture — consume rather than rediscover

Unless explicitly superseded, preserve:

- one native Rust client/server stack and one project-owned `protocol-oteryn`;
- `protocol-canary` reference-only and excluded from target runtime/fallback/translation;
- distinct WorldId/ChannelId/InstanceId/NodeId/GameSessionId and one logical mutation owner per Channel/Instance;
- Platform Identity/Gateway/World Registry versus native-game authority separation;
- PostgreSQL native-game target with separate Platform/game ownership;
- FND-ID-01/FND-02/FND-03/FND-04 typed identity/protocol/runtime/session/admission/fencing/recovery semantics;
- DUR-01 durable representation including non-reused UUIDv7 ItemInstanceId;
- DUR-02 Persistence-v1 migration/transaction/outbox/durable-ack/PITR/schema-evolution architecture;
- ANL-01 event/audit durability/privacy/read-only investigation boundary;
- GAME-VISION minimum product direction and immutable first Reference target after the 2026-07-28 Global Tibia server-save/maintenance boundary;
- GAME-CHAR Stage A/B semantic closure and fail-closed per-behavior parity discipline;
- GAME-ITEM typed item semantic envelope;
- DUR-03 durable item/currency/value conservation/idempotency/anti-duplication envelope;
- GAME-CHANNEL selection/queue/co-location/anti-hopping/multiplicity/qualitative lifecycle and one-World community/economy policy;
- DUR-04 content/package/compiler/bundle/activation/migration/scripting architecture described below.

Do not restart accepted gates because older backlog/predecision prose reflects a pre-acceptance state.

## 4. Accepted DUR-04 — binding result

Canonical sources:

- `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_ANALYSIS.md`;
- `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`.

Delivery evidence:

- PR #212;
- frozen final head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23`;
- owner-directed exact-head self-review `4921665072` — PASS, material findings `0`;
- explicit owner instruction on 2026-08-13 selected the implementing coordinator as terminal reviewer for #212; this is self-review, not independent review;
- repair budget `3/3`;
- Agent Governance `31646998515` — PASS;
- Dependency Review `31646998564` — PASS;
- CodeQL `31646998517` — PASS;
- unresolved material review threads before merge: `0`;
- squash merge `568236c33cd23da017bca1dbd1ed98afc8da71f4`;
- no Codex/OpenAI API/owner-funded AI service invoked by the coordinator for DUR-04.

### Content and package identity

- the canonical model is a typed semantic graph, not YAML/RON/JSON5 or another physical serializer;
- stable namespaced `PackageKey`, immutable `PackageRevision`, stable `ContentKey` and exact deterministic Content Lock;
- no runtime floating/latest dependency resolution;
- legacy/compact numeric IDs are revision-scoped mappings, never canonical semantic identity;
- content/map/ruleset/world-policy/compiler/script-execution-profile/artifact identities remain distinct.

### Deterministic compiler, bundle and loader

- source/import -> typed model -> validation -> dependency/key/alias resolution -> migration normalization -> deterministic lowering -> client/server projection -> immutable artifact -> isolated staging -> explicit activation;
- identical locked inputs/compiler/canonicalization/profile inputs require identical logical output and artifact digest;
- nondeterministic file order, unordered iteration, locale/timezone, wall clock, machine paths, uncontrolled randomness, duplicate precedence and unstable serialization are canonicalized or rejected;
- client-safe content is allowlist projection; server-only authoritative data must not leak into client artifacts;
- artifacts remain immutable/content-addressed and may not use unstable Rust memory layout as a public format;
- loader checks size/count/decompression/checked arithmetic/version/capability/integrity/dependency/index/semantic rules before authoritative publication;
- missing applicable Resource Limits Registry entries block implementation acceptance rather than meaning unlimited.

### Activation, rollback and durable migration

- staging and activation are separate;
- an active immutable revision is never edited in place;
- every authoritative scope can identify its exact governing revisions/artifacts;
- rollback cannot pair old content with semantically incompatible migrated durable state;
- durable changes classify as `COMPATIBLE_NO_MIGRATION`, `READ_COMPATIBLE_NORMALIZE`, `EXPLICIT_DATA_MIGRATION`, `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION` or `REMOVED_WITH_EXPLICIT_POLICY`;
- scripts cannot perform direct SQL or ad-hoc persistence migrations.

### Legacy conversion and provenance

- every external source set records exact revision/archive digest, license/provenance, `COPY|CONVERT|REWRITE|REFERENCE_ONLY|REJECT`, importer identity, deterministic conversion diagnostics and legacy mappings;
- LIR stays at the importer boundary;
- Otheryn/Canary/other OTS sources are migration/reference evidence, not proof of Global behavior.

### Authoritative scripting

- target boundary: WebAssembly Component Model + project-owned versioned WIT capability ABI, unless explicitly superseded;
- Wasmtime is an implementation candidate, not ABI owner;
- exact WIT package/world/interface requirements are compatibility inputs;
- no target process-global Lua/global Game authority;
- no ambient filesystem/network/process/environment/secrets/unrestricted wall clock/OS RNG/direct SQL/mutable Rust-server objects;
- all authoritative reads are invocation-snapshot-bound;
- scripts return typed proposals; extension-state and domain mutations become authoritative only after owning host/domain transaction/workflow validation and commit;
- ActionPlan cannot create distributed/cross-owner atomicity; broader workflows retain owning OperationId/idempotency/compensation semantics.

### Script determinism and persistence

- logical simulation time, invocation-local deterministic RNG, stable query ordering, bounded host calls/results/action plans, deterministic floating/NaN policy and deterministic fuel/resource behavior;
- `script_execution_profile_revision` binds determinism-sensitive engine/runtime compatibility, Wasm features, fuel operator costs and numeric/memory/table behavior;
- engine/profile changes cannot silently reinterpret an existing content revision's fuel/determinism contract;
- VM memory/table state is never durable gameplay state;
- persistent extension state is typed, namespaced, versioned, bounded and persisted/fenced/migrated through accepted ownership;
- a trap/fuel exhaustion/invalid plan/rejected transaction leaves authoritative proposed state unchanged.

### Deliberate DUR-04 deferrals

Still not frozen:

- final source serializer and file extensions;
- final World Bundle container;
- 32x32 versus 64x64 chunking and floor packing;
- compression codec;
- exact WIT function inventory/lowering details;
- exact Wasmtime version/features;
- numeric resource ceilings;
- optional legacy-language adapter;
- production signing/trust roots/CDN;
- Studio implementation details;
- domain-specific quest/NPC/combat/AI formulas.

These choices require their named spike/implementation/release evidence and may not be guessed from the accepted semantic architecture.

## 5. Next ordered paper-only architecture action

After DUR-04 closeout, the remaining near-term pre-VSL paper-only work includes `SIM-DETERMINISM-01` and Reference evidence/parity tooling. They remain independently ownable.

To keep ownership singular, the successor action is exactly:

```text
SIM-DETERMINISM-01 — Authoritative Simulation Determinism Contract
```

A bounded SIM task should consume accepted FND-03, GAME-CHAR, GAME-ITEM, DUR-03, DUR-04 and GAME-VISION evidence discipline and freeze only the architecture required now for:

- authoritative numeric/arithmetic representation boundaries;
- rounding, overflow/underflow and invalid numeric-state policy;
- deterministic RNG identities/streams/substreams and consumption ownership;
- simulation logical time, tick/order and simultaneous-event tie-break semantics;
- deterministic replay input contract and external nondeterminism capture;
- state hashing/divergence localization evidence;
- supported-target determinism and cross-platform comparison policy;
- formula/ruleset revision compatibility;
- relationship between core simulation determinism and DUR-04 `script_execution_profile_revision`;
- acceptance scenarios needed before broad combat/AI formulas or unresolved Character arithmetic can be parity-confirmed.

Do **not** implement simulation/combat/AI/scripts, add runtime dependencies, alter DDL or enable production in that gate.

Reference evidence/parity tooling may proceed separately under its own task/path ownership; selecting SIM as next does not accept it.

## 6. Repository/external/production authority

Routine autonomous writes remain limited to `blakinio/Oteryn-v2` unless an exact later owner instruction authorizes another repository.

`blakinio/Oteryn-Platform`, `blakinio/Otheryn`, `blakinio/otclient`, Canary repositories and other external sources remain read-only evidence inputs unless separately authorized.

No production deployment, protected environment approval, secrets, live account/session/data/database mutation, entitlement activation, broad content import or proprietary asset copying is authorized.

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
9. accepted FND-03/GAME-VISION/GAME-CHAR/GAME-ITEM/DUR-03/DUR-04 sources relevant to SIM;
10. live `main`, open PRs, active tasks/owned paths, review threads and CI.

Live merged repository state overrides this report if state has legitimately advanced.

## 8. Context checkpoint

```yaml
status: ready
terminal_invocation_result: ROTATE
repository: blakinio/Oteryn-v2
trusted_base_sha: 568236c33cd23da017bca1dbd1ed98afc8da71f4
closeout_pr: 213
owned_paths: []
public_contracts: []
last_progress: DUR-04 delivery PR #212 passed final owner-directed exact-head self-review `4921665072`, repair budget 3/3 and Agent Governance/Dependency Review/CodeQL, then squash-merged unchanged as `568236c33cd23da017bca1dbd1ed98afc8da71f4`; lifecycle closeout #213 promotes canonical status and releases DUR-04 ownership.
validation_state: PR #212 delivery PASS; closeout #213 must pass its own exact-head documentation/governance validation before this refreshed handoff becomes canonical.
e2e_state: NOT_APPLICABLE documentation-only architecture/closeout
blocker: null
owner_action_required: false
next_action: From live main after DUR-04 lifecycle closeout, create one bounded paper-only `SIM-DETERMINISM-01` architecture task; do not implement runtime/combat/AI/scripts/DDL/production behavior.
```
