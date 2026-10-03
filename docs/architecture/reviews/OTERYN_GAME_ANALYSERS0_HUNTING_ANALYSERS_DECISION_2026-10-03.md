# ANALYSERS-0 Hunting Analysers

- Decision: `ANALYSERS0-HUNTING-ANALYSERS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol)
  and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - control-plane allocation D298 (#1622), under owner extension D296;
  - the Analysers system of owner decision 1a's Q1a list (CYCLOPEDIA-0 header);
  - the "analyser windows" that SPELL-PRESENT-0 left for a later decision.
- Builds on:
  - SPELL-PRESENT-0 §4 and §9 (the presentation stream; its batches may be dropped whole);
  - D3 (D132 top-damage attribution, D133 the 10 s loot window) and PARTY-PVP-0 §5.3 (party loot
    right);
  - PARTY-PVP-0 §5.1 (shared experience) and the progression receipts of migration `0009`;
  - MARKET-0 §3 and §10 (market prices and wire);
  - BOSS-RAID-0 §6.3 and §13 (boss cooldowns and their wire view);
  - FND-02 (domains, sync units, resync);
  - owner rule 5905825574.
- Amends: none.
- Runtime, migration and production authority: NONE. Each child needs its own #1622 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ANALYSER-WIRE-1 | impl, protocol review | capability 10 `ANALYSER_V1`, domain 15 `ACTOR_ANALYSER`, the fact codecs, bounds and rows (§4, §6) | this decision |
| ANALYSER-EMIT-1 | combat and item lanes, determinism review | emission of the facts at their commit points (§5) | ANALYSER-WIRE-1; ATTACK-1; D3 corpse mint; the consuming item uses |
| ANALYSER-CLIENT-1 | client lane (client owner) | the windows of §3: sessions, rates, prices, the Drop Tracker list | ANALYSER-WIRE-1 |

Later, each with its own decision:
- the Party Hunt Analyser (cross-member aggregation; PARTY-PVP-0 left it for later);
- the market price mode, once MARKET-WIRE-1 carries a price summary;
- attacker names in Damage Input (actor names on the wire are deferred by SPELL-PRESENT-0).

## 1. Question

Which hunting facts does the server send so the client can show Tibia's analysers, and what may
the server keep about them?

## 2. Facts

**PROVEN**

- The presentation stream carries damage, heal, mana and experience numbers, but a batch may be
  dropped whole under its bounds (SPELL-PRESENT-0 §4, §9). Its events are not a reliable count.
- Loot right is decided by top damage (D132) and the party loot right (PARTY-PVP-0 §5.3). The
  corpse and its contents are minted durably at death (D3).
- Experience gains are committed with progression receipts (`0009`); party sharing is
  PARTY-PVP-0 §5.1.
- Boss cooldowns are durable rows with a wire view (BOSS-RAID-0 §6.3, §13).
- No analyser state or wire exists in the repository.

**CIPSOFT_OFFICIAL** (Tibia manual, `interface.md` §3.6.12, capture 2026-09-28; tags as written)

- `[client]` Hunting Session Analyser (duration, XP gain, XP/h, loot, kills); Loot and Supply
  Analysers (items and gold value by NPC, market or custom price); Impact and Damage Input
  Analysers (damage or healing dealt or received); XP Analyser (XP/h with and without bonuses);
  Drop Tracker (per-item drop rate, set up via the Cyclopedia); Party Hunt Analyser (up to 50
  members, each and in total); Boss Cooldowns.

**UNKNOWN**

- Whether Tibia keeps any analyser session on the server across a relog.

## 3. What the client builds (ANALYSER-CLIENT-1)

The analysers are client tools, as the manual tags them. The client keeps the session, the totals,
the hourly rates, the reset button and the custom prices. The server sends facts (§4) and keeps
nothing.

| Analyser | From |
|---|---|
| Hunting Session | `experience`, `kill_loot`, `supply_used`, client clock |
| Loot, Supply | `kill_loot`, `supply_used`, prices (§3.1) |
| Impact | `impact` |
| Damage Input | `damage_input` |
| XP | `experience` (raw and gained) |
| Drop Tracker | `kill_loot`, with the item list the player marks in the client |
| Boss Cooldowns | the BOSS-WIRE-1 cooldown view (BOSS-RAID-0 §13); nothing new |

### 3.1 Prices

- **NPC price:** the item's NPC buy value from the client content export (the same generation the
  client already loads).
- **Custom price:** client-local, Premium only in Tibia; the client reads the Premium state it
  already has.
- **Market price:** not offered until MARKET-WIRE-1 carries a price summary (later decision).

## 4. The analyser stream (ANALYSER-WIRE-1)

- **Capability 10 `ANALYSER_V1`** (owner: the Analyser module), with no command type and state
  domain **15** `ACTOR_ANALYSER` (delta type 1 `ANALYSER_FACTS_DELTA_V1`, snapshot type 1
  `ANALYSER_SNAPSHOT_V1`, both gated by capability 10); not offered before ANALYSER-WIRE-1 ships.
  The numbers are the control plane's leases in STATE (cap 10, domain 15). ANALYSER-WIRE-1
  registers them in the FND-02 registry with its `.proto`.
- **Own session only.** Each fact goes to the one GameSession whose character it concerns. Nothing
  about another player is sent.
- **Snapshot empty.** Delta type 1 is one batch of facts for one sync unit. A resync or reconnect
  loses facts not yet sent; the client keeps its totals. Analysers are advisory; the stream never
  carries state.
- **Revision (FND-02 §15).** One cumulative, monotonic `uint64` per GameSession: 0 at the
  session's initial snapshot, plus 1 per delta sent; never reset or reused at a resync snapshot or
  a reconnect. The empty snapshot carries the current revision, and the next delta's
  `base_revision` equals it.
- **Not dropped for size.** Unlike the presentation stream, a batch is never dropped for its
  size; facts beyond a batch bound continue in the next sync unit. Only the pending cap of §6
  drops facts (the oldest first), and the next batch then carries `dropped {count}` so the client
  can mark its totals incomplete.
- **Facts** (a oneof; an empty oneof fails closed):
  - `experience {raw, gained}`: the experience before and after modifiers (stamina, boosts, party
    share), as committed;
  - `kill_loot {race, corpse_items[(item_type, count) <= 32], gold}`: sent to the holder of the
    loot right when the corpse is minted; `gold` is the coin value in gold;
  - `supply_used {item_type, count}`: a consumed potion, rune or ammunition, when its use commits;
  - `impact {kind (DAMAGE | HEALING), value, element}`: damage the character dealt, or healing it
    gave (to itself or another);
  - `damage_input {value, element, source (CREATURE | PLAYER | NONE), race}`: damage received;
    `race` only for a creature source.
  - `dropped {count}`: facts lost to the pending cap since the last batch.

## 5. Emission (ANALYSER-EMIT-1)

- A fact is emitted after the outcome it reports is committed, in the same sync unit, with the
  decision ordinal of SPELL-PRESENT-0 §4. A retried command emits nothing new.
- Facts are derived, never stored, never read by a rule, never replayed. Dropping all of them
  changes no game outcome.
- `impact` and `damage_input` report the final value applied (after mitigation and PvP factor),
  not the attempted one.
- A kill with no loot-right holder in a session (the holder left) emits no `kill_loot`.

## 6. Bounds and rows (registered by ANALYSER-WIRE-1)

| Row | Value |
|---|---|
| `ANALYSERS0-RL-01` facts per batch | 64 |
| `ANALYSERS0-RL-02` items per `kill_loot` | 32 (more items: the rest in a second fact for the same race) |
| `ANALYSERS0-RL-03` bytes per batch | measured by ANALYSER-WIRE-1, at most 4 KiB |
| Pending facts per session | 1,024; above it the oldest are dropped and the next batch says how many |

## 7. Rejected options

- **Reusing the presentation stream.** Its batches may be dropped whole, so totals would be wrong.
- **Server-side analyser sessions.** The manual tags the analysers `[client]`; a server copy would
  add durable state with no gameplay use.
- **Sending raw combat logs for the client to filter.** Other players' data would reach the
  client.

## 8. Architect rulings (owner rule 5905825574)

- **R1, where the session lives: a) the client**; b) the server. Recommendation and ruling: a).
- **R2, carrier: a) a separate own-session domain**; b) the presentation stream. Recommendation
  and ruling: a).
- **R3, values: a) final applied values**; b) attempted values. Recommendation and ruling: a).
- **R4, Party Hunt Analyser: a) a later decision**; b) now. Recommendation and ruling: a), it needs
  cross-member aggregation that PARTY-PVP-0 deferred.

## 9. Owner questions

None. Every choice above is a reversible architect ruling under owner rule 5905825574.

## 10. Decision test

- **Must decide now:** YES. Control-plane allocation D298 (owner D296).
- **Blocked without it:** the analyser windows; ANALYSER-EMIT-1 hooks in combat and loot.
- **Harder later:** emission points in the combat and item commit paths are cheaper to add before
  those paths settle.
- **Supersede if:** evidence that Tibia keeps analyser sessions on the server; a Party Hunt
  Analyser decision that needs the same facts from other members.
- **Deliberately not decided:** the Party Hunt Analyser; market prices; attacker names; analyser
  layout in the client.

## 11. Before-freeze checklist

1. **Contracts:** none amended.
2. **Serialization:** facts follow the committed outcome in its sync unit; no new write.
3. **Restart:** nothing durable; a resync loses unsent facts only.
4. **Typed references:** CharacterId, race, item type, element.
5. **Wire:** §4, capability 10 `ANALYSER_V1`, state domain 15 `ACTOR_ANALYSER`; no command type.
6. **Split work:** 64 facts per batch, 32 items per fact, 1,024 pending per session.
