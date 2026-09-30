# SPELL-PRESENT-0 Spell and combat presentation

- Decision: `SPELLPRES0-SPELL-AND-COMBAT-PRESENTATION-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  combat and determinism) and protected integration. Owner question P1 (§12) is open; nothing
  else waits on it.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction of 2026-09-30 (build now, full Tibia Global parity); the items
  left open by SPELL-D1 §10 ("exact rejection texts", "cast animations and effects"), CHAT-0 §3
  (spell words to spectators) and ATTACK-0 §3 (a combat-effects view: numbers, animations)
- Builds on: SPELL-D1 to SPELL-D8 (command 3, `SpellCastDisposition`, domain 3 `ACTOR_VITALS`,
  cooldowns runtime-actor-local); the spell authoring schema S9 (closed cooldown groups) and S18
  (asset keys and sound cues); GAME-ABILITY-01 (one pipeline, one commit); MOVE-RL-11 D84-D87 and
  VIS-2 (capability 6, D85 identities, #1392); CHAT-0 §3 (say range) and §7 (the event-like domain
  pattern); ATTACK-0 §3-§4; CONDITIONS-0 §3.2 (ticks) and §7; MAP-WIRE-1 §7 (floors);
  FND-02 §15, §16 and §19; the client asset decision of 2026-09-27 with the owner supersession
  of 2026-09-29; owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of SPELL-PRESENT-0, in this PR: SPELL-D1 §10 (pointer and the
  result `detail`, §8 here); CHAT-0 §3 (pointer); ATTACK-0 §3 (pointer).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| PRESENT-CONTENT-1 | content lane | the binding tables from asset keys and sound cues to 15.30 client ids; creature `race`; the hit table of §5 as content (§3) | this decision |
| PRESENT-WIRE-1 | impl, protocol review | capability `PRESENTATION_V1`, domains `WORLD_PRESENTATION` and `ACTOR_COOLDOWNS`, the result `detail`, codecs, bounds and rows (§4, §7, §8, §9) | this decision; VIS-2 |
| SPELL-PRESENT-1 | spell lane, combat review | server emission for casts: spell words, the refusal smoke, cooldown state, the result `detail` (§6-§8) | PRESENT-WIRE-1; PRESENT-CONTENT-1; SPELL-D4 composition |
| COMBAT-PRESENT-1 | combat lane, combat and determinism review | emission from the Ability commit: impact, area, projectile and hit effects, damage, heal, mana and experience numbers, block effects, condition ticks, sound cues (§5) | PRESENT-WIRE-1; PRESENT-CONTENT-1; ATTACK-1; COND-1 for ticks |
| PRESENT-CLIENT-1 | client lane (client owner) | drawing effects and projectiles, floating numbers, orange spell words, the cooldown bar, refusal texts, sound playback once assets exist (§10) | PRESENT-WIRE-1 |

Later, each with its own decision: parameter spells' words (`exura sio "name"`), rune use
presentation, blood splashes and other volatile ground items, the analyser windows, durable
cooldowns, actor names on the wire.

## 1. Question

What does a player see and hear when anyone casts a spell or deals damage nearby: the spoken
words, the effects, the numbers, the cooldowns and the refusal messages?

## 2. Facts

**PROVEN**

- SPELL-D1: a cast is command 3 with a spell index into the spell book of the session's content
  generation; the result is one `SpellCastDisposition`, at most 4 bytes; unknown fields and enum
  values fail closed (`crates/protocol-oteryn/src/actor_spell.rs:23-37`, `:94-105`). Its §10
  leaves open "exact rejection texts" and "cast animations and effects"; its §3 names a cooldown
  domain as "a later additive domain".
- `SpellBook::spoken` (`apps/game-server/src/spell/mod.rs:268`) has no production caller; CHAT-0
  §3 leaves the spell words to spectators to this decision.
- Content already carries presentation: every Effect may have `impact_asset_binding`,
  `projectile_asset_binding` and `path_asset_binding`; an Ability may have `cast_cue`,
  `impact_cue` and a windup `caster_asset_binding`
  (`apps/game-server/src/content/project/v2/creature.rs:449-451`, `:648-655`, `:1145-1160`).
  Keys are `canary.appearance:effect/<name>`, `canary.appearance:missile/<name>` and
  `canary.sound:<member>` (S18); the starter bundle Ice Strike binds `effect/iceattack` and
  `missile/smallice` and the cues `spell_or_rune` and `spell_ice_strike`. No table maps a key to
  a client id yet.
- Content damage types: physical, fire, earth, energy, ice, holy, death, life_drain, mana_drain,
  drowning, healing, untyped (`content/abilities/effects/`). Creature definitions carry no race;
  a death residue fluid only (`content/creatures/definitions/`).
- The 15.30 client assets are in the repository (`content/assets/files/`, owner supersession of
  2026-09-29); `appearances.dat` holds the effect and missile appearances. The asset manifest
  (`imports/official/client-assets/15.30/manifest.json`, 6,249 files) has **no sound file**.
- VIS-2 entities carry a D85 identity (16 bytes plus a generation), direction, appearance and
  health percentage, no name (`world_spatial_entities.rs:40-60`). The interest area is 18 × 14
  (west 8, north 6), floors by D86.
- FND-02: a delta carries base and new revisions, and a mismatch forces a resync (§15); at most
  256 KiB per delta and 4,096 entries per repeated field (§19). CHAT-0 §7 already uses a domain
  whose deltas are one-shot lines and whose snapshot drops them.
- Cooldown groups are a closed catalogue of 11 keys (`cooldown-groups.json`, S9).

**CIPSOFT_OFFICIAL** (the Tibia manual)

- `magic.md` §5.4.2: not enough mana shows "You do not have enough mana" and a smoke effect; an
  unlearned spell or a low level fails with "You must learn this spell first" or "Your level is too
  low", also with the smoke; casting while a spell or group cools down shows "You are exhausted".
- `interface.md` §3.8: the cooldown bar shows the remaining cooldown of each spell and group as a
  shrinking overlay; group icons always show, spell icons only while cooling down.
- `combat.md` §5.3.1: a hit shows a damage number; a block shows a puff of smoke; armour that
  absorbs a blow shows a spark.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- Words: only a successful cast speaks, with the spell's own words, as talk type `SPELL_USE`, to
  players within ±8 × ±6 on the caster's floor (`game.cpp:7477-7494`, `spells.cpp:151-158`,
  `player.cpp:11026-11066`).
- Refusal: each failed check sends the caster a cancel message and a `POFF` effect at the caster,
  which every spectator sees; an aggressive spell in a protection zone and a rune's cooldown give
  no smoke (`spells.cpp:503-640`).
- Effects: a magic effect goes to players on every floor who can see the tile
  (`game.cpp:9343-9363`, `protocolgame.cpp:8264-8290`); a projectile to players who see its start
  or end (`game.cpp:9378-9400`); both carry a source class, own, others, creatures or global
  (`game.cpp:96-108`), which the client uses to filter effects.
- Hit colours and effects by damage type, and for physical damage by the target's race
  (`game.cpp:8072-8186`); heal numbers pastel red, mana gain maya blue, mana loss blue, experience
  white (`game.cpp:8402`, `:8633`, `:9136`, `player.cpp:3674`); a block shows `POFF`, armour
  `BLOCKHIT`, immunity a hit effect by type, each with the sound `NO_DAMAGE` (`game.cpp:413-452`).
- Numbers travel as typed damage and heal messages with position, primary and secondary value and
  colour (`protocolgame.cpp:6148-6165`), to players on the target's floor in view.
- Cooldowns: spell cooldown (spell id, ms) and group cooldown (group, ms) messages at the cast and
  at login (`protocolgame.cpp:9427-9458`, `player.cpp:13405-13437`).
- Sound: a spell sends its cast and impact sounds at the tile with a source class
  (`combat.cpp:1259`, `protocolgame.cpp:11565-11608`).

## 3. Content bindings (PRESENT-CONTENT-1)

- **Effect and missile ids.** One checked-in table maps each `canary.appearance:effect/<name>` and
  `canary.appearance:missile/<name>` key used in `content/` to its 15.30 client id, with its
  provenance (the Canary 15.30 enum, S14 and S18). The content compiler rejects a key without an
  entry and an id that `appearances.dat` does not hold as an effect or a missile.
- **Sound ids.** The same table maps each `canary.sound:<member>` to its client sound id. S18's
  approved omissions stay silence.
- **Race.** Creature definitions gain `race` (blood, venom, undead, fire, energy, ink and the
  other Canary races), from the Canary monster files with provenance. Players are blood.
- **Hit table.** §5's colour and hit effect per damage type and race is content, not code.
- Only ids travel on the wire; keys, provenance and the table stay on the server.

## 4. The presentation stream (PRESENT-WIRE-1)

- **Domain `WORLD_PRESENTATION`**, the CHAT-0 §7 pattern. Its snapshot is empty. Delta type 1 is
  one batch of events for one sync unit. A replacement snapshot drops everything not yet sent.
- **Events are presentation, not state.** They are derived from outcomes the owner has already
  committed (or, for a refusal, decided). They never draw from a SIM RNG purpose, never change
  state, are never stored, logged as state or replayed. Dropping every event changes no game
  outcome; clients keep the truth from domain 1, `ACTOR_VITALS` and `ACTOR_COOLDOWNS`.
- **Same sync unit.** An event is published in the sync unit of the change it presents: a damage
  number with the target's domain 1 health and the victim's `ACTOR_VITALS`; spell words and the
  cast effects with the caster's mana and cooldowns. A retried command (SPELL-D3) emits nothing
  new.
- **Kinds** (a oneof; an empty oneof fails closed):
  - `magic_effect {position, effect_id, source}`;
  - `missile {from, to, missile_id, source}`;
  - `spell_words {caster, position, spell}`: the D85 identity, the caster's tile and the
    SPELL-D1 index; the client shows the words of that spell from its content generation;
  - `value_text {kind, position, target, attacker, primary {value, color}, secondary {value,
    color}}`: kinds `DAMAGE_DEALT`, `DAMAGE_RECEIVED`, `DAMAGE_OTHERS`, `HEALED`,
    `HEALED_OTHERS`, `MANA`, `EXPERIENCE`, `EXPERIENCE_OTHERS`; the kind is chosen per observer;
    `attacker` is absent when there is none;
  - `sound {position, cue_id, source, secondary_cue_id}`.
- **Source class** per observer: `OWN` (the observer caused it), `OTHERS` (another player),
  `CREATURES`, `GLOBAL` (no source, or an NPC), as the 15.30 client filters effects.
- **Colour** is a closed Oteryn enum of the Tibia text colours §5 uses; the client maps it to its
  palette.
- The revision is one stream per GameSession (FND-02 §15), kept above any seen across a
  reconnect. A batch dropped whole (§9) consumes no revision, so no resync follows.

## 5. What each action shows (emission)

| Action | Events | Owner |
|---|---|---|
| Successful cast | `spell_words`; the Ability's `cast_cue` sound; then the rows below for its effects | SPELL-PRESENT-1, COMBAT-PRESENT-1 |
| Projectile | `missile` from the caster to the target tile, for an Effect with `projectile_asset_binding` | COMBAT-PRESENT-1 |
| Impact and area | `magic_effect` with `impact_asset_binding` on every tile of the resolved area that the area reaches (walls excluded), and the `impact_cue` sound once at the target tile | COMBAT-PRESENT-1 |
| Chain and windup | `missile` between chain hops with `path_asset_binding`; the windup effect on the caster | COMBAT-PRESENT-1 |
| Committed damage | the hit effect of the damage type at the target and a `value_text` (primary and a secondary for an elemental split) | COMBAT-PRESENT-1 |
| Heal, mana gain and loss | `value_text` `HEALED`, `MANA`; a mana loss adds the `loseenergy` effect | COMBAT-PRESENT-1 |
| Block, armour, immunity | `POFF`, `BLOCKHIT`, or the immunity effect by type, with sound `NO_DAMAGE`; no number | COMBAT-PRESENT-1 (ATTACK-1 outcome) |
| Condition tick | as committed damage (CONDITIONS-0 §3.2) | COMBAT-PRESENT-1 |
| Experience | `value_text` `EXPERIENCE` at the killer; `EXPERIENCE_OTHERS` to spectators | COMBAT-PRESENT-1 (VSL-COMBAT-01 outcome) |
| Refused cast | `POFF` at the caster where §6 says so | SPELL-PRESENT-1 |

Hit colours and effects (Canary `game.cpp:8072-8186`, as content in §3):

| Damage type | Colour | Hit effect |
|---|---|---|
| physical, race blood | red | `drawblood` |
| physical, race venom | light green | `hitbypoison` |
| physical, race undead or ink | light grey | `hitarea` |
| physical, race fire | orange | `drawblood` |
| physical, race energy | purple | `energyhit` |
| energy | purple | `energyhit` |
| earth | light green | `green_rings` |
| fire | orange | `hitbyfire` |
| ice | sky blue | `iceattack` |
| holy | yellow | `holydamage` |
| death | dark red | `smallclouds` |
| life_drain | red | `magic_red` |
| drowning | light blue | `loseenergy` |
| mana_drain, mana shield absorption | blue | `loseenergy` |
| healing | pastel red | none |
| untyped | Canary's default: no number, no effect | none |

- The damage still commits when the presentation shows nothing; §5 is display only.
- A hit with no damage after defence shows the block effect, never a 0.

## 6. Spell words and refusals (SPELL-PRESENT-1)

- **Words only on success.** A cast whose PRIMARY COMMIT succeeds speaks; a refused cast speaks
  nothing, as in Tibia. The words are the spell's own words from content (never text the player
  typed), shown in orange above the caster and in the local chat console.
- **Range.** CHAT-0's `say` range: players on the caster's floor within ±8 × ±6 tiles
  (`SPELLPRES0-RL-06`), whether or not they hold `CHAT_V1`.
- **Refusal smoke.** A refused cast sends the caster its result (§8) and emits `POFF` at the
  caster's tile to every spectator who sees it (§7), following Canary's check order. No smoke:
  an aggressive spell in a protection zone, a rune's cooldown, and `REJECTED` (stale index, server
  fault, ineligible actor). A muted cast (CHAT-0 §6) is `REJECTED` with `detail` `MUTED` and no
  smoke, like Canary's refusals that come before the spell checks (`spells.cpp:503-506`).
- `SpellBook::spoken` stays without a production caller: spells are still cast by index
  (SPELL-D1, CHAT-0 §3).

## 7. Who sees what

- **Tile events** (`magic_effect`, `sound`): observers whose interest area (D84) holds the tile on
  a floor they see (D86, MAP-WIRE-1 §7).
- **`missile`:** observers who see its start or its end tile by the same rule; the client clips.
- **`value_text`:** observers whose interest area holds the target tile on the observer's own
  floor. The kind is chosen per observer (dealt, received, others).
- **`spell_words`:** §6's range.
- The per-observer test uses the same interest index as VIS-2; no second spatial query exists.
- `PRESENTATION_V1` requires `WORLD_SPATIAL_ENTITIES` (capability 6), whose D85 identities the
  events name. A session without it receives no events, no `ACTOR_COOLDOWNS` and no `detail`,
  and keeps today's behaviour; it is not refused.

## 8. Refusal texts: the result `detail` (PRESENT-WIRE-1, SPELL-PRESENT-1)

- `WorldActorSpellCastResultV1` gains `detail` (field 2), a closed enum sent only to sessions
  holding `PRESENTATION_V1`: `NOT_LEARNED`, `VOCATION`, `PREMIUM`, `NEEDS_WEAPON`,
  `PROTECTION_ZONE`, `GO_UPSTAIRS`, `GO_DOWNSTAIRS`, `OUT_OF_RANGE`, `NOT_REACHABLE`,
  `ONLY_CREATURES`, `NOT_ENOUGH_ROOM`, `MUTED`. Both fields are one-byte varints, so the result
  stays within 4 bytes; a peer without the capability never sees field 2.
- The disposition stays the outcome; `detail` only picks the text. The client holds the texts:
  the manual's where it states one (§2), else the Tibia text for that refusal.
- A later check with a new text adds a value behind a capability revision.

## 9. Bounds (PRESENT-WIRE-1 registers and measures)

- **Per event.** Worst case `value_text`: a position (18 bytes), two D85 identities (31 each), two
  value-and-colour pairs (10 each, 15 for an experience u64), the kind (2) and framing (4): 111
  bytes. `spell_words` is at most 59, `missile` 46. The cap is `SPELLPRES0-RL-02` (128 bytes).
- **Per batch.** At most `SPELLPRES0-RL-01` (1,024) events per session per sync unit: at most
  `SPELLPRES0-RL-03` (131,136 bytes with a 64-byte header), under FND-02's 256 KiB and 4,096
  entries.
- **Overflow.** Beyond it, the server keeps the observer's own events (its casts, its received
  damage and heals) first, then the nearest by the D87 order, and drops the rest. Ties, including
  events with no entity identity (`magic_effect`, `sound`), break by the authoritative order of
  the committed outcome in the sync unit, then by the event's emission ordinal within that
  outcome; this total order never depends on insertion or container iteration order, so the same
  outcomes keep the same events on every server and every run. No marker: the events are
  presentation (§4).
- **Slow clients.** When a session's egress already holds `SPELLPRES0-RL-07` (2) undelivered
  presentation batches, the next batch is dropped whole. Presentation never fills the egress
  queue and never trips slow-client handling (FND-02 §16).
- **Snapshots.** While a snapshot of any domain is in flight for a session, its events are
  dropped, not retained behind the barrier.
- Before activation PRESENT-WIRE-1 measures on B3 and records on #162 typical and p99 batch
  bytes for a crowded hunt (20 casters with area spells) and server time per sync unit.

## 10. Cooldowns (PRESENT-WIRE-1, SPELL-PRESENT-1, PRESENT-CLIENT-1)

- **Domain `ACTOR_COOLDOWNS`**, own actor only, owned by the channel runtime. It is state: an
  entry is `{kind: SPELL or GROUP, id, expires_at_ms}`, where `id` is the SPELL-D1 index or the
  group's ordinal in the S9 catalogue and `expires_at_ms` is the absolute expiry on the channel
  runtime's monotonic millisecond clock. Each snapshot and delta also carries `server_now_ms`, that
  clock's value when the server encoded it. No duration travels on the wire, so time spent in the
  egress queue, on the network or in a chunked snapshot never extends a cooldown on the client.
- **Snapshot** at admission, reconnect and channel transfer: every running cooldown with its
  expiry. SPELL-D2 keeps cooldowns across a reconnect, so the bar survives it. The clock belongs
  to the channel runtime, so a snapshot re-expresses every expiry on the new runtime's clock.
- **Delta** at each PRIMARY COMMIT that starts or changes a cooldown (a cast, a later reduction):
  the changed entries. Expiry sends nothing.
- **Client countdown.** On each snapshot or delta the client records `offset = server_now_ms -
  local_receipt_ms` from its own monotonic clock; a snapshot resets the estimate and a delta keeps
  the larger offset, the one with the least delivery delay. It shows `remaining = max(0,
  expires_at_ms - (local_now_ms + offset))` and drops an entry at 0. The residual error is at
  most the least observed one-way delay, and the bar still never decides legality.
- Bounds: at most `SPELLPRES0-RL-04` entries (`SPELL-RL-04` plus 16 group slots) of at most
  `SPELLPRES0-RL-05` (16 bytes) each; `expires_at_ms` stays within 6 varint bytes (2^42 ms) and
  `server_now_ms` is per message, outside the entry bound.
- The client draws the bar of `interface.md` §3.8 and never decides legality: `COOLING_DOWN`
  still comes from the server.

## 11. Client (PRESENT-CLIENT-1)

- It draws effects and projectiles from `appearances.dat`, floating numbers by colour, orange
  spell words above the caster and in the local console, the cooldown bar and the refusal texts.
- It honours the source class in its own-and-others effect options.
- It builds the console line of a `value_text` ("A rat loses 5 hitpoints due to your attack.")
  from the actor names it knows; names on the wire are not decided here (§17).
- It never infers damage, legality or cooldowns from events; state comes from the domains.

## 12. Sound

- Sound is in scope, as in Global: the `sound` event carries the cast and impact cues content
  already has (S18) and `NO_DAMAGE` for blocks.
- The 15.30 sound files are not in the repository (§2), so the client plays nothing until they
  are (P1). The events cost one oneof member.

## 13. Rows (registered by PRESENT-WIRE-1 before implementation)

| Row | Value |
|---|---|
| `SPELLPRES0-RL-01` presentation events per session per sync unit | 1,024 |
| `SPELLPRES0-RL-02` bytes per encoded event | 128 |
| `SPELLPRES0-RL-03` bytes per presentation batch | 131,136 |
| `SPELLPRES0-RL-04` entries per `ACTOR_COOLDOWNS` snapshot or delta | `SPELL-RL-04` + 16 |
| `SPELLPRES0-RL-05` bytes per cooldown entry | 16 |
| `SPELLPRES0-RL-06` spell words range | ±8 × ±6, caster's floor (CHAT-0 `say`) |
| `SPELLPRES0-RL-07` undelivered presentation batches per session | 2; the next is dropped whole |

## 14. Owner questions

**P1. Sound files.** The 15.30 client's sound files are not among the committed client assets, and
the owner's 2026-09-29 rights confirmation names only the files present then.
- **a (recommended):** the owner confirms the same redistribution rights for the 15.30 client
  sound files, and they are committed beside the other client assets; the client plays spell and
  combat sounds as in Global.
- **b:** no sound files for now; the `sound` events are sent and the client stays silent until a
  later owner decision.

## 15. Rejected options

- **Sending effect text or asset keys on the wire.** Ids from the client's own `appearances.dat`
  are smaller and hide the server tables.
- **Spell words as a `CHAT` line.** Casters and spectators without `CHAT_V1` would lose them, and
  free text would travel where an index does.
- **Parsing spells from chat.** SPELL-D1 and CHAT-0 §3 keep casts on command 3.
- **Colours computed by the client.** The client does not know a creature's race.
- **Events stored or replayed.** Tibia shows them once; storage would add durable writes per hit.
- **Poff only to the caster.** Tibia and Canary show it to every spectator.
- **Dropping sound from the wire.** Adding it later needs a new capability; the cost now is one
  member.
- **Cooldowns as events only.** A reconnect would lose the bar; SPELL-D2 keeps the cooldowns.

## 16. Owner-rule applications (5905825574, Global parity)

- Words only after a successful cast, the spell's own words, in the say range.
- The refusal smoke visible to spectators, with Canary's exceptions.
- Damage, heal and experience numbers for other actors' fights, by observer kind.
- Colours and hit effects by damage type and race; block, armour and immunity effects.
- The source class, so the client's own-and-others effect filters work.
- Sound in scope (P1 decides only the files).

## 17. Decision test

- **Must decide now:** YES. Spells, attacks and conditions are being built; without it no player
  sees a spell or a hit.
- **Minimum sufficient:** one event domain, one own-actor cooldown domain, one result field, one
  binding table and one content field.
- **Superseding evidence:** measured batch bytes over budget (§9); an official statement of ranges
  or colours; the owner's answer to P1.
- **Deliberately not decided:** actor names on the wire (VIS-2 has none; the console line waits
  for it); parameter spell words; rune use; blood splashes and other volatile ground items
  (MAP-WIRE-1 §4); the analyser windows; durable cooldowns (the manual freezes them offline;
  SPELL-D2 keeps them runtime-only); item-use exhaustion (ITEM-USE-0).

## 18. Before-freeze checklist

1. **Contract amendments:** SPELL-D1 §10 (pointer and `detail`), CHAT-0 §3 and ATTACK-0 §3
   (pointers), each written "pending on acceptance of SPELL-PRESENT-0".
2. **Serialization:** events ride the sync unit of the committed change; cooldowns change only at
   PRIMARY COMMIT on the channel owner.
3. **Restart:** events are lost by design; cooldowns follow SPELL-D2.
4. **Typed references:** D85 identities, SPELL-D1 index, S9 group ordinal, 15.30 effect, missile
   and sound ids, content keys (server only).
5. **Wire:** §4, §8, §10, capability `PRESENTATION_V1`; numbers reserved on #162 at allocation.
6. **Split work:** at most 1,024 events per session per sync unit; overflow drops, never queues.
