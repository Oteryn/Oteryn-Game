# IMBUE-FORGE-0 Imbuements and the Exaltation Forge

- Decision: `IMBUE-FORGE0-IMBUEMENTS-AND-FORGE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, combat and protocol) and protected integration. The gold fees and the new value
  sources are admitted by the owner answers I1 a) and I2 a) (2026-09-30, #162; §19).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner direction of 2026-09-30 (build imbuements and the Exaltation Forge now, full
  Tibia Global parity)
- Builds on: DUR-03 §7, §11, §14-§18, §23, §39 (no durable event per item field tick);
  the composition decision rules 1, 2 and 4; the gold fee decision (D174-D178, §4.2, §4.4) and
  BANK-FEE-0 (coins then bank); BANK-0 (the non-item asset pattern); MARKET-0 §3.1 (default
  state); NPC-0 (SELL of default-state items only, D208); ITEM-USE-0 and RUNE-USE-0 (burn shapes,
  main backpack direct entries); ATTACK-0 (the 60 s in-fight deadline); CHARM-0 (kill-credit
  descendants); GAME-ABILITY-01 §10 and §11; A13 and CHAR-BUILD-1a (#1393, skills are build
  state); owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of IMBUE-FORGE-0, in this PR: MARKET-0 §3.1 (tier and
  imbuements are not default state; tiered wares, §13); the gold fee decision §4.4 (the proposed
  fee variants admitted by owner answer I1, §6 and §10). DUR-03 §15, §17, §18 and §39.3 and the
  composition decision rule 1 are amended by IMBUE-1 and FORGE-1 at allocation, not here (§6, §9).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| IMBUE-CONTENT-1 | content lane | 24 imbuement types × 3 tiers; item slot counts and allowed types; shrines; access predicates (§3) | ITEM-SEM-USE |
| IMBUE-WIRE-1 | impl, protocol review | capability `IMBUE_V1`, the shrine window, apply, clear and tracker messages (§14) | USE-WIRE-V1; ITEM-MOVE-WIRE-1 |
| IMBUE-1 | hard, persistence and economy review | the imbuement item-state table, apply and clear shapes, `ImbueCause`, checkpoints and expiry writes (§4, §6) | GOLD-FEE-2; ITEM-MOVE-2a |
| IMBUE-RT-1 | hard (combat), combat review | the ticking rule, checkpoints, effects through the ability pipeline (§4, §5) | IMBUE-1; ATTACK-1; SPEED-1 |
| IMBUE-SCROLL-1 | hard, persistence review | blank and imbuement scrolls, the etcher (§7) | IMBUE-1; NPC-TRADE-1 |
| FORGE-CONTENT-1 | content lane | classifications, tier caps, prices, dust, sliver and core items (§8, §10) | ITEM-SEM-USE |
| FORGE-1 | hard, persistence and economy review | the tier table, the dust asset and ledger, fusion, transfer and conversions, `ForgeCause` (§8-§10) | GOLD-FEE-2; ITEM-MOVE-2a |
| FORGE-WIRE-1 | impl, protocol review | capability `FORGE_V1`, the forge window, results, history, resource balance (§14) | FORGE-1 |
| FORGE-CREATURE-1 | hard (combat), combat review | influenced and fiendish creatures, dust on kill, sliver loot (§11) | GAME-AI-01 runtime; FORGE-1 |
| TIER-EFFECT-1 | hard (combat), combat review | onslaught, ruse, momentum, transcendence, amplification (§12) | FORGE-1; ATTACK-1; SPELL-D3 |
| MARKET-TIER-1 | hard, persistence and economy review | Market wares keyed by (definition, tier) (§13) | MARKET-1; FORGE-1 |

Later, each with its own decision: materials from the Stash, the house imbuing shrine (a Store
item), exalted cores from the Store, the Soul Pit, the Find Fiend spell, forge history retention
beyond `IMBFORGE0-RL-12`.

## 1. Question

How does a player imbue equipment and raise its tier at the Exaltation Forge, with every material,
fee and resource spent exactly once and the item's new state durable?

## 2. Facts

**PROVEN**

- `game_item_instances` (`0010`) has identity, definition, quantity and lifecycle only; no item
  carries typed state beyond quantity yet. DUR-03 §17 allows `STATE_MUTATION` of a same lifecycle
  under an explicit cause; §16.2 `PRESERVE_INSTANCE` keeps identity; §39 does not require a
  durable event for each high-frequency item field tick.
- The gold fee decision §4.4 (D178): one closed `FeeBurnCause` variant per fee source; "a new fee
  source is admitted only by an amendment ... with its own owner decision". D208 admitted NPC
  trade and travel as value sources on that path. BANK-FEE-0: fees are paid from coins, then the
  (Account, World) bank balance; `F` is bounded by the coins plus `BANK0-RL-01` (999,999,999,999).
- BANK-0 is the pattern for a non-item asset (DUR-03 §18): a balance row, an immutable ledger,
  closed causes, no `CharacterRevision` advance (composition amendment, BANK-0 §7.2).
- MARKET-0 §3.1 and NPC-0 SELL offer only default-state instances. Composition rule 1: an
  item-only transaction does not advance `CharacterRevision`. ATTACK-0: in fight lasts 60 s.
- CHARM-0 §4.1: a kill reward is an independent descendant of a committed death, memoized per
  (death, character), with a single reward principal in the current slice.

**CIPSOFT_OFFICIAL** (the Tibia manual, `docs/reference/tibia-manual/`)

- Imbuing (`characters.md` §5.1.8 a, lines 116-125): up to 3 slots per item; a shrine; the item
  must be unequipped; tiers basic, intricate and powerful, the last two for Premium; gold
  plus materials from backpack or stash; scrolls from blank scrolls (NPC Albinius, loot), usable
  by anyone; 20 hours of equipped hunting time; clearing early for a fee, at a shrine or with an
  etcher. Direct shrine use needs the Temple of the Forgotten Knowledge quest. No success
  chance and no protection option are named.
- Fusion and transfer (lines 127-136): tiers 1-10 by classification; fusion of two items of the
  same name and tier, neither imbued; success: source consumed, target tier +1; failure: target
  unaffected, source tier −1, or consumed at tier 0; up to 2 exalted cores (success chance; keep
  the source tier on failure); convergence fusion (classification 4, same slot, guaranteed, no
  core bonus); transfer: same classification, neither imbued, source tier ≥ 2 consumed, target
  tier 0 gets source − 1; convergence transfer without the −1, at higher gold.
- Tier effects (line 129): weapon extra damage chance, armor full negation chance, helmet
  cooldown reduction chance in combat, legs a 7 s avatar, boots raise the other chances. No
  numbers (line 218); base fusion and transfer chances are not given (line 223).
- Resources come from special monsters, Premium only, found with Find Fiend (line 131); the
  client panel shows forge resources (`interface.md` §3.6.21); the Imbuement Tracker lists
  equipped imbuements (§3.6.24). The Stash refuses altered items such as imbued gear
  (`world.md` line 47). Inspect shows imbuements (`controls.md` line 34).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, `/home/user/opentibiabr/canary`)

- `data/XML/imbuements.xml`: 24 names × 3 tiers (72 records); base prices 5,000, 30,000 and
  200,000; clear cost 15,000; duration 72,000 s (lines 2-4). The `percent` and `protectionPrice`
  fields are loaded (`imbuements.cpp:70-73`) and never used: `Player::onApplyImbuement`
  (`player.cpp:2711-2796`) has no roll and always applies. Categories carry an `agressive` flag
  (lines 7-26; speed, capacity and paralysis deflection are non-aggressive).
- `items.xml` `imbuementslot` gives a slot count and, per allowed type, a maximum tier
  (`items.cpp:124`). An item holds one imbuement per category (`item.cpp:504-544`).
- Ticking (`imbuements.cpp:473-497`): an aggressive imbuement ticks only in fight, outside a
  protection zone and not in a container; a non-aggressive one ticks while equipped.
- Forge (`game.cpp:11178-11209`, `player.cpp:11246-11790`, `config.lua.dist:164-211`): base
  success 50%, a core adds 15%, the tier-loss core keeps the tier with 50%; dust costs fusion 100,
  convergence fusion 130, transfer 100, convergence transfer 160; 60 dust → 3 slivers; 50 slivers
  → 1 core; dust limit 100 to 225, raised by one for (limit − 75) dust. Costs are spent on
  failure. Success bonuses (`tools.cpp:1847`): dust, cores or gold not spent, or the source kept.
- Gold prices per classification and tier (`data/scripts/systems/item_tiers.lua`), up to tens of
  billions for convergence at tier 9-10. Tier chances are quadratic (`item.cpp:559-617`).
- Dust from influenced and fiendish kills: random [stack, 3 × stack] per credited player, capped
  at the limit, each party member with shared experience (`exaltation_forge.lua:44-120`);
  fiendish corpses get 3-7 slivers (`monster.cpp:3415-3432`). NPCs refuse tiered or imbued items
  (`npc.cpp:848`). The Market is keyed by item and tier (`protocolgame.cpp:4429`).

## 3. Imbuing (IMBUE-CONTENT-1, IMBUE-1)

- **Shrine.** A shrine is a world object. USE on it (USE-WIRE-V1, adjacent reach) opens the
  imbuing window under `IMBUE_V1`. The house shrine is a Store item and waits (brief).
- **Catalogue.** 24 types × 3 tiers (Basic, Intricate, Powerful) as Canary lists, with the
  manual as the authority for the tier names and the 20 h. Each record: category, tier, effect
  (§5), materials (definition and count), fee (§6), and the access predicate. Values not in the
  manual carry Canary provenance and `PARITY_PENDING`.
- **Access.** Every direct shrine imbuement, Basic included, needs the Temple of the Forgotten
  Knowledge quest progress the content declares (the manual, §2; Canary's per-type storages).
  Intricate and Powerful also need Premium. The quest, Premium and the item's per-type maximum
  tier are separate checks; any failure is `REQUIREMENT_NOT_MET` before the transaction. Until
  the quest content lands, the quest predicate fails closed. Applying a completed scroll needs
  none of them (§7).
- **Slots.** Each item definition declares 0-3 slots and, per allowed type, a maximum tier. One
  imbuement per category per item. A slot must be empty to imbue (`PARITY_PENDING`: overwrite).
- **Where.** The target item and the materials are main backpack direct entries in the first
  slice (as ITEM-USE-0); the item is therefore unequipped, as the manual requires. Materials from
  the Stash follow the Stash decision (declared difference).
- **Success.** Imbuing always succeeds; there is no protection charm (ruling R2, §18).

## 4. Imbuement state and time (IMBUE-1, IMBUE-RT-1)

### 4.1 Durable state

- **Item state, not Character state.** `game_item_imbuements`: one row per (item instance,
  slot 0-2) with imbuement key and revision, tier, `remaining_ms` (0 to 72,000,000), the applying
  occurrence and TransactionId. No row: an empty slot. Item-only writes: composition rule 1 (no
  `CharacterRevision` advance), rule 2 fence, rule 4 root lock when the item is in inventory.
- **Guard:** `remaining_ms` never increases for one applying occurrence; a row is deleted only by
  an audited clear or expiry.
- Every TRANSFER keeps the row (move, trade, depot, corpse). A transform of an imbued item must
  be `PRESERVE_INSTANCE`; a `REPLACE_INSTANCE` rule is refused while the row exists.

### 4.2 When time runs

- **Aggressive types** tick while the item sits in an equipment slot (not a container), the actor
  is in fight (ATTACK-0 deadline), and the actor's tile is not a protection zone.
- **Non-aggressive types** (speed, capacity, paralysis deflection) tick while equipped.
- Nothing ticks offline or in a container. Effects apply while the item is equipped and has
  time left. This makes the manual's "equipped and hunting" exact (Canary; `PARITY_PENDING`).

### 4.3 Accounting (architect ruling R1)

- The runtime keeps the exact remaining time per ticking slot in memory.
- **Checkpoint:** after each 60 s of ticking (`IMBFORGE0-RL-05`), the runtime writes the exact
  remaining time of every ticking slot of the character in one item-only transaction. It also
  writes it at logout, channel transfer, reconnect loss, and inside every DUR-03 transaction that
  touches an imbued item (a move writes it in the same transaction). Checkpoints carry no DUR-03
  event (§39).
- **Expiry:** at 0 the effects stop at once and one audited `STATE_MUTATION` deletes the row
  under the non-caller cause `ImbueCause::Expire`.
- **Crash:** the node loses at most one checkpoint interval of ticking per slot; the player gets
  that time back. Global's crash rollback returns more. No item or gold is created.
- **Failed checkpoint (fail closed):** when a checkpoint write fails, the character's ticking
  slots stop consuming time and their effects are suspended (§5 reads them as absent) until a
  checkpoint of the unchanged in-memory values commits; the runtime retries it. Committed
  `remaining_ms` is therefore never more than one interval behind consumed time, so a crash
  still returns at most `IMBFORGE0-RL-05`. After a fence loss the last committed value stands.
- A slot reaching 0 whose expiry write has not committed also has its effects suspended; §5.1
  capacity follows the committed state.

## 5. Imbuement effects (IMBUE-RT-1)

Every effect is a GAME-ABILITY-01 input read from the equipped items at the stage it names:

| Category | Stage (GAME-ABILITY-01) | Notes |
|---|---|---|
| Elemental damage (Scorch, Venom, Frost, Electrify, Reap) | §10 contribution on the attacker's weapon damage | converts a share of physical damage to the element |
| Life and mana leech (Vampirism, Void) | §11 descendant of the committed damage result | heal or mana = share of damage dealt; D186 order with charm leech |
| Critical (Strike) | §10 contribution, RNG purpose `imbue_crit` | chance and extra damage |
| Protections (6 elements) | §10 contribution on incoming damage | absorb percent, summed with other absorbs |
| Skill boosts (7) | derived skill read (A13 stays durable) | not a build-state write |
| Speed (Swiftness) | derived speed (SPEED-1) | |
| Capacity (Featherweight) | derived capacity | the item admission check reads it (§5.1) |
| Paralysis deflection (Vibrancy) | CONDITIONS-0 admission of paralyze | chance, RNG purpose `imbue_deflect` |

### 5.1 Capacity in the database

Composition rule 5 checks capacity inside the PostgreSQL transaction. That check counts the
Featherweight rows of the equipped backpack that exist with committed `remaining_ms` above 0.
The runtime's derived capacity uses the same committed state: a Featherweight slot whose effect
is suspended (§4.3: failed checkpoint, or at 0 before its expiry write commits) grants no
capacity in the runtime, and an admission that needs that capacity is refused until the
checkpoint or the expiry write commits. The runtime never grants more capacity than the database
counts. Items already carried stay carried when capacity falls (Global).

## 6. Imbuing in DUR-03 terms (IMBUE-1)

| Operation | Lines | Cause |
|---|---|---|
| Apply | BURN of each material count (§15, §11.1 or §11.5); `STATE_MUTATION` of the target (row insert); fee BURN (coins, then bank) | `ImbueCause::Apply {item, slot, imbuement, occurrence}`; `FeeBurnCause::Imbue` |
| Clear | `STATE_MUTATION` (row delete); fee BURN | `ImbueCause::Clear`; `FeeBurnCause::ImbueClear` |
| Checkpoint | `STATE_MUTATION` of `remaining_ms` down, no event | `ImbueCause::Checkpoint` |
| Expire | `STATE_MUTATION` (row delete), audited | `ImbueCause::Expire` |

- One transaction per operation, one TransactionId, one receipt keyed by (occurrence, character).
  A replay returns the first outcome. Nothing is spent before the state is written.
- **DUR-03 amendment (IMBUE-1):** §15 admits `ImbueCause` as a burn sink; §39.3 admits the four
  shapes (at most 3 material stacks per record, `DUR03-RL-01-IMBUE`). This decision states the
  need; IMBUE-1 writes the text with its migration.
- **Fees (I1):** `FeeBurnCause::Imbue {item, slot, imbuement, occurrence}`,
  `ImbueClear {item, slot, occurrence}` and `ImbueScroll {scroll, imbuement, occurrence}` are
  admitted by owner answer I1 a) at Global prices (D178). Amounts are content (Canary 5,000, 30,000, 200,000; clear
  15,000; `PARITY_PENDING`).

## 7. Scrolls and the etcher (IMBUE-SCROLL-1)

- A blank scroll (NPC trade, D208; or loot) becomes an imbuement scroll at a shrine: BURN of the
  materials, fee BURN, and a `REPLACE_INSTANCE` TRANSFORM of one blank scroll unit into one scroll
  of that imbuement (`ImbueCause::ScrollCreate`). Scrolls are stackable item definitions.
- **Producer eligibility (architect ruling, the manual §2: only worthy Premium characters
  produce scrolls):** `ScrollCreate` of any tier needs Premium and the §3 quest predicate,
  checked before the transaction; a failure is `REQUIREMENT_NOT_MET` and writes nothing. Holding
  a blank scroll from loot or trade grants no eligibility.
- Applying a scroll is a use-with on the target item: BURN of one scroll unit and the row insert
  (`ImbueCause::ScrollApply`), with no fee and no access predicate beyond the item's slots:
  applying a completed scroll stays unrestricted (no quest, no Premium).
- The etcher clears without a shrine; its charges are `PARITY_PENDING` until IMBUE-SCROLL-1.

## 8. Item tier (FORGE-1)

- **Durable attribute.** `game_item_tiers`: one row per item instance with `tier` 1-10. No row
  is tier 0. The row belongs to the ItemInstance and every TRANSFER keeps it (§4.1 rules).
- The definition declares its classification (0-4) and maximum tier (Canary: 1, 2, 3 and 10 for
  classes 1-4). Tiered items are not stackable. A tier change is a `STATE_MUTATION` of the same
  lifecycle (`PRESERVE_INSTANCE`), never a new item.

## 9. Forge resources (FORGE-1)

- **Dust is a Character balance**, the second DUR-03 §18 non-item asset after bank gold:
  `game_character_forge_dust` (CharacterId, `balance` 0 to `dust_limit`, `dust_limit` 100-225,
  `last_entry_id`) with an immutable ledger (`GAIN`, `SPEND`, `CONVERT`, `LIMIT_RAISE`), as
  BANK-0 §3. Character scope, not Account: Global keeps dust per character.
- **Slivers and exalted cores are items**: stackable, marketable and tradeable per content.
- **Dust limit:** raised by one for (limit − 75) dust, up to 225 (Canary; `PARITY_PENDING`). A
  gain above the limit is lost, as in Global; the lost part is recorded in the ledger entry.
- **Composition amendment (FORGE-1):** rule 1 also covers dust ledger entries and balance rows of
  the acting character, as BANK-0 §7.2 did for bank rows. The lock order is rule 4's, then the
  dust row. This decision states the need; FORGE-1 writes it.

## 10. Forge operations (FORGE-1)

The forge is a world object; USE opens the forge window under `FORGE_V1`. Both items are main
backpack direct entries, neither imbued. The roll is drawn from the simulation RNG under purpose
`forge_fusion`, keyed by the operation occurrence and a server seed the client cannot derive; the
outcome is stored in the receipt, so a replay or a new CommandId never rerolls a known outcome.

- **Revision binding (SIM-DETERMINISM-01 §5):** when the forge occurrence is reserved it binds
  its behavior-affecting revision set: the content revision (classification, tier caps, prices,
  dust costs, bonus rates), the ruleset/formula revision (chances, outcome rules) and the
  `SimulationDeterminismProfileRevision`. The set is part of the request binding and is stored in
  the receipt. A retry of the same occurrence, including after a known non-committed abort,
  evaluates only under that bound set; if the active set differs, the retry is refused
  (`REVISION_CHANGED`) and the player must start a new occurrence. The same occurrence is never
  re-evaluated under a newer revision.
- **`REVISION_CHANGED` is terminal (architect ruling, fail-closed).** The refusal is persisted as
  the occurrence's terminal receipt, keyed by (occurrence, character) as every receipt (§6), in a
  receipt-only transaction: the bound set and the rejection, with no item, dust, core, fee or
  state line and no history row. Every later replay of that occurrence, under the same or a new
  CommandId, returns the same rejection, even when the active set later equals the bound set
  again (for example after a rollout rollback). If the terminal write's outcome is unknown, the
  occurrence stays refused until reconciliation reads the receipt (`IMBFORGE0-RL-11`); it is
  never evaluated meanwhile.

| Operation | Lines (all in one transaction) | Outcomes |
|---|---|---|
| Fusion (A, B same definition and tier t) | A `STATE_MUTATION`; B BURN or `STATE_MUTATION`; dust BURN value line; cores BURN (0-2); fee BURN | success: A → t+1, B burned; failure: A unchanged, B → t−1 or burned at t = 0; the tier-loss core keeps B at t with 50% |
| Convergence fusion (class 4, same slot and tier) | as fusion | always success; no core bonus |
| Transfer (donor t ≥ 2, receiver tier 0, same class) | donor BURN; receiver `STATE_MUTATION` to t−1; dust BURN; cores BURN (per tier); fee BURN | always success |
| Convergence transfer (class 4) | as transfer, receiver to t | always success |
| Dust → slivers | dust `CONVERSION` debit 60; sliver `CONVERSION` output 3 | a compatible stack or a new entry; no room: refused |
| Slivers → core | sliver `CONVERSION` input 50; core `CONVERSION` output 1 | as above |
| Dust limit | dust BURN value line; `STATE_MUTATION` of `dust_limit` | +1 |

- **Cause:** closed `ForgeCause {Fusion | ConvergenceFusion | Transfer | ConvergenceTransfer |
  DustToSlivers | SliversToCore | DustLimit, occurrence}`; the fee variants
  `FeeBurnCause::ForgeFusion` and `ForgeTransfer {kind, items, occurrence}` are admitted by I1 a).
- **Costs are spent on failure**, as in Global. The success bonuses (dust, cores or gold not
  spent; B kept at t−1, t or t+1; A to t+2 within the class cap) are content with Canary rates,
  `PARITY_PENDING`; a bonus only omits or changes a line of the same shape.
- Every refusal (resources, class, tier cap, imbued item, no room) comes before the transaction
  and writes nothing. Fees above the coins go to the bank (BANK-FEE-0).
- **DUR-03 amendment (FORGE-1):** §15 admits `ForgeCause` as a sink; §18 names forge dust; §39.3
  admits these shapes (`DUR03-RL-01-FORGE`, `DUR03-RL-03-FORGE`) and the receipt-only
  `REVISION_CHANGED` terminal record. FORGE-1 writes the text.
- **History:** each operation writes a forge history row (`IMBFORGE0-RL-12`).

## 11. Dust and sliver sources (FORGE-CREATURE-1, admitted by I2 a)

- **Influenced creatures:** ordinary creatures picked at random per channel receive 1-5 forge
  stacks (more health and damage, a visible marker). **Fiendish creatures:** at most
  `IMBFORGE0-RL-09` per channel, each for a bounded time. Both are runtime creature state
  (GAME-AI-01), shown through VIS-2; a restart clears them.
- **Dust:** a MINT value line `ForgeDustMint {death, character}`, an independent descendant of a
  committed death (the CHARM-0 §4.1 pattern), memoized per (death, character), capped at the
  limit. Amount drawn in [stack, 3 × stack] under RNG purpose `forge_dust`. Party members with
  shared experience each get their own draw when PARTY-PVP-0's shared experience lands; until
  then the single reward principal (declared difference).
- **Slivers:** 3-7 in a fiendish corpse, by the D3 loot MINT path with a fiendish provenance.
  Values are Canary's, `PARITY_PENDING`. Premium only (manual).

## 12. Tier effects (TIER-EFFECT-1)

| Slot | Effect | Stage |
|---|---|---|
| Weapon | Onslaught: chance of extra damage (60%, `PARITY_PENDING`) | §10 contribution, RNG `tier_onslaught` |
| Armor | Ruse: chance to negate a hit | §10 contribution on incoming damage, RNG `tier_ruse` |
| Helmet | Momentum: chance to cut cooldowns by 2 s in fight, outside a protection zone | §8 periodic occurrence every 2 s (Canary `player.cpp:11095`), RNG `tier_momentum` |
| Legs | Transcendence: 7 s avatar on an offensive use | §11 descendant, a CONDITIONS-0 condition |
| Boots | Amplification: raises the other chances | derived multiplier read by the stages above |

The chance per tier is content (Canary quadratic; `PARITY_PENDING`). Only equipped items count.

## 13. Market, trade, NPCs, Stash (amends MARKET-0 §3.1)

- **Default state** now also means tier 0 and no imbuement row.
- **Imbued items** are never Market wares, sold to an NPC or stowed; trade, drop, depot and
  death keep their state (TRANSFER).
- **Tiered items** are Market wares in Global. MARKET-TIER-1 keys a ware by (definition, tier);
  until then a tiered instance is refused by the Market (declared difference). NPCs refuse them
  (Canary). Stash: `PARITY_PENDING`, decided with the Stash.
- Inspect and the trade window show tier and imbuements (IMBUE-WIRE-1, FORGE-WIRE-1).

## 14. Wire (IMBUE-WIRE-1, FORGE-WIRE-1)

- **Capabilities `IMBUE_V1` and `FORGE_V1`**, each requiring `ITEM_VIEW_MOVE_V1`; numbers are
  reserved on #162 at allocation. Without one, its shrine or forge answers `NOTHING_TO_USE`.
- IMBUE: open (item handle), apply (slot, imbuement index), clear (slot), scroll create; results
  `OK`, `REQUIREMENT_NOT_MET`, `MATERIALS_MISSING`, `INSUFFICIENT_FUNDS`, `SLOT_OCCUPIED`,
  `NOT_ALLOWED`; the tracker (equipped imbuements and remaining time) as a state domain entry.
- FORGE: fusion, transfer (with convergence and core flags), conversion kind; results with
  success and bonus, or `REVISION_CHANGED` (§10, terminal for the occurrence); history paged; the dust balance and limit in the resource balance.
- Item views gain tier and the imbuement summary. Each wire child measures its payloads.

## 15. Rows (registered by each child before implementation)

| Row | Value |
|---|---|
| `DUR03-RL-01-IMBUE` touched items | 1 target + 3 material stacks + fee inputs and change (GOLD-FEE-2) |
| `DUR03-RL-01-FORGE` touched items | 2 targets + 2 core or sliver stacks + fee inputs and change |
| `DUR03-RL-03-FORGE` value lines | at most 3 (dust, bank fee part, dust limit) |
| `IMBFORGE0-RL-01` imbuement slots per item | 3 |
| `IMBFORGE0-RL-02` imbuement duration | 72,000,000 ms (20 h) |
| `IMBFORGE0-RL-03` shrine or forge operations in flight per actor | 1 |
| `IMBFORGE0-RL-04` imbuing and forge commits per channel per second, database p99 | measured before IMBUE-1 and FORGE-1 ship |
| `IMBFORGE0-RL-05` checkpoint interval of ticking time | 60 s |
| `IMBFORGE0-RL-06` checkpoint writes per channel per second at peak | measured by IMBUE-RT-1 |
| `IMBFORGE0-RL-07` item tier | 0-10, capped by class |
| `IMBFORGE0-RL-08` dust limit | 100-225 |
| `IMBFORGE0-RL-09` fiendish creatures per channel | content, `PARITY_PENDING` (Canary 4) |
| `IMBFORGE0-RL-10` influenced creatures per channel | content, `PARITY_PENDING` (Canary 300) |
| `IMBFORGE0-RL-11` operation ambiguity bound | 2,000 ms (as `ITEMUSE0-RL-04`) |
| `IMBFORGE0-RL-12` forge history rows per character | 1,000, oldest dropped |

## 16. Rejected options

- **Imbuements as Character state.** They travel with the item through trade and death.
- **A lease debited ahead, or a write per tick** (R1).
- **Keeping a 90/70/50% success chance with a protection charm.** Not current Global (R2).
- **A new ItemInstance for a fused or transferred item.** Tier is state; `PRESERVE_INSTANCE`.
- **Dust as an item, or per Account.** Global keeps it as a per-character capped balance.
- **A forge roll from the client's CommandId alone.** A client could search ids for good rolls.
- **Refusing tiered items on the Market forever.** Global lists them by tier.

## 17. Owner-rule applications

**Global parity kept:** 24 types × 3 tiers; up to 3 slots; 20 h of equipped hunting time; one
imbuement per category; clearing for a fee; scrolls; imbued items refused by the Market, NPCs and
the Stash; fusion, convergence and transfer with Global outcomes; costs spent on failure; dust as
a capped per-character balance; slivers and cores as items; influenced and fiendish creatures;
five tier effects.

**Declared differences:**
- Imbuing and forging use main backpack direct entries; no Stash materials yet.
- After a node crash, up to 60 s of ticking per imbuement returns (R1).
- Tiered wares wait for MARKET-TIER-1; party dust waits for shared experience.
- Imbuement overwrite, the etcher's charges and all Canary numbers are `PARITY_PENDING`.

## 18. Architect rulings (Global-parity applications, owner rule 5905825574)

**R1. Imbuement time accounting.** a) Checkpoint every 60 s of ticking and at every boundary; a
node crash returns at most 60 s (recommended: no refund path, the durable value only falls);
b) debit a lease ahead: no return after a crash, but a refund or a loss at each unequip; c) write
each second: load for no gain. **Ruled a).**

**R2. Imbuing success.** a) Always succeeds, no protection charm (recommended: the manual names
no chance, Canary never rolls); b) keep 90/70/50% with a paid protection: an unsourced failure
path and one more fee. **Ruled a).** New evidence reopens it by amendment.

## 19. Owner questions

**I1. Admit the imbuing and forge gold fees? (D178)** Context: every new fee source needs an
owner decision; these are imbuing (5,000 / 30,000 / 200,000 by tier), clearing (15,000), scroll
creation, and forge fusion and transfer (25,000 up to tens of billions by class and tier), paid
from coins then the bank. a) Yes, all, at Global prices (recommended); b) imbuing and clearing
only, forge later; c) none now: imbuing and the forge wait.
Owner answer (2026-09-30, #162): a — Yes, all imbuing and forge gold fees, at Global prices.

**I2. Admit the forge value sources? (D208)** Context: new value enters the game: forge dust from
influenced and fiendish kills (a Character balance), sliver loot from fiendish creatures, and
the dust → sliver → core conversions. a) Yes, as Global (recommended); b) dust and conversions
only, no sliver loot (cores come only from dust); c) none now: the forge waits.
Owner answer (2026-09-30, #162): a — Yes, all of them as in Global: dust, slivers and the
conversions.

## 20. Decision test

- **Must decide now:** YES. The owner asked for imbuements and the forge now; no item carries
  typed state yet, and IMBUE-1 and FORGE-1 need the shapes and causes.
- **Minimum sufficient:** two item-state tables, one non-item asset, three closed causes, the fee
  variants (admitted by I1), two capabilities; the fee, bank, loot, kill-credit, ability and ITEM-USE-0
  paths are reused.
- **Superseding evidence:** official chances, prices, dust amounts; overwrite; the etcher.
- **Deliberately not decided:** the later items of the brief.

## 21. Before-freeze checklist

1. **Contract amendments:** MARKET-0 §3.1 and the gold fee §4.4 here; DUR-03 §15, §17, §18,
   §39.3 and composition rule 1 by IMBUE-1 and FORGE-1. Capability numbers at allocation.
2. **Serialization:** one shrine or forge operation per actor; one transaction per operation;
   rule 2 fence; rule 4 root lock, then the dust row; replay by occurrence under its bound
   revision set, `REVISION_CHANGED` persisted as a terminal receipt (§10); a failed imbuement checkpoint suspends ticking and effects (§4.3).
3. **Restart:** tier, imbuements and dust are durable; ticking, influenced and fiendish state
   are runtime; at most one checkpoint of time returns.
4. **Typed references and wire:** item handles, definition and imbuement keys, `ProductionKey`
   in audit; §14. **Split work:** 3 material stacks per imbuement, 2 targets per forge operation.
