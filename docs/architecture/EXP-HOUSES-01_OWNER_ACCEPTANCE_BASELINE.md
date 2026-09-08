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
- `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`;
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

An Account cannot hold both a Residence and an ordinary physical house in the same World.

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

- every live durable item has one authoritative immediate semantic location/custody;
- binding/restrictions, location/custody, authorization and presentation ownership stay distinct;
- ordered/revisioned/retry-safe mutations are required where applicable;
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

### 7.1 Residence -> physical house uses staged replacement

A Residence may be replaced by a physical house, but the Residence is not terminally destroyed merely because acquisition has started.

The acquisition operation must use one stable operation identity and a staged replacement state equivalent in semantics to:

```text
RESIDENCE_REPLACEMENT_PENDING
```

Before the physical-house acquisition becomes terminally successful:

1. revalidate the Account/World slot and expected Residence revision;
2. freeze/fence conflicting personal-housing disposition operations;
3. make Residence contents safe under the item-placement/reclaim rules in section 14 where a move is required;
4. preserve enough authoritative Residence lifecycle state to restore the pre-acquisition result if the acquiring operation aborts;
5. expose neither Residence release nor new-house ownership as two independently committed final personal-slot results.

Only authoritative success of the owning physical-house acquisition permits terminal Residence release and final `PHYSICAL_HOUSE` slot commit.

Known acquisition rejection/abort preserves or restores the prior Residence result. Ambiguous outcome keeps the replacement pending/recoverable and reconciles the same operation; it does not guess success or create a second housing transition.

This applies to public-auction Residence replacement and to Character Bazaar `KEEP_INCOMING_HOUSE` when the buyer already has a Residence.

### 7.2 Physical house -> Residence

Residence cannot become authoritative while same Account/World still consumes a physical-house slot. Physical property/content state must first be safely relinquished/evicted/disposed under an explicit operation that cannot duplicate value or property authority.

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

Exact thresholds remain tunable. Candidate input classes may include account maturity, nominated Character progression, real activity/play history, conduct/security state or another explicit game-owned qualification rule.

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

### 11.2 Every bid nominates the future owning Character

Because canonical physical-house ownership is `HouseId -> CharacterId`, an authoritative bid cannot be Account-only.

Every bid/max intent is bound to one stable subject equivalent in semantics to:

```text
HousingBidSubject {
    account_id: AccountId,
    nominated_owner_character_id: CharacterId,
    world_id: WorldId,
}
```

The exact Rust/DDL/wire shape is not frozen, but these semantics are binding:

- at bid admission, Character Authority proves that `account_id` currently owns `nominated_owner_character_id` and that the Character belongs to the auction World;
- the nominated Character must be in a lifecycle state that can legally become a physical-house owner;
- the nominated Character is part of the stable logical bid intent and cannot be silently substituted on retry, reconnect, repricing or settlement;
- changing the nominated Character requires an explicitly new/changed bid intent under the later concrete auction contract, never an invisible retry-time rewrite;
- if this bid wins, canonical ownership becomes `HouseId -> nominated_owner_character_id` and no other Character is chosen by convenience or iteration order.

Exact client UX for selecting the nominated Character remains deferred.

### 11.3 Funds backing

A winning-capable bid must be backed by authoritative funds reservation/escrow semantics before final value consumption. Housing does not invent a parallel money authority.

### 11.4 Anti-sniping

A valid bid near close extends auction under an extension rule so last-millisecond network timing is not the dominant winner mechanic.

Exact duration, extension window and increment remain deferred balance values.

### 11.5 Final eligibility, winner selection and settlement

Auction settlement is authoritative, idempotent, revision/fence aware, reconciliation-safe, compatible with the personal housing slot and value-conserving.

Final winner selection uses an **effective valid-bid set**. A bid participates in winner selection and proxy-price formation only while its full stable bid subject still satisfies final acquisition guards, including as applicable:

- the nominated `CharacterId` still exists in a legal ownership lifecycle;
- Character Authority still proves the same `AccountId -> CharacterId` binding;
- the nominated Character still belongs to the auction `WorldId` and is not in a conflicting terminal deletion/World-transfer/ownership-transfer state;
- the Account has either an empty same-World personal housing slot or a Residence being safely replaced by this same acquisition operation under section 7.1;
- active Premium;
- current `PhysicalHouseEligibility`;
- current bid/funds reservation validity;
- any other separately accepted final acquisition guard.

If any Character/Account/slot/Premium/eligibility/funds guard fails before final selection:

1. that bid is excluded from the effective valid-bid set;
2. its private maximum is also excluded from proxy-price formation and cannot raise the price paid by a valid bidder;
3. its reservation/escrow and any staged Residence replacement are released/restored or reconciled idempotently under their owning contracts;
4. winner and payable price are recomputed from the remaining effective valid bids under the same later-defined increment/tie semantics.

If no valid bids remain, no property or funds transfer occurs. The `HouseId` stays vacant and remains/returns in the public allocation lifecycle; an invalid top bid cannot by itself block allocation or become a price anchor.

A retry of the same winning settlement must produce the same nominated owner Character or reconcile/abort; it cannot pick another Character from the Account.

Exact auction duration, cadence between allocation attempts, increment and tie rule remain deferred. The subject-binding, invalid-bid exclusion and repricing semantics above are not deferred cancellation policy.

Timeout or dependency unavailability is not proof of property/funds transfer and requires reconciliation of the same logical auction settlement.

### 11.6 No generic direct house market at launch

First generation does not expose a generic player-to-player standalone `HouseId` sale at arbitrary price.

Vacant physical houses return to public allocation. A future direct property market requires explicit owner supersession supported by player/economy evidence.

## 12. Rent, delinquency and eviction

Physical-house rent is recurring and World-scoped. Exact formula, cadence and funding-source representation remain deferred.

### 12.1 Collection

Collection is automatic from a later-defined authoritative economy source and idempotent/reconcilable. Channel count/current Channel/runtime placement cannot duplicate rent or reset lifecycle.

### 12.2 Grace

Insufficient funds enter explicit delinquent/grace state, not immediate silent eviction. Exact grace/notification values remain deferred.

### 12.3 Value-safe eviction uses a content fence before evacuation

Before ownership release after unresolved grace, eviction must enter one stable operation-scoped pending state equivalent in semantics to:

```text
EVICTION_DISPOSITION_PENDING
```

Required ordering:

1. revalidate owner/revision/rent state and acquire an authoritative housing-content mutation fence tied to the eviction operation and expected content/property revision;
2. while that fence is active, reject new housing item placement, removal, container mutation and stale HouseRuntime/Channel item writes, then enumerate/snapshot the authoritative contents at the fenced revision;
3. classify and move every affected durable item using the placement-time `HousingReclaimProvenance` rules in section 14, never by inferring claimant from house owner or ACL;
4. move affected value to an already-legal destination or typed non-gameplay custody and prove that no item authoritative at or admitted after the fenced boundary remains unclassified in the property;
5. retain the content/runtime mutation fence through ownership/ACL release, vacancy and public-allocation publication so no stale writer can place value back into a released house;
6. release ownership and ACL only after the fenced content disposition is terminal or durably pending in accepted custody;
7. mark property vacant and return it to public allocation only after the same eviction operation proves the house contains no unresolved movable value outside explicit custody.

A failed or ambiguous evacuation keeps the property in `EVICTION_DISPOSITION_PENDING` (or equivalent), non-allocatable and mutation-fenced while the same operation reconciles. It does not release ownership/public allocation and then attempt to discover late item writes.

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

If the listing selected `INCLUDE_FURNISHINGS` but the buyer chooses `KEEP_EXISTING_HOUSE`, the furnishings transfer does not commit. Section 14 claimant-aware non-retention disposition must complete or remain explicitly recoverable before the incoming house may become publicly allocatable.

A house on another World does not conflict with this per-World slot.

### 13.4 Buyer has Residence: Residence release is staged behind Character transfer

Buyer chooses:

```text
KEEP_RESIDENCE
or
KEEP_INCOMING_HOUSE
```

`KEEP_RESIDENCE` relinquishes the incoming physical house to public allocation after the Character transfer outcome makes that disposition applicable, and does not require physical-house acquisition eligibility merely to buy the Character.

`KEEP_INCOMING_HOUSE` revalidates active Premium + eligibility but does **not** terminally release the buyer's Residence before the authoritative Character transfer outcome is known.

Before Character Authority attempts Account rebinding, housing stages the buyer-side replacement under the same Bazaar operation identity using a state equivalent in semantics to:

```text
BAZAAR_RESIDENCE_DISPOSITION_PENDING
```

The pending state must:

- bind the expected buyer `AccountId + WorldId`, expected Residence revision and incoming `HouseId`/owning `CharacterId`;
- fence conflicting personal-slot disposition operations;
- keep the prior Residence ownership/lifecycle result durably recoverable;
- move Residence contents only through value-conserving, reversible/reconcilable custody steps where required;
- prevent either Residence or incoming physical house from becoming a conflicting second final personal-slot result while outcome is unresolved.

The authoritative Character transfer outcome controls finalization:

- **Character transfer committed:** finalize value-safe Residence release, retire/fence the old Residence runtime/lifecycle and commit the incoming physical house as the one personal housing slot result;
- **Character transfer rejected/aborted before Account rebinding commit:** cancel/reconcile the pending Residence disposition and preserve or restore the buyer's prior Residence, contents and slot result; the incoming house remains with the sold Character's seller-side result according to the Bazaar failure contract;
- **Character transfer outcome ambiguous/unavailable:** keep the Residence replacement pending/recoverable and do not terminally release Residence or activate a second housing slot until the same Bazaar operation reconciles.

This is a saga/reconciliation boundary, not distributed ACID.

If the listing selected `INCLUDE_FURNISHINGS` but the buyer chooses `KEEP_RESIDENCE`, the furnishings transfer does not commit. Section 14 claimant-aware non-retention disposition must complete or remain explicitly recoverable before the incoming house may become publicly allocatable.

Any physical house that must be relinquished because of the buyer's keep-choice follows the same safety principle as seller `RELINQUISH_HOUSE`: public vacancy/allocation is exposed only after the Character transfer outcome that makes that disposition applicable is authoritative; ambiguity leaves the disposition pending/non-allocatable and reconcilable.

### 13.5 Reconciliation boundary

No distributed ACID between Platform/Character/housing/economy/item domains is assumed.

One semantic outcome is preserved through stable operation identity, current-state revalidation, explicit pending states, fencing, idempotent steps, typed custody, durable operation evidence and reconciliation after timeout/ambiguity.

Public allocation and destructive Residence release are terminal housing effects, not PREPARE steps. A property/Residence disposition whose outcome still depends on an unresolved Character transfer remains recoverable and non-terminal.

For any Bazaar housing prepare that depends on a fenced housing-content set/revision, Character Authority must consume current proof of the same operation/fence/revision before committing Account rebinding. Missing, stale or released content-fence proof is not successful prepare and requires revalidation/reconciliation rather than rebinding against an old snapshot.

Cached listing state never proves current housing eligibility or terminal Character-transfer outcome.

## 14. Housing item placement and reclaim provenance

### 14.1 Why housing needs explicit reclaim provenance

`GAME-ITEM-01` and `DUR-03` deliberately separate World scope, binding/restrictions, immediate location/custody, authorization and presentation ownership. They do not provide a universal persistent item `owner_id`, and housing must not invent one at eviction/Bazaar time.

Therefore first-generation housing defines a **housing-surface fallback disposition fact** at placement time. It is not general item ownership, not binding, and not gameplay mutation authority.

Every movable durable `ItemInstanceId` entering a House or Residence location/custody must, in the same authoritative placement transaction, either:

1. establish/refresh typed `HousingReclaimProvenance`; or
2. be rejected from housing placement if no authoritative reclaim subject/disposition can be established.

A later house sale, eviction, relinquishment, Residence replacement or Bazaar flow may not invent claimant identity from the current house owner, Account, ACL role or current room/container location.

### 14.2 HousingReclaimProvenance semantics

The exact Rust/DDL identifier is deliberately not frozen, but the required semantic fact is equivalent to:

```text
HousingReclaimProvenance {
    world_id: WorldId,
    housing_scope_ref: HouseId | ResidenceRef,
    item_instance_id: ItemInstanceId,
    reclaim_subject_ref: TypedHousingReclaimSubjectRef,
    placement_transaction_id: TransactionId,
    provenance_revision: Revision,
}
```

Rules:

- the provenance is created/updated atomically with the DUR-03 transfer that makes the item authoritative inside housing;
- it is scoped to the item's current housing placement lifecycle and cannot become a competing immediate location;
- it exists only to answer the later forced-disposition question: **which already-authorized subject/domain receives or controls reclaim if housing lifecycle must evacuate this item?**;
- it is not evidence that the subject may currently use, spend, trade, mutate or move the item outside the owning item/session/domain authorization rules;
- it cannot override binding/restrictions or create Account-wide storage.

### 14.3 Authoritative source of the reclaim subject

The reclaim subject is derived at the **placement transition**, where source location and actor/domain authority are known, not reconstructed later from house state.

For first-generation housing:

- **Character-controlled placement:** when a currently authorized Character moves an item from that Character's legal inventory/equipment/container custody into housing, the fallback reclaim subject is that placing `CharacterId`;
- **typed-domain custody placement:** when an item enters housing from another accepted typed custody/domain, that source domain must provide an explicit typed fallback reclaim subject/disposition as part of the transfer intent and authority proof;
- **housing-to-housing or explicit authorized transfer:** the destination placement transaction establishes a new destination-scoped provenance under the new legal transfer result; old provenance is retired/superseded only when the transfer commits;
- **no resolvable subject:** if the source transaction cannot establish a typed reclaim subject/disposition under current authority, placement into housing fails closed before the item becomes authoritative there.

A guest or manager placing an unbound transferable item from their own Character inventory therefore records that **placing CharacterId** as the housing reclaim subject. The house owner does not become fallback claimant merely because they own the address or granted storage permission.

A placement-time Character reclaim subject is only the ordinary forced-exit fallback while that Character remains the intended reclaim subject. A workflow that will transfer control of that Character to another Account cannot use the unchanged `CharacterId` reference to satisfy an explicit seller-retained `HOUSE_ONLY` outcome; section 14.5 must establish a legal pre-transfer seller-retained disposition before Character rebinding.

This is a housing transfer-surface policy layered on DUR-03 conservation. It does not redefine global ItemInstance ownership semantics.

### 14.4 BazaarHousingReclaimCustody

When a direct legal destination for the recorded reclaim subject is unavailable or a multi-transaction housing workflow needs a non-gameplay holding state, housing uses a distinct DUR-03 typed custody family equivalent in semantics to:

```text
BazaarHousingReclaimCustody {
    world_id: WorldId,
    operation_id: OperationId,
    item_instance_id: ItemInstanceId,
    reclaim_subject_ref: TypedHousingReclaimSubjectRef,
    provenance_revision: Revision,
}
```

Required semantics:

- stable scope includes World, operation, item and typed reclaim subject/provenance revision;
- each item still has exactly one authoritative immediate semantic location;
- mutation owner remains the game-owned item/value custody boundary under DUR-03;
- Platform/client/house owner/ACL state is never custody authority;
- custody items remain non-spendable/non-usable until a separately authorized legal exit;
- legal exit must choose a destination permitted for the recorded reclaim subject under item/depot/mail/reclaim rules and is itself value-conserving, fenced and idempotent;
- if provenance is missing, stale, conflicting or cannot be authoritatively resolved, risky housing release fails closed and the property/item workflow remains pending rather than guessing a claimant.

For a Character Bazaar `HOUSE_ONLY` transition, a specialized seller-retained custody/disposition may be required by section 14.5. That specialized transition is created under verified **pre-transfer** authority; it is not derived after rebinding from the sold Character's identity.

### 14.5 HOUSE_ONLY uses a fenced content snapshot through Character rebinding

For `INCLUDE_HOUSE_WITH_CHARACTER`, seller additionally chooses:

```text
HOUSE_ONLY
or
INCLUDE_FURNISHINGS
```

With `HOUSE_ONLY`, Character + address may transfer, but ordinary movable housing contents excluded by that choice must not follow control of the sold Character to the buyer merely because their ordinary `HousingReclaimProvenance` names that Character.

Before any excluded-item classification begins, housing must enter a Bazaar-operation-scoped pending state equivalent in semantics to:

```text
BAZAAR_HOUSE_CONTENTS_PENDING
```

That pending state/fence must bind at least the stable Bazaar operation, expected `HouseId -> CharacterId`, expected property/content revision and seller-side authority. While it is active:

- new housing item placement, removal, container mutation and conflicting durable content writes are rejected/fenced across HouseRuntime/Channel paths;
- the authoritative excluded-content set is enumerated from the fenced content revision, not a live mutable room snapshot;
- Character Authority may commit Account rebinding only while it can validate current proof for the same pending operation/fence/revision;
- a stale, missing, released or mismatched fence/revision makes Bazaar prepare non-authoritative and rebinding must fail closed/reconcile instead of consuming the old set.

With that content fence held, each excluded item is classified before Character Authority may commit Account rebinding:

1. validate current `HousingReclaimProvenance`, item binding/restrictions and current pre-transfer authority against the fenced content revision;
2. if the reclaim subject is a guest/manager/other Character or typed domain not being sold, preserve that subject and route to an already-legal destination or subject-keyed typed custody;
3. if the reclaim subject is the **Character being sold**, revalidate that the pre-transfer seller Account currently controls that Character and whether owning item policy permits the item to be detached into a seller-retained disposition;
4. when such seller retention is legal, move/stage the item before rebinding into a typed non-gameplay seller-retained custody/disposition equivalent in semantics to:

```text
BazaarSellerRetainedHousingCustody {
    world_id: WorldId,
    bazaar_operation_id: OperationId,
    item_instance_id: ItemInstanceId,
    pre_transfer_seller_account_id: AccountId,
    source_character_id: CharacterId,
    source_provenance_revision: Revision,
}
```

This is not a generic Account warehouse and does not redefine item ownership. The pre-transfer seller Account is recorded only because the same authoritative Bazaar prepare step has proven that Account controls the sold Character **before rebinding** and that the item's current binding/transfer policy permits seller-retained exclusion from the house sale. Legal exit still requires a separately authorized legal destination; an alternate Character receives no access merely from Account equality.

If item binding/restrictions do **not** permit a seller-retained disposition independent of the sold Character, `HOUSE_ONLY` cannot commit while that item remains an excluded housing content. The seller must first move/settle it through an already-legal destination under its owning item policy, or the Bazaar housing prepare remains rejected/pending. Housing may not invent seller Account ownership after the fact.

Before Character Authority commit, the owning housing operation must prove that every item in the fenced excluded-content set has a terminal or accepted pending disposition and that no unfenced/late content write entered the house. The content fence remains held across the Character rebinding decision.

Terminal behavior:

- **Character transfer committed:** seller-retained custody remains seller-side under the pre-transfer operation fact; the sold Character and buyer Account cannot withdraw or reclaim those excluded items by virtue of the new `AccountId -> CharacterId` binding. The content fence is released only after the operation proves all excluded contents are no longer authoritative in the transferred house (or are represented by an explicitly accepted included-furnishing result where applicable).
- **Character transfer rejected/aborted before rebinding commit:** seller-retained staging/direct-return state reconciles back to the legal pre-transfer housing/item result under the same operation, without duplication or reassignment; only after that restoration is authoritative may the content fence be released for normal seller-side use.
- **ambiguous Character-transfer outcome:** item disposition and content fence remain non-spendable/pending under the same operation. Housing does not reopen mutations or release an item to either side until Character Authority outcome is reconciled.

Consequences:

- `HOUSE_ONLY` never uses the sold Character's post-transfer identity as the final reclaim key for an excluded seller-retained item;
- a guest/manager/other Character's item cannot be converted to seller-retained custody merely because the house seller selected `HOUSE_ONLY`;
- an alternate Character on the seller Account does not gain direct access merely from Account equality;
- no item can be added to or removed from the authoritative `HOUSE_ONLY` excluded set after it is fenced without invalidating/restarting the same prepare under a new accepted revision;
- if a legal seller-retained or other existing-subject disposition cannot be established before rebinding, the listing fails closed rather than silently transferring house contents.

### 14.6 INCLUDE_FURNISHINGS is conditional on retaining the incoming house

Eligible items transfer to the buyer only through an explicit authoritative manifest/bundle **and only if the buyer's final keep-choice is `KEEP_INCOMING_HOUSE`**.

An item is eligible for the furnishings manifest only when its current binding/restriction and HousingReclaimProvenance/source authorization permit the seller-authorized transfer. House placement or seller ownership of `HouseId` alone does not authorize transfer of an item whose recorded reclaim subject is somebody else.

Required invariants when `KEEP_INCOMING_HOUSE` commits:

- seller opt-in;
- exact set server-authoritative;
- every manifest item individually transfer-authorized;
- DUR-03 identity/location/conservation;
- stale client cannot alter committed set;
- transferred item cannot remain usable in old custody;
- destination placement establishes the correct new housing reclaim provenance for the committed buyer-side result;
- ambiguous settlement reconciles, never duplicates.

If buyer chooses `KEEP_EXISTING_HOUSE` or `KEEP_RESIDENCE`, the incoming physical house is not retained and furnishings transfer **must not commit**. Before that house becomes vacant/publicly allocatable:

1. freeze the manifest candidate set under the same Bazaar operation identity;
2. validate each item's current HousingReclaimProvenance;
3. return it to an already-legal destination for its recorded reclaim subject where possible, otherwise move it to claimant-aware typed custody;
4. prove that no manifest item remains in the relinquished house as a gift to a future occupant and no item becomes buyer or seller property without independent legal authority;
5. only then allow the non-kept house to complete public release.

If Character transfer, buyer keep-choice, item disposition, provenance or custody outcome is ambiguous, the house remains pending/non-allocatable and the same operation reconciles both item and property outcome.

Exact physical tables/containers, reclaim UI, retention/capacity limits, manifest schema, valuation and final withdrawal UX remain deferred. Exact physical names/representations for eviction/Bazaar content fences and content revisions are also deferred. The fence-before-enumeration rule, placement-time provenance source, seller-retained pre-transfer conversion rule, fail-closed missing/illegal-disposition rule and value-conservation semantics are binding.

## 15. Character deletion and World transfer

Terminal Character deletion/finalization cannot leave a physical house bound to nonexistent owner state; housing settlement must complete or remain explicitly recoverable first.

A physical `HouseId` is a World address and cannot migrate across Worlds. Character World transfer must settle/relinquish incompatible physical property first.

Residence is also World-scoped under `AccountId + WorldId`; exact transfer UX is future World-lifecycle work, but no workflow may duplicate Residence contents or personal slots.

A `CharacterId` referenced by active auction bid subject or HousingReclaimProvenance is part of the affected lifecycle guard: deletion/World transfer cannot silently make the reference point to a different Character or Account. The owning operation must settle, invalidate, migrate to a separately legal typed custody/disposition, or remain pending according to its contract.

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

ACL may distinguish property entry, specific doors, room/zone, bed use, storage use, workstation/interactive feature and later accepted capabilities.

Storage permission never implies transfer ownership, reclaim-subject authority or disposition authority over items placed by that actor or by somebody else. Exact vocabulary remains downstream detail if least-authority semantics are preserved.

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

Every durable movable item entering housing must satisfy section 14 placement-time reclaim-provenance rules. Housing ownership, ACL role and storage access do not supersede item binding, location, authorization or reclaim provenance.

When an eviction, Bazaar disposition or other accepted lifecycle operation holds a housing-content mutation fence, ordinary player/ACL/runtime item placement, removal and container mutation are rejected until that fence is authoritatively released. A stale runtime cannot make a post-snapshot write authoritative merely because its prior ACL/session was valid.

Eviction, relinquishment, Residence replacement and Bazaar disposition resolve affected durable items item-by-item and move value through explicit authoritative custody/location transitions before prior property/runtime authority is released.

Destroying, duplicating, abandoning, misassigning or silently gifting forgotten items is not an acceptable simplification.

## 20. Failure and recovery semantics

Housing distinguishes deterministic policy rejection, stale owner/revision/slot, stale runtime/session authority, stale bid subject, funds/escrow conflict, item/provenance/custody conflict, dependency unavailable and ambiguous durable result.

For ambiguity:

1. do not fabricate success/failure;
2. reread authoritative operation/property/slot/Character/item/provenance/value state;
3. reconcile using the same logical operation identity;
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

Semantic authority for `HouseId` lifecycle/availability, physical owner binding, personal housing slot, Residence entitlement/lifecycle, ACL revisions, housing-auction bid subject semantics, rent/delinquency/eviction, housing-placement reclaim provenance, final housing disposition and active housing-runtime ownership.

This semantic domain need not map to one service/process/table.

### 22.2 Character Authority

Remains authority for Character lifecycle/current Account owner/current World and Character ownership rebinding. Housing consumes those authoritative facts for auction nomination/final guards and Bazaar settlement but does not seize Character authority.

### 22.3 Platform

Remains authority for Platform Account identity, entitlement/commercial source data under accepted contracts, portal/commercial UX and Bazaar commercial saga. Platform cache/listing is not authoritative housing proof.

### 22.4 Economy / item domains

Funds and durable value mutations remain with accepted/future economy and DUR-03 owners. Housing declares required effects without inventing distributed ACID or duplicate value authority.

`HousingReclaimProvenance` is a housing-surface fallback disposition policy attached to the authoritative housing placement transaction. It is not generic ItemInstance ownership. Claimant-aware typed custody and Bazaar seller-retained custody remain DUR-03 custody families and do not move item mutation authority to Platform, Account identity, house owner identity or housing UI.

A Bazaar seller-retained Account reference is valid only as an operation-scoped disposition fact established while that Account still authoritatively controls the sold Character and only for an item whose owning policy permits seller-retained exclusion. It creates no general Account inventory/storage authority.

Housing-content pending/fence state is housing lifecycle coordination authority only. It may reject conflicting housing-surface item mutations and provide a stable content revision to an owning cross-domain saga; it does not become generic item mutation authority or permit housing to override DUR-03 binding/custody rules.

### 22.5 Client

Client is untrusted presentation/input. It cannot establish ownership, nominated auction owner, auction winner, ACL authority, reclaim provenance, seller-retained item disposition, item manifest, content-fence state/revision or settlement success.

## 23. Anti-speculation posture

First-generation controls combine:

- active Premium when a new scarce physical house is acquired/retained;
- independent `PhysicalHouseEligibility`;
- one personal slot per Account/World;
- one explicit nominated owner Character per auction bid;
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

- auction bid subject `(AccountId, nominated CharacterId, WorldId)` and final Character/Account/World validation;
- auction bid validity at final selection, invalid-bid exclusion and resulting proxy-price/winner recomputation;
- auction disputes and reservation/Residence-replacement release/reconciliation;
- ownership history;
- personal-slot conflicts/transitions;
- Premium/eligibility decision version/input class;
- rent/delinquency/eviction;
- eviction content-fence acquisition/revision, authoritative evacuation set and fence release;
- ACL revisions/privileged changes;
- Bazaar housing disposition/keep-choice;
- `BAZAAR_DISPOSITION_PENDING`, `BAZAAR_RESIDENCE_DISPOSITION_PENDING` and `BAZAAR_HOUSE_CONTENTS_PENDING` lifecycle plus terminal Character-transfer outcome;
- HousingReclaimProvenance creation/update/retirement source per affected `ItemInstanceId`;
- pre-transfer `HOUSE_ONLY` seller-retained conversion decision, source Character/provenance revision and resulting custody/direct destination;
- Bazaar content-fence/revision proof consumed by Character Authority before rebinding;
- typed housing reclaim-custody and seller-retained-custody transitions;
- furnished-transfer manifest, buyer keep-choice and transfer-versus-reclaim result;
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
9. Residence -> house replacement settles value and produces one final class; failed acquisition restores/preserves Residence.
10. Rent delinquency + valid payment within grace does not double-charge/evict.
11. Eviction moves contents according to placement-time reclaim provenance before ownership release/public allocation.
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
26. Restore validates owner, slot, item custody/provenance graph and runtime fences before mutation resumes.
27. Account-scoped Residence does not silently authorize cross-Character item transfer.
28. Highest proxy bidder becomes ineligible before close: its bid/max is excluded from winner and price formation; winner/price recomputed from remaining valid bids or no transfer occurs.
29. `RELINQUISH_HOUSE` enters pending non-allocatable disposition; Character rebinding rejection preserves/restores seller-side house result and never exposes property to public auction.
30. Guest Character places an unbound transferable item from their own inventory: placement records that CharacterId as reclaim subject; seller/owner cannot reclaim it merely because of house ownership/ACL.
31. Item placement for which no authoritative housing reclaim subject/disposition can be established is rejected before the item becomes authoritative inside housing.
32. `HOUSE_ONLY` with mixed item reclaim subjects preserves per-item provenance and cannot collapse them into seller Account custody.
33. `INCLUDE_FURNISHINGS` + buyer `KEEP_EXISTING_HOUSE` or `KEEP_RESIDENCE`: furnishings transfer does not commit; every manifest candidate is routed by its recorded reclaim provenance before incoming house becomes publicly allocatable.
34. Auction bid is bound to one nominated CharacterId; if the Account no longer owns that Character, the Character leaves the World or enters incompatible lifecycle before close, the bid/max is excluded and cannot become owner/price anchor.
35. Winning auction retry cannot silently substitute a different CharacterId from the same Account.
36. Bazaar buyer with Residence chooses `KEEP_INCOMING_HOUSE`, then Character transfer rejects: Residence and its value are preserved/restored and no second final personal housing slot commits.
37. Bazaar buyer with Residence has ambiguous Character-transfer outcome: Residence replacement and incoming house remain pending/reconcilable without terminal dual ownership or Residence loss.
38. `HOUSE_ONLY` where the sold Character is the placement-time reclaim subject for an otherwise seller-retainable item: before Account rebinding, the item moves/stages to a legal seller-retained disposition; after transfer the buyer/sold Character cannot reclaim it merely through the new Character ownership binding.
39. `HOUSE_ONLY` where an excluded item cannot legally be detached from the sold Character under binding/restriction policy: Bazaar prepare fails closed or requires a legal pre-transfer item disposition; it never invents seller Account ownership after Character rebinding.
40. Eviction acquires a housing-content mutation fence before enumerating contents; an authorized Character racing item placement/removal after the fence cannot create value that is omitted from evacuation, and the fence remains through vacancy/public allocation.
41. `HOUSE_ONLY` prepare fences the authoritative house-content revision before classification; an owner/guest cannot add or remove an excluded item between classification and Character rebinding, and Character Authority rejects stale/missing fence proof.

## 26. Deliberately deferred

Whole-gate acceptance does **not** freeze:

### Auction / rent / economy numbers

- rent amount/formula/cadence;
- grace duration;
- auction duration/increment/anti-sniping window;
- reserve/start price/tie/cancellation rules that do not alter stable bid-subject or invalid-bid exclusion/repricing semantics;
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
- exact OperationId/revision/runtime-generation representation;
- exact physical representation/names for HousingReclaimProvenance, typed housing reclaim custody, seller-retained Bazaar custody, housing-content fences and content revisions;
- reclaim screen/workflow and legal final destination UX consistent with binding provenance/authorization rules;
- exact auction bid UI/wire representation;
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
12. Account-only auction bids that leave winning `CharacterId` selection implicit;
13. substituting another CharacterId on bid retry/settlement;
14. exposing a Bazaar-relinquished house to public allocation before the Character-transfer outcome authorizing final release is authoritative;
15. terminally releasing buyer Residence before a dependent Character Bazaar transfer is authoritative;
16. treating `HOUSE_ONLY` contents as the sold Character's depot contents or generic Account-wide storage;
17. leaving a `HOUSE_ONLY` excluded item keyed only to the sold Character after Account rebinding, allowing the buyer to reclaim it;
18. converting guest/manager/other reclaim subjects into seller-retained Account custody merely because the seller chose `HOUSE_ONLY`;
19. deriving forced-reclaim subject later from house owner, Account, ACL role or physical placement when no placement-time provenance exists;
20. assigning the house seller as reclaim subject merely because an item was placed inside the house by an ACL-authorized guest/manager;
21. accepting new housing item placement when no authoritative reclaim subject/disposition can be established;
22. leaving `INCLUDE_FURNISHINGS` items in an incoming house the buyer chose not to retain, thereby gifting them to a future occupant;
23. enumerating/moving eviction contents before fencing housing item mutations, allowing a late authoritative item write to survive ownership release;
24. allowing `HOUSE_ONLY` content mutation after its authoritative snapshot without invalidating/reconciling the Bazaar prepare before Character rebinding.

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
- explicit auction winning Character identity;
- placement-time forced-reclaim provenance or an equally authoritative replacement;
- seller-retained `HOUSE_ONLY` disposition independent of a sold Character's post-transfer control;
- fence-before-enumeration/fence-through-terminal-outcome semantics for destructive housing content disposition;
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

`AUCTION BID SUBJECT: STABLE ACCOUNTID + NOMINATED OWNER CHARACTERID + WORLDID`

`AUCTION FINAL VALIDITY: NOMINATED CHARACTER REVALIDATED; INVALID BIDS/MAXIMA EXCLUDED FROM WINNER AND PRICE FORMATION`

`GENERIC DIRECT HOUSE SALE: NOT ENABLED FIRST GENERATION`

`CHARACTER BAZAAR: EXPLICIT RELINQUISH OR INCLUDE HOUSE + BUYER KEEP-CHOICE`

`BAZAAR RELINQUISH: PENDING + NON-ALLOCATABLE UNTIL AUTHORITATIVE CHARACTER-TRANSFER OUTCOME`

`BAZAAR RESIDENCE -> INCOMING HOUSE: RESIDENCE RELEASE STAGED UNTIL AUTHORITATIVE CHARACTER-TRANSFER COMMIT`

`BAZAAR CONTENTS: HOUSE_ONLY OR EXPLICIT INCLUDE_FURNISHINGS`

`HOUSING ITEM RECLAIM: PLACEMENT-TIME TYPED RECLAIM PROVENANCE; NO SALE-TIME OWNER/ACL INFERENCE`

`EVICTION CONTENTS: MUTATION FENCE BEFORE ENUMERATION, HELD THROUGH OWNERSHIP RELEASE/PUBLIC ALLOCATION`

`HOUSE_ONLY SOLD-CHARACTER ITEMS: PRE-TRANSFER SELLER-RETAINED LEGAL DISPOSITION OR FAIL CLOSED BEFORE REBINDING`

`HOUSE_ONLY CONTENT SET: FENCED REVISION HELD THROUGH CHARACTER REBINDING; STALE/MISSING FENCE PROOF FAILS CLOSED`

`INCLUDE_FURNISHINGS: TRANSFER ONLY WHEN INCOMING HOUSE IS RETAINED; OTHERWISE PROVENANCE-AWARE RECLAIM BEFORE PUBLIC RELEASE`

`RENT: RECURRING -> GRACE -> VALUE-SAFE FENCED EVICTION -> PUBLIC AUCTION`

`ACL: OWNER -> MANAGER/SUBOWNER -> GUEST + FINE-GRAINED CAPABILITIES`

`ACL PLAYER UX: GUI/PANEL ONLY; PLAYER-FACING TEXT-COMMAND ACL ADMINISTRATION FORBIDDEN`

`RESTED: BASELINE PARITY BETWEEN RESIDENCE AND ORDINARY PHYSICAL HOUSE`

`RESIDENCE ACCOUNT SCOPE: NOT AUTOMATIC CROSS-CHARACTER ITEM AUTHORITY`

`IMPLEMENTATION_AUTHORITY: NONE`
