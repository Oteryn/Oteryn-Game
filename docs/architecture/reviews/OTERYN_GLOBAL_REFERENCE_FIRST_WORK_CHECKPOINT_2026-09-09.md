# Oteryn Global Reference-first work checkpoint — 2026-09-09

- Status: **DURABLE WORK / RECOVERY CHECKPOINT**
- Repository: `Oteryn/Oteryn-Game`
- Authoring protected base: `main@247818666bbbcb13fa27e24ec90fee24c793a416`
- Parent product control plane: #162
- Programme tracker: #486
- Reference evidence tracker: #483
- Architecture coordination: #220
- Immutable target: `global-tibia-observable-2026-07-28-post-server-save`
- Implementation authority: **NONE BY THIS DOCUMENT**
- Allocation authority: **#162 ONLY**
- Merge authority: **repository control plane / protected Merge Queue only**
- Live deployment authority: **NONE**

This document persists the architecture/evidence/readiness work performed while production gameplay dependencies were partially blocked. It is a recovery packet, not a second scheduler and not a replacement for GitHub LIVE state.

Before acting on any lifecycle fact, refresh the relevant Issue/PR/protected-main state. Historical SHAs, statuses and comments below are evidence/checkpoints only.

---

## 1. Binding Reference-first direction

The programme sequence remains:

```text
official / controlled Global evidence
-> GLOBAL_REFERENCE_TARGET
-> native Oteryn shared engine/client/protocol implementation
-> compare implementation to target
-> PARITY_CONFIRMED or explicit DECLARED_DIFFERENCE
-> Oteryn Reference readiness
-> only then separately authorized Oteryn Evolved work
```

Global Tibia is an external behavior/evidence authority. There is no requirement to create a separate production runtime engine/profile called `Tibia Global`.

`Oteryn Reference` must implement the selected Global behavior inside the native Oteryn architecture. `Oteryn Evolved` remains later work and must not leak into Reference.

### Source hierarchy

Use sources in this order:

1. official Tibia/CipSoft primary material and dated production notices;
2. lawful controlled black-box observation/capture;
3. reputable community corroboration where primary evidence is insufficient;
4. Canary, Crystal and other OTS repositories only as `OTS_HYPOTHESIS_ONLY`.

OTS code may be used to discover fields, edge cases, possible formulas and tests. It must never independently prove Global behavior or overwrite stronger official evidence.

A current official page also does **not** automatically prove the 2026-07-28 target state. Dated production evidence takes precedence when current pages are stale or conflict with target chronology.

Important official locators include:

- target boundary: `https://www.tibia.com/news/?id=8905&subtopic=newsarchive`
- starting manual: `https://www.tibia.com/gameguides/?section=starting&subtopic=manual`
- controls manual: `https://www.tibia.com/gameguides/?section=controls&subtopic=manual`
- characters manual: `https://www.tibia.com/gameguides/?section=characters&subtopic=manual`
- world manual: `https://www.tibia.com/gameguides/?section=world&subtopic=manual`
- combat manual: `https://www.tibia.com/gameguides/?section=combat&subtopic=manual`
- trading/depot manual: `https://www.tibia.com/gameguides/?section=controls_trading&subtopic=manual`
- spells library: `https://www.tibia.com/library/?subtopic=spells`
- creatures library: `https://www.tibia.com/library/?subtopic=creatures`

Detailed per-source classification continues under #483 and the protected evidence pack.

---

## 2. Live repository checkpoint at persistence time

Fresh protected-main readback immediately before this document was authored:

```text
main@247818666bbbcb13fa27e24ec90fee24c793a416
```

Material programme/lifecycle facts that must be refreshed before future mutation:

- #54 remains open; first-production CONTENT is not a free ownership surface.
- #498 is protected-integrated and repaired the separate `EXPLICIT_EVENT_POLICY_REQUIRED` fail-closed P1.
- #490/#492 remain the canonical staged-generation/canonical-WorldId repair lineage and must not be replaced by a duplicate worker.
- PR #494 remains the architecture candidate selecting `crate::foundation::WorldId` as canonical Game-side cross-boundary `WorldId`; exact candidate head at this checkpoint: `e7cdf36d1a9e5dbef580bd7169f128e1c5e8da0c`.
- PR #500 is the first Reference NPC-service ownership architecture candidate; exact candidate head at this checkpoint: `85e7bda6bb2d4c23a33c1284c5e98da6c502153e`.
- #480 World/VFX physical prototype gate is complete and protected through PR #489.
- #64 OTBM migration/composite world design checkpoint is active again for bounded Reference-corridor preparation, but it still does not freeze the permanent World Project/World Bundle format.
- #502 is the renderer working-set resource-bound gate.
- #504 is the `REFERENCE_PLAYABLE_CONTENT_PROFILE/v1` successor-profile architecture/resource gate.

META policy 3.1 adopts the native queue route `merge-async` with exact `sha` and `merge_action="merge_queue"`. If the active execution surface does not expose that exact operation, integration remains `BLOCKED_CAPABILITY_UNAVAILABLE`; direct merge and generic auto-merge are not substitutes.

---

## 3. Target-delta chronology that Reference must consume

Do not treat the target as either current Tibia or the raw Summer Update release state.

The first target is after the 2026-07-28 server-save boundary and therefore must consume the production delta chain leading into that date, including at least:

```text
2026-06-16 vocation-adjustment production release
-> 2026-06-23 fixes
-> 2026-07-07 material balancing/nerf delta
-> 2026-07-10 Summer final/test-server deltas where promoted to release
-> 2026-07-13 Summer Update production release
-> 2026-07-14 fixes
-> 2026-07-16 fixes
-> 2026-07-21 fixes
-> 2026-07-28 target boundary
```

Post-target fixes may be useful reverse evidence: a later CipSoft fix can reveal a defect that existed at the 2026-07-28 cut. Such evidence does not automatically prove the exact earlier state; bind it explicitly and classify it conservatively.

Official test-server values are not production truth. A concrete example is Energy Ring: test material used a different mana/damage ratio than the final Release State. Always distinguish test-preview evidence from production evidence.

---

## 4. R0 — Global Reference evidence / parity findings

### 4.1 Abilities / spells

#### Ice Strike

Strong pre-target official anchors exist for:

- formula/name family: `exori frigo`;
- Druid/Sorcerer eligibility;
- Free Account eligibility;
- level 8 after the 2025 change.

Current Library also exposes mana/cooldown/group/range information, but fields without an adequate target-continuity chain remain independently evidence-gated.

Canary demonstrates why OTS cannot be authority: a Canary Ice Strike definition inspected during this work used level 15 and Premium=true, conflicting with current official CipSoft data.

#### Light Healing

Official historical/current material strongly supports `exura` as a self-heal family, but exact 2026-07-28 level/mana/cooldown/formula continuity remains weaker than for Ice Strike.

Do not promote Light Healing metadata merely to make the first attack/heal pair symmetrical.

#### Wound Cleansing

This is currently a stronger field-level heal candidate than Light Healing for Reference fixture research:

- official Vocation Adjustments Release State records base power 70;
- official 2026-07-07 production balancing records mana 60;
- current official Library is consistent with `exura ico`, level 8, 2s/2s and Premium=no.

The release-state/base-power evidence must still be tied honestly to the target cut; do not reinterpret `base power 70` as an unconditional fixed 70 HP heal.

### 4.2 Attack lifecycle

Official manuals support a minimal lifecycle:

- one current main target;
- attack starts on selected target;
- subsequent attacks continue while target/range/legality remain valid;
- stop/change target is explicit observable behavior.

Exact attack interval, hit formula and target-range geometry remain separate evidence fields.

### 4.3 Death

Strong official continuity supports Global death causing both experience and skill loss plus temple/home-city re-entry.

Oteryn Reference already has an explicit accepted difference:

```text
DeathSkillLoss = 0
DeathMagicLevelLoss = 0
XP loss basis uses LevelXPSpan(current_level)
```

This is an Oteryn `DECLARED_DIFFERENCE`, not Global truth. Never rewrite the Global target fixture to zero skill loss just because Oteryn intentionally differs.

First minimal death fixture recommendation was refined to **level 7**, no promotion/blessings/PvP, so item-loss ambiguity is excluded. Current official materials conflict around the exact level-8 item-loss threshold (`up to level 8` versus `as soon as level 8`), so level 8 item-loss remains `CONFLICT` until resolved.

### 4.4 Loot authorization

Strong official continuity supports the classic first-10-second corpse rule:

- highest-damage character is authorized to loot for the first 10 seconds;
- party members share that authorization;
- corpse movement is blocked during that window;
- the rule does not apply to dead characters in the same way.

R5 must split **corpse authorization/immovability** from **loot-table RNG**. Do not require random loot probabilities merely to prove authorization.

### 4.5 Stamina / loot

Current official behavior includes the low-stamina rule where sufficiently depleted stamina can destroy/deny creature loot. Treat this as a separate player-state modifier rather than baking it into the generic loot table.

### 4.6 Party XP

There is a material official conflict requiring target observation/reconciliation:

- June 2026 release material records increased two-/three-vocation group bonuses (35% / 70%);
- current official material can present 30% / 60%.

No sufficiently strong pre-28.07 reversion proof was found in this work. Keep party XP out of the first solo fixture and classify the target values as `CONFLICT` until resolved.

### 4.7 Discovery movement boost

Official pre-target material supports a movement-speed bonus family of roughly **100..195 speed** depending on completed Discovery areas, and the feature was live before the target.

The exact mapping function from Discovery progress to bonus remains evidence-gated. A first static local-step fixture should explicitly set Discovery bonus inactive rather than silently ignore the system.

### 4.8 Weapon Proficiency

Weapon Proficiency is part of the target-era production game and is not merely an item stat. Official 2025/2026 material establishes:

- per-character proficiency progress;
- per-weapon trees/perks;
- Dust as character-bound progression/economy input;
- 2026 slot modification/reshape/refine/maximise/clear behavior with PZ restrictions.

It belongs across R7 progression and R2 definition/content boundaries. It should not be copied from OTS perk tables.

### 4.9 Spawn / AI evidence

A directly target-anchored R6 spawn example exists for the 2026-07-28 boundary: Radiant Skyhold `devoted` threshold = 2000 regular kills. This can prove `1999 -> 2000 -> eligibility` without inventing the later statue-trigger behavior.

AI evidence is weaker. `Maior Domus retargets more frequently` is qualitative and does not prove a timer/algorithm.

A smaller AI continuity candidate is Newhaven `Goblin Intruder`: official tutorial material says that after it is attacked it fights back. This supports a bounded retaliatory-AI candidate without retarget RNG.

Respawn also needs per-creature policy. Official material has a general nearby-character blocking rule plus explicit historical exceptions; do not encode one universal `players-nearby => no respawn` bool.

---

## 5. R1 — World / map / import readiness

### 5.1 Existing tooling must be reused

Do **not** write a new OTBM parser for the first Reference corridor.

Protected Game tooling already includes:

- `tools/game-atlas-fullworld-source/` — Game-owned full-world source projection from exact pinned migration inputs;
- `tools/tibia-worldmap-reconstruction/` — normalized observed/reference map validation/comparison with explicit `MATCH`, `STACK_ORDER_MISMATCH`, `REFERENCE_CONFLICT`, `NOT_OBSERVED`, `UNMAPPED_ID`, etc.;
- Atlas semantic-search exports usable as locator hypotheses.

Current pinned migration inputs include:

```text
legacy repository: blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm sha256: 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
15.32.zip sha256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
```

These are migration/source evidence only. OTBM/Crystal topology does not become Global target truth merely because it is available.

### 5.2 Newhaven / Targuna chronology

Official production chronology proves:

- Newhaven is the modern entry/tutorial area before the target;
- from 2026-03-17, level-8 progression goes to **Targuna**, not the old direct Newhaven -> Thais route;
- March/April fixes confirm the Newhaven/Targuna flow was live before 2026-07-28.

Current Library pages can still contain stale old-route wording, so dated production news outranks that stale current text for target chronology.

### 5.3 Bounded corridor recommendation

Do not begin with a full-world import.

First world evidence/prototype should use two bounded scenes:

1. Newhaven village + one nearby hunting/tutorial micro-segment;
2. level-8 transition + Targuna service core + one nearby micro-zone.

Legacy locator hints currently available:

- Newhaven waypoint hypothesis: approximately `(32534, 32513, -7)` in the pinned legacy/Atlas projection;
- Targuna has a separate pinned town record in Atlas (`semantic-record:57d9ef64607b0360d0c01d338f7c70ba`) on floor `-7`.

These coordinates/records are **locator hypotheses only**, not target coordinate proof.

### 5.4 Geometry evidence rule

Official map/manual material proves topology/service intent, not exact cell geometry/stack order.

Exact floor/cell/placement geometry should be classified by comparing:

```text
controlled normalized target capture
vs
pinned migration/OTBM projection
```

Result classes should remain explicit. Do not silently accept Crystal geometry when the target capture disagrees or is missing.

### 5.5 First-production profile is not the world profile

`FIRST_PRODUCTION_CONTENT_PROFILE/v1` is intentionally limited to one floor and a tiny bootstrap spatial envelope. It must not be widened inside #54 merely to hold the Reference corridor.

#504 owns the later Reference-playable successor-profile decision.

---

## 6. R2 — Content / successor profile

Current protected `FIRST_PRODUCTION_CONTENT_PROFILE/v1` is a compiler/staging/activation bootstrap, not a Reference content profile.

Its shape includes exact cardinalities such as:

- 1 region, 1 area, 1 terrain;
- 3..=1024 cells;
- 1 relocation, 1 behavior;
- 3 presentations;
- 1 creature, 1 spawn and aggregate spawn population 1;
- 1 formula, 1 effect, 1 ability;
- 1 materializable item;
- 1 loot table / 1 loot entry;
- 1 XP definition / 1 RNG purpose;
- x/y span <=32;
- exactly 1 floor.

Its current Content vocabulary also exposes only `EffectFamily::Damage`.

The required Reference delta is therefore semantic, not merely numeric. Protected production types currently lack several capabilities needed by Reference Playable, including:

- Heal effect family in Content;
- multiple attack/heal definitions in one profile;
- creature combat/HP/value bindings sufficient for real Reference cases;
- explicit corpse/loot/XP relationship semantics;
- ordered semantic placements/world-object references in cells;
- richer item/equipment capability bindings;
- NPC/dialogue/service definition records when #499 is protected.

Issue **#504** now owns the architecture/resource successor gate for:

```text
REFERENCE_PLAYABLE_CONTENT_PROFILE/v1
```

Do not implement this by expanding the current #54 repair lineage.

The successor should reuse the production compiler/artifact/provenance/Content Lock/staging/activation safety architecture while adding only Reference-required semantics and separately evidenced resource maxima.

---

## 7. R3 — Movement / interaction readiness

No production Movement module should be invented opportunistically. #139 remains the existing Movement owner/gate and is not reactivated by this checkpoint.

A smallest future component proof has been shaped as:

```text
REFERENCE_LOCAL_STEP_STATIC_KERNEL/v1
```

Scope:

- one cardinal local step;
- one authoritative actor/current owner context;
- static walkable/blocked destination fact;
- exactly one accepted/rejected result;
- no diagonal movement;
- no speed formula;
- no Discovery bonus in this first component fixture;
- no occupancy arbitration;
- no visibility expansion;
- no relocation/teleport;
- no pathfinding;
- no AI movement;
- no command batch/backlog.

Resource analysis indicates this structural-local proof may not need a new Movement-specific hard maximum if every variable-size dimension is excluded and the existing FND-02 outstanding-command limit remains the only ingress bound. This does **not** mean full VSL-MOVE has no resource gates.

---

## 8. R4 — Ability / spell integration readiness

The current Ability core already supports typed:

```text
Effect::Damage
Effect::Heal
```

However, this is still a bounded/fixture-oriented engine and must not be described as a fully integrated production combat engine.

The production Content path currently has only `EffectFamily::Damage` and an exact single-ability profile.

Therefore the next R4 work is not a second ability engine. It is a later, freshly allocated evidence-bound composition slice that:

1. represents Damage + Heal in the successor content profile;
2. binds each spell field to evidence/revision state;
3. passes typed intents through the existing Ability legality/commit pipeline;
4. leaves unresolved formula/range/LoS/condition fields fail-closed;
5. does not hardcode Global formulas in generic Ability/Content primitives.

The simultaneous attack+heal Reference case depends on #504 successor-profile capacity; merely adding `Heal` to the enum is insufficient.

---

## 9. R5 — Combat / death / XP / corpse / pickup

### 9.1 Decomposition rule

Do not build one monolithic `combat test` that requires every Global number first.

Use separate fixtures for:

- attack lifecycle;
- lethal transition;
- XP consequence;
- corpse authorization/immovability;
- loot-table selection/RNG;
- durable loot materialization;
- pickup/inventory transaction;
- death loss/respawn.

A lethal-transition fixture may start with a creature already at lethal remaining HP. That avoids falsely claiming full natural-spawn/max-HP combat parity when only death consequence is under test.

### 9.2 Progression arithmetic

Character owns durable progression facts, but exact XP/level/death arithmetic is ruleset/SIM-owned.

The existing `simulation-determinism` crate already provides shared checked numeric/rounding primitives (`ExactI64`, `FixedScale`, explicit rounding modes). It deliberately does not define gameplay formulas or Reference values.

A future progression successor should therefore apply a revision-bound progression delta through those primitives rather than introduce a private Character `XP calculator` or promote the OTS experience formula to authority.

### 9.3 OTS XP formula use

A common OTS/OTClient formula reproduces current official Experience Table samples including levels 23/24. It is useful as a regression hypothesis only.

No official target-era formula publication was established during this work. Exact progression thresholds/formula remain target-evidence work where required.

### 9.4 Durable item pickup

GAME-ITEM structural primitives exist for typed items, location/equipment/container legality, but there is no accepted transaction-capable durable `corpse/container -> CharacterInventory` runtime implementation yet.

VSL-COMBAT already defines the desired ownership chain:

```text
death
-> LOOT_SETTLEMENT_PENDING
-> LOOT_READY
-> Interaction intent
-> GAME-ITEM legality
-> DUR-03 PREPARE / COMMIT / reconcile
-> Character inventory
```

The missing work is implementation + transaction hard maxima, not a new Combat ADR.

---

## 10. R6 — Creature AI / spawn readiness

Current AI implementation is a bounded pure-local/proposal bootstrap. Controlled actor mutation, spawn integration, timers, memory and movement are not already production-ready merely because the AI module exists.

A smallest successor proof has been shaped as:

```text
AI_ACTION_INTEGRATION_STATIONARY_RETALIATION/v1
```

Intended scope:

```text
AcquireCandidate
-> zero or one immediate transient Ability proposal
-> no queue/backlog
-> no Movement
-> no timer
-> no memory/leash
-> no spawn ownership
```

An `AiAbilityAdapter` already exists, so do not invent a second intent API.

Important blocker: current fixture-local `AI CandidateId(u64)` and Ability `TargetId(String)` are not a production identity bridge. Production AI action integration must use the same authoritative Target Resolver + Legality Evaluation as other Ability origins. Never bridge fixture IDs by `to_string()` or equivalent convenience conversion.

---

## 11. R7 — Character / item / progression readiness

### Equipment legality

Official material supports a bounded first legality proof around hand/equipment slot rules, one-handed vs two-handed constraints, shield/quiver relationships, vocation/level requirements and capacity.

Test legality/custody separately from item balance/attack/resistance/imbuement values.

### Starter item evidence

Current official Quickstart shows a tutorial chest giving a dagger and health potion, with the dagger equipped to hand and potion stored in the backpack before the first Goblin Intruder kill.

Do not freeze that exact pair as target truth from the current page alone.

Official 2026-01-20 material fixed a wrong Newhaven starting health potion for knight/monk, proving that starter-potion mapping was target-era and vocation-sensitive. Preserve starter item identity/count/potion mapping as independently evidence-gated fields.

---

## 12. R8 — NPC / quest / services

There was no previously named owner for ephemeral NPC conversation/service lifecycle.

Issue **#499** was created as the bounded architecture escalation. PR **#500** contains the current one-file decision candidate selecting:

```text
GAME-NPC-SERVICE
```

as a logical semantic role inside the existing `ChannelRuntime` / `InstanceRuntime`, not a new process/service.

Selected ownership split:

- NPC-local idle/movement/perception -> GAME-AI where applicable;
- NPC conversation/service lifecycle -> GAME-NPC-SERVICE;
- generic child proposal/reconciliation -> Interaction where useful;
- immutable dialogue/service/catalogue/price definitions -> Content/ruleset revisions;
- item legality -> GAME-ITEM;
- durable item/value mutation/reconciliation -> DUR-03/downstream value owner;
- client -> input/presentation only.

The first trader is split into two gates:

```text
NPC_DIALOGUE_TRADE_WIDGET_V1
  talk/greet -> trade capability -> server-projected Buy/Sell catalogue
  no value mutation

NPC_SINGLE_TRADE_COMMIT_V1
  one real BUY/SELL
  requires GAME-ITEM + transaction-capable DUR-03 + target-evidenced price/catalogue
```

This prevents a missing economy transaction engine from blocking early dialogue/UI plumbing while preserving the final Reference service requirement.

Official pre-target CipSoft material from the Newhaven rollout supports the modern NPC dialogue window/clickable keywords and `trade` shortcut opening the trade widget. Exact target prices/catalogue/keywords remain evidence-gated; do not use OTS shop tables as authority.

Quest durable state remains a later separate ownership refinement; do not hide quest state inside ephemeral NPC conversation state.

---

## 13. R9 — Native client / renderer / presentation

### Protected evidence now complete

The non-production World/VFX prototype gate #480 is complete and protected through PR #489.

Physical evidence on Molehill-PC / RX 9070 XT includes:

- 81 primary runs + 3 presentation-family smoke runs;
- reliable GPU timestamps in the measured matrix;
- zero overflow fallback in the final calibrated matrix;
- zero surface/device-loss failures;
- Classic / Enhanced / HD gameplay-signature equality.

Evidence-bounded verdicts:

```text
custom Rust + wgpu renderer foundation -> ADOPT
bounded visible working set with eviction -> ADOPT
current simple hybrid resource layout -> REJECT
atlas vs texture arrays -> INSUFFICIENT_EVIDENCE
KTX2 vs DDS/container -> INSUFFICIENT_EVIDENCE
particle backend -> INSUFFICIENT_EVIDENCE
hard light/VFX budgets -> INSUFFICIENT_EVIDENCE
batch thresholds -> INSUFFICIENT_EVIDENCE
production RAM/VRAM budgets -> INSUFFICIENT_EVIDENCE
final filtering/mipmap policy -> INSUFFICIENT_EVIDENCE
```

Do not convert the experiment's page capacities into production limits.

### Production cache gap

Current production `crates/renderer/src/resources.rs` is generation-fenced but uses an unbounded `BTreeMap` and has no capacity/eviction/byte-work accounting.

Issue **#502** now owns the first production renderer working-set resource decision for:

```text
RENDER_VISIBLE_RESOURCE_CACHE_V1
```

Candidate resource dimensions include resident pages, retained decoded/uploadable bytes, uploads per frame/work cycle, upload bytes, in-flight uploads, eviction/destruction work, cold prewarm work and diagnostic volume.

No renderer-specific hard maxima were found in the current registry during this work.

A further product-support dependency remains: no accepted minimum supported GPU/VRAM/client hardware floor was found. Do not choose production cache maxima from the RX 9070 XT benchmark as if that high-end card were the support baseline.

### Native gameplay entry blocker

The native client still depends on the real admission/protocol/Server Seam chain for physical gameplay entry. Do not build a full Targuna UI mock and call it a playable Reference client while `request_gameplay_entry()` remains pre-native/unavailable in the production flow.

---

## 14. R10 — Reference QA / milestone ladder

Structural readiness must not be confused with Global parity.

Recommended milestone separation:

### M0 — structural/component readiness

May use synthetic/project-owned fixtures to prove:

- typed boundaries;
- deterministic ordering;
- resource ceilings;
- fail-closed unknowns;
- ownership/revision fencing;
- no duplicate mutation.

M0 **does not** prove Reference parity.

### M1 — bounded physical Reference scene A

Newhaven micro-scene proving as dependencies become available:

- real world entry/presentation;
- local presence and one cardinal move;
- one creature encounter/corpse consequence;
- no claim of full tutorial parity if the exact tutorial focused-attack spell/XP values remain unknown.

The official tutorial `Goblin Intruder` flow is a useful observable scenario, but exact Goblin Intruder HP/XP/loot/spell parameters remain independently evidence-gated.

### M2 — bounded physical Reference scene B

Separate level-8 precondition:

- transition to Targuna;
- service-area presence;
- NPC dialogue/trade-widget plumbing;
- logout/re-entry/reconnect observation as native client/server dependencies permit.

Do not require the entire level-1 -> level-8 progression to prove this scene.

### M3+ — composition completion

Add in bounded order:

- real attack + heal;
- XP/progression application;
- durable loot materialization/pickup;
- inventory/equipment transaction basics;
- representative AI/spawn;
- one mutating trade;
- death/respawn/re-entry;
- restart/reconnect/durability composition.

Final Reference readiness remains the gate already defined by #486. None of these intermediate milestones activates Oteryn Evolved.

---

## 15. Open architecture/resource gates created or materially refined by this work

### #499 — first Reference NPC dialogue/service ownership

Purpose: define the smallest owner boundary for NPC conversation/service lifecycle.

Current candidate: PR #500, `GAME-NPC-SERVICE` role inside the current runtime owner.

Implementation remains blocked until the architecture is protected and #162 issues a fresh exact allocation.

### #502 — production renderer visible working-set resource bounds

Purpose: turn #480's accepted bounded-cache direction into an evidence-backed production resource profile without copying benchmark capacities as hard maxima.

Implementation/registry mutation authority: none until the resource decision is accepted and separately serialized/allocated.

### #504 — Reference Playable content successor profile

Purpose: define `REFERENCE_PLAYABLE_CONTENT_PROFILE/v1` as a later explicit capability profile instead of widening the #54 bootstrap silently.

Dependencies include #54 terminal release, canonical WorldId closure, #64 corridor evidence, #483 field evidence and relevant gameplay/Item/DUR-03/NPC-service gates.

---

## 16. Current serial dependency chain

The product control plane remains #162.

Do not create duplicate writers for blocked work.

Current high-level sequence is:

```text
#494 protected WorldId decision
-> same #492 repair lineage applies P1-2 and completes #490/#54 repair closure
-> #54 ownership terminally released

in parallel where path-disjoint:
#483 evidence
#64 bounded world-corridor evidence
#499/#500 NPC-service architecture
#502 renderer resource evidence
#504 successor-profile architecture/resource design

then fresh #162 allocations for the smallest ready successors:
- Reference Content successor pieces
- R4 attack/heal composition
- progression apply
- transaction-capable loot/pickup/equipment
- R3 local-step / later full Movement
- R6 stationary retaliation / later AI integration
- NPC dialogue/trade-widget
- renderer bounded cache / presentation consumer

production physical composition still obeys:
WP/dependency chain -> fresh G0 -> existing Server Seam #247 -> native client/gameplay entry -> Reference physical QA
```

No checkpoint, green local component test, docs PR, or evidence merge bypasses that production chain.

---

## 17. Fail-closed rules for future agents

1. Refresh GitHub LIVE state before mutation/integration claims.
2. #162 remains sole allocation/integration control plane for product work.
3. Resume existing canonical worker/branch/PR; do not create a replacement because it is waiting.
4. Official target evidence first; OTS only hypothesis/test discovery.
5. `UNKNOWN`/`CONFLICT` remains blocked; do not fill from current Global/Canary/Crystal by convenience.
6. A current official page does not override a dated production target chronology automatically.
7. Test-server values do not become production truth without a production promotion/release chain.
8. Do not change Global target expectations to match an intentional Oteryn `DECLARED_DIFFERENCE`.
9. Do not move gameplay authority into renderer/client/Content scripts/AI/Interaction for convenience.
10. Do not copy experiment limits into production resource registry without same-resource evidence.
11. Do not choose permanent `.omap/.owb`, chunk, atlas/array, KTX2/DDS or final UI technology merely to complete the first Reference slice.
12. Do not start Oteryn Evolved implementation before an explicit later owner/control-plane release after the Reference readiness checkpoint.
13. Preserve Merge Queue/protected-main discipline; direct merge/force-push/protection weakening are forbidden substitutes.
14. No personal owner name/surname belongs in produced artifacts/commits.

---

## 18. Durable cross-reference index

Primary trackers/decisions used by this checkpoint:

- #54 — first-production CONTENT lifecycle
- #64 — OTBM migration/composite world checkpoint
- #123 — historical DUR-03 resource gate context
- #139 — Movement gate/owner
- #162 — sole product allocation/integration control plane
- #220 — architecture coordination
- #247 — existing Server Seam lineage
- #433 — first production CONTENT architecture decision
- #472 — first production CONTENT resource-limit serialization
- #480 / PR #489 — completed World/VFX physical evidence gate
- #481 — merged first-production CONTENT implementation
- #483 / PR #485 — official-first Reference evidence pack
- #486 / PR #487 — Global Reference-first programme
- #490 / #492 — canonical post-merge CONTENT repair lineage
- #491 / PR #494 — canonical WorldId architecture resolution
- #498 — protected explicit-event multiplicity repair
- #499 / PR #500 — first Reference NPC-service ownership decision
- #502 — renderer production working-set resource gate
- #504 — Reference Playable content successor-profile gate

Important checkpoint comments created during this work include:

- #483 `5604659489` — first bounded spell/creature candidates
- #483 `5604709640` — Ice Strike/creature continuity refinement
- #483 `5604768143` — death + 10-second loot authority
- #483 `5604988747` — R0 Discovery/Weapon-Proficiency/target-era additions
- #483 `5605175530` — R8 service evidence
- #483 `5605387896` — candidate next manifest package
- #483 `5605507941` — world-entry/logout evidence
- #483 `5605576352` — R6 spawn/AI refinement
- #483 `5605590824` — attack lifecycle
- #483 `5606096975` — heal evidence refinement
- #483 `5606115419` — post-target fix/reverse-evidence discipline
- #483 `5606365968` — Newhaven/Targuna onboarding
- #483 `5606434493` / `5606439690` — R1/R9 client/world evidence
- #483 `5606653270` / `5606709561` — death fixture and level-8 item-loss conflict correction
- #483 `5606714623` — tutorial combat split
- #483 `5606732416` — Wound Cleansing heal candidate
- #486 `5606167927` — future attack+heal Content cardinality gate
- #486 `5606241155` — R6 action integration handoff
- #486 `5606267283` — R3 local-step readiness
- #486 `5606464990` — two-scene Reference physical proof
- #486 `5606570542` — future transaction-capable DUR-03 child
- #486 `5606597884` — milestone ladder
- #486 `5606617541` — reuse existing Interaction/Ability/AI kernels
- #486 `5606665469` — critical-path packet
- #486 `5606793565` — Character progression successor
- #486 `5607140733` — protected graphics evidence reconciliation
- #486 `5607417510` — consolidated R0/R2/R3/R4/R5/R6/R8/R9 checkpoint
- #64 `5607277655` / `5607293706` — bounded corridor + existing import-tool reuse
- #502 `5607411817` — hardware-support/product-floor blocker
- #504 `5607371526` — REQUIRED_NOW / DEFERRED / FORBIDDEN semantic-delta packet

The comment IDs are recovery references, not standalone authority. Prefer current protected files and LIVE issue/PR state when they differ.

---

## 19. Next legal actions

When resuming from this checkpoint:

1. fresh-read protected `main`, #162, #54, #490/#492, #494, #500, #64, #483, #502 and #504;
2. if #494 is protected, immediately resume the **same #492 lineage** for the bounded `foundation::WorldId` P1-2 repair; do not create a replacement branch;
3. if #500 is protected, prepare only a #162 allocation proposal for `NPC_DIALOGUE_TRADE_WIDGET_V1`; do not implement without allocation/resource preflight;
4. continue #64 corridor evidence with the existing full-world producer + normalized comparator, using Newhaven/Targuna locators only as source-window hints;
5. continue #502 only after resolving/accepting the supported-client hardware floor needed to choose production cache maxima;
6. continue #504 semantic/resource design, but do not mutate #54 production code under the successor gate;
7. continue #483 controlled target observation for unresolved spell values, starter items, party XP, exact world geometry, trader prices/catalogue and other `UNKNOWN/CONFLICT` fields;
8. once #54 ownership is terminally released, request fresh #162 allocations for the smallest path-disjoint R4/progression/transaction/world/content successor slices;
9. keep Server Seam/native-client physical integration on the existing dependency chain; never create a parallel protocol/gameplay-entry route.

`OTERYN_REFERENCE_FIRST_CONTINUES = true`
`OTERYN_EVOLVED_IMPLEMENTATION_ACTIVE = false`
