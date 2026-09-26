# Oteryn Game — Housing ACL / GUI Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected access-control direction for physical houses after reviewing the current Global Tibia house-rights model. Oteryn keeps the useful permission semantics but does not copy Tibia's text-command / house-spell UX.

## Reference baseline reviewed

Current official Global Tibia house rights provide separate concepts for:

- owner;
- invited guests;
- sub-owners who may maintain the guest list and kick characters;
- per-door rights for front/interior doors;
- guild/member/rank patterns in access lists;
- owner override over house doors.

Global Tibia exposes these rights through house-spell commands such as `Aleta Sio`, `Aleta Som`, `Alana Sio` and `Aleta Grav`.

Oteryn intentionally preserves the permission concepts while rejecting text-command administration as the primary product UX.

## Owner-selected role hierarchy

`OWNER_SELECTED`.

The first-generation ordinary physical-house ACL uses at least these semantic roles:

```text
OWNER
  |
  +-- MANAGER / SUBOWNER
  |
  +-- GUEST
```

### OWNER

The canonical `CharacterId` house owner has full house-access administration authority, subject to authoritative server validation.

Owner-only housing authority includes at least:

- appointing/removing Managers/Subowners;
- managing Guests directly;
- assigning/revoking door, zone and functional permissions;
- changing ACL policy;
- initiating owner-only house lifecycle actions under the owning housing contracts.

Manager/Guest permissions never confer property ownership or disposal authority.

### MANAGER / SUBOWNER

A Manager/Subowner is a delegated house administrator, not an owner.

Initial semantic direction:

- may invite/remove Guests where the Owner permits;
- may remove/kick non-owner occupants where the policy permits;
- may administer only the delegated ACL surfaces exposed by the Owner;
- cannot sell, relinquish, include the house in Bazaar, change rent ownership, bypass eviction, or otherwise transfer `HouseId` ownership;
- cannot grant themselves or others an ownership-equivalent capability.

Exact delegation granularity remains a later contract/UI detail.

### GUEST

A Guest receives only explicitly granted access/use capabilities.

Being a Guest does not imply:

- Manager authority;
- ownership;
- access to every door/room;
- use of every bed/storage/workstation/service;
- authority to modify ACLs.

## Capability-oriented access

Oteryn should not reduce the entire ACL to a single `may_enter_house` boolean.

The later house contract/UI may expose explicit capabilities such as:

- `ENTER_HOUSE`;
- `OPEN_DOOR` for a particular door/zone;
- `USE_BED`;
- `USE_STORAGE`;
- `USE_WORKSTATION` or equivalent house service;
- `INVITE_GUEST`;
- `REMOVE_GUEST` / `KICK_OCCUPANT`;
- later guild-related capabilities when the guild architecture exists.

Exact capability names and persistence schema remain deferred; the semantic requirement is that sensitive functions can be granted independently rather than accidentally inheriting every right from simple house entry.

## Door / room / zone rights

`OWNER_SELECTED_DIRECTION`.

Oteryn preserves the useful Global Tibia idea that access can be narrower than whole-house access.

The house model must support explicit restriction of at least meaningful interior doors or zones so that large houses can contain private rooms/areas.

The exact representation may be per-door, per-zone or a later unified spatial ACL object. The product must not require the owner to maintain raw text patterns or memorize commands to configure this.

## GUI-only administration requirement

`OWNER_SELECTED`.

Player-facing house access administration MUST be performed through a graphical UI/panel.

The first-generation product must not require Tibia-style typed commands or magic-word/spell syntax for ordinary ACL administration.

Conceptual GUI surfaces may include:

- `House Management` panel;
- Members / Guests list;
- Managers/Subowners list;
- doors/rooms/zones list or visual selector;
- permissions/capabilities toggles;
- search/add Character flow;
- remove/revoke actions;
- clear indication of inherited/delegated permissions;
- server-confirmed success/failure and current revision.

The exact visual layout is a later client/UI task, but **GUI administration itself is part of the selected product direction**, not an optional implementation detail.

Chat commands may exist later only as non-authoritative convenience aliases if explicitly approved; they must never be required to manage house rights and must resolve to the same authoritative ACL mutations as the GUI.

## Authoritative ACL and stale-client behavior

House ACL is World-scoped authoritative state, not client-local configuration.

Requirements:

- every ACL mutation is validated by the authoritative house domain/runtime;
- stale UI state cannot grant or preserve a revoked permission;
- permission changes are revisioned or otherwise fence stale writes;
- entry and value-bearing/sensitive actions revalidate the currently authoritative permission where required;
- a Manager removed by the Owner loses Manager powers authoritatively even if an old panel remains open;
- risky ACL mutations fail closed when current authority cannot be proven.

## Owner override and safety

The Owner always retains authoritative administration access to the owned house while ownership is valid, except where a later explicit safety/lifecycle state (for example eviction settlement) fences mutation.

A Manager/Subowner cannot lock the Owner out, revoke ownership, or create a circular ACL state that prevents authoritative recovery.

## Beds remain a separate decision

Global Tibia currently treats beds as one-person-at-a-time house facilities and ties their use to Premium status.

Oteryn does not inherit that Premium-bed coupling from this decision.

`USE_BED`, bed occupancy, Rested-XP consequences, public-inn versus house-bed strength and non-Premium residence behavior remain separate owner decisions under the existing Rested/housing direction.

## Guild integration deferred

Guild/rank-based house access is useful reference behavior, but Oteryn does not yet have the owning guild-system architecture required to freeze it safely.

Therefore:

- guildhouse ownership remains `GuildId` as previously selected;
- guildhouse ACL, guild ranks, leader/vice semantics and automatic guild membership grants remain `DEFERRED_TO_GUILD_ARCHITECTURE`;
- ordinary physical-house ACL must be designed so future guild-derived principals/capabilities can be added without replacing the base owner/manager/guest model.

## Relationship to Character Bazaar / ownership changes

ACL never overrides the selected ownership/Bazaar rules.

When ownership is relinquished, evicted or otherwise changes under an accepted lifecycle:

- stale Manager/Guest authority must not survive accidentally into the next ownership generation;
- the later settlement contract must define which ACL state is cleared, migrated or reconstructed;
- a Character+House Bazaar bundle does not implicitly transfer unrelated private ACL assumptions unless the later contract explicitly says so.

The safe default is fresh authoritative ACL generation after ownership-changing settlement, with explicit preservation only where a later product decision requires it.

## Deliberately deferred

This checkpoint does not freeze:

- exact GUI layout/widgets;
- protocol messages;
- table/schema/DDL representation;
- exact number of Managers/Guests;
- exact per-role default capabilities;
- nested custom roles;
- temporary/time-limited invitations;
- audit-log retention/UX;
- bed permissions/Rested values;
- guild rank integration;
- residence/apartment ACL differences;
- production/runtime implementation.

## Decision

`HOUSE ACL ROLES: OWNER -> MANAGER/SUBOWNER -> GUEST`

`HOUSE ACCESS: CAPABILITY-AWARE, INCLUDING DOOR/ZONE/FUNCTION-SPECIFIC RIGHTS`

`PLAYER-FACING ACL ADMINISTRATION: GUI/PANEL REQUIRED`

`TIBIA-STYLE HOUSE COMMANDS/SPELLS AS REQUIRED UX: REJECTED`

`GUILD ACL INTEGRATION: DEFERRED TO GUILD ARCHITECTURE`

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
