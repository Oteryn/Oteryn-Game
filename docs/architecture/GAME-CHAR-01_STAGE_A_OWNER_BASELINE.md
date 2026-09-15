# GAME-CHAR-01 — Stage A Baseline-Neutral Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-11
- Owner decision time: 18:28 +02:00
- Repository recording date: 2026-08-11
- Gate: `GAME-CHAR-01`
- Scope: baseline-neutral character lifecycle/progression architecture only
- Source type: `USER_SOURCE`
- Overall `GAME-CHAR-01` DecisionStatus: **PROPOSED** until Reference-sensitive Stage B is accepted
- Overall `GAME-CHAR-01` DeliveryStatus after this partial-baseline lifecycle closes: **PLANNED**
- ImplementationStatus: **NOT_STARTED**
- Runtime authority: **NONE**
- Does not authorize: runtime/client/protocol/persistence-schema/content/Platform implementation, production rollout, exact Reference target selection, exact character formulas/rules or acceptance of Stage B

## 1. Purpose

Persist the product owner's explicit acceptance of the complete recommended Stage-A package in section 22 of `GAME-CHAR-01_PREDECISION_ANALYSIS.md`.

Stage A freezes only character semantics that are safe and useful independently from the exact first Global Tibia Reference baseline. It gives bounded `DUR-02` discovery a trustworthy semantic envelope without allowing persistence layout, technical convenience or guessed Tibia behavior to become product policy.

Stage A does **not** complete `GAME-CHAR-01`. Reference-sensitive Stage B remains hard-blocked on selection of the exact first Reference baseline.

## 2. Owner source and acceptance

### USER_SOURCE — accepted 2026-08-11 18:28 +02:00

The owner was presented with the complete recommended Stage-A package after the nonbinding pre-decision dossier had been delivered and lifecycle-closed. The owner was asked whether the complete recommended `GAME-CHAR-01 Stage A` should be accepted.

The owner explicitly answered:

> tak

This acceptance applies to the complete fourteen-item package in section 22 of `GAME-CHAR-01_PREDECISION_ANALYSIS.md` as one coherent baseline-neutral decision.

It does not accept any Stage-B subject, any exact Reference behavior, any physical schema or any implementation.

## 3. Status-model rule

`ARCHITECTURE_STATUS_MODEL.md` defines only `PROPOSED`, `CANDIDATE`, `ACCEPTED` and `SUPERSEDED` for `DecisionStatus`; it contains no `PARTIAL` value.

Therefore this document is the binding record for its declared partial scope, while the **overall gate remains**:

```text
GAME-CHAR-01
DecisionStatus       = PROPOSED
DeliveryStatus       = PLANNED after Stage-A task lifecycle closeout
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
```

Do not infer that the whole character gate is accepted merely because this partial baseline is binding.

## 4. Already accepted parent boundaries remain authoritative

Stage A consumes and does not reopen:

- `CharacterId` as a game-owned strongly typed UUIDv7 durable semantic identity;
- stable CharacterId through rename, legal world transfer and legal account transfer;
- CharacterId non-reuse after terminal retirement/deletion;
- Oteryn-v2 Character Authority ownership of character lifecycle, current AccountId binding, current WorldId membership and final name reservation;
- Platform ownership of AccountId/authentication/commercial orchestration and prohibition on steady-state direct native character-table writes;
- at most one authoritative online character for one AccountId;
- FND-04 ownership of AccountPresenceClaim, CharacterLease, GameSession, TransportBinding and control/recovery authority;
- `DUR-02` ownership of physical persistence schema/transactions/migrations/recovery;
- `GAME-ITEM-01` / `DUR-03` ownership of item semantics, conservation, inventory/equipment/container and value-transfer invariants;
- GAME-VISION Reference parity precedence and the hard gate against guessing concrete Reference semantics.

## 5. Accepted Stage-A decision 1 — bounded Character aggregate

The semantic Character aggregate owns character-owned concerns only.

It may own or carry authoritative character facts in these categories where the active ruleset/profile requires them:

1. identity and semantic lifecycle;
2. current validated AccountId ownership binding;
3. current logical `WorldId` membership;
4. current name state plus revision/reservation metadata required by the eventual naming policy;
5. authoritative persisted character-progression facts;
6. character build/profession facts such as vocation/class/promotion/mastery/specialization where defined by the active ruleset;
7. character-scoped long-lived capability/unlock facts that semantically belong to the character rather than another aggregate;
8. character revision/fencing and ruleset/profile/content revision context required for safe mutation and migration.

This is a semantic ownership list, **not** a PostgreSQL column list.

### Explicit exclusions

Character does not become the universal MMO aggregate merely because another domain references CharacterId. Separate authorities remain for at least:

- item instances, inventory, equipment, containers, item provenance and item conservation;
- market, trade, bank, depot and other value-transfer transactions;
- houses;
- party/guild/friends/presence and other social aggregates;
- independently owned quest/content/encounter durable aggregates;
- Platform entitlements/commercial state;
- active GameSession/CharacterLease/control authority;
- runtime combat/AI/instance state except for a durable character consequence explicitly committed through the owning gameplay transaction.

References and projections do not transfer semantic ownership.

## 6. Accepted Stage-A decision 2 — minimal semantic lifecycle

The baseline-neutral semantic lifecycle is:

```text
successful create
      |
      v
    ACTIVE
      |
      | schedule deletion, if product/ruleset supports it
      v
DELETION_SCHEDULED
      |              \
      | finalize      \ restore/cancel before finalization
      v                \
    RETIRED <----------- ACTIVE
```

### `ACTIVE`

The semantic character exists and may participate in ordinary product workflows subject to current account/world/session/ruleset restrictions. `ACTIVE` does not mean online.

### `DELETION_SCHEDULED`

A reversible deletion intent has been accepted, but terminal retirement has not occurred. Exact grace duration, visibility and allowed gameplay actions remain Stage-B/product/privacy-owned where applicable.

### `RETIRED`

Terminal semantic retirement means:

- ordinary gameplay admission is impossible;
- CharacterId is never reassigned or reused;
- restoration to the same active semantic character is not available after terminal retirement under this first baseline;
- integrity/security/legal/privacy policy may retain the minimum tombstone/audit/provenance evidence required by its owning contract;
- privacy erasure is a separate data-lifecycle concern and must not imply identity reuse.

Transient facts such as `ONLINE`, `IN_COMBAT`, `DISCONNECTED`, `RENAME_PENDING`, `TRANSFER_PENDING`, `BAZAAR_LISTED` or `BANNED` are not permanent Character lifecycle states by default. Their owning session/runtime/operation/commercial/moderation contracts remain separate unless later evidence proves a durable Character-lifecycle state is required.

## 7. Accepted Stage-A decision 3 — atomic idempotent creation

Character creation is one authoritative, idempotent Character Authority operation. A character must not become externally usable as a partially initialized semantic object.

A successful creation conceptually binds coherently:

```text
fresh CharacterId
+ owner AccountId
+ initial WorldId
+ final authoritative name reservation
+ applicable profile/ruleset/content/starter-template revision context
+ initial authoritative character-owned progression/build facts
+ initial character revision
```

Required invariants:

- ambiguous-response retry cannot create a second semantic character;
- final name reservation/conflict resolution belongs to the authoritative operation, not an advisory Platform preview;
- starter state derives from an explicit versioned ruleset/content template rather than hidden client/server defaults;
- client-provided initial values are intent, not authority;
- applicable quota/eligibility is revalidated at the authoritative commit boundary;
- failed creation cannot expose a half-character as a valid product entity.

Exact creation choices, starter facts, naming rules and quota values remain Stage-B/product-sensitive.

## 8. Accepted Stage-A decision 4 — Character revision/fence

Authoritative durable Character state has a monotonic mutation revision/fence sufficient to reject or reconcile stale Character mutations.

This character-state revision is semantically distinct from:

- CharacterId identity;
- FND-04 CharacterLease generation;
- GameSessionId;
- connection generation;
- FND-03 runtime ownership generation;
- ruleset/content/profile revisions.

A stale durable Character mutation must not overwrite a newer accepted state merely because its session, operation or transport once held valid authority.

Exact physical representation, locking strategy, compare-and-swap/update mechanism and database transaction design belong to `DUR-02`.

## 9. Accepted Stage-A decision 5 — quiescent first-generation high-impact mutations

For the first native architecture, these high-impact mutations require the player-controlled actor to be quiescent before the authoritative mutation commits:

- terminal retirement/final deletion;
- world transfer;
- account ownership transfer.

Minimum safe condition:

```text
actor presence = ABSENT
AND no current playable CharacterLease/control authority
```

This is deliberately conservative. It prevents the first persistence/session design from needing live cross-world/account migration while a runtime actor is still authoritative.

Online/live transfer is deferred and requires a later explicit architecture decision with session, runtime, durability, item/economy/social and rollback evidence.

## 10. Accepted Stage-A decision 6 — progression facts versus derived values

Persisted **authoritative character progression facts** are distinct from values that can be deterministically derived under an explicit ruleset/profile revision.

Stage A does not assume that every progression fact is numerically monotonic. A valid ruleset may define gains, losses, resets, caps, conversion or migration behavior.

Architecture must avoid persisting duplicated derived state merely for convenience when it can be deterministically recomputed from authoritative facts and a known compatible rule revision, unless an owning performance/history/audit requirement justifies a stored projection.

Exact level, experience, skill, attribute, capacity, derived-stat formulas, rounding and limits remain Stage B / simulation / ruleset owned.

## 11. Accepted Stage-A decision 7 — death consequence boundary

A death or equivalent progression-loss outcome may commit **character-owned consequences** only through an idempotent, revision-aware authoritative operation/transaction boundary.

A retry or ambiguous response must not apply the same character loss twice.

Stage A does not define Reference death, respawn, experience/skill loss, blessing/protection or PvP death formulas.

Item loss, corpse/loot ownership, equipment/inventory conservation and value transfer remain under the appropriate item/combat/durability authorities and must not be duplicated as Character-owned state merely because death triggered them.

## 12. Accepted Stage-A decision 8 — explicit ruleset/profile migration

Existing durable Character state is never silently reinterpreted merely because a server starts with a new ruleset/profile/content revision.

Any incompatible progression/build semantic change requires an explicit migration or compatibility decision with at least:

- source revision/context;
- destination revision/context;
- deterministic validation/transformation where transformation is required;
- idempotency and revision/fencing;
- auditable outcome;
- rollback/forward-only policy owned by the migration contract;
- fail-closed behavior when compatibility cannot be proven.

Reference revisions remain immutable releases under GAME-VISION; promotion to a later named Reference revision is an explicit product/release action, not silent state drift.

## 13. Accepted Stage-A decision 9 — vocation/build state is ruleset-owned

Character build/profession state is versioned ruleset/profile-owned semantic state, not an excuse for separate engines, protocols or hard-coded universal vocation/class assumptions.

The engine must support the accepted profile architecture without assuming every profile has the same vocation roster, promotion/mastery graph or respec capability.

If respec or equivalent transformation is supported, it is an explicit authoritative operation with migration/consequence semantics, not client-side presentation state.

Exact Reference vocation/class/promotion behavior remains Stage B.

## 14. Accepted Stage-A decision 10 — offline progression is opt-in

Offline training/progression is **not** an implicit engine behavior.

It exists only when explicitly enabled and defined by an active ruleset/product capability. Its inputs, timing, caps, resources, interruption, migration and durable effects must be deterministic and auditable under the later owning decision.

Absence of an accepted capability means no background progression should be invented by persistence or runtime code.

Exact Reference offline-training behavior remains Stage B where applicable.

## 15. Accepted Stage-A decision 11 — naming and quota authority only

Character Authority remains the final arbiter for authoritative name reservation/conflict decisions and game-domain character quota/eligibility decisions.

Stage A intentionally does **not** freeze:

- name namespace scope;
- Unicode/canonicalization/normalization details;
- name recycling/history/redirect policy;
- reserved-word policy;
- exact account/world character-slot numbers;
- Premium/entitlement effects on slots or creation;
- observable Reference naming/quota behavior.

Those values/rules remain Reference/product/privacy/entitlement-sensitive as appropriate.

## 16. Accepted Stage-A decision 12 — transfer is a capability, not a launch promise

The architecture preserves safe future **capability** for legal world transfer and legal account ownership transfer without asserting that either feature is enabled at first launch.

If enabled later:

### World transfer

- CharacterId remains stable;
- exactly one authoritative current WorldId exists after commit;
- first-generation transfer uses the quiescent actor rule;
- stale source-world character mutation is fenced;
- destination profile/ruleset/content compatibility is validated before commit;
- item/economy/house/guild/social consequences remain with their owning contracts;
- ambiguous outcomes reconcile from authoritative operation/current-world state;
- cross-profile-family transfer remains forbidden until separately accepted.

### Account ownership transfer

- CharacterId remains stable;
- exactly one authoritative current AccountId binding exists after commit;
- first-generation transfer uses the quiescent actor/no-playable-lease rule;
- Platform Bazaar/commercial state remains a separate saga from the game-owned ownership mutation;
- timeout is not proof that ownership changed;
- retries cannot apply ownership transfer twice.

Exact launch availability, eligibility, cooldown, price and migration restrictions remain later decisions.

## 17. Accepted Stage-A decision 13 — bounded DUR-02 consumption

Before Stage B, bounded `DUR-02` discovery may consume these Stage-A invariants:

- one semantic Character aggregate with the bounded ownership categories above;
- explicit current AccountId and WorldId bindings;
- explicit semantic lifecycle state;
- character revision/fence distinct from session/lease generations;
- explicit profile/ruleset/content revision context where compatibility/migration requires it;
- separation of Character-owned progression/build facts from item/economy/social/session authorities;
- idempotent mutation/operation evidence needs;
- migration/rollback/fencing requirements.

Before Stage B, `DUR-02` must **not** freeze:

- final physical progression columns/types based on a guessed Tibia target;
- final name unique-index scope or normalization semantics;
- final starter-state layout;
- exact death/progression fields or formula-specific persistence;
- physical structures that make the later selected Reference semantics impossible without destructive rearchitecture.

Final character-bearing `DUR-02` schema remains blocked on full `GAME-CHAR-01` acceptance.

## 18. Accepted Stage-A decision 14 — full GAME-CHAR remains open

Stage A does not accept the complete `GAME-CHAR-01` gate.

Reference-sensitive Stage B remains mandatory before the gate may become `DecisionStatus=ACCEPTED` and before final Reference character schema/fixtures or broad progression implementation.

At minimum Stage B must reconcile the selected exact Reference target for:

- name namespace/normalization/recycling behavior;
- creation choices and starter state;
- persistent progression catalogue and exact semantics;
- vocation/class/promotion state relevant to durability;
- death/respawn/progression-loss/blessing/protection behavior;
- offline training/progression where present;
- slot/quota behavior where Reference-visible;
- deterministic fixtures/formulas under their owning gameplay/simulation contracts.

If Stage B evidence conflicts with a Stage-A rule, the project must distinguish a safety/integrity architecture invariant from an observable Reference rule and use an explicit superseding decision where required. Defect compatibility never overrides safety/integrity.

## 19. Failure scenarios retained for final GAME-CHAR/DUR work

Stage A requires later design/implementation evidence to preserve at least:

1. duplicate create retry creates one semantic character;
2. same-name race has one authoritative winner under the later selected namespace policy;
3. rename racing with admission/projection cannot split CharacterId identity;
4. schedule-delete racing with restore/finalize has one revision-linearized result;
5. RETIRED CharacterId is never reused or admitted;
6. world transfer cannot create two current-world authorities;
7. account transfer cannot create two owners or bypass account/session exclusion;
8. login/admission racing with world/account transfer fails safely;
9. stale session/lease/operation cannot overwrite post-transition Character state;
10. ruleset/profile migration is deterministic/auditable and never silently reinterprets incompatible old state;
11. death-consequence retry cannot apply the same Character-owned loss twice;
12. item effects remain conserved under separate item transaction authority;
13. stale Platform projections after rename/delete/transfer do not become authority;
14. privacy erasure never causes CharacterId reuse or destroys integrity evidence contrary to accepted retention policy.

Exact executable tests remain owned by the later implementation contracts.

## 20. Deliberately unresolved after Stage A

Stage A leaves unresolved:

- exact first Global Tibia Reference patch/date/behavior baseline;
- exact Reference naming, creation, slot/quota and starter behavior;
- exact persistent progression vocabulary required by that Reference target;
- XP/level/skill/attribute/capacity formulas and rounding;
- exact vocation/class/promotion/mastery/respec mechanics;
- exact death/respawn/loss/blessing/protection mechanics;
- exact offline-training mechanics;
- exact world/account transfer product availability and restrictions;
- PostgreSQL DDL, indexes, locking/isolation/retry and migration implementation;
- exact API/wire/UI representation;
- deterministic arithmetic/RNG details owned by simulation/ruleset gates;
- privacy erasure/retention timelines;
- Evolved progression/death redesign.

## 21. Programme consequence

After this partial baseline becomes canonical and its delivery lifecycle closes:

```text
GAME-CHAR-01 Stage A
-> OWNER-ACCEPTED PARTIAL BASELINE
-> binding within declared baseline-neutral scope

GAME-CHAR-01 overall
-> PROPOSED / PLANNED / NOT_STARTED

next material GAME-CHAR input
-> select exact named first Reference baseline
-> reconcile Reference-sensitive Stage B

parallel work already permitted
-> GAME-CHANNEL-01 architecture
-> bounded DUR-02 discovery using Stage-A invariants only
```

No runtime, schema, gameplay implementation, Platform write, production activation or first-launch feature promise is authorized by this baseline.

## 22. Supersession rule

A later change to a Stage-A decision requires an explicit superseding owner decision with named evidence. Appropriate evidence may include:

- selected Reference-baseline semantics demonstrating a genuine conflict;
- durability/session failure analysis showing the baseline is unsafe or insufficient;
- implementation/scale evidence showing a different boundary is required;
- legal/privacy constraints;
- explicit product-owner strategy change.

Implementation convenience alone is insufficient.
