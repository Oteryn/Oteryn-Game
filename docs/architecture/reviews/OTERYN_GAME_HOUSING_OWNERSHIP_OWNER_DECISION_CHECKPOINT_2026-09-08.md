# Oteryn Game — Physical-House Ownership Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Source owner decision comment: Issue #220 comment `5585044388`
- Admission protected main: `744f88963a0bd371a3d5f7154c568b3f549b0fe0`
- Related hybrid-housing checkpoint: PR #435 / pending protected integration
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected semantic ownership model for scarce physical Oteryn houses and guildhouses without freezing auction mechanics, rent values, database schema or production implementation.

This checkpoint builds on, but does not depend for identity correctness on, the separately pending hybrid-housing checkpoint. If that topology is later superseded, the ownership decision remains independently reviewable and migratable rather than being hidden inside runtime topology.

## Existing authority preserved

Current accepted architecture already establishes that:

- `HouseId` is World-scoped and does not contain `ChannelId`;
- physical-house ownership/rent/access is one World-scoped authority;
- house item state is singular rather than copied per Channel;
- `CharacterId`, `GuildId` and `AccountId` are distinct semantic identity types;
- the authoritative AccountId-to-CharacterId relation is game-owned and private by default;
- lifecycle operations such as character deletion, world transfer and account ownership transfer may affect houses and therefore require explicit game-domain handling rather than client or Platform inference;
- value/item mutations retain idempotency, fencing and one-authoritative-location guarantees.

This checkpoint does not weaken or replace any of those boundaries.

## Owner-selected physical-house owner identity

`OWNER_SELECTED`.

An ordinary scarce physical `HouseId` is semantically owned by exactly one `CharacterId` in the owning `WorldId`.

Conceptually:

```text
WorldId + HouseId
        |
        v
owner = CharacterId
```

The owning Character is the public/gameplay property identity presented by ordinary house ownership semantics. Rename does not change ownership because `CharacterId` remains stable.

The physical house MUST NOT use `AccountId` as its public/canonical property owner identity merely because the Character belongs to that account.

## Guildhouse owner identity

`OWNER_SELECTED`.

A guildhouse is semantically owned by exactly one `GuildId` in the owning `WorldId`.

Conceptually:

```text
WorldId + HouseId
        |
        v
owner = GuildId
```

Guildhouse ownership must not be represented as ownership by a nominated guild leader Character whose resignation, deletion, transfer or account change would accidentally move or orphan guild property.

Guild membership, roles and administrative permissions remain separately owned social/guild semantics. `GuildId` ownership does not imply every member has full property mutation authority.

## AccountId role

`OWNER_SELECTED`.

`AccountId` may be consumed privately by later housing/economy policy for controls such as:

- account-level house-count caps;
- auction eligibility;
- anti-speculation and anti-alt controls;
- cooldowns after acquisition or release;
- entitlement/commercial eligibility where separately accepted;
- billing/reconciliation where a later product model requires it;
- abuse and investigation evidence under applicable privacy/audit policy.

These controls do **not** make the Account the semantic house owner.

The system must not expose alternate-character relationships merely because an Account-level eligibility control exists. Public ownership projection should expose only the owner identity authorized by the housing/social presentation contract.

## Why Account-owned physical houses are not selected now

`REJECTED_DIRECTION_FOR_FIRST_MODEL`.

Making a physical house canonically Account-owned would couple scarce world property directly to Platform/account lifecycle and make Character-facing prestige less precise. It would also create unnecessary migration/visibility complexity for Character Bazaar, account transfer and alt privacy.

The architecture keeps the option reversible: a later owner decision may supersede the ownership model with explicit migration evidence. Do not pre-emptively create an untyped polymorphic owner field solely to keep every hypothetical owner type open.

## Character Bazaar / account ownership transfer

`OWNER_SELECTED`.

A `CharacterId` ownership transfer between Accounts does not silently transfer the physical house as part of the commercial Character transaction.

Because the house is game-owned scarce World property with its own eligibility, auction/rent, item-custody and social consequences, the Character transfer workflow must reach an explicit house disposition before the Character ownership mutation is allowed to become incompatible with the existing house binding.

The later owning contract must choose an exact permitted disposition such as release, separately authorized house transfer, or another bounded settlement. This checkpoint does not select that disposition.

Required invariant:

```text
Character ownership/account transfer
!= implicit HouseId ownership transfer
```

Timeout or ambiguous cross-domain result is never evidence that the house transferred successfully.

## World transfer

`OWNER_SELECTED`.

A Character world transfer cannot carry a physical `HouseId` into another World and cannot leave an invalid cross-World owner binding behind.

Before a Character that owns a physical house leaves the World, the later world-transfer/housing contract must prove a valid house disposition and item-custody consequence.

Required invariant:

```text
HouseId belongs to source World
Character World transfer
-> explicit house settlement/release required
-> no cross-World HouseId migration by accident
```

The exact eligibility rule — for example whether owning a house simply blocks transfer until release — remains a later product decision.

## Character deletion/finalization

`OWNER_SELECTED`.

Terminal Character deletion/finalization cannot silently delete, strand, duplicate or arbitrarily reassign the Character's physical house or its durable items.

Deletion must pass an explicit housing disposition boundary before terminal finalization whenever the Character still owns a physical house.

The exact grace/reclaim/auction behavior remains deferred.

## House ownership transfer

A physical-house ownership transfer is a distinct World-scoped game-domain operation.

Later implementation must provide semantics equivalent to:

```text
expected HouseId revision / current owner
+ intended next owner
+ stable operation identity
+ eligibility proof
+ item/ACL/rent disposition proof where required
-> one authoritative ownership commit or bounded rejection/ambiguity
```

Required properties:

- idempotent retry;
- optimistic/revision or equivalent stale-state fencing;
- current-owner revalidation;
- destination-owner eligibility revalidation;
- no client authority over committed owner state;
- no duplicate success after timeout/retry;
- durable audit evidence when ownership/economy policy requires it;
- no item duplication or stale runtime overwrite during transfer.

Exact command names, transaction shape and schema remain deferred.

## Co-ownership

`DEFERRED / NOT_SELECTED`.

This decision intentionally selects one semantic owner identity per physical `HouseId`.

It does not establish multiple equal owners. Rich collaboration should initially be modeled through revisioned ACL/roles/delegated permissions unless a later product decision proves true co-ownership is necessary.

This keeps auction/rent/liability/transfer semantics deterministic and makes future co-ownership an explicit extension instead of an accidental many-owner ambiguity.

## Guildhouse administration

Guildhouse semantic owner is `GuildId`; operational administration remains role/capability based.

A later social/housing contract must decide:

- which guild roles may bid/acquire a guildhouse;
- which roles may pay rent or alter ACL;
- which roles may move high-value house contents;
- leadership-transfer consequences;
- guild dissolution consequences;
- member removal/revocation behavior;
- audit requirements for high-value administrative actions.

Leadership change alone must not change the guildhouse semantic owner.

## Private anti-speculation controls

The owner identity model deliberately separates property ownership from anti-abuse eligibility.

A later policy may enforce, for example:

```text
one Account may indirectly control at most N scarce physical houses
```

without rewriting each property's owner from Character to Account.

Any such control must use authoritative game-owned Character ownership relationships and privacy-safe evaluation. Exact values, cooldowns and exceptions are balance/economy decisions and are not frozen here.

## Auction consequences

This checkpoint does not yet select the physical-house allocation mechanism.

Both a Tibia-like auction model and another explicit World-economy allocation model remain possible provided they produce the selected owner identity and satisfy the same durable settlement/fencing requirements.

The next owner decision should therefore compare:

1. Tibia-like timed auction to an eligible `CharacterId` / eligible `GuildId`;
2. fixed-price or listing allocation;
3. hybrid auction with reserve/eligibility/cooldown controls;
4. any other realistic mechanism justified by scarcity, economy health and player experience.

Do not select numeric rent, fees, auction duration, bid increments, house caps or anti-sniping values before that decision.

## Reversibility

`OWNER_SELECTED`.

The ownership model remains intentionally migratable.

Future implementation should keep:

```text
HouseId
owner semantic type + owner semantic id
ownership revision/history
eligibility policy
ACL/administration
item custody
auction/rent workflow
```

as explicit concepts rather than conflating all of them into one opaque record or client-visible name.

This does not require a permanently generic database schema now. The final schema should implement the accepted current model cleanly and gain a migration only if a later owner decision changes semantic ownership.

Potential future supersession may include Account-owned personal property, organization-owned residences or another product model, but it must provide explicit durable migration/compatibility treatment.

## Rejected unsafe shortcuts

The following are not permitted by this decision:

- owner identity = display name;
- owner identity = current AccountId for ordinary physical houses;
- guildhouse ownership = current guild leader CharacterId;
- automatic house transfer with Character Bazaar sale;
- automatic cross-World house migration with Character transfer;
- terminal Character deletion while house/item disposition is ambiguous;
- client-authored owner/ACL state;
- timeout interpreted as successful ownership transfer;
- Channel-local property ownership;
- item copies generated to simplify eviction/transfer.

## Decision timing

### Must decide now?

`YES`.

The owner identity must be explicit before auction, rent, deletion, transfer and anti-speculation policies are designed, otherwise those systems can accidentally couple to AccountId, display names or Channel topology.

### What downstream work is unblocked?

`EXP-HOUSES-01` may now design:

- physical-house allocation/auction lifecycle;
- house ownership eligibility and caps;
- Character lifecycle/transfer settlement;
- guildhouse administration;
- ACL/guest/co-owner policy;
- rent/eviction/reclaim semantics;
- audit/projection requirements;

without reopening the first semantic owner identity.

### Evidence that may justify supersession

- player evidence that Character-owned houses make alt/account management materially worse;
- Character Bazaar/product evidence that separate house settlement creates disproportionate friction;
- economy evidence requiring account-level ownership rather than private account-level eligibility;
- social evidence requiring true multi-owner property;
- guild administration evidence requiring another stable organization model;
- explicit later owner product decision.

## Deliberately unresolved next decisions

- auction/allocation mechanism;
- personal physical-house ownership caps;
- account-level anti-speculation caps;
- guildhouse acquisition eligibility;
- true co-ownership versus ACL delegation;
- rent/fees/taxes;
- grace and eviction;
- item reclaim/depot/escrow behavior;
- transfer/deletion exact disposition;
- scalable residence ownership identity;
- DDL/schema/migration;
- runtime/client implementation;
- production rollout.

## Next owner decision

Continue one material decision at a time. The next bounded decision is the **physical-house allocation mechanism**, comparing a Tibia-like timed auction with alternative World-economy allocation models while preserving the selected `CharacterId` / `GuildId` owner identities and keeping numeric balance deferred.

No runtime/client/server/DDL/migration/deployment/production implementation is authorized by this checkpoint.
