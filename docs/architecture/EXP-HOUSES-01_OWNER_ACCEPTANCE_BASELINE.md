# EXP-HOUSES-01 — Housing Architecture Owner Acceptance Baseline

- Status: **OWNER-ACCEPTED WHOLE-GATE ARCHITECTURE**
- DecisionStatus: `ACCEPTED`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-09-08
- Gate: `EXP-HOUSES-01`
- Owner disposition: `ACCEPT`
- Coordination issue: `#220`
- Protected composition base: `main@351f4a7d47419f25009b759a68fd2833ccd73260`
- Runtime/client/server/protocol/DDL/migration/Platform/production authority: **NONE**

## 1. Purpose and status axes

This baseline composes the owner-approved Oteryn housing decisions into one normative `EXP-HOUSES-01` architecture package.

It closes the first-generation housing **semantic architecture** sufficiently for later implementation planning while deliberately leaving numeric balance, physical schema, service decomposition, exact protocol/UI representation and rollout details tunable or downstream-owned.

The status axes are intentionally separate:

```text
DecisionStatus       = ACCEPTED
DeliveryStatus       = IN_REVIEW
ImplementationStatus = NOT_STARTED
```

Owner acceptance of this architecture does not authorize executable implementation.

## 2. Composition sources and precedence

This package consumes:

- protected Oteryn Game architecture on the composition base;
- Issue `#220`, the active owner-driven architecture continuation;
- owner-selected housing checkpoints in draft PRs `#435`, `#436`, `#437`, `#438`, `#440`, `#442`, `#443`, `#444`, `#445` and `#446`;
- `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`;
- `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`;
- `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`;
- `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`;
- accepted FND/DUR/GAME-CHANNEL/GAME-CHAR/GAME-ITEM/entitlement authority boundaries.

The checkpoint PRs remain owner-decision provenance. Once this composition is canonically integrated, this baseline supersedes their housing semantics as the single whole-gate authority; they should not be independently merged as competing long-lived housing baselines.

Where older architecture says housing topology or `EXP-HOUSES-01` is still unresolved, this later owner-accepted baseline wins for the exact semantic scope accepted here.

## 3. Accepted product model

First-generation Oteryn uses a hybrid housing model:

```text
one logical World
    |
    +-- finite scarce physical HouseId addresses
    |      - visible/persistent World addresses
    |      - one world-global property state
    |      - public World auction
    |      - recurring rent / grace / eviction
    |      - Premium + PhysicalHouseEligibility at acquisition
    |      - canonical ordinary owner = CharacterId
    |
    +-- non-scarce Residence / Apartment capability
           - genuine personal housing
           - available without Premium
           - one per AccountId + WorldId at most
           - no scarcity auction
           - no speculative resale market at first generation
```

Personal housing on one Account/World is mutually exclusive:

```text
PersonalHousingSlot(AccountId, WorldId) =
    NONE
  | RESIDENCE
  | PHYSICAL_HOUSE
```

A player cannot hold both a Residence and an ordinary physical house on the same `AccountId + WorldId`.

Guildhouses are a separate guild-owned class under `GuildId`; detailed guild lifecycle remains deferred to the guild/social architecture.

## 4. Binding World / Channel invariants

### 4.1 World-global physical property

Every physical `HouseId` belongs to exactly one `WorldId`.

`HouseId` does not contain and is not semantically scoped by `ChannelId`.

The following state exists once for the property across all Channels of the World:

- ownership;
- auction/allocation lifecycle;
- rent/delinquency/eviction;
- ACL/access revisions;
- durable house item/container state.

Adding or removing a Channel must not create houses, copy houses, reset rent, duplicate auctions or change physical-house scarcity.

### 4.2 Rejected Channel models

First generation explicitly rejects:

- one independent physical-house copy per Channel;
- Channel-local `HouseId`;
- Channel-local ownership/rent/ACL;
- separately writable regional mirrors of one physical-house interior;
- using infrastructure/channel scaling to manufacture more copies of scarce addresses.

### 4.3 Item/value invariants

Housing does not create a parallel item identity or conservation system.

Accepted `GAME-ITEM-01` / `DUR-03` semantics remain binding:

- every durable item has one authoritative semantic location/custody state;
- item moves are ordered, revisioned and retry-safe where required;
- duplicate retries do not create duplicate item/value mutations;
- ambiguous commit requires reconciliation rather than a blind new operation;
- stale writers cannot overwrite a newer authoritative state.

## 5. Physical-house runtime topology

### 5.1 One authoritative active interior

When a physical house is active, it has one logical authoritative world-scoped interior runtime owner.

Characters entering the same `HouseId` from different Channels converge on the same authoritative interior/presence/item state rather than entering independent copies.

### 5.2 Reuse of instance mechanics

Implementation may reuse accepted instance-style transfer/runtime primitives, but canonical property identity remains `HouseId`.

Any internal `InstanceId`, runtime ID or generation is runtime-lifecycle identity only and may not replace `HouseId` in:

- ownership;
- auction;
- rent;
- ACL;
- property history;
- durable item semantics.

### 5.3 Entry and origin Channel

Entering a physical house is an explicit authoritative context/ownership handoff, not a hidden Channel switch.

The Character retains validated origin-Channel routing metadata.

Normal exit returns through validated origin routing unless a separately accepted same-World fallback is necessary because the origin Channel is unavailable, draining or incompatible.

House entry/exit may not bypass:

- combat/PvP Channel-switch restrictions;
- admission/capacity rules;
- session/lease fencing;
- protected transactions;
- revision compatibility;
- duplicate-session prevention.

### 5.4 Recovery and one writer

One active house interior has one logical authoritative mutation owner at a time.

Crash recovery, replacement or placement change requires generation/revision fencing so an old runtime cannot resume mutation after a newer owner becomes authoritative.

Durable ownership/item truth remains in the World durable domain. Rebuildable runtime presence may be restored only from accepted authoritative durable/content state.

## 6. Residence / Apartment model

### 6.1 Product role

A Residence is a separate, non-scarce personal housing capability intended to give Free/non-Premium players a genuine home without creating new copies of scarce physical World addresses.

Baseline Residence utility direction includes:

- persistent personal interior;
- decoration/personalization;
- graphical access management;
- baseline social/housing utility;
- compatibility with accepted Rested semantics.

It is not a crippled Premium preview.

### 6.2 Account / World scope

Residence entitlement is bounded to at most one Residence per `AccountId + WorldId`.

Account scope does **not** by itself authorize unrestricted item transfer or a shared warehouse between alternate Characters. Exact access to placed items/storage across Characters remains item/storage policy and must preserve `DUR-03` conservation/authority.

### 6.3 No scarcity market

First-generation Residence does not require:

- a public scarcity auction;
- player-to-player resale;
- nomination/transfer of a scarce address;
- a market price driven by finite property supply.

### 6.4 Runtime boundary

A Residence may later use an instance-style runtime primitive, but:

- active Residence simulation has one authoritative runtime owner;
- Channel scaling cannot duplicate Residence contents;
- durable value remains under accepted World durable authority;
- origin routing and simulation-owner transitions are explicit/fenced.

Exact `ResidenceId`, runtime ID, hosting placement, templates and persistence representation remain deferred.

## 7. Personal housing-slot invariant

### 7.1 Capacity rule

For each `AccountId + WorldId`:

- at most one Residence;
- at most one ordinary physical house;
- never both simultaneously;
- alternate Characters cannot bypass the aggregate slot;
- another World has an independent slot.

This aggregate guard does not replace the canonical physical-house owner relation `HouseId -> CharacterId`.

### 7.2 Residence -> physical house

An Account with a Residence may acquire/retain a physical house only through a value-safe authoritative replacement settlement.

Before the physical-house slot commit:

1. current slot state is revalidated;
2. Residence item/value state is moved to accepted authoritative custody/location where required;
3. stale Residence runtime authority is fenced;
4. Residence lifecycle/entitlement state is released;
5. physical-house ownership consumes the slot.

The durable final state must never contain both personal housing classes.

### 7.3 Physical house -> Residence

A Residence cannot become authoritative while the same Account/World still consumes the physical-house slot.

Physical-house ownership and durable contents must first be safely relinquished/evicted/disposed under accepted housing/item semantics.

### 7.4 Failure/retry

A failed or ambiguous housing-class transition is not success.

Recovery rereads authoritative slot/property/item/operation state and reconciles using the same logical operation identity. It must not invent a new blind attempt that can duplicate ownership or value movement.

## 8. Ordinary physical-house owner identity

The canonical public/semantic owner of an ordinary physical `HouseId` is one `CharacterId` in the owning World.

`AccountId` is used privately/aggregately for:

- personal housing-slot enforcement;
- acquisition eligibility;
- anti-concentration;
- anti-abuse controls.

It is not substituted for the canonical `HouseId -> CharacterId` relation.

Consequences:

- Character rename preserves ownership;
- multiple Characters of one Account cannot each own a same-World ordinary physical house;
- Character Bazaar requires explicit housing disposition;
- terminal Character deletion/finalization requires housing settlement first;
- World transfer cannot move a physical `HouseId` to another World.

## 9. Guildhouse boundary

Canonical guildhouse owner identity is `GuildId`.

A guildhouse does not consume the personal `AccountId + WorldId` slot.

Selling a guild leader Character does not transfer guildhouse ownership merely because that Character changes Account owner.

Detailed guildhouse rules are deliberately deferred until the guild system exists, including:

- bid/acquisition authority;
- leadership succession;
- guild-rank ACL inheritance;
- rent funding;
- guild dissolution/merge behavior;
- Rested semantics;
- world-transfer/Bazaar consequences.

## 10. PhysicalHouseEligibility and Premium

Acquiring an ordinary scarce physical house requires BOTH:

```text
active Premium
+
PhysicalHouseEligibility
```

Neither gold alone nor Premium alone is sufficient.

`PhysicalHouseEligibility` must represent meaningful real participation/progression on the relevant World so a trivial fresh account cannot immediately become a cheap property-holding shell.

Exact thresholds remain versioned/tunable. Possible input classes may include:

- account maturity;
- Character progression;
- real activity/play history;
- conduct/security state;
- another explicit game-owned qualification rule.

The rule applies when the Account will actually **acquire or retain a new physical-house slot**, including:

- public auction settlement;
- Character Bazaar when the buyer elects `KEEP_INCOMING_HOUSE`.

### 10.1 Premium lapse after acquisition

Expiration of Premium after a legitimate physical-house acquisition does **not** by itself evict the owner or release the property.

Property retention is governed by explicit in-game rent/grace/eviction/lifecycle rules.

A later change making Premium lapse an ownership-loss trigger requires explicit owner supersession.

## 11. Public physical-house auction

Vacant ordinary physical houses enter one World-scoped public auction/allocation lifecycle.

Vacancy may result from:

- newly available property;
- voluntary relinquishment;
- Bazaar disposition;
- value-safe eviction;
- another later explicit lifecycle cause.

### 11.1 Proxy bidding

First-generation physical-house auction uses proxy semantics:

- bidder submits a private maximum;
- automatic bidding advances only when actual competition requires it;
- automatic bidding never exceeds that bidder's maximum;
- winner pays the minimum valid amount required to beat the next-best competing maximum under the later-defined increment rule, not automatically the full maximum.

### 11.2 Funds backing

A winning-capable bid must be backed by authoritative funds reservation/escrow semantics before final value consumption.

Housing does not invent a parallel money authority; future economy/bank contracts and accepted value conservation remain binding.

### 11.3 Anti-sniping

Auction close uses extension-based anti-sniping semantics.

A valid bid near the close can extend the auction rather than making last-millisecond network timing the dominant winner-selection mechanic.

Exact auction duration, extension window and bid increment remain balance constants.

### 11.4 Settlement properties

Auction settlement must be:

- authoritative;
- idempotent;
- revision/fence aware;
- retry/reconciliation safe;
- compatible with the personal slot;
- compatible with final Premium + eligibility revalidation;
- value-conserving.

Timeout/unavailability is not proof that property or funds changed owner.

### 11.5 No generic direct property market at launch

First-generation Oteryn does not expose a generic player command/API equivalent to:

```text
SellPhysicalHouseDirectlyTo(CharacterId buyer, arbitrary_price)
```

Vacant physical houses return to public allocation.

A future direct property market may be added only by explicit supersession if real player/economy evidence justifies the extra speculation/abuse surface.

## 12. Rent, delinquency and eviction

Physical-house rent is recurring and World-scoped.

Exact rent amount/formula, cadence and funding-source representation remain deferred.

### 12.1 Automatic collection

Collection is automatic from a later-defined authoritative economy funding source and must be idempotent/reconcilable.

Channel count/current Channel/runtime placement cannot duplicate a rent charge or reset the rent lifecycle.

### 12.2 Grace

Insufficient funds enter an explicit delinquent/grace state rather than causing immediate silent eviction.

Exact grace duration, warnings and notification cadence remain deferred.

### 12.3 Value-safe eviction

If grace expires unresolved, eviction is an authoritative revision-fenced settlement.

Before ownership release:

1. current owner/revision/rent state is revalidated;
2. outgoing durable items/value are moved to safe authoritative reclaim/depot/custody;
3. stale HouseRuntime/Channel mutation rights are fenced;
4. ownership and ACL are released;
5. `HouseId` becomes vacant;
6. property returns to the public World auction/allocation lifecycle.

Forgotten ordinary items are not destroyed and do not silently become property of the next owner.

## 13. Character Bazaar housing disposition

Character Bazaar commercial workflow remains Platform-owned; Character ownership rebinding remains Game Character Authority-owned. Housing owns the housing disposition/slot/property effects required by the sale.

A physical house never follows a Character sale silently.

Seller explicitly chooses:

```text
BazaarHousingDisposition =
    RELINQUISH_HOUSE
  | INCLUDE_HOUSE_WITH_CHARACTER
```

### 13.1 RELINQUISH_HOUSE

Before Character ownership transfer can safely complete:

- current house owner/revision is revalidated;
- durable house value is safely settled;
- stale runtime writers are fenced;
- `HouseId` is released;
- property becomes vacant and enters public allocation;
- Bazaar reconciles from authoritative Character/housing operation state.

Failure cannot leave a sold Character with ambiguous property ownership.

### 13.2 INCLUDE_HOUSE_WITH_CHARACTER

If seller includes the house:

```text
CharacterId C owns HouseId H
AccountId A owns CharacterId C

Bazaar transfers CharacterId C to AccountId B

=> CharacterId C remains canonical owner of HouseId H
=> Character ownership binding changes A -> B
=> AccountId B must resolve its same-World personal housing slot
```

This is an explicit Character+house Bazaar case, not a standalone sale of `HouseId`.

### 13.3 Buyer choice: existing physical house

If buyer Account already holds another ordinary physical house on the same World, buyer must explicitly choose:

```text
KEEP_EXISTING_HOUSE
or
KEEP_INCOMING_HOUSE
```

Only one may remain authoritative after settlement.

The non-kept house is value-safely relinquished and returns to public allocation.

If buyer chooses `KEEP_INCOMING_HOUSE`, final Premium + `PhysicalHouseEligibility` revalidation applies because a new incoming physical-house slot is being retained.

If buyer chooses `KEEP_EXISTING_HOUSE`, the incoming house is relinquished; the Character purchase itself does **not** require Premium/eligibility merely because the listing originally contained a house. Existing-house retention remains governed by its existing ownership/rent lifecycle, including the accepted rule that Premium lapse alone does not evict.

A physical house on another World does not conflict with this per-World slot.

### 13.4 Buyer choice: Residence

If buyer Account holds a Residence on the same World, buyer must explicitly choose:

```text
KEEP_RESIDENCE
or
KEEP_INCOMING_HOUSE
```

If `KEEP_INCOMING_HOUSE`:

- active Premium + `PhysicalHouseEligibility` are revalidated;
- Residence value is safely settled/released;
- incoming physical house becomes the single personal housing class.

If `KEEP_RESIDENCE`:

- Residence remains the personal housing class;
- incoming physical house is value-safely relinquished to public allocation;
- Premium/physical-house eligibility is not required merely to purchase the Character because the buyer is not retaining a physical house.

### 13.5 Bazaar reconciliation

No distributed ACID between Platform, Character, housing and economy/item domains is assumed.

The saga must nevertheless preserve one semantic outcome through:

- stable operation identity;
- authoritative current-state revalidation;
- fencing;
- idempotent steps;
- explicit intermediate custody;
- durable receipts/state where required;
- reconciliation after timeout/ambiguity.

Platform listing/cache state is never proof that current physical-house acquisition is still eligible.

## 14. Bazaar contents: seller decides

For `INCLUDE_HOUSE_WITH_CHARACTER`, seller additionally chooses:

```text
HOUSE_ONLY
or
INCLUDE_FURNISHINGS
```

### 14.1 HOUSE_ONLY

The Character and physical address may transfer, but ordinary movable seller-owned house contents do not silently become buyer property.

Affected items are safely moved to seller/reclaim/depot/custody before the ownership settlement requires them to leave the house.

### 14.2 INCLUDE_FURNISHINGS

Eligible furnishings/items may transfer only through an explicit authoritative manifest/bundle.

Required properties:

- opt-in inclusion;
- server-authoritative exact included set;
- `DUR-03` item identity/location/conservation;
- stale client state cannot alter the committed set;
- committed transfer items cease being available in old seller custody;
- ambiguous settlement is reconciled, not duplicated.

Exact manifest schema, eligible item classes, capacity, valuation and UI remain deferred.

## 15. Character deletion and World transfer

### 15.1 Terminal Character lifecycle

Terminal deletion/finalization cannot leave a physical house bound to a nonexistent owner state.

Required housing settlement/relinquishment must complete or remain in an explicit recoverable state before terminal Character lifecycle completion.

### 15.2 World transfer

A physical `HouseId` is a World address and cannot migrate with a Character to another World.

World transfer must settle/relinquish incompatible physical-house ownership before Character World membership changes authoritatively.

Residence entitlement is also scoped by `AccountId + WorldId`; exact transfer UX remains future World-lifecycle work, but no workflow may duplicate Residence contents or create two authoritative personal housing slots.

## 16. ACL hierarchy

Housing ACL is server-authoritative, revisioned World-scoped state.

First-generation role hierarchy begins:

```text
OWNER
  -> MANAGER / SUBOWNER
      -> GUEST
```

### 16.1 OWNER

Owner has ultimate personal-house administration authority, subject to property/rent/lifecycle rules.

Only owner-authorized workflows may change ownership/disposition-related state.

### 16.2 MANAGER / SUBOWNER

Manager/Subowner is delegated administration, never property ownership.

It may manage bounded guest/access capabilities but cannot independently:

- sell/transfer/relinquish the property;
- bypass rent/eligibility/personal-slot rules;
- rebind canonical owner;
- override owner lifecycle settlement.

### 16.3 GUEST

Guest receives only explicitly granted capabilities.

Entry permission does not automatically imply all housing capabilities.

### 16.4 Fine-grained direction

ACL may distinguish:

- property entry;
- specific door;
- room/zone;
- bed use;
- storage use;
- workstation/interactive feature;
- other later accepted housing capability.

Exact final capability vocabulary remains implementation/product detail provided least-authority semantics are preserved.

### 16.5 Revocation

At meaningful entry/mutation boundaries, server revalidates current ACL revision/authority.

Stale clients/caches/runtimes cannot retain revoked authority.

## 17. GUI-only player administration

Player-facing housing access administration MUST be available through a graphical House Management UI/panel.

Tibia-style text command/spell administration such as `Aleta Sio`, `Aleta Som` or `Aleta Grav` is not the canonical Oteryn UX and is not required for first generation.

The GUI remains presentation/intent only:

- server state is authoritative;
- local UI edits grant no authority by themselves;
- mutations carry expected revision/fence context where required;
- stale/invalid requests fail explicitly rather than silently applying partial outdated state.

Exact screen layout, widgets, search/filter UX and protocol representation remain deferred.

## 18. Rested parity

Residence and ordinary physical-house eligible rest use the same baseline Rested recovery semantics.

Scarcity, purchase price or Premium-gated acquisition of a physical house does not by itself grant a stronger per-Character Rested multiplier.

Physical houses may still differ by non-power/social/space characteristics, for example:

- number of beds;
- layout/size;
- guest capacity;
- location/prestige;
- decoration surface;
- convenience.

The following remain deliberately tunable:

- Rested pool size;
- recovery rate;
- recovery thresholds/duration;
- bed counts;
- Guest/Manager bed permissions;
- whether Residence Rested always requires a bed;
- offline-training interaction;
- general Premium Rested behavior not specifically tied to scarce-house ownership.

A numeric balance adjustment preserving equivalent Residence/physical-house baseline parity does not reopen topology/ownership.

A material progression multiplier granted specifically because the player owns a scarce physical house requires explicit owner supersession.

## 19. Storage and item placement

### 19.1 Physical house

Placed physical-house items/containers have one authoritative semantic location/custody and one authoritative mutable property state across Channels.

### 19.2 Residence

Residence item placement follows the same conservation rules and cannot become per-Channel copied durable storage.

### 19.3 Free-storage abuse boundary

Free Residence is not authorization for effectively unlimited free storage accounts.

Exact placement/storage budgets remain future item/storage/economy decisions.

Account-scoped Residence ownership does not automatically make all placed/storage items account-wide transferable between alternate Characters.

### 19.4 Lifecycle custody

Eviction, relinquishment, Residence replacement and Bazaar disposition move affected durable value through explicit authoritative custody/location transitions before prior property/runtime authority is released.

Destroying, duplicating or silently gifting forgotten items is not an acceptable simplification.

## 20. Failure and recovery semantics

Housing operations distinguish at least:

- deterministic policy/eligibility rejection;
- stale owner/revision/personal-slot state;
- stale runtime/session authority;
- funds/escrow conflict;
- item/custody conflict;
- dependency unavailable;
- ambiguous durable result requiring reconciliation.

For ambiguity:

1. do not fabricate success/failure;
2. reread authoritative operation/property/slot/item/value state;
3. reconcile using the same logical operation identity;
4. do not create a blind replacement mutation;
5. fail closed for risky new mutation while required authority is unavailable.

Safe read-only/presentation degradation may be separately proven later; it is not implied here.

## 21. Cross-region behavior

One World may have Channels in multiple geographic regions.

Housing must not solve latency by creating independently writable house copies.

Accepted trade-off:

- ordinary regional Channel combat/movement/tick stays free of added synchronous WAN persistence round trips;
- explicit shared durable house/item/ownership operations may pay latency to their authoritative World durable boundary;
- runtime placement/migration may later be optimized without changing semantic property identity or creating a second durable writer.

Exact placement and migration policy remains PERF/OPS work.

## 22. Authority boundaries

### 22.1 Game housing domain

Semantic authority for:

- `HouseId` lifecycle/availability;
- canonical physical-house owner binding;
- personal housing-slot state;
- Residence entitlement/lifecycle semantics;
- ACL/revisions;
- rent/delinquency/eviction state;
- final house/Residence disposition authorization;
- active housing-runtime ownership state.

This semantic domain need not map to one process/service/table.

### 22.2 Character Authority

Remains authority for Character lifecycle, current Account owner, current World and Character ownership rebinding.

Housing may gate a Character lifecycle workflow but does not seize Character ownership authority.

### 22.3 Platform

Remains authority for Platform Account identity, entitlement/commercial source data under accepted contracts, portal/commercial UX and Character Bazaar commercial saga state.

Platform listing/cache state is not authoritative housing ownership/eligibility proof.

### 22.4 Economy / item domains

Funds reservation/charges/refunds and item/value movement remain with their accepted/future economy and `DUR-03` owners.

Housing declares required effects but does not invent distributed ACID or duplicate foreign-domain value authority.

### 22.5 Client

Client is untrusted presentation/input software.

It sends intents and renders authoritative outcomes; it cannot establish ownership, auction winner, ACL authority, transfer manifest or settlement success.

## 23. Anti-speculation posture

First-generation design reduces easy property speculation by combining:

- active Premium at scarce physical-house acquisition/retention of an incoming house;
- independent `PhysicalHouseEligibility`;
- one personal housing slot per Account/World;
- no generic standalone private `HouseId` resale market;
- public allocation of vacant properties;
- Character Bazaar requiring transfer of the owning Character rather than just the address;
- Bazaar revalidation of incoming-house eligibility when it is retained;
- recurring in-game rent as ongoing holding cost;
- auditable authoritative settlement state.

This does not claim multi-account speculation is impossible.

Exact anti-abuse account-linking/risk scoring, cooldowns, taxes, holding periods, Bazaar surcharges or sanctions remain evidence-driven future decisions.

Weak signals such as shared IP/device must not automatically become punitive ownership decisions without a separately accepted abuse/security policy.

## 24. Observability requirements

Later implementation must retain sufficient structured/durable evidence to diagnose at least:

- auction bid/close/settlement disputes;
- property ownership history;
- personal-slot conflicts/transitions;
- Premium/eligibility decision version/input class;
- rent/delinquency/eviction lifecycle;
- ACL revisions and privileged changes;
- Bazaar housing disposition and buyer keep-choice;
- furnished-transfer manifest/result;
- item reclaim/custody transitions;
- runtime ownership generations/stale-writer rejection;
- ambiguous-operation reconciliation.

Analytics/audit projections observe owning-domain evidence; they do not become mutation authority.

## 25. Required conformance scenarios before activation

A future implementation must prove at least:

1. **Channel scaling:** adding/removing Channel does not change physical-property supply/state.
2. **Cross-Channel house convergence:** Characters from different Channels enter one `HouseId` and see one authoritative interior/item state.
3. **No hidden Channel switch:** entry/exit preserves validated origin routing and cannot bypass switching restrictions.
4. **Stale runtime:** old runtime generation cannot mutate after replacement/recovery.
5. **Auction retry:** ambiguous settlement retry yields one owner/payment result.
6. **Auction eligibility:** stale eligibility at final acquisition fails without inconsistent property/value mutation.
7. **Premium lapse:** existing property remains owned; rent/lifecycle governs retention.
8. **Personal slot race:** concurrent workflows cannot commit Residence + physical house or two ordinary physical houses.
9. **Residence -> house:** value-safe replacement yields exactly one final personal housing class.
10. **Rent recovery:** delinquency + valid late payment within grace does not double-charge/evict.
11. **Eviction:** contents reach safe custody before ownership release/public re-auction.
12. **Bazaar relinquish:** Character transfer cannot finalize with ambiguous prior house ownership.
13. **Bazaar include/no conflict:** same CharacterId remains house owner after Account owner rebinding.
14. **Bazaar existing-house conflict:** explicit keep-choice produces one same-World physical house.
15. **Bazaar Residence conflict:** explicit keep-choice produces Residence or incoming house, never both.
16. **Bazaar no-retention Premium rule:** buyer choosing existing house/Residence may buy the Character without satisfying incoming physical-house acquisition eligibility because incoming house is relinquished.
17. **HOUSE_ONLY:** forgotten furnishings do not silently become buyer property.
18. **INCLUDE_FURNISHINGS:** exact authoritative manifest transfers once without duplication.
19. **ACL revocation:** stale access attempt after revision change is rejected.
20. **GUI non-authority:** local UI manipulation alone cannot change effective ACL.
21. **Rested parity:** equivalent Residence/physical-house baseline rest does not gain a physical-house-only multiplier.
22. **Character deletion:** terminal finalization cannot leave orphan physical-house owner state.
23. **World transfer:** `HouseId` cannot move/duplicate across Worlds.
24. **Authority outage:** risky housing mutation fails closed when authority is unavailable.
25. **Restore integrity:** owner, personal slot, item custody graph and runtime fences are validated before mutations resume.
26. **Residence account scope:** account-scoped Residence does not silently create unauthorized cross-Character item transfer.

## 26. Deliberately deferred

Whole-gate acceptance deliberately does **not** freeze the following.

### 26.1 Auction / rent / economy numbers

- rent amount/formula;
- rent cadence;
- grace duration;
- auction duration;
- increment;
- anti-sniping window;
- start/reserve price;
- tie rule;
- bid cancellation/lowering;
- fees/taxes/surcharges;
- reclaim fees/capacity;
- anti-flipping cooldown/holding-period rules if later evidence requires them.

### 26.2 Eligibility / anti-abuse numbers

- account age threshold;
- level/progression threshold;
- playtime/activity threshold;
- conduct/security policy details;
- anti-abuse signals/scoring/sanctions;
- which thresholds are public.

### 26.3 Residence details

- final public name (`Residence`, `Apartment`, etc.);
- exact ID representation;
- acquisition quest/progression rule;
- templates/layout/size catalogue;
- decoration/item-placement budget;
- storage representation;
- maintenance/rent fee if any;
- Premium cosmetic/convenience upgrades;
- monetization prices.

### 26.4 Beds / Rested

- numeric pool/rate/timing;
- exact bed counts;
- exact bed capability rules;
- offline-training integration;
- general Premium Rested semantics not tied specifically to scarce physical-house ownership.

### 26.5 Guildhouse

All detailed guildhouse lifecycle/admin/economy/Rested rules remain downstream of the future guild/social architecture.

### 26.6 Physical implementation

- PostgreSQL tables/indexes/constraints;
- isolation/locking implementation;
- service/process/crate decomposition;
- HTTP/RPC/internal IDL;
- exact OperationId/revision/runtime-generation representation;
- runtime placement algorithm;
- client screen layout;
- protocol fields/messages;
- rollout/migration/feature flags;
- SLOs/capacity targets.

## 27. Rejected first-generation defaults

The accepted architecture rejects:

1. per-Channel copies of physical houses;
2. Channel-local physical-house ownership;
3. independently writable regional interior mirrors;
4. scarce physical houses as the only route to baseline housing utility;
5. generic private `HouseId` resale at launch;
6. Premium lapse as automatic property-loss trigger;
7. stronger Rested multiplier solely because a player owns scarce/Premium-gated property;
8. text-command-only housing ACL administration;
9. client-authoritative ownership/auction/ACL/item settlement;
10. Residence as a second concurrent same-World personal property alongside a physical house.

## 28. Explicit supersession

### 28.1 ADR-0001 section 11

`ADR-0001-native-rust-multichannel-platform.md` remains authoritative history and preserves the invariant that a house exists once per logical World.

Its statement that final physical-house presence/topology remains unresolved is superseded by this later owner-accepted baseline:

```text
physical HouseId
-> one World-global property
-> one authoritative active world-scoped interior runtime
-> explicit fenced entry/exit
-> preserved origin Channel routing
```

The historical ADR need not be rewritten.

### 28.2 Multichannel scope matrix

Its fixed safety invariants remain binding.

Its `provisional/deferred` wording for physical-house runtime topology/presence is superseded for `DecisionStatus` by this baseline.

### 28.3 Gap/horizon/global register

Older `REGISTERED_UNRESOLVED`, `DEFERRED` or future-gate wording for `EXP-HOUSES-01` is superseded for the semantic scope accepted here.

Those coordinator/status surfaces may be updated later in a bounded status-only reconciliation. Until then, this baseline has precedence for `EXP-HOUSES-01 DecisionStatus`.

### 28.4 Checkpoint PRs

PRs `#435`, `#436`, `#437`, `#438`, `#440`, `#442`, `#443`, `#444`, `#445`, `#446` remain decision provenance.

After canonical integration of this composition, their housing semantic content is superseded by this file; they should be closed/superseded rather than independently merged as competing architecture authorities.

## 29. What this acceptance enables

After canonical integration **and separate implementation authorization**, downstream work can design:

- persistence/schema/transactions;
- HouseRuntime/ResidenceRuntime boundaries;
- housing protocol and client GUI;
- public auction implementation;
- rent/eviction/reclaim workflows;
- Character Bazaar housing integration;
- item placement/custody integration;
- observability/conformance fixtures;
- balance/telemetry-driven numeric registries.

Implementation may not invent deferred guild/product/balance values merely because the semantic gate is accepted.

## 30. Supersession criteria

Reopen this architecture only with concrete evidence, for example:

- measured player/economy data shows the scarce + Residence split materially harms usability/liquidity;
- runtime evidence proves one-authority physical interior cannot meet requirements despite semantics-preserving optimization;
- observed abuse requires material change to property identity/market rules;
- sustained demand/evidence justifies a dedicated direct property market;
- future guild architecture requires a different guildhouse boundary;
- Rested/progression evidence justifies a deliberate physical-house progression distinction;
- accepted World merge/lifecycle architecture requires different reconciliation semantics;
- security/duping evidence disproves current custody/fencing assumptions;
- a later accepted FND/DUR/GAME/Platform contract changes a consumed authority boundary.

Any supersession must explicitly preserve or replace:

- one authoritative physical property state;
- no Channel-count-driven scarcity mutation;
- item/value conservation;
- one authoritative owner/personal-slot result;
- stale-writer fencing;
- idempotent/reconcilable settlement;
- server-authoritative ownership/auction/ACL;
- explicit Character lifecycle/Bazaar/World-transfer handling;
- safe failure behavior.

## 31. Current-status precedence

Until coordinator-owned global status surfaces are reconciled, this file is authoritative for `EXP-HOUSES-01 DecisionStatus`.

```yaml
EXP-HOUSES-01:
  DecisionStatus: ACCEPTED
  DeliveryStatus: IN_REVIEW
  ImplementationStatus: NOT_STARTED
```

Checkpoint PRs remain provenance while this delivery is in review. They do not authorize runtime or broaden repository authority.

## 32. Decision

`EXP-HOUSES-01 DECISIONSTATUS: ACCEPTED`

`PRODUCT: SCARCE PHYSICAL HOUSES + NON-SCARCE RESIDENCE`

`PHYSICAL HOUSE: ONE WORLD-GLOBAL HOUSEID / ONE AUTHORITATIVE STATE`

`PERSONAL SLOT: NONE | RESIDENCE | PHYSICAL_HOUSE PER ACCOUNTID + WORLDID`

`PHYSICAL HOUSE OWNER: CHARACTERID`

`GUILDHOUSE OWNER: GUILDID / DETAILED GUILD LIFECYCLE DEFERRED`

`PHYSICAL HOUSE ACQUISITION: ACTIVE PREMIUM + PHYSICALHOUSEELIGIBILITY WHEN THE NEW HOUSE IS RETAINED`

`PREMIUM EXPIRY: NOT AN EVICTION TRIGGER`

`VACANT PHYSICAL HOUSE ACQUISITION: WORLD PUBLIC PROXY AUCTION`

`GENERIC DIRECT HOUSE SALE: NOT ENABLED FIRST GENERATION`

`CHARACTER BAZAAR: EXPLICIT RELINQUISH OR INCLUDE HOUSE + BUYER KEEP-CHOICE`

`BAZAAR CONTENTS: HOUSE_ONLY OR EXPLICIT INCLUDE_FURNISHINGS`

`RENT: RECURRING -> GRACE -> VALUE-SAFE FENCED EVICTION -> PUBLIC AUCTION`

`ACL: OWNER -> MANAGER/SUBOWNER -> GUEST + FINE-GRAINED CAPABILITIES`

`ACL PLAYER UX: GUI/PANEL, NOT TEXT-COMMAND-ONLY`

`RESTED: BASELINE PARITY BETWEEN RESIDENCE AND ORDINARY PHYSICAL HOUSE`

`RESIDENCE ACCOUNT SCOPE: NOT AUTOMATIC CROSS-CHARACTER ITEM AUTHORITY`

`IMPLEMENTATION_AUTHORITY: NONE`
