# qcx63 Adventurers Guild compatibility audit

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Facts commit `460e1d8c2c0e88c438ae5e54910db00822dc0129`. Crystal source revision `201189f871d23597b82caf88fd611454a2ddefb8`.

## The Lost Brother

- Global lists premium, no minimum level and level 200 as a recommendation. Its sequence is Tarun's `mission` offer, `yes` acceptance, the palace cracks, discovery south of the skeleton and the return to Tarun with `mission`. The production import already had the discovery step at (32959, 32674, 4) and the corresponding journal track. Tarun's dialogue was source-only. The generated NPC records now admit the offer, refusal, reminder, acceptance and return. Acceptance and reward require premium. [Quest](https://tibia.fandom.com/?curid=75165&oldid=933445) · [Spoiler](https://tibia.fandom.com/?curid=75304&oldid=749371)
- Tarun's return now awards one Red Gem (3039, Crystal `data-global/npc/tarun.lua:81`) and 3,000 XP in the same transition to stage 3. The existing Tarun offers had no Lost Brother requirement. The generated shop gates those offers on stage 3. Global lists no outfit or separate access grant beyond trade. The return leaves the shared Adventurers Guild quest in progress. [Quest](https://tibia.fandom.com/?curid=75165&oldid=933445) · [Spoiler](https://tibia.fandom.com/?curid=75304&oldid=749371)
- Global awards Lost Palace Raider only when Asura Palace was previously completed. Crystal's return omits the achievement. The existing reward selector requires a unique matching callback before checking completed-quest requirements in the writer, so an unconditional fallback and a completion-gated award would collide. [Spoiler](https://tibia.fandom.com/?curid=75304&oldid=749371)
- The quest page updates the spoiler's Massive Fire Elementals to Greater Fire Elementals. The facts do not identify the exact placement records or the cracks' numeric selectors. [Quest](https://tibia.fandom.com/?curid=75165&oldid=933445) · [Spoiler](https://tibia.fandom.com/?curid=75304&oldid=749371)

## The Great Dragon Hunt

- Global lists premium and no minimum level, recommends 40, and starts the quest automatically in any of three passages. The import has journal tracks and source transitions without active callbacks for this title. Crystal starts the guild log and resets the kill counter from the skeleton action. Global starts before the skeleton and gives only one Red Dragon Scale across all three skeletons (5882, Crystal `actions_warrior_skeleton.lua:6`, action 50806 at line 21). [Quest](https://tibia.fandom.com/?curid=75167&oldid=847146) · [Spoiler](https://tibia.fandom.com/?curid=75348&oldid=1211828)
- Global counts 50 Dragons and Dragon Lords combined across the Kha'zeel lairs. Crystal checks seven rectangles and uses party death credit in `creaturescripts_the_great_dragon_hunt.lua:2-24`. No active per-quest death credit is wired in the import. Global offers simultaneous progress through both species and immediate reset after claiming. [Spoiler](https://tibia.fandom.com/?curid=75348&oldid=1211828)
- No active hoard reward claim was found. Global and Crystal agree on the six random bundles: 5 Platinum Coins, one each of Small Ruby/Emerald/Sapphire (donor IDs 3029/3032/3030 as a set), one Life Ring (3089), one Red Gem (3039), 10 Strong Mana Potions (237) or 8 Strong Health Potions (236). The counter resets to zero and the tenth claim grants Hoard of the Dragon. Crystal `actions_treasure.lua:1-14,19-40,46` uses action 50808. Global lists no XP, outfit or additional access. [Quest](https://tibia.fandom.com/?curid=75167&oldid=847146) · [Spoiler](https://tibia.fandom.com/?curid=75348&oldid=1211828)

## Remaining work

- BLOCKER Lost Brother achievement optional achievement in the same hand-in based on prior quest completion `data-global/npc/tarun.lua:76-84`
- BLOCKER Great Dragon Hunt hoard random multi-item bundle in a repeatable placed callback `data-global/scripts/quests/the_great_dragon_hunt_quest/actions_treasure.lua:19-38`
- OPEN-FACTS Great Dragon Hunt exact automatic-start tiles, exhaustive eligible lair boundaries, party-credit rules, exact hoard placements and bundle probabilities. Lost Brother cracks selector and exact elemental spawn records. The facts name rewards without IDs, so the listed IDs come from Crystal.
- PARTIAL Great Dragon Hunt automatic start, once-only skeleton grant, lair death credit, hoard delivery and tenth-claim achievement. Lost Brother conditional achievement and palace route verification.
