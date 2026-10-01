# Character appearance owner decision

- Decision: `CHARACTER-APPEARANCE-OWNER-V1`
- Status: **CANDIDATE with owner decisions D47, D49 and D61 taken (§2)**. Acceptance requires
  exact-head validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Follows: `OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md` §4.3,
  whose follow-up "Character appearance owner" this closes
- Owner decisions posted: #707 5867311210, #162 5867312726 (D47, D49), #162 5871032954 (D61)
- Admission baseline: `main@ae252ce`
- Runtime, migration, protocol and production authority: **NONE**. Each child in §5 needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

A player needs a look: an outfit, four colours, addons and optionally a mount. Other players must
see it. D47 made outfit, addon and mount unlocks account-scoped. No owner, persisted state,
command or protocol field exists for any of it. Who owns the look, what is stored, and how does
it reach the client?

## 2. Owner decisions

| # | Decision | Source |
|---|---|---|
| D47 | Outfits, addons, mounts and Store purchases belong to the account, not the character. | #707 decision §2 |
| D49 | Cosmetics apply on every world where the content is compatible, and fail closed otherwise. | #707 decision §2, §4.3 |
| D61 | In V1, every unlocked outfit, addon and mount is usable by everyone, premium ones included, while Premium stays unactivated. A declared Reference difference. | "Wszystkie dla każdego w V1" |

## 3. Facts

**PROVEN** (main `ae252ce`)

- No appearance field exists in character roots (`migrations/0005_character_authority.sql:49-63`),
  the bootstrap intent (`character_bootstrap_intent.rs:35-47`) or any protocol message
  (`world_spatial_v1.proto:37-56` sends only the player's own position).
- `AccountUnlock(account_id, unlock_key)` is decided (#707 decision §4.3): write-once, with
  appearance-definition provenance, fails closed on incompatible content, gameplay effects per world.
- Content identities exist without look data: 133 Outfit identities
  (`content/world/definitions/declarations.json`), 252 mount records
  (`content/cosmetics/mounts/`), and `content/cosmetics/outfits/index.json` is
  `READY_UNPOPULATED`.
- Look semantics are already specified for rendering (`OTERYN_ATLAS_ANIMATED_APPEARANCES_V1.md`
  :50-68): a look type from the outfit category fails closed if unknown; addon row y is shown when
  `addons & (1 << (y-1))`; a two-layer colour mask maps to head, body, legs and feet. The palette is
  `tibia-hsi-19x7-v1` with 133 colours (`tools/game-atlas-appearances/export.py`).
- NPC and monster schemas already model a look as look type, four colours, addons and mount
  (`npc.schema.json:39-45`, `monster.schema.json:1295-1360`).
- The client graphics are pinned to 15.30 as a checksum manifest only (#1025); no look-type table
  is in the repository.
- Character sex is not yet a creation input (GAME-CHAR-01 Stage B reconciliation lists name, sex
  and world as UNKNOWN). Outfits have a male and a female look type.

**UNKNOWN**

- Reference starter outfits per sex, free versus premium outfits, addon acquisition, the mount list
  at the target date, and outfit or mount gameplay bonuses. These are content facts (§4.6).

## 4. Decision

### 4.1 Owner

The **Character appearance** component of the GAME-CHAR domain owns:

- each character's selected appearance (§4.2);
- the account's `AccountUnlock` set for outfits, addons and mounts (#707 decision §4.3).

Presentation belongs to the client; the server owns what is selected and whether it is allowed.

### 4.2 Selected appearance (durable, per character)

| Field | Meaning | Bound |
|---|---|---|
| `outfit_key` | Outfit definition key; the look type follows from the character's sex | a key of the active content |
| `colours` | head, body, legs, feet | each an index of the content palette (133 colours) |
| `addons` | addon bits of that outfit | a subset of the unlocked addons |
| `mount_key` | selected mount, or none | an unlocked mount key |
| provenance | content revision the selection was validated under | ≤ 128 B |

- Persisted as Character state under DUR-02, written only inside a fenced character event
  (session-generation fence). The physical schema belongs to the implementing child.
- **Mount activation is deferred.** `mount_key` is stored and validated, but mounting, the mounted
  state, its projection into the actor's appearance and any speed effect wait for a separate
  mount decision (with the world ruleset's speed fact, D47). Until then no mount is shown to
  anyone, and the observation contract (§4.5) carries no mount.

**Amendment (owner decision 2026-10-01, #162): mount speed now.** The deferral above ends for speed. An active mount adds +10 speed, a ruleset fact of the Reference profile (D47: mount speed belongs to the world ruleset), applied now. Mount activation is Global (D125, `OTERYN_GAME_OWNER_DECISION_BATCH_D118_D128_2026-09-28.md` §2.5). Source: TibiaWiki "Mounts" (https://tibia.fandom.com/wiki/Mounts): "All mounts give +10 speed." Per-mount exceptions stay **UNKNOWN** and default to +10. The speed value is profile data, not a Character field, and `mount_key` storage and validation are unchanged.

### 4.3 Allowed selection

The owner accepts a selection only if, on this world's active content:

1. the outfit is authorized by exactly one of:
   - **starter:** the active content marks it a starter outfit for the character's sex. No unlock
     fact exists or is needed; it is validated against the active content revision only;
   - **earned unlock:** its key is in the account's `AccountUnlock` set, and the active definition
     is compatible with the unlock's recorded provenance (#707 decision §4.3);
   - **Store purchase:** a usable Platform entitlement for that cosmetic (#707 decision §4.3 and
     §4.5; the Store unlock follows the Platform lifecycle, not the `AccountUnlock` fact). This path
     stays unavailable until the gap register §32 delivery decision defines how Game reads it;
2. every selected addon bit is authorized the same way (a starter addon from content, an earned
   unlock with compatible provenance, or a usable Store entitlement);
3. every colour is within the palette;
4. the mount, if any, is authorized the same way (for storage only while mount activation is
   deferred, §4.2).

Premium flags do not restrict usability in V1 (D61). When Premium activation is decided, that
decision defines the premium gate and what happens to a premium selection on a non-premium account.

### 4.4 Incompatible world

If the stored selection is not valid on a world (a key missing or incompatible there, or a Store
entitlement no longer usable), the world shows the content's fallback look for the character's
sex (§4.6). The stored selection is not changed, and the key is never reinterpreted (D49).

### 4.5 Change and observation

- The player changes the look with one command. The owner validates it (§4.3), commits it durably
  and publishes it. A rejected command changes nothing.
- Other players observe the look through the actor appearance state of the Combat child E
  protocol lane (creature appear, move, look). The player's own client receives the same state.
- Mount speed and any other gameplay effect stay in the world's profile ruleset (D47).

### 4.6 Content inputs

- Outfit definitions carry: the male and female look types, the addon count, the starter flag per
  sex, a premium flag (recorded, not enforced in V1) and unlock sources.
- Mount definitions carry the mount look type, the speed fact and the premium flag.
- The palette is content, not an engine constant.
- The active content declares exactly one **fallback look** per sex: one starter outfit key and
  four explicit colour indices, with no addons and no mount. Content validation rejects a content
  generation without it.
- Values come from the target-date sources (tibia.com, then the wikis; the client 15.30
  `appearances.dat` resolves look types through the #1025 loader follow-up). Missing values fail
  content validation.

### 4.7 Unlock sources

- Starter outfits: implicit, per sex, from content; no fact is written.
- Quest and NPC grants: an `AccountUnlock` inserted inside the fenced character event of the grant
  (#707 decision §4.6).
- Store: authorized by a usable Platform entitlement, not by an `AccountUnlock` fact (§4.3);
  delivery and lifecycle stay under gap register §32, which is a prerequisite for this path.

### 4.8 Dependencies

- Character sex must become a creation input before look types can resolve. The bootstrap intent
  and the Character creation owner add it. Until then only the native entry-room test character
  exists, with a content-declared default.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| APP-1 | Durable selected appearance and the AccountUnlock set; validation §4.3; fence | Character authority; progression readiness migration numbering |
| APP-2 | Outfit and mount definitions with look types, starter flags, palette | content pipeline; #1025 loader follow-up |
| APP-3 | Change command and actor appearance state | Combat child E protocol lane; registry lease |
| APP-4 | Client outfit window and rendering of other actors | APP-3; client owner |

## 6. Rejected options

- **Appearance on the account instead of the character.** D47 moves unlocks to the account, not
  the selection; each character keeps its own look.
- **Client-chosen look without server validation.** A client could show locked or premium outfits
  to others.
- **Reinterpreting a missing key as a nearby outfit.** It violates D49.

## 7. Decision test

- **Must decide now:** YES. The Combat child E protocol lane needs the actor appearance shape, and
  the account unlock model (D47) has no consumer.
- **Minimum sufficient:** one selection per character, validated against the unlock set and
  content; no outfit bonuses, no premium gate in V1.
- **Superseding evidence:** a Premium activation decision; Reference outfit bonuses; Evolved
  cosmetics beyond outfits and mounts (wings, familiars).
- **Deliberately not decided:** the physical schema, the Store catalogue, outfit bonuses, sex change
  and familiar or wing attachments.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
owner_decisions: [D47, D49, D61]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: false   # until this decision is protected-integrated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (Character state, account unlocks, protocol shape)"
implementation_lanes: [APP-1, APP-2, APP-3, APP-4]
required_revalidation:
  - "APP-1: a locked outfit, addon or mount is rejected; an out-of-palette colour is rejected; a stale character fence writes nothing; a selection survives relog"
  - "APP-3: other players see a committed look; a rejected change publishes nothing"
  - "a world without a compatible definition shows the content fallback look (one per sex, explicit colours) and keeps the stored selection"
  - "a starter outfit is accepted from the active content without any unlock fact; a Store cosmetic is rejected until the §32 path exists; no mount is shown while mount activation is deferred"
remaining_unknowns:
  - Reference outfit facts at the target date (content)
  - Character sex as a creation input
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates APP-1 and APP-2."
```
