# Social Presence and Contact Consent Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-06
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: social presence, contacts/VIP, party invitations, Party Finder, client social UI, privacy, protocol and audit contracts

## Purpose

Record the project owner's accepted privacy-first direction for online presence, channel visibility and consent-based social contacts.

This document is canonical architecture input. It does not authorize implementation and does not yet freeze protocol schemas, persistence layout, UI design or the complete social-system contract.

## Accepted privacy-first presence model

Presence and placement are separate concepts.

A character may expose a coarse presence state without exposing exact runtime placement. Exact placement includes at least:

- `ChannelId`;
- `InstanceId`;
- `NodeId` or other infrastructure placement;
- exact map position, area, subarea or coordinates;
- current activity when it could reveal location or behavior.

Accepted rules:

- exact channel, instance, node and map placement are not public information;
- an unauthorized observer must not receive exact placement through the client, protocol, API, cache, event stream, log or indirect error behavior;
- a normal non-contact may see at most a coarse state such as `online on this world`, subject to the observed player's privacy settings;
- exact `ChannelId` may be visible by default only to current party members and mutually accepted contacts/VIP entries;
- the observed player may reduce visibility further, including hiding exact channel or using a more restrictive presence mode;
- widening exact-placement visibility beyond the accepted default classes requires explicit opt-in in a later privacy contract;
- membership of the same world, guild, chat channel or Party Finder queue does not by itself grant exact-location access;
- server-side routing may use exact placement internally, but internal necessity does not create client-visible permission;
- moderation and operational access require a separately governed privileged-access policy and audit trail.

The privacy policy applies even when `ChannelId` or another placement identifier is technically easy to obtain or globally unique. Identifier availability is not authorization.

## Accepted consent-based contact/VIP model

The legacy unilateral VIP-list model is rejected for Oteryn social relationships.

Adding another player as an accepted contact/VIP requires an explicit invitation that the target may accept or reject.

A future contact invitation must have an explicit identity and lifecycle equivalent to:

```text
ContactInviteId
requester identity
target identity
created_at
expires_at
state
revision / idempotency context
```

Minimum lifecycle states:

```text
PENDING
ACCEPTED
DECLINED
EXPIRED
CANCELLED
```

A later contract may add `BLOCKED`, `REVOKED` or other states, but it must not reinterpret rejection as acceptance.

Accepted invariants:

- sending a contact request does not create an accepted VIP/contact relationship;
- only an authoritative transition from `PENDING` to `ACCEPTED` creates the relationship;
- acceptance, decline, cancellation and expiry are idempotent and stale revisions are rejected;
- the target may decline without revealing a reason;
- declining or ignoring a request must not disclose hidden presence or placement;
- either side may later remove the relationship;
- blocking must prevent repeated contact invitations and must not leak more information than necessary;
- rate limits, spam controls and abuse controls are mandatory inputs to the later contract;
- a contact relationship does not automatically create a party, party invitation, guild relation or permission to teleport/follow;
- a party invitation remains a separate consent operation that the target may accept or decline;
- removal from contacts does not silently remove a player from an already valid party; those lifecycles are separate;
- current party membership may temporarily grant the accepted party-level presence visibility without creating a permanent contact relation.

## Accepted character-scoped and account-wide relationship boundary

The project owner accepted on 2026-08-06 that the default social relationship is between specific characters, not automatically between every character owned by two accounts.

Accepted rules:

- the default contact/VIP request targets one specific character identity;
- accepting that request creates a relationship only between the explicitly named requester character and target character;
- adding one character must not automatically reveal, discover, subscribe to or add other characters owned by the same account;
- character-scoped contact consent must not be silently promoted into account-wide consent;
- presence, messaging and exact-channel permissions granted by the character relationship apply only to the accepted character pair unless another accepted policy explicitly widens them;
- a relationship covering entire accounts may exist only as a separate optional relationship type;
- an account-wide relationship requires a distinct invitation and explicit acceptance by both sides;
- account-wide acceptance must be clearly distinguishable in the client from accepting one character as a contact;
- neither an account identifier nor ownership linkage may be exposed merely to prove that an account-wide relationship exists;
- rejection, expiry, revocation or blocking of an account-wide request must not alter unrelated character-scoped relationships unless the user explicitly chooses a broader action;
- removing a character-scoped relationship must not silently revoke a separately accepted account-wide relationship, and removing an account-wide relationship must not silently recreate or delete character relationships without an explicit lifecycle rule;
- party, guild, house, trade and moderation relationships remain separate from both character-scoped and account-wide contact consent.

This model establishes two distinct concepts:

```text
CharacterContact
    explicit relationship between one requester CharacterId and one target CharacterId

AccountContact
    optional, separately consented relationship between two account identities
```

The concrete identifier types, storage model and public representation remain future contract work. Strong typing must prevent a character request from being interpreted as an account request or vice versa.

## Accepted alternate-character privacy default

The project owner accepted on 2026-08-06 that alternate characters remain hidden by default even after two users establish a mutually accepted account-wide relationship.

Accepted rules:

- account-wide friendship does not automatically expose the names, identities, presence, world placement, channel placement, activity or ownership linkage of alternate characters;
- the default alternate-character visibility policy is `HIDDEN`;
- only the owner of an account may widen alternate-character visibility through an explicit privacy setting or an explicit per-contact/per-character sharing action;
- changing alternate-character visibility must be a deliberate user action and must not be inferred from accepting account friendship, joining a party, joining a guild, messaging, trading or sharing one character;
- the user must be able to keep selected characters hidden even if broader account-level sharing is enabled;
- a future client must clearly distinguish between sharing all eligible characters, selected characters and no alternate characters;
- the owner may revoke previously granted alternate-character visibility without ending the underlying account-wide relationship;
- revocation and privacy-setting changes must invalidate cached roster, presence and ownership-linkage data promptly and fail toward less disclosure;
- a contact must not be able to discover hidden alternate characters through search, invitation responses, Party Finder, guild rosters, timing, sequential identifiers, shared transport endpoints or error differences;
- the server must enforce alternate-character privacy before sending data; hiding an already transmitted roster only in the client is insufficient;
- newly created characters remain hidden unless the owner's active policy explicitly and safely includes them;
- account recovery, ownership transfer, account merge/split or moderation actions must not silently broaden alternate-character visibility;
- privileged operational access to account-character linkage requires separate authorization and audit and does not create social visibility.

A later contract must define the exact user-facing options, but the semantic default is fixed:

```text
AccountContact accepted
    does not imply alternate-character disclosure

Alternate-character disclosure
    requires a separate owner-controlled opt-in policy
```

## VIP list becomes a consent-based social surface

The client may retain the familiar `VIP` name for usability, but semantically it becomes a consent-based contact/social surface rather than a unilateral tracking list.

It may present, subject to authorization and privacy settings:

- accepted character contacts;
- separately accepted account-wide contacts where the feature is enabled;
- only those alternate characters that their owner explicitly shared under the active privacy policy;
- pending incoming and outgoing contact invitations with their relationship scope clearly identified;
- coarse online/offline or privacy-preserving presence;
- exact channel only where authorized;
- current party membership and remote-party status;
- actions such as invite to party, message, remove contact, decline, cancel or block.

The client is a presentation and command surface. It must not infer hidden channel, instance, account linkage or alternate-character ownership from stale caches, previous sessions, Party Finder data or transport endpoints.

## Presence authority and freshness

The authoritative game domain owns current gameplay placement. Platform identity remains the authority for reusable account credentials and account-security boundaries.

A later contract must define:

- the authoritative owner of character-contact, account-contact, alternate-character-sharing and privacy-preference records;
- how Platform and game services exchange only the minimum necessary identity and presence data;
- revisioned presence and sharing-policy updates with stale-update rejection;
- reconnect, channel-change, instance-entry and logout transitions;
- timeout behavior when presence becomes uncertain;
- privacy-preserving cache invalidation;
- audit events for privileged presence, account-linkage and alternate-character access and for abuse-relevant invitation actions.

A stale presence or sharing record must fail toward less disclosure. It must not keep exposing an old exact channel, alternate-character roster or ownership linkage after authorization, party membership, contact status or sharing permission has ended.

## Relationship to world-scoped parties

The accepted world-scoped party model remains unchanged:

- a party is identified semantically by `WorldId + PartyId`;
- party members may temporarily be on different channels of the same world;
- party organization and chat may remain active across channels;
- shared open-world simulation still requires one common channel;
- shared instanced simulation requires one common concrete instance under later contracts.

The social presence model supports this by allowing an authorized party member to see enough placement information to coordinate, without making exact placement public to the whole world or revealing unrelated alternate characters.

## Privacy and abuse requirements

Later implementation contracts must include at least:

- user-visible privacy controls;
- safe defaults;
- contact-invite and party-invite rate limits;
- block lists and anti-harassment handling;
- no presence, account-linkage or alternate-character enumeration through sequential identifiers, timing, search errors or invitation responses;
- bounded retention for invitation, presence and sharing history;
- pseudonymous analytics where exact character or account identity is unnecessary;
- role-separated access for moderation, support, analytics and operations;
- durable audit for privileged exact-location, account-linkage or alternate-character access;
- protection against using social APIs to locate streamers, PvP targets, moderators or players who chose restricted visibility.

## Deliberately unresolved

This baseline does not yet decide:

- whether character-scoped contacts may span worlds or remain world-scoped;
- whether account-wide contacts are global or separately enabled per world;
- whether the accepted relationship is always symmetric or may support separately consented one-way following;
- whether contact invitations persist while the target is offline;
- exact invitation expiry, limits and cooldowns;
- exact presence states and user-interface wording;
- whether accepted contacts see exact channel automatically or only after an additional per-contact permission;
- the exact alternate-character sharing controls, such as global account policy, per-contact policy, per-character allowlist or a controlled combination;
- whether sharing may expose identity only, coarse presence, exact channel, messaging availability or separate permission tiers;
- whether an account-wide relation creates derived character contacts or only an authorized account-level social edge;
- guild, alliance, house, mentor, family or staff visibility policies;
- whether instance identity or only a generic `in instance` state may be shown;
- privacy behavior for Party Finder listings;
- database representation, protocol encoding and service ownership;
- migration behavior for imported legacy VIP entries;
- whether a private local notes list may exist without presence or social permissions.

Until resolved, no unilateral legacy VIP import may silently create mutual contact consent, account-wide consent, alternate-character disclosure or exact-location visibility.

## Rejected interpretations

### Publicly expose exact channel

Rejected because exact channel placement enables tracking, harassment, respawn interference and PvP targeting.

### Treat a contact request as an accepted relationship

Rejected because the target must explicitly accept.

### Allow unilateral VIP tracking with exact presence

Rejected because it bypasses consent and conflicts with the privacy-first direction.

### Promote one character contact to all account characters

Rejected because consent to contact one character is not consent to reveal or follow every alternate character owned by the same account.

### Create account-wide friendship implicitly

Rejected because an account-wide relationship is a separate optional relationship requiring its own clear, mutual acceptance.

### Reveal alternate characters automatically after account-wide acceptance

Rejected because account friendship and alternate-character disclosure are separate permissions. Alternate characters remain hidden until their owner deliberately widens visibility.

### Let the client hide an already disclosed alternate-character roster

Rejected because unauthorized account-character linkage must not be sent to the client in the first place.

### Couple contacts and parties into one lifecycle

Rejected because permanent social relationships and temporary gameplay groups have different consent, expiry and removal semantics.

### Trust the client to enforce privacy

Rejected because unauthorized placement and account-linkage data must not be sent to the client in the first place.

### Preserve stale exact presence after access ends

Rejected because stale data must fail toward less disclosure.

## Programme effect

- Oteryn adopts privacy-by-default for social presence.
- Exact channel and instance placement are non-public.
- Current party members and mutually accepted contacts/VIP entries are the default authorized classes for exact channel visibility, subject to later user controls.
- Adding a contact/VIP requires an invitation and explicit acceptance.
- The default contact relationship is character-to-character.
- Contacting one character does not reveal or add alternate characters from the same account.
- Account-wide friendship is a separate optional relationship requiring distinct mutual consent.
- Alternate characters remain hidden by default even after account-wide friendship is accepted.
- Alternate-character disclosure requires a separate owner-controlled privacy setting or explicit sharing action and may be revoked independently.
- Party invitations remain separate accept/decline operations.
- The legacy unilateral VIP-tracking model is not the target behavior.
- Cross-world scope, exact sharing-control granularity, storage, protocol and UI details remain future contract work.
- No implementation is authorized by this document.
