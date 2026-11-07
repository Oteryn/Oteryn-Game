# Oteryn v2 — Stage-C Vertical-Slice Owner Decision Package

- Date: 2026-08-16
- Coordination issue: #310
- Trusted base: `main@bf2a2ae279516f62626a5d8f4dc1aeb587535c62`
- Package status: `OWNER_DECISIONS_REQUIRED`
- Runtime/client/server/protocol/content/DDL/Platform/production authority: **NONE**
- Executor prompts: **HOLD**

## 1. Decision requested

This package asks the repository owner to dispose the final three architecture gates currently blocking movement/combat/content implementation prompts:

```text
VSL-MOVE-01
VSL-COMBAT-01
VSL-CONTENT-01
```

For each gate choose:

- `ACCEPT` — bind the candidate architecture for its declared minimum slice scope;
- `REWORK` — keep the gate unaccepted and name the material clause(s) requiring change;
- `DEFER` — keep executors blocked because the decision is not yet required or evidence is insufficient.

Coordinator recommendation: **ACCEPT all three**.

## 2. Why these gates are the last current pre-executor architecture blockers

After PR #309 / merge `bf2a2ae279516f62626a5d8f4dc1aeb587535c62`, the first-wave foundation/gameplay/client/analytics architecture is owner-accepted. Current status/register still identifies these three Stage-C gates as `BLOCKS_VERTICAL_SLICE` and not yet accepted.

They are not technology-selection cleanups. They own concrete safety seams that implementation would otherwise have to guess:

- movement authority and local relocation vs scope handoff;
- death/loot/XP/pickup identity and anti-dup recovery;
- the minimum content compiler/loader/activation proof while final physical format remains evidence-gated.

## 3. Common accepted invariants

All three candidates preserve:

- server-authoritative gameplay;
- current `ChannelRuntime`/`InstanceRuntime` one-writer ownership;
- FND-02 CommandRef, connection-generation, server-sequence, state-revision and resync semantics;
- FND-04 GameSession/CharacterLease authority;
- exact behavior-affecting revision binding under SIM/DUR;
- GAME-ABILITY as the one typed effect/combat mutation pipeline;
- GAME-INTERACTION child identity/retry/reconciliation authority;
- GAME-AI proposal-only action/path behavior and no-value authority;
- GAME-ITEM/DUR-03 item/value legality, conservation, durable idempotency and anti-duplication;
- GAME-CHAR persistent Character progression/death consequence ownership;
- GAME-CHANNEL value-source multiplicity/eligibility boundaries;
- ALPHA-CLIENT non-authoritative projection;
- QA-E2E real-boundary evidence requirements;
- Reference evidence fail-closed semantics.

No candidate creates a new process/service merely to satisfy the slice.

## 4. VSL-MOVE-01

### Recommendation

`RECOMMENDATION: ACCEPT`.

### Binding decisions

1. Local authoritative position, dynamic occupancy consequences and same-scope relocation belong to the current FND-03 `ChannelRuntime` or `InstanceRuntime` owner through a logical Movement domain role inside that owner.
2. The client, path worker, GAME-AI, GAME-INTERACTION and renderer never directly commit position.
3. Three movement classes are distinct:
   - `LOCAL_STEP`;
   - `LOCAL_RELOCATION` within one current authority scope;
   - `SCOPE_HANDOFF`, which is not a local teleport and remains under accepted FND/Channel handoff authority.
4. One movement occurrence derives identity from an authoritative source occurrence + semantic movement discriminator + actor + exact semantic revisions; no new mandatory global MovementId.
5. Static legality uses the exact immutable content/map revision; dynamic legality uses current authoritative runtime state.
6. Post-movement interaction triggers derive stable GAME-INTERACTION children from the committed movement occurrence.
7. A pure immutable same-scope relocation edge may be consumed by Movement as spatial topology; stateful relocation/door/script workflows remain GAME-INTERACTION-owned and cannot create distributed atomicity implicitly.
8. Authoritative visibility/interest is a bounded deterministic server-derived state domain delivered through FND-02 sequence/revision/snapshot-delta reconciliation.
9. Exact Global movement timing/visibility geometry remains evidence-gated. Structural E2E may use explicit non-shipping fixture profiles.
10. Concrete resource ceilings are mandatory before implementation acceptance but no number is invented by the architecture.

### Player impact

- eliminates client-authoritative movement/teleport ambiguity;
- ensures duplicate/stale commands cannot move twice;
- ensures reconnect/resync repairs observed position rather than guessing;
- keeps teleport and future handoff behavior extensible without cross-Channel exploits.

### Producer/operational impact

- no separate movement service is required;
- runtime implementation can use one owner-local spatial mutation lane and bounded indexes;
- protocol payloads remain later typed registration work;
- performance-sensitive view ranges/path algorithms remain measurable implementation choices.

### Remaining deliberate non-decisions

Exact movement/LOS/speed formulas, click-to-move/path algorithm, client prediction, renderer interpolation, wire IDs, Rust layout, numeric resource limits and cross-scope handoff implementation.

## 5. VSL-COMBAT-01

### Recommendation

`RECOMMENDATION: ACCEPT`.

### Binding decisions

1. All player/AI attack intents use accepted GAME-ABILITY; no second VSL combat engine exists.
2. Ordinary creature death is a stable post-commit descendant of the first accepted lethal state transition and is owned by the current creature lifecycle role inside ChannelRuntime/InstanceRuntime.
3. One creature lifecycle generation can create at most one semantic death occurrence.
4. Corpse runtime projection remains FND-03 state; durable item/value truth remains DUR-03.
5. Loot selection is a deterministic bounded descendant of the death occurrence with SIM purpose-isolated RNG and exact content/revision provenance.
6. A loot candidate becomes acknowledged durable value only through a DUR-03-conforming stable materialization operation/transaction tied to the same death occurrence.
7. Durable loot ambiguity remains `PENDING`/reconciliation of the same logical occurrence; no fresh duplicate mint.
8. XP is a separate stable descendant workflow. The first slice supports one eligible Character principal; GAME-CHAR owns persistent progression mutation. Loot and XP are not one distributed transaction.
9. Pickup uses CommandRef -> GAME-INTERACTION child -> GAME-ITEM legality -> DUR-03 prepare/commit/reconcile -> authoritative client state.
10. Exact Reference damage/XP/drop values remain unknown where evidence is absent. Structural proof may use an explicit non-shipping `VSL_COMBAT_FIXTURE_PROFILE`, never Reference or product policy.
11. Player durable death/PvP/party/boss breadth is not required for the first creature-death slice and remains with existing/later owners.

### Player impact

- prevents duplicated loot or XP after retry/crash/lost response;
- avoids fake corpse/inventory success before durable value is committed;
- makes client/AI use the same combat authority path;
- keeps future Reference formulas and PvP rules replaceable without rewriting transaction identity.

### Producer/operational impact

- asynchronous durable loot/XP can yield the runtime writer instead of blocking the simulation;
- crash recovery has stable death/reward/materialization identities;
- first technical slice can prove real boundaries before exact Reference evidence is complete;
- Reference fixture status cannot be confused with parity.

### Remaining deliberate non-decisions

Exact formulas/rates, conditions, PvP, party/shared XP, multi-contributor attribution, boss/event rewards, corpse lifetime/ownership, concrete libraries, wire IDs, physical persistence, numeric limits.

## 6. VSL-CONTENT-01

### Recommendation

`RECOMMENDATION: ACCEPT`.

### Binding decisions

1. The first slice needs only a bounded native semantic content set: small world/spatial fixture, collision, pure local relocation, creature/spawn, ability, loot table, XP fixture, at least one materializable item and a bounded client-safe presentation projection.
2. Canonical identity remains PackageKey/PackageRevision/ContentKey + exact revision/digest context; runtime compact IDs never become cross-build identity.
3. Before the final World Project encoding is selected, a versioned bounded `VSLContentFixture` or programmatic typed fixture builder may feed the canonical semantic graph for evidence only.
4. Compiler logic is coupled to the typed canonical graph, not to the fixture syntax.
5. Compiler output must create separate server-authoritative and allowlisted client-safe projections from one locked graph.
6. The first slice may use a versioned **non-production `VSL_BUNDLE_EVIDENCE_PROFILE`** to exercise real deterministic bytes, corruption checks, bounded loading and activation.
7. That evidence profile is disposable and cannot become the permanent World Bundle contract by inertia.
8. Loader/staging/activation must already satisfy DUR-04 fail-closed security and all-or-nothing publication semantics.
9. Active runtime/client scopes bind exact artifact/content/map/ruleset/world-policy/compiler/lock revisions.
10. Test-only fixture profiles must be distinguishable from ordinary release profiles.
11. Final physical World Project/Bundle encoding remains explicitly blocked on DUR-04's required format spike and a later owner format-selection decision.

### Player impact

- avoids shipping a throwaway test serializer as a permanent compatibility surface;
- guarantees client/server use compatible immutable content revisions;
- prevents server-only loot/rules/security fields leaking into client assets;
- makes malformed/oversized/incompatible content fail before world activation.

### Producer/operational impact

- implementation can prove compiler/loader/gameplay integration now without waiting for full Studio;
- final format remains evidence-driven rather than fashion-driven;
- the content format spike becomes a named future executable/evidence lane, not hidden architecture work inside the compiler implementation;
- broad import/Studio/scripting breadth remains off the critical path.

### Remaining deliberate non-decisions

Final source/bundle serializer/container/file extensions, final chunk/floor packing, compression/delta/CDN/signing, Studio UI/viewport, broad import, Wasm runtime for the slice, numeric limits, production rollout/hot reload.

## 7. Cross-contract composition

The three candidates compose as:

```text
VSL-CONTENT exact active immutable revision
  -> VSL-MOVE static spatial/collision/relocation facts
  -> current runtime owner commits position
  -> FND-02 projects authoritative movement/visibility

VSL-CONTENT creature/ability/loot/item fixture definitions
  -> GAME-AI typed intent where used
  -> GAME-ABILITY authoritative damage/effect
  -> VSL-COMBAT stable creature death
  -> DUR-03 durable loot materialization
  -> GAME-INTERACTION + DUR-03 retry-safe pickup
  -> GAME-CHAR single-principal XP progression mutation
  -> FND-02 projects authoritative outcomes
```

No edge above transfers the owning domain's authority to a neighboring layer.

## 8. Structural fixture policy

The coordinator explicitly recommends accepting **test-only fixture profiles** as evidence tools rather than inventing Reference facts.

Rules common to all fixtures:

- versioned and deterministic;
- exact fixture values retained in evidence;
- non-shipping by default;
- not Reference;
- not Evolved product policy;
- cannot establish parity;
- ordinary release activation must reject/disable fixture-only profiles unless an explicit test environment makes their status unambiguous.

This is an architecture safety mechanism, not a shortcut around product evidence.

## 9. Resource-limit policy

None of the three contracts invents numeric limits.

Before executable acceptance, affected dimensions must have concrete finite values/units/failure classes/boundary tests in the accepted Resource Limits Registry or an explicitly owned implementation profile. Missing required limits block the affected executable claim rather than implying unlimited resources.

## 10. Evidence / E2E boundary

Acceptance of these contracts does not prove the vertical slice.

Terminal implementation evidence must cross the real boundaries it claims:

- Tier 1: Platform/Gateway/protocol/server/persistence path as applicable using production protocol schemas/codecs plus FND-02 independent wire evidence;
- Tier 2: native client semantic input, networking, reconciliation and presentation path;
- Tier 3: later packaged release proof where required by product milestone.

Direct runtime/domain calls, mocked database success or synthetic client mutation are useful component evidence but cannot replace terminal real-boundary proof.

## 11. Dependencies that remain after Stage-C acceptance

Stage-C architecture acceptance would **not** automatically make the project external-alpha ready.

Later/lane-specific obligations still include:

- actual FND-02/FND-03/FND-04 runtime/protocol/admission implementation;
- PostgreSQL durability implementation under DUR-01/02/03;
- concrete resource-limit values and PERF evidence;
- QA-E2E implementation/evidence;
- final content-format spike + format owner decision before permanent content encoding;
- Reference mechanic evidence/parity before Reference product claims;
- later alpha gates (`ALPHA-RULESET`, `ALPHA-CONTENT`, quality/milestone, LiveOps, compatibility, privacy, client security, GM/support, etc.) before full alpha completeness;
- `PROD-ENTITLEMENTS-01` before Premium/VIP/game-consumed entitlement implementation/activation;
- OPS-CHANNEL/PERF evidence before production scaling/recovery/capacity claims.

Those do not require movement/combat/content implementation agents to invent architecture when starting the bounded first technical slice.

## 12. Decision timing test

### VSL-MOVE-01

- Must decide now: **YES**.
- Blocks: safe movement/client state implementation.
- Expensive if delayed: competing position authority and incompatible retry/teleport semantics.
- Superseding evidence: measured/fault evidence showing owner-local spatial semantics insufficient.
- Deliberately not decided: formulas/algorithms/technology/numbers.

### VSL-COMBAT-01

- Must decide now: **YES**.
- Blocks: death/loot/XP/pickup implementation and anti-dup proof.
- Expensive if delayed: transient death callbacks become value identity; retry can duplicate loot/progression.
- Superseding evidence: crash/replay/reward-domain evidence requiring a stronger descendant workflow.
- Deliberately not decided: Reference formulas/product breadth/technology/numbers.

### VSL-CONTENT-01

- Must decide now: **YES** for semantic compiler/loader/evidence seams.
- Final physical format must decide now: **NO** — DUR-04 requires format-spike evidence.
- Blocks: immutable revisioned content consumption and loader security proof.
- Expensive if delayed: test serializer/source layout becomes accidental product contract or runtime couples to source structs.
- Superseding evidence: format-spike/scale evidence showing the seam needs change.
- Deliberately not decided: permanent physical encoding and broad tooling.

## 13. Alternatives considered

### Implement movement/combat/content directly from current broad contracts

Rejected. It leaves the concrete cross-domain identity/commit/recovery seams to executor judgment.

### Wait for complete Global Tibia evidence before any structural VSL

Rejected. It would block native system proof on facts that can remain explicitly fixture/evidence gated without weakening eventual Reference parity.

### Freeze a final World Bundle serializer now

Rejected by accepted DUR-04. The required format spike is intentional architecture, not unfinished thinking.

### Make loot + XP + death one distributed transaction

Rejected. Owner-local death plus named idempotent descendant workflows preserves authority/failure boundaries and avoids unnecessary cross-domain atomicity.

### Let GAME-INTERACTION own movement because teleports/doors are interactions

Rejected. FND-03 already places authoritative local position in ChannelRuntime/InstanceRuntime. GAME-INTERACTION owns trigger/retry/reconciliation semantics; Movement retains final local position commit.

## 14. Owner choices

```yaml
VSL-MOVE-01:
  choices: [ACCEPT, REWORK, DEFER]
  coordinator_recommendation: ACCEPT

VSL-COMBAT-01:
  choices: [ACCEPT, REWORK, DEFER]
  coordinator_recommendation: ACCEPT

VSL-CONTENT-01:
  choices: [ACCEPT, REWORK, DEFER]
  coordinator_recommendation: ACCEPT
```

If all three are accepted, a later acceptance delivery must:

1. record owner baselines without rewriting the historical candidates;
2. lifecycle-close issue/task;
3. update current status/register/index so Stage-C is no longer a blocker;
4. terminally reconcile stale executor prompt PR #305;
5. build and audit a fresh implementation-prompt DAG with explicit prerequisites and lane authority;
6. keep entitlement implementation excluded unless separately accepted;
7. explicitly include the content-format spike as an evidence lane before permanent physical format selection.

## 15. Current executor state

```text
EXECUTOR_PROMPTS: HOLD
IMPLEMENTATION_AUTHORITY: NONE
```
