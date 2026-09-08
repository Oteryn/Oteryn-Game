# EXP-HOUSES-01 — Housing Architecture Owner Acceptance Baseline

- Status: **OWNER-ACCEPTED WHOLE-GATE ARCHITECTURE**
- DecisionStatus: `ACCEPTED`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-09-08
- Gate: `EXP-HOUSES-01`
- Owner disposition: `ACCEPT`
- Coordination issue: `#220`
- Protected composition base: `main@b6411e9bd280a1b48a8c356492a332d084ac7672`
- Runtime/client/server/protocol/DDL/migration/Platform/production authority: **NONE**

## 1. Purpose and status axes

This baseline composes the owner-approved Oteryn housing decisions into one normative `EXP-HOUSES-01` architecture package.

It closes first-generation housing **semantic architecture** sufficiently for implementation planning while deliberately leaving numeric balance, physical schema, service decomposition, exact protocol/UI representation and rollout details tunable or downstream-owned.

```text
DecisionStatus       = ACCEPTED
DeliveryStatus       = IN_REVIEW
ImplementationStatus = NOT_STARTED
```

Architecture acceptance does not authorize executable implementation.

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

Where older architecture says housing topology or `EXP-HOUSES-01` remains unresolved, this later owner-accepted baseline wins for the exact semantic scope accepted here.

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

Guildhouses are a separate guild-owned class under `GuildId`; detailed guild lifecycle remains deferred to guild/social architecture.

## 4. Binding World / Channel / value invariants

### 4.1 World-global physical property

Every physical `HouseId` belongs to exactly one `WorldId` and is not scoped by `ChannelId`.

The following exists once across all Channels of the World:

- ownership;
- auction/allocation lifecycle;
- rent/delinquency/eviction;
- ACL/access revisions;
- durable house item/container state.

Adding/removing a Channel must not create houses, copy houses, reset rent, duplicate auctions or change physical-house scarcity.

### 4.2 Rejected Channel models

First generation rejects:

- independent physical-house copies per Channel;
- Channel-local `HouseId`;
- Channel-local ownership/rent/ACL;
- separately writable regional mirrors of one house interior;
- using infrastructure scaling to manufacture scarce addresses.

### 4.3 Durable items and stale writers

Housing does not create a parallel item/value authority.

Accepted `GAME-ITEM-01` / `DUR-03` semantics remain binding:

- every durable item has one authoritative semantic location/custody;
- ordered/revisioned/retry-safe mutations where required;
- duplicate retries do not duplicate value;
- ambiguous commit reconciles instead of starting a blind replacement operation;
- stale writers cannot overwrite newer authoritative state.

## 5. Physical-house runtime topology

### 5.1 One authoritative active interior

When active, one physical house has one logical authoritative world-scoped interior runtime owner.

Characters entering the same `HouseId` from different Channels converge on the same authoritative presence/item state, not independent copies.

### 5.2 Instance primitive reuse

Implementation may reuse instance-style runtime/transfer primitives, but canonical property identity remains `HouseId`.

Any internal `InstanceId`, runtime ID or generation is runtime-lifecycle identity only and does not replace `HouseId` in ownership, auction, rent, ACL, property history or durable item semantics.

### 5.3 Entry, exit and origin Channel

House entry is an explicit authoritative context/ownership handoff, not a hidden Channel switch.

The Character retains validated origin-Channel routing metadata. Normal exit returns through validated origin routing unless a separately accepted same-World fallback is required because origin is unavailable/draining/incompatible.

Entry/exit cannot bypass:

- combat/PvP Channel-switch restrictions;
- admission/capacity;
- session/lease fencing;
- protected transactions;
- revision compatibility;
- duplicate-session prevention.

### 5.4 Recovery

One active interior has one logical mutation owner at a time.

Crash recovery/replacement/relocation requires generation/revision fencing so an old runtime cannot resume mutation after a newer owner becomes authoritative.

Durable owner/item truth remains in the World durable domain; rebuildable runtime state is reconstructed only from accepted authoritative state.

## 6. Residence / Apartment model

### 6.1 Product role

Residence is a separate non-scarce housing capability for genuine personal housing, including Free/non-Premium players.

Baseline direction includes:

- persistent personal interior;
- decoration/personalization;
- graphical access management;
- baseline social/housing utility;
- compatibility with accepted Rested semantics.

It is not a crippled Premium preview.

### 6.2 Scope

At most one Residence may belong to one `AccountId + WorldId`.

Account scope does **not** automatically authorize unrestricted item transfer or a shared warehouse between alternate Characters. Exact cross-Character access to placed/storage items remains item/storage policy and must preserve `DUR-03` authority/conservation.

### 6.3 Market/scarcity boundary

First-generation Residence requires no:

- public scarcity auction;
- player-to-player resale;
- scarce-address nomination/transfer;
- market price driven by finite address supply.

### 6.4 Runtime boundary

Residence may later use instance-style runtime primitives, but active simulation has one authoritative owner, Channel scaling cannot duplicate contents, durable value remains under World durable authority, and origin/runtime ownership transitions are explicit/fenced.

Exact `ResidenceId`, hosting, templates and persistence representation remain deferred.

## 7. Personal housing-slot invariant

For each `AccountId + WorldId`:

- max one Residence;
- max one ordinary physical house;
- never both simultaneously;
- alternate Characters cannot bypass the aggregate slot;
- another World has an independent slot.

This aggregate guard does not replace canonical physical ownership `HouseId -> CharacterId`.

### 7.1 Residence -> physical house

Before physical-house acquisition/retention commits:

1. revalidate current slot;
2. move Residence item/value state to accepted authoritative custody where required;
3. fence stale Residence runtime authority;
4. release Residence lifecycle/entitlement state;
5. commit physical house as the one personal housing class.

### 7.2 Physical house -> Residence

Residence cannot become authoritative while same Account/World still consumes a physical-house slot. Physical property/content state must first be safely relinquished/evicted/disposed.

### 7.3 Failure/retry

Ambiguous transition is not success. Recovery rereads authoritative slot/property/item/operation state and reconciles using the same logical operation identity.

## 8. Ordinary physical-house owner identity

Canonical public/semantic owner of an ordinary physical `HouseId` is one `CharacterId` in the owning World.

`AccountId` is used for personal-slot enforcement, acquisition eligibility, anti-concentration and anti-abuse; it does not replace `HouseId -> CharacterId` ownership.

Consequences:

- rename preserves ownership;
- multiple Characters of one Account cannot each own a same-World ordinary physical house;
- Character Bazaar requires explicit housing disposition;
- terminal Character deletion requires housing settlement first;
- World transfer cannot move a physical `HouseId` to another World.

## 9. Guildhouse boundary

Guildhouse canonical owner is `GuildId` and it does not consume the personal Account/World slot.

Selling a guild leader Character does not transfer the guildhouse merely because Account ownership of the Character changes.

Detailed guildhouse acquisition, leadership succession, rank ACL, rent funding, dissolution/merge, Rested and lifecycle interactions remain deferred until the guild system exists.

## 10. PhysicalHouseEligibility and Premium

Acquiring or retaining a **new** ordinary scarce physical-house slot requires BOTH:

```text
active Premium
+
PhysicalHouseEligibility
```

Neither gold alone nor Premium alone is sufficient.

`PhysicalHouseEligibility` must represent meaningful real participation/progression on the relevant World so trivial fresh accounts cannot immediately become cheap property-holding shells.

Exact thresholds remain tunable. Candidate input classes may include account maturity, Character progression, real activity/play history, conduct/security state or another explicit game-owned qualification rule.

The rule applies when the Account actually acquires/retains the incoming physical house, including:

- public auction settlement;
- Bazaar choice `KEEP_INCOMING_HOUSE`.

### 10.1 Premium lapse

Premium expiration after legitimate acquisition does **not** itself evict the owner or release property.

Retention is governed by in-game rent/grace/eviction/lifecycle rules. Reversing this requires explicit owner supersession.

## 11. Public physical-house auction

Vacant physical houses use one World-scoped public allocation lifecycle, including newly available, voluntarily relinquished, Bazaar-released and evicted properties.

### 11.1 Proxy bidding

- bidder submits a private maximum;
- automatic bidding advances only when actual competition requires;
- automated bid never exceeds maximum;
- winner pays minimum valid amount required to beat the next-best maximum under a later-defined increment, not automatically full maximum.

### 11.2 Funds backing

A winning-capable bid must be backed by authoritative funds reservation/escrow semantics before final value consumption. Housing does not invent a parallel money authority.

### 11.3 Anti-sniping

A valid bid near close extends auction under an extension rule so last-millisecond network timing is not the dominant winner mechanic.

Exact duration, extension window and increment remain deferred balance values.

### 11.4 Final eligibility, winner selection and settlement

Auction settlement is authoritative, idempotent, revision/fence aware, reconciliation-safe, compatible with the personal housing slot and value-conserving.

Final winner selection uses an **effective valid-bid set**. A bid participates in winner selection and proxy-price formation only while its bidder still satisfies all acquisition guards required at the final selection/settlement boundary, including as applicable:

- same owning `WorldId`;
- an available same-World personal housing slot for the incoming physical house;
- active Premium;
- current `PhysicalHouseEligibility`;
- current bid/funds reservation validity;
- any other separately accepted final acquisition guard.

If a previously leading bidder no longer satisfies a final acquisition guard:

1. that bid is excluded from the effective valid-bid set;
2. its private maximum is also excluded from proxy-price formation and cannot raise the price paid by a valid bidder;
3. the bidder's reservation/escrow state is released or reconciled idempotently under the owning economy contract;
4. winner and payable price are recomputed from the remaining effective valid bids under the same later-defined increment/tie semantics.

If no valid bids remain, no property or funds transfer occurs. The `HouseId` stays vacant and remains/returns in the public allocation lifecycle; an invalid top bid cannot by itself block allocation or become a price anchor.

Exact auction duration, cadence between allocation attempts, increment and tie rule remain deferred. The validity/exclusion/repricing semantics above are not deferred cancellation policy.

Timeout or dependency unavailability is not proof of property/funds transfer and requires reconciliation of the same logical auction settlement.

### 11.5 No generic direct house market at launch

First generation does not expose a generic player-to-player standalone `HouseId` sale at arbitrary price.

Vacant physical houses return to public allocation. A future direct property market requires explicit owner supersession supported by player/economy evidence.

## 12. Rent, delinquency and eviction

Physical-house rent is recurring and World-scoped. Exact formula, cadence and funding-source representation remain deferred.

### 12.1 Collection

Collection is automatic from a later-defined authoritative economy source and idempotent/reconcilable. Channel count/current Channel/runtime placement cannot duplicate rent or reset lifecycle.

### 12.2 Grace

Insufficient funds enter explicit delinquent/grace state, not immediate silent eviction. Exact grace/notification values remain deferred.

### 12.3 Value-safe eviction

Before ownership release after unresolved grace:

1. revalidate owner/revision/rent state;
2. move durable contents/value to safe authoritative reclaim/depot/custody;
3. fence stale HouseRuntime/Channel mutation rights;
4. release ownership and ACL;
5. mark property vacant;
6. return it to public allocation.

Forgotten items are not destroyed and do not silently become property of the next owner.

## 13. Character Bazaar housing disposition

Platform retains Bazaar commercial workflow authority; Character ownership rebinding remains Game Character Authority-owned; housing owns housing disposition/slot/property effects.

House never follows Character sale silently.

Seller chooses:

```text
RELINQUISH_HOUSE
or
INCLUDE_HOUSE_WITH_CHARACTER
```

### 13.1 RELINQUISH_HOUSE uses staged, non-allocatable disposition

`RELINQUISH_HOUSE` is a staged Bazaar housing disposition, not an immediate public release of the property before the Character transfer outcome is known.

Before Character Authority attempts Account rebinding, housing may prepare the seller-approved disposition under the stable Bazaar operation identity by:

1. revalidating the expected `HouseId -> CharacterId`, property revision and seller authority;
2. placing the property into an explicit `BAZAAR_DISPOSITION_PENDING`-equivalent semantic state;
3. fencing conflicting house/disposition mutations while that state is active;
4. moving affected movable value into explicit typed pending/reclaim custody where required and where the transition remains idempotently reconcilable.

While disposition is pending:

- the house is **not vacant and not publicly allocatable**;
- it cannot enter or influence a new public house auction;
- the prior property ownership/disposition state remains recoverable until authoritative Character transfer outcome is known;
- normal conflicting property use/disposition is fail-closed according to the later concrete lifecycle contract.

The authoritative Character transfer outcome then determines housing finalization:

- **Character transfer committed:** finalize the already seller-authorized house release, settle remaining custody/ACL effects, mark the property vacant, and only then enter public allocation;
- **Character transfer rejected/aborted before Account rebinding commit:** cancel/reconcile the pending disposition and preserve or restore the prior seller-side house ownership/personal-slot result without public allocation or value loss;
- **Character transfer outcome ambiguous/unavailable:** keep the house pending and non-allocatable while the same Bazaar operation is reconciled; do not guess, auction the house, or start a replacement disposition.

The pending disposition authorization is bound to the seller's pre-transfer authority, expected property revision and Bazaar operation identity. If Account rebinding commits before the final housing release step completes, the new Account owner does not gain an opportunity to cancel or repurpose that already committed seller-authorized disposition; normal house use remains fenced until the disposition reconciles to its terminal outcome.

This ordering does not create distributed ACID. It prevents an irreversible public property release from racing ahead of the authoritative Character-transfer result while preserving stable-operation idempotency and reconciliation.

### 13.2 INCLUDE_HOUSE_WITH_CHARACTER

```text
CharacterId C owns HouseId H
AccountId A owns CharacterId C

Bazaar transfers CharacterId C to AccountId B

=> CharacterId C remains canonical owner of HouseId H
=> Character ownership binding changes A -> B
=> AccountId B resolves its same-World personal housing slot
```

This is an explicit Character+house sale case, not standalone `HouseId` trading.

### 13.3 Buyer already has physical house

Buyer chooses:

```text
KEEP_EXISTING_HOUSE
or
KEEP_INCOMING_HOUSE
```

Only one remains. Non-kept house is value-safely relinquished to public allocation.

`KEEP_INCOMING_HOUSE` requires final active Premium + `PhysicalHouseEligibility` because a new incoming physical-house slot is retained.

`KEEP_EXISTING_HOUSE` relinquishes incoming house. The Character purchase itself does **not** require physical-house Premium/eligibility merely because the listing contained a house; existing-house retention remains governed by its existing rent/lifecycle and Premium lapse rule.

If the listing selected `INCLUDE_FURNISHINGS` but the buyer chooses `KEEP_EXISTING_HOUSE`, the furnishings transfer does not commit. Section 14.2 claimant-aware non-retention disposition must complete or remain explicitly recoverable before the incoming house may become publicly allocatable.

A house on another World does not conflict with this per-World slot.

### 13.4 Buyer has Residence

Buyer chooses:

```text
KEEP_RESIDENCE
or
KEEP_INCOMING_HOUSE
```

`KEEP_INCOMING_HOUSE` revalidates active Premium + eligibility and value-safely settles/releases Residence first.

`KEEP_RESIDENCE` relinquishes incoming physical house to public allocation and does not require physical-house acquisition eligibility merely to buy the Character.

If the listing selected `INCLUDE_FURNISHINGS` but the buyer chooses `KEEP_RESIDENCE`, the furnishings transfer does not commit. Section 14.2 claimant-aware non-retention disposition must complete or remain explicitly recoverable before the incoming house may become publicly allocatable.

Any physical house that must be relinquished because of the buyer's keep-choice follows the same safety principle as seller `RELINQUISH_HOUSE`: public vacancy/allocation is exposed only after the Character transfer outcome that makes that disposition applicable is authoritative; ambiguity leaves the disposition pending/non-allocatable and reconcilable.

### 13.5 Reconciliation boundary

No distributed ACID between Platform/Character/housing/economy/item domains is assumed.

One semantic outcome is preserved through stable operation identity, current-state revalidation, explicit pending states, fencing, idempotent steps, typed custody, durable operation evidence and reconciliation after timeout/ambiguity.

Public allocation is a terminal housing effect, not a PREPARE step. A property whose Bazaar disposition still depends on an unresolved Character transfer outcome remains non-allocatable.

Cached listing state never proves current housing eligibility or terminal Character-transfer outcome.

## 14. Bazaar house contents: seller decides

For `INCLUDE_HOUSE_WITH_CHARACTER`, seller additionally chooses:

```text
HOUSE_ONLY
or
INCLUDE_FURNISHINGS
```

House location, ACL role and storage permission do not by themselves prove who may dispose of a durable item. Every movable item affected by Bazaar housing settlement is classified item-by-item under existing GAME-ITEM/DUR-03 binding, custody and authorization semantics before a transfer or reclaim outcome is allowed.

### 14.1 HOUSE_ONLY uses claimant-aware typed reclaim custody

Character + address may transfer, but ordinary movable contents excluded by `HOUSE_ONLY` do not silently become buyer property and are not placed into the sold Character's ordinary depot/inventory as a shortcut.

For each affected `ItemInstanceId`, the settlement resolves its authoritative legal claimant/disposition independently. An item placed by a guest, manager or other actor cannot be reclassified as seller property merely because it is physically inside the seller's house or because the actor had storage access.

When a direct legal destination for the current claimant is unavailable or the multi-step Bazaar operation needs a non-gameplay holding state, the architecture requires a distinct DUR-03 typed custody family equivalent in semantics to:

```text
BazaarHousingReclaimCustody {
    world_id: WorldId,
    bazaar_operation_id: OperationId,
    item_instance_id: ItemInstanceId,
    claimant_ref: TypedItemClaimantRef,
}
```

`TypedItemClaimantRef` is not a generic `owner_id` and does not invent a new ownership authority. It is a typed reference to the already-lawful claimant subject resolved from the owning item/binding/custody rules at the Bazaar prepare boundary. It may identify the pre-transfer seller Account only for an item for which that Account is actually the lawful reclaim claimant; another Character/account/domain claimant remains distinct.

The exact Rust/DDL identifier is deliberately not frozen, but the custody semantics are:

- **stable key/scope:** `WorldId + BazaarOperationId + ItemInstanceId + typed claimant`, with each item retaining exactly one authoritative immediate custody location;
- **claimant preservation:** settlement records and preserves the authoritative per-item claimant/disposition basis; house ownership/ACL never rewrites it;
- **mutation owner:** the game-owned item/value custody boundary under `DUR-03`, coordinated by the Bazaar housing settlement; Platform/client state is never custody authority;
- **lifecycle:** an item may return directly to an existing legal claimant destination, or enter typed non-gameplay custody before/while Character+house transfer settles; custody items remain non-spendable/non-usable until legal exit;
- **authorization boundary:** claimant identity is not general gameplay mutation authority and does not create Account-wide or cross-Character warehouse semantics;
- **legal exit:** withdrawal/materialization must choose a separately legal destination under the owning item/depot/mail/reclaim rules and must itself be value-conserving, fenced and idempotent.

Consequences:

- the sold Character and buyer Account B cannot claim `HOUSE_ONLY` contents merely because `CharacterId`/house ownership moved;
- the pre-transfer seller Account A may reclaim only items for which it is the resolved lawful claimant; it cannot absorb a guest/manager/other claimant's item;
- an alternate Character of a claimant Account does not gain direct access merely from matching `AccountId`;
- no generic `owner_id`, free-form JSON/string location or sold-Character depot is accepted as the custody representation;
- if Bazaar transfer aborts before becoming authoritative, the same operation reconciles pending custody/direct-return state back to each item's legal pre-transfer result rather than duplicating, abandoning or reassigning value.

The exact physical container/table, reclaim UI, retention/capacity policy and legal final withdrawal destinations remain downstream implementation/item-storage decisions; per-item claimant preservation, typed scope, authority and lifecycle semantics above are binding.

### 14.2 INCLUDE_FURNISHINGS is conditional on retaining the incoming house

Eligible items transfer to the buyer only through an explicit authoritative manifest/bundle **and only if the buyer's final keep-choice is `KEEP_INCOMING_HOUSE`**.

An item is eligible for the furnishings manifest only when its authoritative current claimant/binding/custody policy permits the seller-authorized transfer. House placement or seller ownership of the `HouseId` alone does not authorize transfer of an item whose legal claimant is another Character/account/domain subject.

Required invariants when `KEEP_INCOMING_HOUSE` commits:

- seller opt-in;
- exact set server-authoritative;
- every manifest item is individually transfer-authorized under its owning item policy;
- `DUR-03` identity/location/conservation;
- stale client cannot alter committed set;
- transferred item cannot remain usable in old claimant/custody;
- ambiguous settlement reconciles, never duplicates.

If buyer chooses `KEEP_EXISTING_HOUSE` or `KEEP_RESIDENCE`, the incoming physical house is not retained and the furnishings transfer **must not commit**. Before that house becomes vacant/publicly allocatable:

1. freeze the manifest candidate set under the same Bazaar operation identity;
2. classify each candidate item by its authoritative pre-transfer legal claimant/disposition;
3. return it to an already-legal claimant destination where possible, otherwise move it to the claimant-aware typed `BazaarHousingReclaimCustody` semantics from section 14.1;
4. prove that no manifest item remains in the relinquished house as a gift to a future occupant and no item becomes buyer or seller property without independent legal authority;
5. only then allow the non-kept house to complete public release.

If Character transfer, buyer keep-choice, item disposition or custody outcome is ambiguous, the house remains pending/non-allocatable and the same operation reconciles both the manifest and property outcome; a timeout cannot silently commit furnishings transfer or public release.

Exact manifest schema, item eligibility detail, capacity, valuation, claimant destination UI and physical custody representation remain deferred.

## 15. Character deletion and World transfer

Terminal Character deletion/finalization cannot leave a physical house bound to nonexistent owner state; housing settlement must complete or remain explicitly recoverable first.

A physical `HouseId` is a World address and cannot migrate across Worlds. Character World transfer must settle/relinquish incompatible physical property first.

Residence is also World-scoped under `AccountId + WorldId`; exact transfer UX is future World-lifecycle work, but no workflow may duplicate Residence contents or personal slots.

## 16. ACL hierarchy and authority

Housing ACL is server-authoritative, revisioned World-scoped state.

```text
OWNER
  -> MANAGER / SUBOWNER
      -> GUEST
```

### 16.1 OWNER

Owner has ultimate personal-property administration subject to lifecycle/economy rules. Only owner-authorized workflows may change ownership/disposition state.

### 16.2 MANAGER / SUBOWNER

Delegated administration only, never ownership. It may manage bounded access but cannot independently sell/relinquish property, bypass rent/eligibility/slot rules, rebind owner or override owner settlement.

### 16.3 GUEST

Guest gets only explicitly granted capabilities. Entry permission does not automatically grant every function.

### 16.4 Fine-grained direction

ACL may distinguish property entry, specific doors, room/zone, bed use, storage use, workstation/interactive feature and later accepted capabilities. Storage permission never implies transfer ownership or claimant authority over items placed by that actor or by somebody else. Exact vocabulary remains downstream detail if least-authority semantics are preserved.

### 16.5 Revocation

At meaningful entry/mutation boundaries the server revalidates current ACL revision/authority. Stale client/cache/runtime cannot retain revoked authority.

## 17. House Management GUI is the only player-facing ACL administration path

Player-facing creation, editing and removal of housing access permissions MUST be performed through a graphical House Management UI/panel.

First-generation Oteryn MUST NOT expose player-facing Tibia-style text commands/spells (for example `Aleta Sio`, `Aleta Som`, `Aleta Grav`) as an alternative ACL-administration path.

This is a product/authority requirement, not merely a preference for one presentation over another. Internal operator/admin tooling, if later required, is separately governed and is not a player-facing bypass.

The GUI remains presentation/intent only:

- server housing state is authoritative;
- local UI edits grant no authority by themselves;
- mutations carry expected revision/fence context where required;
- stale/invalid mutations fail explicitly and do not partially apply old state.

Exact screen layout, widgets, search/filter UX and protocol representation remain deferred.

## 18. Rested parity

Residence and ordinary physical-house eligible rest use the same baseline Rested recovery semantics.

Scarcity, price or Premium-gated physical-house acquisition does not itself grant a stronger per-Character Rested multiplier.

Physical houses may differ by non-power/social/space traits such as number of beds, layout, guest capacity, location/prestige, decoration surface and convenience.

Rested pool size/rate/timing, bed counts, bed ACL, Residence-bed requirement, offline-training interaction and general Premium Rested behavior not tied specifically to scarce-house ownership remain tunable/deferred.

Numeric tuning that preserves equivalent baseline parity does not reopen topology/ownership. Material progression advantage specifically for physical-house ownership requires explicit owner supersession.

## 19. Storage and item placement

Physical-house and Residence items use one authoritative semantic location/custody and cannot be copied per Channel.

Free Residence is not authorization for unlimited free durable storage. Exact placement/storage budgets and representation remain future item/storage/economy decisions.

Account-scoped Residence ownership does not automatically make placed/storage items account-wide transferable between alternate Characters.

Housing ownership, ACL role and storage access do not supersede item binding/claimant semantics. Eviction, relinquishment, Residence replacement and Bazaar disposition resolve affected durable items item-by-item and move value through explicit authoritative custody/location transitions before prior property/runtime authority is released.

For Bazaar `HOUSE_ONLY`, and for `INCLUDE_FURNISHINGS` when the buyer does not retain the incoming house, required reclaim/holding state uses the claimant-aware typed custody semantics defined in section 14; it is not a generic Account warehouse and not automatically the sold Character's depot.

Destroying, duplicating, abandoning, misassigning or silently gifting forgotten items is not an acceptable simplification.

## 20. Failure and recovery semantics

Housing distinguishes deterministic policy rejection, stale owner/revision/slot, stale runtime/session authority, funds/escrow conflict, item/custody conflict, dependency unavailable and ambiguous durable result.

For ambiguity:

1. do not fabricate success/failure;
2. reread authoritative operation/property/slot/item/value state;
3. reconcile using same logical operation identity;
4. do not create a blind replacement mutation;
5. fail closed for risky new mutation while required authority is unavailable.

Safe read-only/presentation degradation may be separately proven later; it is not implied.

## 21. Cross-region behavior

One World may host Channels in multiple regions. Housing cannot solve latency by creating independently writable house copies.

Accepted trade-off:

- ordinary regional Channel combat/movement/tick stays free of added synchronous WAN persistence round trips;
- explicit shared durable house/item/ownership operations may pay latency to authoritative World durable boundary;
- runtime placement/migration may later optimize latency without changing property identity or creating second durable writer.

Exact placement/migration is PERF/OPS work.

## 22. Authority boundaries

### 22.1 Game housing domain

Semantic authority for `HouseId` lifecycle/availability, physical owner binding, personal housing slot, Residence entitlement/lifecycle, ACL revisions, rent/delinquency/eviction, final housing disposition and active housing-runtime ownership.

This semantic domain need not map to one service/process/table.

### 22.2 Character Authority

Remains authority for Character lifecycle/current Account owner/current World and Character ownership rebinding. Housing may gate lifecycle but does not seize Character authority.

### 22.3 Platform

Remains authority for Platform Account identity, entitlement/commercial source data under accepted contracts, portal/commercial UX and Bazaar commercial saga. Platform cache/listing is not authoritative housing proof.

### 22.4 Economy / item domains

Funds and durable value mutations remain with accepted/future economy and `DUR-03` owners. Housing declares required effects without inventing distributed ACID or duplicate value authority.

The claimant-aware typed `BazaarHousingReclaimCustody` semantics in section 14 define a required housing/Bazaar use of a DUR-03 custody family; they preserve each item's already-lawful claimant and do not move item mutation authority to Platform, Account identity, house owner identity or the housing UI.

### 22.5 Client

Client is untrusted presentation/input. It cannot establish ownership, auction winner, ACL authority, item manifest or settlement success.

## 23. Anti-speculation posture

First-generation controls combine:

- active Premium when a new scarce physical house is acquired/retained;
- independent `PhysicalHouseEligibility`;
- one personal slot per Account/World;
- no standalone private `HouseId` resale;
- public allocation of vacant properties;
- Bazaar requiring transfer of owning Character rather than address alone;
- incoming-house eligibility revalidation when retained;
- recurring in-game rent as holding cost;
- auditable authoritative settlement.

This does not claim multi-account speculation is impossible.

Exact account-linking/risk scoring, cooldowns, taxes, holding periods, Bazaar surcharges and sanctions remain evidence-driven. Weak signals such as shared IP/device do not automatically become punitive ownership decisions without separately accepted abuse/security policy.

## 24. Observability requirements

Later implementation must retain enough evidence to diagnose:

- auction bid validity at final selection, invalid-bid exclusion and resulting proxy-price/winner recomputation;
- auction disputes and reservation release/reconciliation;
- ownership history;
- personal-slot conflicts/transitions;
- Premium/eligibility decision version/input class;
- rent/delinquency/eviction;
- ACL revisions/privileged changes;
- Bazaar housing disposition/keep-choice;
- `BAZAAR_DISPOSITION_PENDING` lifecycle and terminal Character-transfer outcome used to finalize/cancel it;
- claimant resolution/disposition basis per affected `ItemInstanceId`;
- typed Bazaar housing reclaim-custody key/claimant/item transitions;
- furnished-transfer manifest, buyer keep-choice and transfer-versus-reclaim result;
- reclaim/custody transitions;
- runtime generations/stale-writer rejection;
- ambiguous-operation reconciliation.

Analytics/audit projections observe owning-domain evidence and do not become mutation authority.

## 25. Required conformance scenarios before activation

A future implementation must prove at least:

1. Channel scaling does not change physical-property supply/state.
2. Characters from different Channels entering same `HouseId` converge on one authoritative interior/item state.
3. House entry/exit preserves validated origin routing and cannot bypass Channel-switch rules.
4. Old runtime generation cannot mutate after replacement/recovery.
5. Ambiguous auction retry yields one owner/payment result.
6. Stale acquisition eligibility at final settlement fails without inconsistent property/value mutation.
7. Premium lapse after ownership does not evict by itself.
8. Concurrent workflows cannot commit Residence + physical house or two ordinary physical houses for same Account/World.
9. Residence -> house replacement settles value and produces one final class.
10. Rent delinquency + valid payment within grace does not double-charge/evict.
11. Eviction moves contents to safe custody before ownership release/public allocation.
12. Bazaar `RELINQUISH_HOUSE` cannot finalize with ambiguous old ownership.
13. Bazaar include/no conflict preserves same `CharacterId` house owner through Account rebinding.
14. Bazaar existing-house conflict requires keep-choice and produces one physical house.
15. Bazaar Residence conflict requires keep-choice and produces one personal housing class.
16. Buyer keeping existing house/Residence may buy Character without incoming-house acquisition eligibility because incoming house is relinquished.
17. `HOUSE_ONLY` prevents accidental furnishings transfer.
18. `INCLUDE_FURNISHINGS` with `KEEP_INCOMING_HOUSE` transfers the exact authoritative, individually transfer-authorized manifest once without duplication.
19. ACL revocation rejects stale access.
20. Local GUI manipulation alone cannot change effective ACL.
21. Player-facing text-command ACL mutation is unavailable/rejected; GUI/panel is the supported player path.
22. Equivalent Residence/physical-house baseline rest has no physical-house-only multiplier.
23. Character finalization cannot leave orphan physical-house owner state.
24. World transfer cannot move/duplicate `HouseId` across Worlds.
25. Risky housing mutation fails closed during authority outage.
26. Restore validates owner, slot, item custody graph and runtime fences before mutation resumes.
27. Account-scoped Residence does not silently authorize cross-Character item transfer.
28. Highest proxy bidder becomes ineligible before close: its bid/max is excluded from both winner selection and price formation; the winner/price are recomputed from remaining valid bids, or no transfer occurs if none remain.
29. `RELINQUISH_HOUSE` enters pending non-allocatable disposition; Character rebinding rejection preserves/restores the seller-side house result and never exposes the property to public auction.
30. `HOUSE_ONLY` with mixed item claimants preserves claimant per item: a seller-claimable item may enter seller-claimant custody, while a guest/manager/other claimant's item returns to its legal claimant destination or claimant-keyed typed custody and cannot be reclaimed by the house seller merely because of location/ACL.
31. `INCLUDE_FURNISHINGS` + buyer `KEEP_EXISTING_HOUSE` or `KEEP_RESIDENCE`: furnishings transfer does not commit; every manifest candidate is returned/routed by authoritative claimant before the incoming house becomes publicly allocatable.

## 26. Deliberately deferred

Whole-gate acceptance does **not** freeze:

### Auction / rent / economy numbers

- rent amount/formula/cadence;
- grace duration;
- auction duration/increment/anti-sniping window;
- reserve/start price/tie/cancellation rules that do not alter the binding invalid-bid exclusion/repricing semantics;
- cadence/restart UX when no valid auction bids remain;
- fees/taxes/surcharges;
- reclaim fees/capacity;
- anti-flipping cooldown/holding periods if later needed.

### Eligibility / anti-abuse numbers

- account age;
- level/progression;
- playtime/activity;
- conduct/security policy;
- risk signals/scoring/sanctions;
- threshold visibility.

### Residence details

- final public name;
- exact ID representation;
- acquisition quest/progression;
- templates/layout/size;
- decoration/item-placement budgets;
- storage representation;
- maintenance/rent if any;
- Premium cosmetic/convenience upgrades;
- monetization prices.

### Beds / Rested

- numeric pool/rate/timing;
- bed counts/capabilities;
- offline-training integration;
- general Premium Rested policy not tied specifically to scarce-house ownership.

### Guildhouse

Detailed lifecycle/admin/economy/Rested rules remain downstream of future guild/social architecture.

### Physical implementation

- PostgreSQL tables/indexes/constraints;
- isolation/locking implementation;
- service/process/crate decomposition;
- RPC/HTTP/internal IDL;
- exact operation/revision/runtime-generation representation;
- exact storage representation for pending Bazaar disposition and claimant-aware housing reclaim custody;
- claimant-resolution/reclaim screen/workflow and legal final destination choices consistent with the binding custody authorization boundary;
- runtime placement algorithm;
- client screen layout;
- protocol messages;
- rollout/migration/flags;
- SLO/capacity values.

## 27. Rejected first-generation defaults

The accepted architecture rejects:

1. per-Channel physical-house copies;
2. Channel-local physical-house ownership;
3. independently writable regional interior mirrors;
4. scarce physical house as the only baseline housing path;
5. generic private `HouseId` resale at launch;
6. Premium lapse as automatic property loss;
7. stronger Rested multiplier solely because property is scarce/Premium-gated;
8. player-facing text-command ACL administration, including as an alternative bypass to GUI;
9. client-authoritative ownership/auction/ACL/item settlement;
10. Residence plus physical house simultaneously on same Account/World;
11. using an ineligible/invalid auction maximum to determine a valid bidder's payable price;
12. exposing a Bazaar-relinquished house to public allocation before the Character transfer outcome that authorizes final release is authoritative;
13. treating `HOUSE_ONLY` contents as the sold Character's depot contents or as generic Account-wide storage;
14. assigning the house seller as claimant of an item merely because that item was placed inside the house or by an ACL-authorized guest/manager;
15. leaving `INCLUDE_FURNISHINGS` items in an incoming house that the buyer chose not to retain, thereby gifting them to a future occupant.

## 28. Explicit supersession

### 28.1 ADR-0001 section 11

`ADR-0001-native-rust-multichannel-platform.md` remains authoritative history and preserves one-house-per-logical-World state invariants.

Its statement that final physical-house presence/topology remains unresolved is superseded by:

```text
physical HouseId
-> one World-global property
-> one authoritative active world-scoped interior runtime
-> explicit fenced entry/exit
-> preserved origin Channel routing
```

Historical ADR need not be rewritten.

### 28.2 Multichannel scope matrix

Its safety invariants remain binding. Its provisional/deferred physical-house topology/presence wording is superseded for `DecisionStatus` by this baseline.

### 28.3 Gap/horizon/global register

Older `REGISTERED_UNRESOLVED`, `DEFERRED` or future-gate wording for `EXP-HOUSES-01` is superseded for this accepted semantic scope. Coordinator/status surfaces may be reconciled later in a bounded status-only update; until then this baseline has DecisionStatus precedence.

### 28.4 Checkpoint PRs

PRs `#435`, `#436`, `#437`, `#438`, `#440`, `#442`, `#443`, `#444`, `#445`, `#446` remain decision provenance.

After canonical integration of this composition their housing semantic content is superseded and they should be closed/superseded rather than independently merged as competing authorities.

## 29. What acceptance enables

After canonical integration **and separate implementation authorization**, downstream work may design persistence/schema/transactions, HouseRuntime/ResidenceRuntime, protocol/client GUI, auction, rent/eviction/reclaim, Bazaar housing integration, item placement/custody, observability/conformance and balance registries.

Implementation cannot invent deferred guild/product/balance values merely because semantic architecture is accepted.

## 30. Supersession criteria

Reopen this architecture only with concrete evidence such as measured player/economy harm, runtime infeasibility of one-authority interiors, observed abuse requiring material property-model change, strong demand/evidence for direct property market, future guild architecture conflict, progression evidence for deliberate physical-house distinction, World-lifecycle reconciliation needs, security/duping findings, or later accepted upstream authority changes.

Any supersession must explicitly preserve or replace:

- one authoritative physical property state;
- no Channel-count-driven scarcity mutation;
- item/value conservation;
- one authoritative owner/personal-slot result;
- stale-writer fencing;
- idempotent/reconcilable settlement;
- server-authoritative ownership/auction/ACL;
- explicit Character/Bazaar/World-transfer lifecycle handling;
- safe failure behavior.

## 31. Current-status precedence

Until coordinator-owned status surfaces are reconciled, this file is authoritative for `EXP-HOUSES-01 DecisionStatus`:

```yaml
EXP-HOUSES-01:
  DecisionStatus: ACCEPTED
  DeliveryStatus: IN_REVIEW
  ImplementationStatus: NOT_STARTED
```

Checkpoint PRs remain provenance while this delivery is in review and do not authorize runtime.

## 32. Decision

`EXP-HOUSES-01 DECISIONSTATUS: ACCEPTED`

`PRODUCT: SCARCE PHYSICAL HOUSES + NON-SCARCE RESIDENCE`

`PHYSICAL HOUSE: ONE WORLD-GLOBAL HOUSEID / ONE AUTHORITATIVE STATE`

`PERSONAL SLOT: NONE | RESIDENCE | PHYSICAL_HOUSE PER ACCOUNTID + WORLDID`

`PHYSICAL HOUSE OWNER: CHARACTERID`

`GUILDHOUSE OWNER: GUILDID / DETAILED GUILD LIFECYCLE DEFERRED`

`PHYSICAL HOUSE ACQUISITION: ACTIVE PREMIUM + PHYSICALHOUSEELIGIBILITY WHEN NEW HOUSE IS RETAINED`

`PREMIUM EXPIRY: NOT AN EVICTION TRIGGER`

`VACANT PHYSICAL HOUSE ACQUISITION: WORLD PUBLIC PROXY AUCTION`

`AUCTION FINAL VALIDITY: INVALID BIDS/MAXIMA EXCLUDED FROM WINNER AND PRICE FORMATION; RECOMPUTE FROM VALID BIDS`

`GENERIC DIRECT HOUSE SALE: NOT ENABLED FIRST GENERATION`

`CHARACTER BAZAAR: EXPLICIT RELINQUISH OR INCLUDE HOUSE + BUYER KEEP-CHOICE`

`BAZAAR RELINQUISH: PENDING + NON-ALLOCATABLE UNTIL AUTHORITATIVE CHARACTER-TRANSFER OUTCOME`

`BAZAAR CONTENTS: HOUSE_ONLY OR EXPLICIT INCLUDE_FURNISHINGS`

`BAZAAR CONTENT RECLAIM: ITEM-BY-ITEM AUTHORITATIVE CLAIMANT VIA TYPED NON-GAMEPLAY DUR-03 CUSTODY`

`INCLUDE_FURNISHINGS: TRANSFER ONLY WHEN INCOMING HOUSE IS RETAINED; OTHERWISE CLAIMANT-AWARE RECLAIM BEFORE PUBLIC RELEASE`

`RENT: RECURRING -> GRACE -> VALUE-SAFE FENCED EVICTION -> PUBLIC AUCTION`

`ACL: OWNER -> MANAGER/SUBOWNER -> GUEST + FINE-GRAINED CAPABILITIES`

`ACL PLAYER UX: GUI/PANEL ONLY; PLAYER-FACING TEXT-COMMAND ACL ADMINISTRATION FORBIDDEN`

`RESTED: BASELINE PARITY BETWEEN RESIDENCE AND ORDINARY PHYSICAL HOUSE`

`RESIDENCE ACCOUNT SCOPE: NOT AUTOMATIC CROSS-CHARACTER ITEM AUTHORITY`

`IMPLEMENTATION_AUTHORITY: NONE`
