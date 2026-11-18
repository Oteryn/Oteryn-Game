# qcx82 Dreamer's Challenge Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Global evidence is the cached [quest page, curid 12354 oldid 869211](https://tibia.fandom.com/?curid=12354&oldid=869211) and [spoiler, curid 12355 oldid 891457](https://tibia.fandom.com/?curid=12355&oldid=891457). Crystal `201189f871d23597b82caf88fd611454a2ddefb8` source extracts are retained in the [quest evidence](../evidence/quests-4-20261102/dreamers_challenge_quest.json), the [import inventory](../../../tools/content-schema/quest-authoring/batches/quests4/dreamers_challenge_quest.json) and the M07 references under `docs/reference/content-prep`.

| Global | Import and native dispatch |
| --- | --- |
| Premium, no minimum level, recommended 40 | Definition matches Premium and minimum 0. The carrot callback consumes one i3595 after Premium and custody checks. Its production placement UID remains unverified. |
| Rope, shovel, pick, carrot, Fire Bug, two Death Rings, Life Ring, water and Fire Field | Fire Bug i5467 opens the entrance and the Death Ring callback consumes worn i6299 and removes the tree barrier. The retained entry references do not establish the production UID2241/2244 bindings or the complete shovel and pick approach. |
| Four-color matching and right exit | The shared grid rotations, completion check and scramble are absent in native quest bindings. |
| SE, SW, NE, NW levers, 12 damaging pillars and success/fallback exit | The four native controls remove and restore their own wall segments independently. The ordered solution, pillar encounter and gated exit are absent from this quest's native bindings. Donor wall timers are 7s/8s. |
| Shared 6×6 tic-tac-toe, optional Boots hint and Lost Heroes book | The book i2821 ×1 at UID6196 has the six exact names. Board validation, shared gate and NPC hint are absent from the native quest path. |
| Life Ring north, Death Ring south, water west, fire east, switch activates well, center exit | The repository has the six stateless Hugo exit and maze selectors, but lacks the linked offering/well/portal cycle and full environmental path. |
| Lucid Dreamer on arriving at A Ghostly Guardian | The arrival callbacks teleport without an achievement or progress write. |
| Four boxes with one bag, vial of wine, brown bread and certificate each | UID6197–6200 are alternate placements for one claim. Bag i2853 contains i2874 ×1 (wine subtype 2), i3602 ×1 and i6498 ×1. Crystal chest.lua:2131–2161 agrees. Tests qualify the grant against independent wiki expectations. |
| Quest rewards and XP | The cache lists no XP reward. Crystal additionally provides backpack UID6195 (i2854 containing one each of i2816, i3285, i3352 and i3558) and UID6201 (i3028 ×3). Their status in Global is open. |
| Exclusive faction, faction entry achievement and access | Reciprocal faction and Premium guards exist in the separately keyed faction callbacks and offers. They require the Dreamer's s4 parent flag, which the current maze exits do not set. The disabled Dreamer's recipe instead selects Nightmare alone and skips the narrative path through the guardian. |
| Rank advancement costs 500/1000/1500 i6499, with early 1500 accepted | The disabled Dreamer's recipe uses 100. Live faction callbacks charge 500/1000 for the first two ranks and lack the third and prepaid branch. |
| Nightmare Shield i6390 or Necromancer Shield i6432 at the second advancement | The faction rewards grant the leader document instead of also granting the shield. |
| Follower, Officer, Leader documents and full-outfit achievements | Faction callbacks support document use but grant Nightmare Walker/Skull and Bones at the leader promotion, before all three scrolls are used. Nightmare Officer/Leader use masks 1/2 whereas the spoiler images identify 2/1. The cache describes use eligibility and leaves automatic document handouts unstated. |
| Lord Protector or Dread Lord, diamond document crafting and nine pearl teleports | The final rank and achievement, rank-gated altar crafting, and three tiered groups of pearl destinations lack quest bindings. |
 
## Blockers and open facts

`BLOCKER Mission 2 colored grid shared permutation, validation and scramble data-global/scripts/quests/dreamers_challenge_quest/movement_riddle_teleport.lua:11`

`BLOCKER Mission 3 wall exit shared obstruction predicate and success/fallback STEP routing data-global/scripts/quests/dreamers_challenge_quest/movement_wall_teleport.lua:1`

`BLOCKER Mission 4 shared tic-tac-toe reset coupled board, pillars and portal mutation data-global/scripts/quests/dreamers_challenge_quest/actions_tic_tac_teleport.lua:1`

`BLOCKER Mission 5 dream room mixed ring, fluid and field activation with shared portal reset data-global/scripts/quests/dreamers_challenge_quest/movement_sacrifice_teleport.lua:8`

`BLOCKER Mission 5 Lucid Dreamer generic STEP-bound achievement receipt data-global/scripts/quests/dreamers_challenge_quest/movement_teleport.lua:6`

`BLOCKER Knightwatch Tower atomic pearl debit with either-faction rank guard data-global/scripts/quests/dreamers_challenge_quest/movement_tower.lua:2`

**OPEN-FACTS** — Global numeric item IDs, exact damage/timers, tile and UID/action mappings including portal squares, whether the extra Crystal chests are present, document handout timing, pearl debit vs. possession and exact destination tiles. The spoiler describes order, eligibility and destination names. It leaves those numeric or implementation details unstated.

**PARTIAL** — Production entry selector coverage, native color/wall/tic-tac-toe/dream-room progression, Lucid Dreamer, parent flag and faction connection, faction reward differences under separately owned quest keys, final rank, altar crafting and tower access. No source-only claim or callback status was promoted.
