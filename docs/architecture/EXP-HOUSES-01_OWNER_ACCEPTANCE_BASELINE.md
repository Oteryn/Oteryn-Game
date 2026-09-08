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

## 1. Purpose

This baseline composes the owner-approved Oteryn housing decisions into one normative `EXP-HOUSES-01` architecture package.

It closes the first-generation housing architecture sufficiently for later implementation planning without freezing balance constants, physical schema, service/library choices, exact UI layout or production rollout details that do not need to be decided yet.

The gate deliberately separates:

```text
DecisionStatus = ACCEPTED
DeliveryStatus = IN_REVIEW
ImplementationStatus = NOT_STARTED
```

Architecture acceptance does not authorize runtime implementation.

## 2. Evidence and composition sources

The accepted package is composed from:

- protected Oteryn Game architecture on the composition base;
- Issue `#220`, which is the active owner-driven architecture continuation for multichannel world topology and housing;
- the owner-selected housing checkpoints recorded in draft PRs `#435`, `#436`, `#437`, `#438`, `#440`, `#442`, `#443`, `#444`, `#445` and `#446`;
- `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`;
- `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`;
- `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`;
- `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`;
- accepted FND/DUR/GAME-CHANNEL/GAME-CHAR/GAME-ITEM/entitlement authority boundaries.

The checkpoint PRs are composition evidence and owner-decision provenance. This baseline is the intended canonical semantic consolidation; it does not require those checkpoint documents to become independent long-lived architecture authorities.

## 3. Whole-gate decision summary

The accepted first-generation housing model is:

```text
one logical World
    |
    +-- finite scarce physical HouseId addresses
    |      - one world-global property state
    |      - public World auction
    |      - recurring rent / grace / eviction
    |      - Premium + PhysicalHouseEligibility at acquisition
    |      - canonical ordinary owner = CharacterId
    |
    +-- non-scarce Residence / Apartment capability
           - available without Premium
           - account-scoped per World
           - no scarcity auction
           - no speculative resale market

per AccountId + WorldId personal housing slot:
    NONE | RESIDENCE | PHYSICAL_HOUSE
```

A personal account/World may never hold both a Residence and an ordinary physical house at the same time.

Guildhouses are a separate guild-owned class under `GuildId`; their detailed lifecycle remains deferred to the future guild/social architecture.

## 4. Binding upstream invariants

`EXP-HOUSES-01` consumes and preserves the following accepted invariants.

### 4.1 World and Channel identity

- one `WorldId` remains one durable community/economy boundary;
- `ChannelId` is parallel public-world simulation capacity inside the World;
- Channel creation/removal is infrastructure/gameplay-capacity work and must not change housing scarcity or duplicate durable housing state;
- host, process, GameNode and physical region are placement facts, not house property identity.

### 4.2 Physical house identity

- every physical `HouseId` belongs to exactly one `WorldId`;
- `HouseId` does not contain or semantically depend on `ChannelId`;
- the same physical address is not copied once per Channel;
- ownership, rent, ACL and durable item state exist once for the World property.

### 4.3 Durable item/value safety

Accepted `GAME-ITEM-01` and `DUR-03` remain authoritative for item identity, semantic location, custody, conservation and anti-duplication.

Housing therefore may not create a weaker item path. Every durable item associated with a house or Residence must have exactly one authoritative semantic location/custody state.

### 4.4 Character/session fencing

Character and runtime ownership transfers remain session/generation/revision fenced under accepted Character, Channel, runtime and durability contracts.

A stale Channel, stale HouseRuntime, stale client or stale Platform projection never acquires mutation authority merely because it previously observed the house.

## 5. Housing product classes

### 5.1 Ordinary physical house

An ordinary physical house is a finite, scarce, prestigious address in the persistent World.

Its product value may include:

- recognizable physical location on the World map;
- social/prestige value;
- larger or distinctive layouts;
- public scarcity and competitive acquisition;
- decoration/personalization;
- access-management capability;
- beds and other housing functions under later balance rules.

Physical-house scarcity is intentional and must not become the only route to baseline housing utility or baseline Rested participation.

### 5.2 Residence / apartment

A Residence is a separate non-scarce personal housing capability.

It is a genuine usable home, not a crippled Premium preview.

The baseline Residence direction includes:

- persistent personal interior;
- decoration/personalization;
- graphical access management;
- baseline social/housing utility;
- compatibility with accepted Rested semantics;
- no requirement for active Premium merely to obtain or retain the baseline Residence.

Residence supply is independent of physical `HouseId` scarcity and Channel count.

### 5.3 Guildhouse

The canonical guildhouse owner identity is `GuildId`.

A guildhouse does not consume the personal `AccountId + WorldId` housing slot.

Detailed guildhouse acquisition eligibility, leadership succession, guild-rank administration, rent funding, dissolution behavior and Bazaar consequences remain `EXP-SOCIAL-01` / future guild-system work.

Selling a guild leader Character does not transfer the guildhouse merely because Account ownership of that Character changes.

## 6. Personal housing-slot invariant

For ordinary personal housing, the accepted aggregate scope is:

```text
PersonalHousingSlot(AccountId, WorldId) =
    NONE
  | RESIDENCE
  | PHYSICAL_HOUSE
```

Binding consequences:

- maximum one Residence per `AccountId` per `WorldId`;
- maximum one ordinary physical house per `AccountId` per `WorldId`;
- the account may hold either class on a World, never both;
- alternate Characters on the same Account cannot bypass the slot;
- another World has an independent personal housing slot;
- the personal slot is an aggregate eligibility/ownership guard and does not replace the canonical physical-house owner identity of `CharacterId`.

### 6.1 Residence -> physical house

If an Account with a Residence acquires a physical house, authoritative settlement must resolve the Residence before the physical-house ownership commit can complete.

At minimum:

1. current housing-slot state is revalidated;
2. Residence item/value state is safely settled into an accepted authoritative custody/location;
3. stale Residence runtime writers are fenced;
4. Residence entitlement/runtime state is released according to the later concrete lifecycle contract;
5. only then may the physical-house acquisition consume the personal housing slot.

The operation must be idempotent/reconcilable and may not expose an intermediate durable state in which both personal housing classes are authoritatively held.

### 6.2 Physical house -> Residence

A player cannot activate a Residence while their Account still consumes the same-World physical-house slot.

The physical house must first be safely relinquished/evicted/disposed under the accepted physical-house lifecycle; durable contents/value are settled before the Residence becomes authoritative.

### 6.3 Failure behavior

A failed or ambiguous transition does not mean success.

The system must reread authoritative housing-slot/property/operation state and reconcile using the same logical operation identity. It may not create a fresh blind settlement attempt that could duplicate ownership or item movement.

## 7. Physical-house topology and runtime ownership

### 7.1 One World-global property

A physical house has one logical property state shared by every Channel of its World.

The following are explicitly rejected:

- one independent physical-house copy per Channel;
- Channel-local `HouseId`;
- Channel-local ownership/rent/ACL;
- independently mutable regional mirrors of one house interior;
- using Channel count to manufacture additional copies of scarce addresses.

### 7.2 Authoritative house-interior runtime

When active, one physical house uses one logical authoritative world-scoped interior runtime owner.

Characters entering the same `HouseId` from different Channels converge on the same authoritative interior/presence and observe the same mutable item state.

The implementation may reuse an accepted instance-style runtime primitive, but:

- canonical property identity remains `HouseId`;
- any internal `InstanceId` or runtime generation is runtime-lifecycle identity only;
- `InstanceId` must not replace `HouseId` in ownership, rent, auction, ACL or durable item semantics.

### 7.3 Origin Channel

House entry is an explicit authoritative context/ownership transfer, not a hidden Channel switch.

The entering Character retains validated origin-Channel routing metadata.

Normal exit returns the Character through the same validated origin route unless a separately accepted same-World fallback is required because the original Channel is unavailable, draining or incompatible.

House entry/exit may never be used to bypass:

- combat or PvP channel-switch restrictions;
- capacity/admission rules;
- session/lease fencing;
- active protected transactions;
- Channel profile/revision compatibility.

### 7.4 One writer and recovery

One active house interior has one logical authoritative mutation owner at a time.

Replacement, relocation or crash recovery requires generation/revision fencing so an old runtime cannot resume and overwrite newer house/item/presence state.

Durable ownership/item truth remains in the World durable authority; rebuildable runtime presence may be reconstructed only from accepted authoritative durable/content state.

## 8. Residence topology boundary

A Residence is not a duplicate physical map address.

It may later use an instance-style runtime primitive, but first-generation architecture requires only that:

- Residence state belongs to one `AccountId + WorldId` personal-housing entitlement;
- active Residence simulation has one authoritative runtime owner;
- Channel scaling does not duplicate Residence contents;
- durable item/value mutations use accepted World durable authority and `DUR-03` semantics;
- origin routing and simulation-owner transfer are explicit and fenced.

The exact `ResidenceId` type, hosting placement, runtime identifier, template format and persistence representation remain deliberately deferred.

## 9. Ordinary physical-house ownership identity

The canonical public/semantic owner of an ordinary physical `HouseId` is one `CharacterId` in the owning World.

`AccountId` is used for the personal housing slot, acquisition eligibility, anti-concentration and anti-abuse controls; it is not substituted for the public/canonical `HouseId -> CharacterId` ownership relation.

Consequences:

- multiple Characters of one Account cannot each independently own a physical house on the same World;
- Character rename preserves ownership because `CharacterId` is stable;
- Character lifecycle operations that affect identity availability must explicitly settle housing;
- Character ownership change through Bazaar requires the explicit rules in this contract;
- World transfer cannot move a physical `HouseId` to another World.

## 10. Physical-house acquisition eligibility

Acquiring an ordinary scarce physical house requires BOTH:

```text
active Premium
+
PhysicalHouseEligibility
```

Neither Premium alone nor gold alone is sufficient.

`PhysicalHouseEligibility` must be based on meaningful account/Character participation or progression evidence on the relevant World so trivial newly created accounts cannot immediately become cheap property-holding shells.

The exact eligibility policy remains versioned/tunable and is deliberately not frozen here. Candidate input classes may include account maturity, Character progression, activity/play history, conduct/security state or another explicit game-owned qualification rule.

The rule applies at least to:

- public physical-house auction settlement;
- a Bazaar buyer electing to retain an incoming Character-owned physical house.

### 10.1 Premium lapse after acquisition

Expiration of Premium after a legitimate acquisition does **not** by itself evict the owner, destroy the house entitlement or transfer property.

Retention is governed by in-game rent/grace/eviction and other explicit lifecycle rules, not by continuing subscription payment.

If a later product decision wants Premium lapse to change ownership, it must explicitly supersede this baseline.

## 11. Public physical-house auction

Vacant ordinary physical houses enter one World-scoped public allocation/auction lifecycle.

This includes, as applicable:

- newly available physical houses;
- voluntarily relinquished houses;
- houses released through Character Bazaar disposition;
- houses released after value-safe eviction;
- other explicit future vacancy causes.

### 11.1 Proxy bidding

First-generation auction direction uses proxy bidding:

- a bidder supplies a private maximum;
- automated bidding advances only when real competing bids require it;
- automated bidding never exceeds the bidder's maximum;
- the winner pays the minimum valid amount required to beat the next-best competing maximum under the later-defined increment rule, rather than automatically paying the full private maximum.

### 11.2 Funds backing

A bid capable of becoming the winner must be backed by authoritative funds reservation/escrow semantics before final settlement can consume value.

The housing gate does not invent a separate currency model; accepted item/value conservation and future economy/bank authority remain responsible for funds state.

### 11.3 Anti-sniping

The auction has extension-based anti-sniping semantics so a valid bid near the close boundary can extend the close rather than making network timing the dominant winner-selection mechanic.

Exact auction duration, extension window and increment are balance/product constants and remain deferred.

### 11.4 Settlement

Auction close/settlement must be:

- authoritative;
- idempotent;
- revision/fence aware;
- retry/reconciliation safe;
- compatible with the personal housing-slot invariant;
- compatible with Premium + `PhysicalHouseEligibility` revalidation;
- value-conserving under the owning economy/item contracts.

A timeout or unavailable dependency is not proof that the house or funds changed ownership.

### 11.5 No first-generation standalone private property market

First-generation Oteryn does not expose a generic operation equivalent to:

```text
SellPhysicalHouseDirectlyTo(CharacterId buyer, arbitrary_price)
```

Nor does it provide a free-form player-to-player private property market for `HouseId`.

This reduces direct flipping/cornering surface while preserving a future extension point if real player/economy evidence later justifies a dedicated property market.

The Character Bazaar rule below is the explicit first-generation exception because the owning Character itself is the commercial object.

## 12. Recurring rent, delinquency and eviction

Physical-house rent is recurring and World-scoped.

The exact amount/formula, cadence and authoritative funding source are balance/economy decisions and remain deferred.

### 12.1 Automatic collection

Rent collection is automatic from a later-defined authoritative economy funding source and must be idempotent/reconcilable.

Channel count, current Character Channel or house runtime placement cannot duplicate a rent charge or reset the rent lifecycle.

### 12.2 Grace / delinquency

Insufficient funds do not cause immediate silent eviction.

The property enters an explicit delinquent/grace state that is visible to the owning product workflow and recoverable according to later notification/balance policy.

Exact grace duration and notification cadence remain deferred.

### 12.3 Eviction settlement

If delinquency remains unresolved through the accepted grace boundary, eviction is an authoritative revision-fenced settlement.

Before ownership release:

1. current owner/revision/rent state is revalidated;
2. outgoing durable items/value are moved to an accepted safe authoritative reclaim/depot/custody location;
3. stale HouseRuntime/Channel mutation rights are fenced;
4. ownership and ACL are released;
5. the property becomes vacant;
6. the property returns to the World public auction/allocation lifecycle.

Ordinary forgotten items are never silently inherited by the next owner and are not destroyed merely to complete eviction.

## 13. Character Bazaar housing disposition

Character Bazaar commercial workflow remains Platform-owned under `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`; authoritative Character ownership mutation remains game-owned.

Housing adds an explicit game-owned housing disposition boundary.

A house must never follow a Character sale implicitly.

When a listed Character currently owns an ordinary physical house, the seller explicitly chooses:

```text
BazaarHousingDisposition =
    RELINQUISH_HOUSE
  | INCLUDE_HOUSE_WITH_CHARACTER
```

### 13.1 RELINQUISH_HOUSE

Before Character ownership transfer can safely settle:

- house ownership/revision is revalidated;
- durable house contents/value are safely settled;
- stale house/runtime writers are fenced;
- the `HouseId` is released from the Character;
- the house becomes vacant and returns to the public World auction lifecycle;
- Character sale proceeds only after the required housing disposition state is authoritative/reconcilable.

Failure cannot produce both a sold Character and an ambiguously owned house.

### 13.2 INCLUDE_HOUSE_WITH_CHARACTER

If the seller includes the house:

```text
CharacterId C owns HouseId H
AccountId A owns CharacterId C

Bazaar transfers CharacterId C from AccountId A to AccountId B

=> CharacterId C remains canonical owner of HouseId H
=> Character ownership binding changes A -> B
=> AccountId B must satisfy all same-World personal-housing constraints
```

This is not a standalone `HouseId` sale. The commercial object is the Character listing with an explicit included-house attribute.

### 13.3 Buyer revalidation

Before final Character+house settlement, the buyer side must revalidate at least:

- current Character/Bazaar operation authority;
- active Premium for incoming physical-house acquisition;
- current `PhysicalHouseEligibility` for the relevant World;
- current personal housing-slot state;
- expected `HouseId -> CharacterId` ownership/revision;
- rent/eviction/other housing lifecycle conflicts;
- incompatible Character/world/session/transfer operations.

A stale listing or cached read model does not prove the buyer may retain the house.

### 13.4 Buyer already has a physical house

If Account B already holds another ordinary physical house on the same World, Bazaar cannot commit both.

The buyer must explicitly choose:

```text
KEEP_EXISTING_HOUSE
or
KEEP_INCOMING_HOUSE
```

The non-kept physical house is value-safely relinquished and returns to the public auction lifecycle as part of the same idempotent/reconcilable Bazaar housing settlement.

A house on another World does not conflict with this per-World slot.

### 13.5 Buyer has a Residence

If Account B holds a Residence on the same World, the buyer must explicitly choose:

```text
KEEP_RESIDENCE
or
KEEP_INCOMING_HOUSE
```

If the incoming house is kept, the Residence is value-safely settled/released before the physical-house slot commit.

If the Residence is kept, the incoming physical house is relinquished to the public auction lifecycle rather than creating dual personal housing.

### 13.6 Bazaar atomicity/reconciliation semantics

No cross-system distributed ACID is assumed.

The workflow must nevertheless preserve semantic atomicity through durable operation state, fencing, idempotent steps and reconciliation:

- Platform owns listing/bidding/wallet/commission/saga state;
- Character Authority owns Character owner rebinding;
- housing authority owns physical-house/Residence disposition and slot state;
- item/economy owners preserve value/custody;
- timeout is not success or failure proof;
- retries use the same logical operation identity;
- partial durable progress remains explicitly recoverable and cannot create two authoritative owners.

## 14. Character Bazaar house contents

When the seller chooses `INCLUDE_HOUSE_WITH_CHARACTER`, the seller also controls whether movable house contents are deliberately included.

The semantic product choices are:

```text
HOUSE_ONLY
or
INCLUDE_FURNISHINGS
```

### 14.1 HOUSE_ONLY

The owning Character and physical address may transfer through Bazaar, but ordinary movable seller-owned house contents do not silently become buyer property.

They are moved to safe authoritative seller/reclaim/depot/custody before the ownership settlement completes.

### 14.2 INCLUDE_FURNISHINGS

A seller may deliberately include eligible furnishings/items only through an explicit authoritative transfer manifest/bundle representation.

Required invariants:

- inclusion is opt-in;
- the exact included set is server-authoritative;
- item identity/location/value conservation is preserved under `DUR-03`;
- stale client display state cannot add or remove transfer items;
- one item cannot remain spendable/usable in the seller's prior custody after it has committed into the transfer bundle;
- ambiguous bundle settlement is reconciled, not duplicated.

The exact manifest schema, eligible/ineligible item classes, UI, capacity and valuation rules remain deferred.

## 15. World transfer and Character deletion

### 15.1 World transfer

A physical `HouseId` is an address in one World and does not migrate with a Character to another World.

Character World transfer must resolve/relinquish incompatible physical-house ownership before the world-membership transfer commits.

A Residence is also World-scoped under `AccountId + WorldId`; exact behavior for an Account/Character world-transfer product workflow remains a later World-lifecycle detail, but no transfer may create dual authoritative personal housing or duplicate contents.

### 15.2 Character deletion/finalization

Terminal Character deletion/finalization cannot leave a physical house owned by a nonexistent active owner identity.

Housing must be explicitly settled/relinquished before terminal lifecycle completion.

Exact recovery/grace UX remains Character/housing lifecycle work.

## 16. Access control model

Housing access is server-authoritative World-scoped state.

First-generation personal-house ACL begins with:

```text
OWNER
  -> MANAGER / SUBOWNER
      -> GUEST
```

### 16.1 Owner

The canonical owner has ultimate personal-house administration authority, subject to lifecycle/economy rules.

Only owner-authorized workflows may change ownership/disposition-related state.

### 16.2 Manager / Subowner

A Manager/Subowner is delegated administrator authority, not property ownership.

The role may manage bounded guest/access settings allowed by the later concrete capability policy, but cannot independently:

- sell/transfer/relinquish the property;
- bypass rent/eligibility/slot rules;
- rebind canonical ownership;
- override owner lifecycle settlement.

### 16.3 Guest

A Guest receives only explicitly granted use/access capabilities.

Being permitted to enter the property does not implicitly grant every housing capability.

### 16.4 Fine-grained capability direction

ACL may distinguish meaningful scopes such as:

- property entry;
- specific door;
- room/zone;
- bed use;
- storage use;
- workstation/interactive feature;
- later accepted house capability.

Exact final capability vocabulary remains implementation/product contract detail as long as least-authority semantics are preserved.

### 16.5 Revocation

ACL changes are revisioned/fenced.

At meaningful entry/mutation boundaries, the server revalidates current authority. A stale client or stale runtime cache cannot retain revoked mutation authority.

## 17. House Management GUI is mandatory player UX

Player-facing housing access administration must be performed through a graphical House Management UI/panel.

Oteryn does not require or treat Tibia-style text commands/spells such as `Aleta Sio`, `Aleta Som` or `Aleta Grav` as the canonical housing administration interface.

The GUI is an intent/presentation surface only:

- server housing state remains authoritative;
- the client never grants itself access by locally editing UI state;
- edits carry expected revision/fencing context where needed;
- validation failures are explicit and do not silently partially apply stale changes.

The exact screen layout, widgets, labels, search UX and client implementation remain deferred.

## 18. Beds and Rested parity

Residence and ordinary physical-house eligible rest use the same baseline Rested recovery semantics.

Physical-house scarcity, purchase price or Premium acquisition does not by itself grant a higher per-Character Rested multiplier.

This prevents scarce/Premium housing from becoming mandatory progression power.

Physical houses may still be differentiated by non-power or social/space characteristics such as:

- number of available beds;
- physical layout;
- guest capacity;
- location/prestige;
- decoration surface;
- social convenience.

The following remain deliberately tunable/deferred:

- exact Rested pool size;
- exact recovery rate;
- exact recovery duration/thresholds;
- exact bed count by housing template;
- Guest/Manager bed permissions;
- whether all Residence rest requires a bed;
- offline-training interaction;
- any general Premium Rested behavior that is not specifically a physical-house ownership advantage.

A later numeric balance change that preserves parity between equivalent Residence/physical-house rest does not require reopening housing topology or ownership.

Any material progression multiplier granted specifically because a player owns the scarce physical house requires explicit owner supersession.

## 19. Item placement, storage and custody

Housing does not own a parallel item identity/conservation system.

### 19.1 Physical houses

Placed items/containers inside a physical house use one authoritative semantic location/custody state and one authoritative mutable house state across Channels.

### 19.2 Residences

Residence item placement follows the same one-location/conservation principles and cannot become per-Channel copied durable storage.

### 19.3 Storage anti-abuse boundary

A Free Residence is not authorization for unlimited free durable storage.

Exact Residence-specific storage or item-placement budgets remain deferred and must be bounded under the later item/storage/economy contract.

The gate does not decide whether Residence storage is a separate durable container, a placement budget over ordinary item storage or another safe representation.

### 19.4 Settlement custody

Eviction, relinquishment, Residence replacement and Bazaar disposition move affected durable value through explicit accepted custody/location transitions before prior ownership/runtime authority is released.

Destroying, duplicating or silently gifting forgotten items is not an acceptable simplification.

## 20. Failure, timeout and recovery rules

Housing operations distinguish at least:

- deterministic eligibility/policy rejection;
- stale expected owner/revision/slot state;
- stale runtime/session authority;
- economy/escrow conflict;
- item/custody conflict;
- dependency unavailable;
- ambiguous durable result requiring reconciliation.

For ambiguous results:

1. do not fabricate success or failure;
2. reread authoritative operation/property/slot/item/value state;
3. reconcile with the same stable logical operation identity;
4. do not issue a blind replacement operation that could duplicate settlement;
5. fail closed for risky new mutation while required authority is unavailable.

A house-service outage may later permit separately proven safe read/presentation degradation, but risky entry/mutation/ownership/value operations fail closed unless the owning contract proves a safe degraded path.

## 21. Cross-region consequences

One World may have Channels in multiple regions.

Housing must not solve regional latency by creating independently writable house copies.

The accepted trade-off is:

- ordinary regional Channel tick/combat/movement remains free of added synchronous WAN persistence round trips;
- explicit shared durable house/item/ownership operations may pay the latency required to reach their authoritative World durable boundary;
- house runtime placement/migration may later be optimized without changing semantic property identity or creating another durable writer.

Exact placement and migration policy remain operations/performance work.

## 22. Authority boundaries

### 22.1 Game housing authority

The Game housing domain is semantic authority for:

- physical `HouseId` lifecycle/availability;
- canonical physical-house owner binding;
- personal housing-slot validation/state;
- Residence entitlement/lifecycle semantics;
- housing ACL/revisions;
- rent/delinquency/eviction state;
- final house/Residence disposition authorization;
- authoritative active housing-runtime ownership state.

This does not imply one physical service/process/table.

### 22.2 Character Authority

Character Authority remains authoritative for Character lifecycle/current Account owner/current World and the Character ownership mutation used by Bazaar.

Housing may block/condition a Character lifecycle operation but does not seize Character ownership authority.

### 22.3 Platform

Platform remains authoritative for Platform Identity/account identity, Premium/entitlement source under accepted entitlement boundaries, portal/commercial UX and Character Bazaar commercial saga state.

A Platform cache/listing does not become authoritative house ownership/eligibility proof.

### 22.4 Economy/item domains

Funds reservation/charges/refunds and item/value movement remain under accepted/future economy and DUR-03 owners.

Housing defines when those effects are required but does not create distributed ACID or duplicate foreign-domain value authority.

### 22.5 Client

The client is untrusted presentation/input software. It sends intents and displays authoritative results; it never establishes ownership, winning bids, ACL authority, item manifests or settlement success by itself.

## 23. Anti-speculation and abuse posture

The first-generation design intentionally reduces the easiest property-speculation surfaces through composition of multiple controls rather than one heuristic:

- active Premium required at scarce physical-house acquisition;
- independent `PhysicalHouseEligibility` based on meaningful real participation/progression;
- one personal housing slot per `AccountId + WorldId`;
- no generic standalone private `HouseId` resale market;
- public auction for vacant properties;
- Bazaar transfer requires selling the owning Character, not merely flipping the address;
- Bazaar buyer revalidates Premium, eligibility and personal housing slot;
- recurring rent creates a real ongoing in-game holding cost;
- server-authoritative operation/audit state enables later abuse analysis.

This baseline does not claim multi-account speculation is impossible.

Exact anti-abuse risk scoring, account-linking signals, cooldowns, Bazaar surcharges, transfer taxes, holding periods and enforcement policy remain evidence-driven future decisions. They should be added only if observed abuse/economy data justifies the friction.

No automatic punitive decision should be inferred solely from one weak signal such as shared IP/device without a separately accepted abuse/security policy.

## 24. Observability and audit requirements

Later implementation must retain enough durable/structured evidence to diagnose at least:

- auction bid/close/settlement disputes;
- ownership transition history;
- personal housing-slot conflicts;
- Premium/eligibility decision inputs/version;
- rent charge/delinquency/eviction lifecycle;
- ACL revisions and privileged access changes;
- Bazaar house disposition and buyer keep-choice;
- furnished transfer manifest/result;
- item reclaim/custody transitions;
- runtime ownership generations and stale-writer rejection;
- ambiguous-operation reconciliation.

Analytics/audit projections remain non-authoritative; they observe owning-domain evidence and do not mutate housing truth.

Exact event schema and retention policy remain ANL/privacy/operations work.

## 25. Deterministic conformance scenarios required before activation

A future implementation must prove at least the following semantic scenarios before housing activation.

### HOUSES-01 — Channel scaling does not change property supply

Adding/removing a Channel leaves physical `HouseId` count, owner, rent, auction and ACL state unchanged.

### HOUSES-02 — Same house from two Channels

Two eligible Characters from different Channels enter the same `HouseId` and converge on one authoritative interior/item state; they do not see independent copies.

### HOUSES-03 — No hidden Channel switch

Character enters/exits a physical house and returns through validated origin routing without bypassing Channel-switch restrictions.

### HOUSES-04 — Stale runtime rejection

Old HouseRuntime/Channel ownership generation attempts a mutation after replacement/recovery and is rejected without overwriting current state.

### HOUSES-05 — Duplicate auction settlement retry

Ambiguous response followed by retry produces one semantic house owner/payment result, not duplicate charges or double allocation.

### HOUSES-06 — Acquisition eligibility revalidation

Bid/listing preview was created while eligible, but final settlement sees missing Premium or invalid `PhysicalHouseEligibility`; physical-house acquisition fails safely without consuming property/value inconsistently.

### HOUSES-07 — Premium expires after legitimate ownership

Premium lapses after acquisition; ownership remains intact and is governed by rent/lifecycle rules rather than subscription eviction.

### HOUSES-08 — Personal slot prevents double housing

Same `AccountId + WorldId` cannot commit both Residence and physical house or two ordinary physical houses under concurrent/retried workflows.

### HOUSES-09 — Residence -> house replacement

Account with Residence wins/retains a physical house; Residence value is safely settled and the transition commits exactly one final personal housing class.

### HOUSES-10 — Rent delinquency / recovery

Insufficient rent enters explicit grace; successful payment before expiry restores good standing without duplicate charge or eviction.

### HOUSES-11 — Value-safe eviction

Unresolved grace causes one fenced eviction; durable contents move to safe custody before ownership release; property returns to public auction.

### HOUSES-12 — Bazaar relinquish

Seller chooses `RELINQUISH_HOUSE`; contents/ownership are safely settled before Character ownership transfer becomes final.

### HOUSES-13 — Bazaar include, buyer has no conflict

Seller includes the house; same `CharacterId` remains house owner while Character Account ownership changes; buyer eligibility is revalidated.

### HOUSES-14 — Bazaar include, buyer already has physical house

Buyer explicitly chooses existing or incoming house; only one remains in the same-World personal slot and the other returns value-safely to public allocation.

### HOUSES-15 — Bazaar include, buyer has Residence

Buyer explicitly chooses Residence or incoming physical house; settlement never commits both.

### HOUSES-16 — HOUSE_ONLY prevents accidental item sale

Character+house sale completes while ordinary forgotten furnishings remain with/safely return to seller custody rather than silently becoming buyer property.

### HOUSES-17 — INCLUDE_FURNISHINGS is exact and idempotent

Only server-authoritatively selected eligible items in the manifest transfer; retry does not duplicate items and stale client state cannot alter the committed set.

### HOUSES-18 — ACL revocation

Owner revokes Guest/Manager capability; a stale client/cache attempts an action and current revision authority rejects it.

### HOUSES-19 — GUI is presentation, not authority

Manipulating local House Management UI state without accepted server mutation cannot change effective access.

### HOUSES-20 — Rested parity

Equivalent eligible Residence and physical-house rest paths use the same baseline Rested recovery semantics for a Character; physical-house ownership alone does not multiply progression.

### HOUSES-21 — Character deletion

Terminal Character finalization cannot complete leaving a physical house bound to a nonexistent owner state.

### HOUSES-22 — World transfer

Character World transfer cannot move or duplicate a physical `HouseId`; required disposition completes or transfer is blocked/reconciled.

### HOUSES-23 — House-authority outage

Risky housing mutation during authoritative house-service unavailability fails closed rather than guessing from client/cache state.

### HOUSES-24 — Restore/recovery integrity

After restore/recovery, one owner, one personal housing slot state, one authoritative item location/custody graph and newer runtime fences are proven before mutations resume.

## 26. Deliberately deferred

The following are explicitly **not** frozen by whole-gate acceptance because they do not need to block the current semantic architecture and are better resolved by balance evidence, implementation design, guild/social architecture, operations or measurement:

### 26.1 Balance/economy values

- exact physical-house rent amount/formula;
- rent cadence;
- grace duration;
- auction duration;
- proxy-bid increment;
- anti-sniping extension window;
- starting/reserve prices;
- tie resolution;
- cancellation/bid-lowering rules;
- taxes/fees/surcharges;
- reclaim fees/capacity;
- anti-flipping cooldown/holding-period rules if later needed.

### 26.2 Eligibility/anti-abuse values

- exact account age;
- exact level/progression thresholds;
- exact playtime/activity requirements;
- exact conduct/security eligibility details;
- exact anti-abuse telemetry/risk scoring and sanctions;
- exact visibility of anti-abuse thresholds.

### 26.3 Residence product details

- final public name (`Residence`, `Apartment` or later name);
- exact `ResidenceId` representation;
- exact acquisition quest/level/play-history rule;
- number/size/layout/template catalogue;
- exact decoration/item-placement budget;
- exact storage integration;
- maintenance/rent fee, if any;
- Premium cosmetic/convenience upgrades;
- monetization prices.

### 26.4 Beds / Rested details

- exact Rested pool/rate/timing;
- exact bed counts;
- exact bed ACL rules;
- offline-training integration;
- other general Premium Rested policy not tied specifically to physical-house ownership.

### 26.5 Guildhouse details

- guild creation/role prerequisites;
- who may bid/manage on behalf of a Guild;
- leadership succession;
- guildhouse rent funding;
- dissolution/merge lifecycle;
- guild/rank ACL inheritance;
- guildhouse Rested semantics;
- guild Bazaar/world-transfer interactions.

These remain coupled to future guild/social contracts.

### 26.6 Physical implementation

- PostgreSQL tables/indexes/constraints;
- exact transaction/isolation implementation;
- exact service/process/crate decomposition;
- RPC/HTTP/internal IDL;
- exact `OperationId`, revision and runtime-generation representation;
- exact HouseRuntime/ResidenceRuntime placement policy;
- regional placement/migration algorithm;
- exact client screen layout;
- exact protocol messages;
- rollout/feature flags/migration plan;
- production SLOs and capacity values.

## 27. Rejected first-generation models

This whole-gate acceptance explicitly rejects as first-generation defaults:

1. **per-Channel physical-house copies** — violates one-property/one-state and makes infrastructure scale alter scarcity;
2. **Channel-local physical-house ownership** — conflicts with World-scoped community/economy/property semantics;
3. **independently writable regional interior mirrors** — creates stale-writer/item-duplication hazards;
4. **scarce physical housing as the only baseline housing path** — makes finite real estate a progression/utility gate;
5. **generic private `HouseId` resale market at launch** — adds speculative flipping/cornering surface without current product need;
6. **Premium lapse => immediate property loss** — converts retained earned property into a continuing subscription hostage rather than an in-game rent/lifecycle object;
7. **physical house => stronger Rested multiplier solely due scarcity/Premium** — creates unnecessary progression pressure toward scarce paid-gated property;
8. **text-command-only ACL administration** — rejects the selected modern GUI product requirement;
9. **client-authoritative access/auction/item settlement** — violates server authority and value safety.

## 28. Supersession of older unresolved/provisional wording

This owner-accepted baseline explicitly supersedes **only** older unresolved/provisional housing wording for the semantic scope accepted here.

In particular:

### 28.1 ADR-0001 section 11

`ADR-0001-native-rust-multichannel-platform.md` remains historical/architectural authority for the durable invariant that a house exists once per logical World and must not be duplicated per Channel.

Its statement that final physical-house presence/topology remains unresolved is superseded by this later accepted `EXP-HOUSES-01` baseline:

```text
physical HouseId
-> one World-global property
-> one authoritative world-scoped active interior runtime
-> explicit fenced entry/exit
-> preserved origin Channel routing
```

The historical ADR file itself need not be rewritten for this supersession to be effective.

### 28.2 Multichannel scope matrix

The matrix's fixed house safety invariants remain binding.

Its `provisional/deferred` status wording for physical-house items/runtime topology and presence/entry is superseded by this baseline for DecisionStatus only.

### 28.3 Gap/horizon/register status surfaces

Any older `REGISTERED_UNRESOLVED`, `DEFERRED` or future-gate wording for `EXP-HOUSES-01` is superseded for the accepted semantic scope by this baseline.

Those coordination/status surfaces may be reconciled in a later bounded status-only update. Until then, this later owner-acceptance baseline is authoritative for `EXP-HOUSES-01 DecisionStatus`.

### 28.4 Checkpoint PRs

Housing checkpoint PRs `#435`, `#436`, `#437`, `#438`, `#440`, `#442`, `#443`, `#444`, `#445` and `#446` remain provenance/evidence for the owner-decision sequence.

Once this composition baseline is canonically integrated, those checkpoint deliveries are semantically superseded by the composition for their housing decision content and should not be independently merged as competing architecture authorities.

## 29. Decision timing

### Must decide now?

`YES` for the semantic whole-gate package.

Without one composed baseline, later implementation could independently choose incompatible house topology, property identity, Channel behavior, account limits, acquisition rules, item custody, Bazaar semantics, Residence ownership or ACL authority.

That would create expensive persistence/protocol/UI migrations and meaningful duping/economy risk.

### What downstream work is unblocked?

After canonical integration and normal implementation authorization, this gate is sufficient input for later work to design:

- physical persistence/schema and transaction implementation;
- HouseRuntime and ResidenceRuntime boundaries;
- housing protocol/client GUI;
- public auction implementation;
- rent/eviction/reclaim workflows;
- Character Bazaar housing integration;
- item placement/custody integration;
- housing observability/test fixtures;
- balance/telemetry-driven numeric registries.

Implementation remains separately authorized and may not invent deferred product values or guild semantics.

### What becomes harder later?

The accepted design intentionally commits to:

- one physical property state per World rather than per Channel;
- one personal housing slot per Account/World;
- `CharacterId` physical-house owner identity;
- public auction/no standalone private house market at first generation;
- explicit Bazaar exception;
- Residence as a separate non-scarce account/World capability;
- value-safe idempotent transitions;
- GUI ACL rather than text-command-only administration.

Reversing those later may require data migration, product migration, economy transition and client UX work.

Numeric tuning and physical implementation choices remain intentionally easier to change.

## 30. Supersession criteria

Reopen this accepted architecture only with concrete evidence such as:

- measured population/economy data shows the scarce + Residence product split causes material player harm or unusable liquidity;
- actual runtime evidence shows one authoritative house interior cannot meet required latency/availability despite semantics-preserving placement optimization;
- observed abuse demonstrates current acquisition/slot/Bazaar controls are insufficient and requires a material property-market or identity change;
- players materially need direct standalone property sale and evidence shows it can be introduced without unacceptable speculation/abuse;
- guild architecture proves the current guildhouse boundary must change;
- Rested/player progression evidence justifies an explicit physical-house progression distinction;
- accepted World lifecycle/merge architecture requires a different housing reconciliation boundary;
- security/duping findings prove current custody/fencing model insufficient;
- a later accepted FND/DUR/GAME/Platform contract materially changes an authority boundary consumed here.

Any supersession must explicitly preserve or replace:

- one authoritative physical property state;
- no Channel-count-driven scarcity mutation;
- item/value conservation;
- one authoritative owner/slot result;
- stale-writer fencing;
- idempotent/reconcilable settlement;
- server-authoritative ACL/auction/ownership;
- explicit lifecycle handling for Character transfer/deletion/World transfer;
- safe failure behavior.

## 31. Current-status precedence

Until coordinator-owned global status/register surfaces are reconciled after this acceptance delivery, this baseline is the authoritative source for the `EXP-HOUSES-01` **DecisionStatus**.

The intended current state is:

```yaml
EXP-HOUSES-01:
  DecisionStatus: ACCEPTED
  DeliveryStatus: IN_REVIEW
  ImplementationStatus: NOT_STARTED
```

Checkpoint PRs remain provenance until this composition delivery is integrated; they do not independently broaden implementation authority.

## 32. Decision

`EXP-HOUSES-01 DECISIONSTATUS: ACCEPTED`

`FIRST-GENERATION PRODUCT MODEL: SCARCE PHYSICAL HOUSES + NON-SCARCE RESIDENCE`

`PHYSICAL HOUSE SCOPE: ONE WORLD-GLOBAL HOUSEID / ONE AUTHORITATIVE STATE`

`PERSONAL HOUSING SLOT: ONE OF NONE | RESIDENCE | PHYSICAL_HOUSE PER ACCOUNTID + WORLDID`

`PHYSICAL HOUSE OWNER: CHARACTERID`

`GUILDHOUSE OWNER: GUILDID / DETAILED GUILD LIFECYCLE DEFERRED`

`PHYSICAL HOUSE ACQUISITION: ACTIVE PREMIUM + PHYSICALHOUSEELIGIBILITY`

`PREMIUM EXPIRY: NOT AN EVICTION TRIGGER`

`VACANT PHYSICAL HOUSE ACQUISITION: WORLD PUBLIC PROXY AUCTION`

`GENERIC DIRECT HOUSE SALE: NOT ENABLED FIRST GENERATION`

`CHARACTER BAZAAR: EXPLICIT RELINQUISH OR INCLUDE HOUSE`

`BAZAAR CONTENTS: SELLER-SELECTED HOUSE_ONLY OR EXPLICIT INCLUDE_FURNISHINGS`

`RENT: RECURRING -> GRACE -> VALUE-SAFE FENCED EVICTION -> PUBLIC AUCTION`

`ACL: OWNER -> MANAGER/SUBOWNER -> GUEST + FINE-GRAINED CAPABILITIES`

`ACL PLAYER UX: GUI/PANEL, NOT TEXT-COMMAND-ONLY`

`RESTED: BASELINE PARITY BETWEEN RESIDENCE AND ORDINARY PHYSICAL HOUSE`

`IMPLEMENTATION_AUTHORITY: NONE`
