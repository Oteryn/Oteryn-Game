# qcx82 Dreamer's Challenge Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Global evidence is the cached [quest page, curid 12354 oldid 869211](https://tibia.fandom.com/?curid=12354&oldid=869211) and [spoiler, curid 12355 oldid 891457](https://tibia.fandom.com/?curid=12355&oldid=891457). Crystal `201189f871d23597b82caf88fd611454a2ddefb8` source extracts are retained in the [quest evidence](../evidence/quests-4-20261102/dreamers_challenge_quest.json), the [import inventory](../../../tools/content-schema/quest-authoring/batches/quests4/dreamers_challenge_quest.json) and the M07 references under `docs/reference/content-prep`.

| Global | Import and native dispatch |
| --- | --- |
| Premium, no minimum level, recommended 40 | Definition matches Premium and minimum 0. The carrot callback consumes one i3595 after Premium and custody checks. Its production placement UID remains unverified. The held registry directory contains no Dreamer's Challenge file. |
| Rope, shovel, pick, carrot, Fire Bug, two Death Rings, one Life Ring, 1–3 water containers and 1–2 Fire Field runes | Fire Bug i5467 opens the entrance and the Death Ring callback consumes inactive i6299 and removes the tree barrier. The retained entry references do not establish the production UID2241/2244 bindings or the complete shovel and pick approach. |
| Four-color matching and right exit | The shared grid rotations, completion check and scramble are absent in native quest bindings. |
| SE, SW, NE, NW levers, 12 damaging pillars and success/fallback exit | Baseline controls removed their wall segments in any order. The authored controls now require each earlier lever to be active, in the Global SE, SW, NE, NW order. The donor wall/lever timers are 7s/8s. The pillar encounter and gated exit remain unbound. |
| Shared 6×6 tic-tac-toe, optional Boots hint and Lost Heroes book | The book i2821 ×1 at UID6196 has the six exact names. Board validation, shared gate and NPC hint are absent from the native quest path. Chest USE persists the book text. The coverage registry records carried-book READ as unavailable. |
| Life Ring north, Death Ring south, water west, fire east, switch activates well, center exit | The repository has one Hugo exit and six stateless maze selectors, but lacks the linked offering/well/portal cycle. Portal protection-zone handling, harmless dream monsters and underwater drowning remain unqualified on this path. |
| Lucid Dreamer on arriving at A Ghostly Guardian | The arrival callbacks teleport without an achievement or progress write. |
| Four boxes with one bag, vial of wine, brown bread and certificate each | UID6197–6200 are alternate placements for one claim. Bag i2853 contains i2874 ×1 (wine subtype 2), i3602 ×1 and i6498 ×1. Crystal chest.lua:2131–2161 agrees. Tests qualify the grant against independent wiki expectations. |
| Quest rewards and XP | The cache lists no XP reward. Crystal additionally provides backpack UID6195 (i2854 containing one each of i2816, i3285, i3352 and i3558) and UID6201 (i3028 ×3). Both placements remain unbound pending Global evidence. |
| Exclusive faction, faction entry achievement and access | Reciprocal faction and Premium guards exist in the separately keyed faction callbacks and offers. They require the Dreamer's s4 parent flag, which the current maze exits do not set. The disabled Dreamer's recipe instead selects Nightmare alone and skips the narrative path through the guardian. |
| Rank advancement costs 500/1000/1500 i6499, with early 1500 accepted | The disabled Dreamer's recipe uses 100. Live faction callbacks charge 500/1000 for the first two ranks and lack the third and prepaid branch. |
| Nightmare Shield i6390 or Necromancer Shield i6432 at the second advancement | The faction rewards grant the leader document instead of also granting the shield. |
| Follower, Officer, Leader documents and full-outfit achievements | Faction callbacks support document use but grant Nightmare Walker/Skull and Bones at the leader promotion, before all three scrolls are used. Nightmare Officer/Leader use masks 1/2 whereas the spoiler images identify 2/1. The cache describes use eligibility and leaves automatic document handouts unstated. |
| Lord Protector or Dread Lord, diamond document crafting and nine pearl teleports | The final rank achievement and crafting lack quest bindings. Global calls for one small diamond at the north, west or south altar for Follower, Officer or Leader. The unbound pearl destinations are Carlin/Thais/Dream Realm at rank 1, Ab'Dendriel/Fibula/Kazordoon at rank 2 and Darashia/Ankrahmun/Mintwallin at rank 4. |
 
## Blockers and open facts

`BLOCKER Mission 2 colored grid shared permutation, validation and scramble data-global/scripts/quests/dreamers_challenge_quest/movement_riddle_teleport.lua:11`

`BLOCKER Mission 3 wall exit shared obstruction predicate and success/fallback STEP routing data-global/scripts/quests/dreamers_challenge_quest/movement_wall_teleport.lua:1`

`BLOCKER Mission 4 shared tic-tac-toe reset coupled board, pillars and portal mutation data-global/scripts/quests/dreamers_challenge_quest/actions_tic_tac_teleport.lua:1`

`BLOCKER Mission 5 dream room mixed ring, fluid and field activation with shared portal reset data-global/scripts/quests/dreamers_challenge_quest/movement_sacrifice_teleport.lua:8`

`BLOCKER Mission 5 Lucid Dreamer generic STEP-bound achievement receipt data-global/scripts/quests/dreamers_challenge_quest/movement_teleport.lua:6`

`BLOCKER Knightwatch Tower atomic pearl debit with either-faction rank guard data-global/scripts/quests/dreamers_challenge_quest/movement_tower.lua:2`

`BLOCKER Mission 4 Lost Heroes book carried-book READ/TextWindow data-global/startup/tables/chest.lua:2124`

**OPEN-FACTS** — Global numeric item IDs, exact damage/timers, tile and UID/action mappings including portal squares, whether the extra Crystal chests are present, guardian chest repeat and grouping semantics, document handout timing, pearl debit vs. possession and exact destination tiles. The spoiler describes order, eligibility and destination names. It leaves those numeric or implementation details unstated.

**PARTIAL** — Production entry selector coverage, native color/wall/tic-tac-toe/dream-room progression, Lucid Dreamer, parent flag and faction connection, faction reward differences under separately owned quest keys, final rank, altar crafting and tower access. No source-only claim or callback status was promoted.

## Regression evidence

- Baseline production data at `871c1dec7d9963420d455210e4cddc0c9df80d26` with the new tests and local pgsplit files. PostgreSQL 18.6 returned `Committed` on the first northwest USE where the regression expects `Rejected`. The chest case passed in 149.508 s and the baseline run had one pass and one failure.
- On the first corrected run, the refusal assertion passed and the next USE hit dispatch exhaustion. The session logs recorded one exhaustion and the session encoded it as `Rejected`. The fixture now advances the owner clock between fresh requests, as it already did for the reset assertions.
- Pin refresh reproduced the existing World digest. Pin verification passed using a temporary output directory, which this checkout's CLI requires. All 44 quest-authoring check commands, workspace formatting and dispatch library Clippy with `-D warnings` passed.
- PG via shard (pgsplit build files applied on `1ffb3bc880fdbe231c66071446d8b22fd89ece30`). PostgreSQL 18.6 passed both cases in 173.78 s. The guardian bag and book case took 152.105 s (UID6196 24.318 s, UID6197 31.597 s, UID6198 19.420 s, UID6199 31.824 s, UID6200 26.293 s). The northwest lever case took 21.667 s. It uses real dispatch for the initial refusal and the successful pull with the three prerequisite appearances seeded, then checks both reset deadlines.
- With `OTERYN_TEST_POSTGRES_ADMIN_URL` unset, both cases returned `Ok(())` in 0.00 s.
