# GAME-CHAR-01 — Pre-Decision Character Lifecycle and Progression Analysis

- Status: **PRE-DECISION ANALYSIS / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-CHAR-01`
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Decision owner: product owner
- Trusted-base analysis point: `blakinio/Oteryn-v2@9510a93b024b92a761176b18373c8853c30a6617`
- Purpose: separate baseline-neutral character architecture that can safely be decided now from Reference-sensitive gameplay semantics that must remain blocked until the exact first Reference baseline is selected
- Does not authorize: runtime/client/protocol/persistence/content implementation, PostgreSQL schema, exact Reference target selection, exact gameplay formulas, Platform writes, production rollout or acceptance of any recommendation below

## 1. Problem

`GAME-VISION-01` is now accepted for its minimum product-vision scope and makes `GAME-CHAR-01` the next product-sensitive architecture gate before final character-bearing `DUR-02` persistence semantics.

The character domain already has strong accepted identity, authority and session boundaries, but the remaining GAME-CHAR scope mixes two different classes of decisions:

1. **baseline-neutral semantic architecture** that must be stable regardless of which exact Reference revision is selected;
2. **Reference-sensitive gameplay rules** such as exact progression/death/name/creation behavior that must not be guessed before the exact first Global Tibia baseline is frozen.

Treating both classes as one immediate decision would violate the accepted GAME-VISION hard gate. Deferring the entire character gate would also be unnecessarily blocking, because several lifecycle and ownership semantics are already decidable without a specific patch.

The correct problem is therefore to determine **what can be safely frozen now, what must wait, and what DUR-02 is allowed to consume from each stage**.

## 2. Already accepted — do not reopen

### PROVEN

The following are already canonical inputs.

### 2.1 Identity and authority

- `CharacterId` is a strongly typed, full 128-bit UUIDv7 issued by game-domain Character Authority.
- `CharacterId` is global semantic character identity and is not `WorldId + CharacterId`, character name or an account slot.
- rename preserves `CharacterId`;
- legal world transfer preserves `CharacterId` while changing authoritative current `WorldId`;
- legal account ownership transfer preserves `CharacterId` while changing current `AccountId` owner binding;
- terminal deletion/retirement never allows reuse of that CharacterId;
- Oteryn Platform owns `AccountId`, authentication and product/commercial orchestration;
- Character Authority owns the character aggregate, current account ownership binding, current world membership and final name reservation/uniqueness enforcement;
- Platform projections/caches are never gameplay or mutation authority;
- native Platform direct SQL writes to character tables are forbidden steady-state design.

### 2.2 Online authority and session constraints

- one `AccountId` may have at most one current authoritative online character;
- knowing AccountId/CharacterId/GameSessionId does not itself grant authority;
- FND-04 owns AccountPresenceClaim, CharacterLease, GameSession and TransportBinding/control continuity;
- healthy authoritative gameplay control is not preempted merely by a duplicate login;
- same-character recovery preserves the existing actor where applicable rather than creating/resetting character state;
- actor/control presence states are semantically `ABSENT`, `PRESENT_CONTROLLED`, `PRESENT_UNCONTROLLED` under FND-04.

### 2.3 Product direction

- first external evaluation is Reference-first;
- exact Reference mechanics follow the selected named Reference baseline, not future-facing Evolved preferences;
- `solo viable, party rewarded` and `PvP = secondary pillar` are long-term product directions but do not rewrite Reference mechanics;
- first Evolved differentiation remains reliability/UX-first and does not automatically redesign progression/death;
- exact death/progression/PvP/party formulas are downstream-owned;
- the exact first Global Tibia Reference baseline is `DEFERRED WITH HARD GATE`: if a downstream decision cannot remain baseline-neutral, it must stop rather than guess.

### 2.4 Durability split

- DUR-01 fixes durable UUID representation but does not define the character schema;
- `DUR-02` owns physical schema, transactions, locking, migrations, revisions, checkpoints and recovery;
- `GAME-CHAR-01` must define enough character semantics before final character-bearing DUR-02 schema is frozen;
- `GAME-ITEM-01`/`DUR-03` own item semantics/conservation and must not be silently absorbed into the character aggregate;
- exact authoritative arithmetic/rounding/RNG remains subject to `SIM-DETERMINISM-01` and the owning ruleset/gameplay gates.

## 3. Decision timing result

### Must decide now?

**YES — baseline-neutral minimum only.**

A minimum character semantic contract is required before DUR-02 can safely decide what kind of authoritative character state exists, how lifecycle transitions are fenced, and which state belongs to other aggregates.

### Can full GAME-CHAR-01 close now?

**NO.**

The current hard gate prevents freezing concrete Reference behavior without an exact named Reference baseline. Full GAME-CHAR completion still requires Reference-sensitive decisions such as the exact persisted progression vocabulary/behavior, name namespace behavior, creation choices, death/respawn consequences and other observable character rules where the selected Global target is authoritative.

### Recommended staging

Use one GAME-CHAR gate with two explicit decision stages rather than inventing separate permanent engines/contracts:

```text
Stage A — baseline-neutral character semantics
  -> may be accepted now
  -> constrains DUR-02 discovery and schema architecture

Stage B — Reference-sensitive character rules
  -> blocked on exact first Reference baseline
  -> required before final Reference character schema/fixtures and broad progression implementation
```

These are decision stages inside `GAME-CHAR-01`, not automatically new canonical gate IDs.

## 4. Recommended Stage A — character aggregate boundary

### RECOMMENDATION — owner decision required

Define one semantic **Character aggregate** whose authoritative state is limited to character-owned concerns.

### 4.1 Character-owned semantic state categories

The aggregate should conceptually own:

1. **identity/lifecycle** — CharacterId and lifecycle state;
2. **current ownership** — current validated AccountId binding where an account owner exists;
3. **current logical-world membership** — current WorldId;
4. **current name state** — current display/canonical name plus whatever revision/reservation metadata the later naming policy requires;
5. **character progression facts** — authoritative persisted progression values defined by the active ruleset/profile;
6. **character build/profession state** — vocation/class/promotion/mastery/specialization facts only where the active ruleset defines them;
7. **character-scoped long-lived capability/unlock state** where it is semantically part of the character rather than a separate content/account aggregate;
8. **revision/fencing state** required to reject stale character mutation and to make ruleset/profile migrations explicit.

This is a semantic ownership list, not a PostgreSQL column list.

### 4.2 Explicit exclusions from the character aggregate

Do **not** turn Character into a giant MMO aggregate.

The Character aggregate must not become authoritative owner merely because a system references CharacterId. In particular, keep separate owners for:

- item instances, inventory/equipment/container conservation and item provenance — `GAME-ITEM-01` / `DUR-03`;
- market/bank/depot/trade transaction authority;
- houses;
- guild/party/friends/presence social aggregates;
- quests/content instances that have their own durable owner/lifecycle;
- account entitlements/commercial state — Platform / `PROD-ENTITLEMENTS-01`;
- active GameSession/CharacterLease/control state — FND-04;
- runtime combat/AI/instance state except the durable character consequence explicitly committed by the owning gameplay transaction.

A Character may hold references or projections needed by a transaction, but references do not transfer semantic ownership.

### Why this is recommended

- prevents one giant row/lock domain from becoming the entire game;
- keeps item/economy conservation independently auditable;
- allows character progression and lifecycle to evolve without coupling every social/economy system;
- aligns with ADR-0012's single Character Authority while preserving other domain authorities.

## 5. Recommended Stage A — minimal lifecycle model

### RECOMMENDATION — owner decision required

Use the smallest durable lifecycle state machine that represents product meaning without encoding workflow internals as permanent states:

```text
create transaction succeeds
        |
        v
     ACTIVE
        |
        | schedule deletion (if enabled by product/ruleset)
        v
DELETION_SCHEDULED
        |             \
        | finalize     \ restore/cancel before finalization
        v               \
     RETIRED <----------- ACTIVE
```

### 5.1 `ACTIVE`

The semantic character exists and may be eligible for ordinary product workflows subject to account/world/session/ruleset restrictions.

`ACTIVE` does not mean online.

### 5.2 `DELETION_SCHEDULED`

The product has accepted a reversible deletion intent but terminal retirement has not occurred.

Exact waiting duration, visibility and allowed actions are not selected here.

If the selected Reference baseline uses different observable behavior, Stage B maps the policy while preserving an explicit nonterminal versus terminal distinction where needed for safe recovery.

### 5.3 `RETIRED`

Terminal semantic character retirement:

- CharacterId never becomes another character;
- ordinary gameplay admission is impossible;
- restore to the same active semantic character is not allowed after terminal retirement under this recommended minimum;
- minimum tombstone/audit/provenance retention may remain where required by integrity/security/legal/privacy contracts;
- privacy erasure is a separate data-lifecycle operation and must not be conflated with semantic identity reuse.

### 5.4 Do not make these permanent character lifecycle states by default

Avoid embedding transient workflow/authority facts such as:

- `ONLINE`;
- `IN_COMBAT`;
- `TRANSFER_PENDING`;
- `RENAME_PENDING`;
- `BAZAAR_LISTED`;
- `BANNED`;
- `DISCONNECTED`.

Those belong to session/runtime/operation/commercial/moderation authorities unless a later contract proves a durable character-lifecycle meaning is necessary.

### Alternative considered — one generic status enum for all workflows

Rejected as the default because it conflates orthogonal concerns, creates invalid state combinations and makes one character row the coordinator for unrelated systems.

## 6. Recommended Stage A — atomic creation boundary

### RECOMMENDATION — owner decision required

Character creation is one idempotent Character Authority operation. A successfully visible character must not exist in a partially initialized semantic state.

The creation result conceptually binds atomically:

```text
fresh CharacterId
+ owner AccountId
+ initial WorldId
+ final accepted name reservation
+ applicable profile/ruleset/content/starter-template revision context
+ initial authoritative character-owned progression/build facts
+ character revision
```

Exact field layout is DUR-02 work.

### Required properties

- retry after an ambiguous response does not create a second semantic character;
- final name reservation/conflict enforcement is part of the authoritative operation, not a Platform preview;
- starter state comes from an explicit versioned ruleset/content template rather than hard-coded hidden defaults;
- no client-supplied initial stat value becomes authoritative merely because it appeared in creation intent;
- quota/slot eligibility is authoritatively revalidated at commit time;
- a failed create leaves no externally usable half-character.

### Still Reference-sensitive

Do not choose now:

- exact creation choices;
- starter level/skills/items/location;
- vocation/class choice timing;
- exact slot/quota number;
- exact name namespace/allowed-name behavior.

Reference uses the later selected exact target; Evolved differences require explicit profile decisions.

## 7. Recommended Stage A — character revision and stale-mutation rule

### RECOMMENDATION — owner decision required

Every durable character mutation must have one monotonically advancing **character-state revision/fence concept** or an equivalent concurrency token owned by the character persistence contract.

This is not CharacterId and not GameSessionId.

Semantic requirement:

```text
mutation reads expected character state
-> owning transaction validates current revision/fence
-> exactly one valid mutation commits
-> resulting character revision advances
-> stale competing mutation fails/reconciles
```

Exact type, width, database lock/isolation strategy and retry mechanics remain DUR-02.

### Why required

This is needed for races such as:

- rename versus rename;
- delete versus restore;
- world transfer versus login/admission;
- account transfer versus seller login;
- progression save versus ownership/lifecycle transition;
- ruleset migration versus ordinary mutation;
- duplicated Platform command retry.

FND-04 session/lease fencing and character-state revision solve different problems and must not be collapsed into one number.

## 8. Recommended Stage A — quiescent high-impact mutation baseline

### RECOMMENDATION — owner decision required

For the first safe native architecture, require **quiescent actor authority** before operations that change identity ownership/placement or terminally remove the character from ordinary gameplay.

### Must require actor `ABSENT` and no current playable CharacterLease before commit

- terminal retirement/finalization;
- legal account ownership transfer;
- legal world transfer.

This deliberately rejects first-generation live world/account transfer of a currently controlled actor.

### Rename

Rename is lower blast-radius because identity and current world/owner do not change. The architecture should support a policy choice:

- simplest v1: commit rename only when actor is `ABSENT`;
- later explicit policy may support a coordinated online rename if all client/social/cache/projection consequences are proven.

Do not require online rename now merely for convenience.

### Why quiescent-by-default

- removes GameSession/CharacterLease/world-transfer split-brain races from the first implementation;
- avoids half-updated social/item/world projections;
- makes rollback/reconciliation tractable;
- aligns with the vertical-slice bias and leaves live transfer as an evidence-backed later capability.

### Evidence that could justify superseding later

Real product demand plus proven handoff/fencing/recovery semantics may justify online/live transfer in a later dedicated gate. It is not required to prove the first Reference product.

## 9. Recommended Stage A — progression facts versus derived values

### RECOMMENDATION — owner decision required

Separate **authoritative durable progression facts** from **derived gameplay/presentation values** at the semantic contract level.

### Authoritative progression facts

A ruleset may define facts such as level/experience, skill progress, magic/profession progress, promotion/mastery state or other persistent counters/capabilities.

GAME-CHAR should own the rule that these facts are authoritative game state, but it must not invent their exact catalogue before the Reference target/ruleset is known.

### Derived values

Values that are deterministically derivable from authoritative progression facts plus accepted ruleset/equipment/conditions should not automatically become a second independent authority merely because the client wants to display them.

Whether a derived value is materialized as a cache/performance optimization is later implementation/persistence work; if materialized, it must be reproducible or revision-bound and must not silently diverge from the authoritative rule inputs.

### Important non-monotonicity

Do **not** assume all progression only increases. Reference death/risk semantics may reduce some progression values. The invariant is revisioned authoritative mutation, not monotonic numeric growth.

### Exact formulas remain blocked

- XP thresholds;
- skill advancement curves;
- derived-stat formulas;
- capacity/HP/mana formulas;
- death loss;
- promotion bonuses;
- offline training yields;
- rounding and modifier ordering

must not be guessed. Reference-sensitive formula selection waits for the exact Reference baseline and deterministic arithmetic decisions in their owning gates.

## 10. Recommended Stage A — death and respawn boundary

### RECOMMENDATION — owner decision required

Treat death as an authoritative gameplay outcome that may commit character-owned progression/risk consequences, but do not make GAME-CHAR the owner of the entire combat/item transaction.

Conceptual split:

```text
combat/gameplay authority
-> proves death cause/outcome trigger

GAME-CHAR semantic consequence
-> character-owned progression/death-state consequences under active ruleset

GAME-ITEM / DUR-03 where applicable
-> item/value consequences

DUR-02 / DUR-03
-> atomicity/recovery mechanics across the owning durable mutations
```

Required semantic properties:

- reconnect/login cannot reset a committed death or erase committed consequences;
- one death event cannot apply character loss twice on retry/replay;
- exact Reference death/respawn/blessing/protection-loss behavior remains Stage B;
- later Evolved death redesign requires an explicit versioned gate and does not silently alter Reference.

## 11. Recommended Stage A — ruleset/profile revision interpretation

### RECOMMENDATION — owner decision required

A character does not independently choose an arbitrary gameplay profile. Current logical world membership determines the applicable world/profile/ruleset revision under accepted world-profile architecture.

Character-owned durable facts must be interpreted under explicit versioned rules and must not be silently reinterpreted when a world/profile revision changes.

For a ruleset/profile revision upgrade:

```text
old accepted character facts
+ explicit source revision
+ explicit destination revision
+ deterministic validated migration/compatibility decision
-> new accepted character facts
```

Requirements:

- no in-place semantic drift simply because new server code was deployed;
- migration may be no-op when semantics are compatible, but compatibility must be explicit;
- irreversible migration requires backup/rollback/verification evidence in the owning durability/release gates;
- a world transfer to an incompatible profile/revision fails before changing current WorldId unless a dedicated migration contract supports it;
- cross-profile family character/value transfer remains forbidden by the accepted product baseline until separately accepted.

Exact migration data structures remain DUR-02/content/ruleset work.

## 12. Recommended Stage A — vocation, promotion, mastery and respec boundary

### RECOMMENDATION — owner decision required

Treat vocation/class/promotion/mastery/specialization as **versioned ruleset-owned character state**, not as foundation-hardcoded engine forks.

Do not freeze:

- the exact vocation/class roster;
- enum values;
- promotion levels;
- mastery trees;
- respec availability/cost;
- class-specific formulas.

Reference follows its selected target. Evolved changes require explicit ruleset/product revisions.

If respec exists, it must be an explicit authoritative domain operation with validation/audit and deterministic before/after state. There is no generic admin/client ability to edit progression fields arbitrarily.

## 13. Recommended Stage A — offline progression/training boundary

### RECOMMENDATION — owner decision required

Do not create implicit background progression as a foundation behavior.

Offline training/progression is an explicit **ruleset capability**:

```text
not enabled by contract
unless active ruleset explicitly defines it
```

If enabled later, its owning contract must define:

- trusted time source;
- maximum accrual horizon/bounds;
- eligibility;
- offline/online transition behavior;
- exact progression formula and rounding;
- retry/idempotency;
- ruleset migration behavior;
- abuse and clock-skew handling.

This keeps the architecture compatible with a Reference target that supports such a mechanic without inventing it before the target is selected.

## 14. Naming policy — architecture now, concrete Reference behavior later

### RECOMMENDATION — owner decision required

Freeze only these baseline-neutral naming invariants now:

- current name is mutable namespace state, not identity;
- Character Authority owns final normalization/reservation/conflict enforcement;
- one successful rename has one atomic winner under character/name namespace concurrency;
- public/private name history and redirects are separate product/privacy decisions;
- name recycling can never recycle CharacterId;
- Platform name-availability checks remain advisory unless backed by Character Authority.

Do **not** yet freeze:

- whether uniqueness is global, per world or another scope;
- exact normalization/case/allowed-character rules;
- reserved words;
- old-name hold duration;
- whether/when retired names are reusable;
- public rename history/redirect behavior.

Those are observable Reference/product semantics. Where the first Reference target governs them, Stage B must consume that exact target rather than guessing.

### DUR-02 consequence

Final uniqueness/index strategy must wait until the namespace scope is known. DUR-02 discovery may preserve the need for an atomic namespace authority but must not freeze the physical uniqueness key prematurely.

## 15. Character slots/quotas and entitlement boundary

### RECOMMENDATION — owner decision required

Freeze the authority split, not a number:

- Character Authority performs the final authoritative eligibility/quota check during creation;
- Platform may supply Platform-owned entitlement/commercial facts through an accepted contract when such entitlements exist;
- stale Platform projection cannot grant an extra character slot;
- exact quota number/scope and any Premium/VIP effect are not selected here;
- monetized entitlement behavior remains blocked on `PROD-ENTITLEMENTS-01` and its cross-repository rollout contract.

Reference-specific visible slot behavior waits for the exact Reference target where parity applies.

## 16. World transfer and account transfer product boundary

### RECOMMENDATION — owner decision required

Architecture should support the **capability** without assuming it is enabled at first launch.

If enabled:

### World transfer

- same CharacterId;
- one authoritative current WorldId after commit;
- source actor quiescent under the first-generation safety baseline;
- source stale writers fenced;
- destination profile/ruleset/content compatibility validated before commit;
- item/economy/house/guild/social restrictions are checked by their owning contracts;
- ambiguous outcome reconciles from authoritative transfer state rather than guessing;
- cross-profile family transfer remains forbidden until separately accepted.

### Account ownership transfer

- same CharacterId;
- one authoritative current AccountId after commit;
- actor quiescent and no current playable CharacterLease before commit;
- Platform Bazaar/commercial saga remains independent from game-owned transfer result;
- timeout never means ownership changed;
- retries cannot transfer twice.

Exact product eligibility, cooldowns, pricing, migration restrictions and launch availability remain later decisions.

## 17. What Stage A deliberately does not decide

Even if the baseline-neutral recommendations above are accepted, the following remain open:

- exact first Global Tibia Reference baseline;
- exact character name namespace and naming/recycling rules;
- exact character slot/quota values and visible entitlement behavior;
- exact creation choices and starter character facts;
- exact persistent progression catalogue required by the selected Reference target;
- level/XP/skills/stat/capacity formulas;
- exact vocation/class/promotion/mastery behavior;
- exact death/respawn/loss/blessing/protection rules;
- exact offline training/progression behavior;
- exact world/account transfer product availability and restrictions;
- physical PostgreSQL schema/constraints/indexes/locking;
- exact wire/API/UI representations;
- deterministic arithmetic/rounding/RNG details owned by simulation/ruleset gates;
- privacy erasure/retention timelines.

## 18. Stage B hard gate — exact first Reference baseline

### Finding

Full `GAME-CHAR-01` cannot be honestly marked accepted before the exact first Reference target is selected **to the extent that the target determines character-visible mechanics or durable vocabulary**.

This is not architecture paralysis. Stage A can define safe ownership/lifecycle/revision boundaries now. But Stage B must use the actual target as evidence for concrete Reference semantics.

### Reference-sensitive closure must at minimum reconcile

- name namespace/normalization/recycling behavior;
- creation choices/starter state;
- persistent progression categories and exact semantics;
- vocation/class/promotion state relevant to durability;
- death/respawn/progression-loss semantics;
- offline training/progression if present;
- slot/quota behavior where it is a Reference gameplay/product property;
- deterministic fixtures required by the selected mechanics.

If the exact target reveals that a supposedly baseline-neutral recommendation conflicts with required Reference behavior, the project must distinguish:

1. architecture invariant that remains mandatory for safety/integrity; versus
2. observable rule that belongs in the Reference profile.

Safety/integrity defects are never copied merely for parity.

## 19. DUR-02 allowed consumption before Stage B

### RECOMMENDATION — owner decision required

Bounded DUR-02 discovery may consume Stage-A invariants to design around:

- one CharacterId semantic aggregate;
- separate current AccountId and WorldId bindings;
- explicit lifecycle state;
- authoritative character revision/fence;
- ruleset/profile/content revision context where needed;
- separation of character-owned progression facts from item/social/economy/session authorities;
- idempotent mutation/operation evidence requirements;
- migration/rollback needs.

DUR-02 must **not** yet freeze:

- the final physical progression columns/types merely by guessing a Tibia target;
- final name unique-index scope;
- exact starter-state layout;
- exact death/progression fields;
- schema choices that make later Reference target semantics impossible or require destructive rearchitecture.

This preserves forward progress without letting persistence choose gameplay policy by accident.

## 20. Failure scenarios required by the final GAME-CHAR contract

Later accepted/implemented contracts must prove at least:

1. duplicate create retry produces one semantic character;
2. same-name race has one atomic winner under the selected namespace policy;
3. rename racing with login/projection updates cannot split identity;
4. schedule-delete racing with restore/finalize has one revision-linearized outcome;
5. terminal retirement never permits CharacterId reuse or later admission;
6. world transfer cannot create simultaneous source/destination current-world authority;
7. account transfer cannot create two account owners or bypass one-online-character/session fences;
8. login/admission racing with world/account transfer fails safely;
9. stale CharacterLease/GameSession cannot write after a character ownership/placement transition;
10. ruleset/profile migration is deterministic, auditable and does not silently reinterpret old state;
11. death consequence retry cannot apply the same character loss twice;
12. item loss/ownership effects remain conserved under the separate item transaction authority;
13. Platform stale projections after rename/delete/transfer cannot become authority;
14. privacy erasure does not cause CharacterId reuse or destroy required integrity/audit correlation contrary to policy.

## 21. Options for baseline-neutral closure

### Option A — minimal aggregate + three-state lifecycle + quiescent high-impact mutations

Accept the recommendations in sections 4-16 as Stage A.

**Benefits**

- smallest durable semantic surface;
- clear ownership and concurrency;
- avoids live-transfer complexity;
- keeps Reference formulas untouched;
- gives DUR-02 meaningful boundaries without guessing schema details;
- easy to test and later supersede with evidence.

**Costs**

- first implementation cannot support live world/account transfer;
- some convenient online rename/workflow behavior may be deferred;
- full GAME-CHAR still waits for exact Reference baseline.

### Option B — generalized workflow-rich character state machine now

Add transfer/rename/Bazaar/session/moderation substates directly to Character lifecycle and design online transitions up front.

**Benefits**

- more future workflows appear modeled immediately.

**Costs/risks**

- mixes authorities;
- expands durable schema before consumers exist;
- creates many invalid state combinations;
- violates vertical-slice bias;
- increases FND-04/DUR-02 coupling before runtime evidence.

### Option C — defer all GAME-CHAR decisions until exact Reference baseline

**Benefit**

- avoids any chance of Reference mismatch.

**Costs/risks**

- unnecessarily blocks DUR-02 discovery and character lifecycle safety decisions that are target-independent;
- persistence may remain unable to progress despite already accepted identity/authority contracts.

### Recommendation

**Option A.**

## 22. Recommended owner decision package

### RECOMMENDATION — NOT OWNER-ACCEPTED

Accept the following baseline-neutral Stage-A package as a **partial GAME-CHAR owner baseline**, while keeping full `GAME-CHAR-01` DecisionStatus unresolved until Stage B:

1. **Character aggregate boundary** — character-owned lifecycle/current owner/current world/name/progression/build/revision semantics; inventory/economy/social/session/etc. remain separate authorities.
2. **Minimal lifecycle** — `ACTIVE -> DELETION_SCHEDULED -> RETIRED`, restore only before terminal retirement; privacy erasure separate; CharacterId never reused.
3. **Atomic idempotent creation** — CharacterId + owner + world + final name + versioned starter/ruleset context + initial character facts become visible coherently.
4. **Character revision/fence** — separate from GameSession/CharacterLease; stale durable character mutations fail/reconcile.
5. **Quiescent high-impact mutations** — terminal retirement, world transfer and account transfer require actor `ABSENT` and no current playable lease in the first native architecture; online/live transfer deferred.
6. **Progression facts vs derived values** — authoritative ruleset-defined persistent facts are distinct from derived stats; no assumption of monotonic progression; exact formulas remain hard-gated.
7. **Death boundary** — character-owned consequences are idempotent/versioned but exact Reference death rules wait for Stage B; item effects remain item-transaction-owned.
8. **Ruleset/profile migration discipline** — no silent reinterpretation; explicit source/destination revisions and validated migration/compatibility.
9. **Vocation/build state is ruleset-owned** — no engine fork/hardcoded universal roster; respec only explicit authoritative operation if supported.
10. **Offline progression is opt-in ruleset capability** — no implicit background progression.
11. **Naming/slot authority only** — Character Authority remains final arbiter; exact namespace/recycling/quota values remain Reference/product-sensitive.
12. **Transfer capability, not launch promise** — stable CharacterId and one owner/world after commit; exact eligibility/availability later.
13. **DUR-02 may consume Stage-A invariants only**; final Reference-sensitive schema/fixtures cannot guess Stage-B mechanics.
14. **Full GAME-CHAR remains NOT ACCEPTED** until the exact first Reference baseline is selected and Reference-sensitive character rules are reconciled.

## 23. What owner acceptance would do

If the owner accepts the Stage-A package, a dedicated partial owner baseline should record it and current coordination should state:

```text
GAME-CHAR-01
DecisionStatus      = PARTIAL / Stage A accepted
DeliveryStatus      = lifecycle of that partial baseline
ImplementationStatus = NOT_STARTED
Runtime authority   = NONE

Stage B
= BLOCKED on exact first Reference baseline
```

The exact status vocabulary should follow `ARCHITECTURE_STATUS_MODEL.md`; if that model does not permit `PARTIAL`, use a dedicated `OWNER_ACCEPTED PARTIAL BASELINE` document while the gate row remains `PROPOSED`/not fully accepted.

After Stage-A acceptance, the next material owner/product decision for GAME-CHAR is no longer another abstract character architecture question. It is the **exact first Reference baseline**, followed by evidence-backed reconciliation of Reference-sensitive character semantics.

## 24. What rejection or partial modification would mean

All already accepted identity/authority/session baselines remain unchanged.

The owner may replace only the unresolved Stage-A recommendation, for example:

- require terminal deletion to remain restorable under a separately defined retention model;
- permit online rename in the first architecture;
- require a live world-transfer architecture from day one;
- move a named state category into/out of the Character aggregate;
- choose a different deletion lifecycle model.

Any such choice should name the downstream requirement that justifies the extra complexity.

## 25. Deliberately not decided here

- exact first Global Tibia Reference version/date/behavior target;
- exact Reference gameplay formulas or content;
- physical schema/SQL;
- implementation crate/service layout;
- client UI/UX;
- private service API/wire encoding;
- exact timings/quotas/numeric limits;
- runtime or production rollout;
- any Evolved progression/death redesign.

Until the product owner explicitly accepts or replaces the Stage-A package, this document remains **PRE-DECISION ANALYSIS / NOT ACCEPTED** and creates no new architecture authority.
