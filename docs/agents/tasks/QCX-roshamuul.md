# QCX71 Roshamuul Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Scope is `oteryn:quest.roshamuul_quest`.

- Global requirements and rewards. [TibiaWiki Roshamuul Quest, curid 66027, oldid 846322](https://tibia.fandom.com/?curid=66027&oldid=846322).
- Global objectives, access, quantities and NPC transcripts. [TibiaWiki Roshamuul Quest/Spoiler, curid 66033, oldid 1104388](https://tibia.fandom.com/?curid=66033&oldid=1104388).
- Donor. Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. The retained quest shard cites earlier Canary/Crystal extractions. The pinned Crystal tree supplies lower-Roshamuul scripts and NPC files. [Crystal Roshamuul scripts](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/roshamuul_quest).
- Item identity cross-check. The retained native client Cyclopedia capture names the items at IDs 20047–20050, 20053–20057, 20062, 20138, 20139, 20141, 20169, 20170, and 20210. [Capture](../../reference/content-prep/item-cyclopedia-capture-1/items.jsonl).

## Differences

| Part | Global facts | Base and donor |
| --- | --- | --- |
| Entry and journal | Travel from Thais. Premium, level minimum 0, recommended 200, and quest log entries. | The minimum and Premium requirement match. The definition omits the recommended level. Captain Bluebear's active outbound service requires Premium; Chelop's active return does not. The definition is `script_only` with `shown_in_quest_log=false`; the native state has no missions. |
| World access | Server save advances bridge, wall and first Gaz'haragoth defeat stages. Lower Roshamuul → Guzzlemaw Valley → Upper Roshamuul → city NPCs. Valley Sights and Dark Grounds are separate temporary unlocks. Dark Grounds requires 20 Soothing Bad Dreams completions. The quest also unlocks Umbral creation/refinement server-wide. | No live quest-stage gates. The disabled chosen recipe proposes a once-only personal Upper Roshamuul route with five mortar sites. Crystal has lower actions and a Gaz raid, with no matching permanent stage sequence in those scripts. |
| Gravel and chalk | Pick first, then collect 10 Fine Gravel. Failed mining spawns a Frazzlemaw; rock returns in one minute. Collect 10 Chalk with a two-minute site respawn. Ramp destinations hold for two minutes. | No active gathering. The chosen recipe asks for 10 gravel and 5 bones. Crystal uses `> 10` in both gathering scripts, has a special fine-chalk variant, and rerolls a cave on each entry. Use Global's 10 and two-minute routing when implementing. |
| Mortar preparation | Mix one gravel bucket and one chalk bucket, then use the mixture on a water bucket. | Both steps were imported as interactions without native dispatch. Crystal mixture preserves two buckets by emptying the used bucket and transforming the target. Crystal mortar requires bucket fluid subtype 1 and returns an empty bucket alongside the mortar. |
| Mortar delivery and monster credit | Deliver one mortar to a trough for 100 Inquisition Gold. Silencer and Frazzlemaw each earn 1. Sandomo handles gratitude → repairs. | No native trough callback, quest monster credit binding or Sandomo quest dialogue. Crystal records the delivery and kills in player storage, then handles bookkeeping and a quantity-based exchange for Cluster of Solace (20062). |
| Horns | Daily box grants one mouthpiece (20057). Search a pile for horn (20055) or spawn a Guzzlemaw; pile resets in two minutes. Assemble horn (20056) and use at the wall for 100 gold. | Source-only bone search; Crystal specifies 20% and a timed transform. No live daily box, assembly, horn wall interaction or reward. |
| Shrines | Use Silencer Excretion on a shrine skull; spawns Guzzlemaw and gives 10 gold, with ten-minute repeat. | No quest callback or reward. The pinned donor's quest directory contains no shrine action. |
| Luring Silencers | Each successful chamber gives 50 gold; five trigger the plateau raid and Luring Silence. Four Silencer waves culminate in 3–5 Sights of Surrender. | No active quest callback, reward or achievement. Crystal starts its raid on one successful use, with three Silencer waves and 3–8 Sights in its fourth wave. |
| Sight of Surrender | 10 gold per kill and Never Surrender after participating in 100 kills. | No quest reward or achievement binding. Crystal lower death dispatcher credits only Frazzlemaw and Silencer. |
| Ancient Dreams | Morax directs player to the Nebuliser box (20049, every three hours). Lantern enables extraction for ten seconds and recovers in 150 seconds; traces recover in 300 seconds. Outcomes are crude remains, hostile Feversleep/Terrorsleep, or Ancient Dream (20047), up to nine before tool break. Three dreams yield one Essence of Dread (20138), repeat three hours. | No active quest tools, extraction, cooldowns or exchange. Crystal Morax has basic NPC focus only. |
| A Cup of Tea | Three pottery pieces per component type; combine leaves and cup, use filled cup (20210) at the river, then spoon. Pillow leads to Morax; `parched` yields one Cluster of Solace (20062). | No live pottery pickups, tea chain, pillow or Morax claim. The chosen recipe associates one cluster with a personal bridge route. |
| A Retched Beginning | 100 Retching Horror kills including pre-acceptance; Dream Butterfly turn-in yields three Essence of Dread (20138) and Ending the Horror; three-hour repeat. | No live kill/offer, reward or achievement binding. Crystal Dream Butterfly has basic NPC focus only. |
| Face Your Fears | 500 Choking Fear kills including pre-acceptance; Dream Butterfly turn-in yields 12 Essence of Dread (20138); three-hour repeat. | No live kill/offer or reward binding. |
| Mend What's Broken | Loot or forage Broken Dreams, then use on a Half-Buried Face. Spawned Nightmare or butterfly puts the face on a three-minute cooldown; crumbling permits immediate retry. Three butterflies per cycle; report `mend`, use Stony Pond and claim two Essence of Wishful Thinking (20139). Two-hour repeat and Dream Wright at 100 butterflies. | No live quest foraging, face, pond, claim or achievement binding. |
| Not Afraid of the Dark | Prior Terrorsleep kills count toward 100; Feversleep excluded. Morax `terrors` yields two Essence of Dread (20138). | No live kill/offer or reward binding. |
| Soothing Bad Dreams | Dream Butterfly supplies Delicate Pan (20048); sieve river for Dream Sand. Learn all three flowers and use the third again for songs (20050). Equip songs in shield slot, soothe five dreams, report, then open tar-room bookcase for three Essence of Dread (20138). Timed room, three-hour repeat; retain songs and surplus sand. | No live offer, sieving, song, equipment-gated use, reward room or claim binding. |
| Waters of Life | Morax `waters` gives Pendulum (20141). Clear Poison Spines and operate all three wheels personally; roughly two-minute lever reuse. One player at a time hypnotizes all three Shiversleep and enters the timed reward room. Bones yield one Essence of Wishful Thinking (20139); report to Morax and keep it. | No live offer, cistern callbacks, room admission or claim. |
| The First Mate | Eleven fixed clues rotate one bottle per server save. Fishing success records a clue without removing the bottle for others. All eleven permit digging to skeleton and claiming Nevermending Story and two papers. | No live rotation, fishing credit, dig gate, skeleton claim or achievement. |
| XP and outfits | Neither is listed as a quest grant in the capture. | No such quest grants found. |

World callback, held callback, reward-claim and NPC callback/offer catalogues have no Roshamuul quest rows to promote. The generated native state preserves the source-only quest and the completion candidate has no dispatch, NPC or reward binding. In the native placement regions covering Global mapper sectors 130–131 by 126–127, the decoded item stacks contain 58 chalk objects (20125), 19 bone piles (20179) and one cave-exit trigger (20122 at 33498, 32602, floor 8), with no action or unique IDs on those stacks. The same scan found no donor trough (20216), Fine Gravel (20133), or cave-ramp AID 55000 in those sectors. This limits what can be bound directly from the donor selectors. A world object with the same appearance alone does not establish its Global quest role.

## Selected runtime correction

Register the dry mortar mixture in the existing carried-recipe owner. One gravel bucket (20053) and one chalk bucket (20054), in either order, produce one mixture bucket (20169) and one empty bucket (2873). The existing owner spends the selected carried inputs and mints both outputs in one transaction. Crystal `data-global/scripts/quests/roshamuul_quest/actions_mixture.lua:1-20` supplies the item mapping and empty-bucket behavior; Global's Delivering Mortar section confirms the mixture step. The three Roshamuul-specific native Item profiles reuse the canonical identities, pinned Canary XML and retained 15.30 appearance facts. The existing Item profile supplies the empty bucket.

## Blocked mechanics

Main has a world counter and scheduled saves. Its closed `CounterKind` in `crates/gameplay/world-change/src/counter.rs` has no Roshamuul contribution reducer or first-defeat stage. This requires an additional shared counter and map/NPC stage binding, and Global leaves the first two thresholds open.

```text
BLOCKER World stages server-save world-wide progression and first-kill NPC activation data-global/raids/roshamuul/gaz_haragoth.xml:1
BLOCKER Chalk caves two-minute stable randomized destination rotation data-global/scripts/quests/roshamuul_quest/movements_lower_roshamuul.lua:21-49
BLOCKER Gravel and horn search randomized temporary object depletion with failure spawn data-global/scripts/quests/roshamuul_quest/actions_bone.lua:9-23
BLOCKER Wet mortar fluid-qualified registration in the quest carried-recipe owner data-global/scripts/quests/roshamuul_quest/actions_mortar.lua:2-10
BLOCKER Mortar delivery and lower kill rewards joint quest and world contribution ledger data-global/scripts/quests/roshamuul_quest/actions_trough.lua:3-12
BLOCKER Sandomo quantity-based NPC redemption of a quest ledger data-global/npc/sandomo.lua:97-165
BLOCKER Silencer Plateau success-accumulated raid waves with world cooldown and cleanup data-global/scripts/quests/roshamuul_quest/actions_resonance_chamber.lua:75-101
BLOCKER Upper dream chains timed extraction, repeatable claims and single-occupant timed room with creature replacement data-global/npc/morax.lua:26-56
```

## OPEN-FACTS

- Exact bridge and wall thresholds and world-save contribution windows. The spoiler calls the bridge figures estimates. The Valley Sight-spawn threshold and most event durations are unspecified.
- Gravel success is an estimate. Horn, dream gathering, mending, sieving and fishing odds are unspecified. The plateau lockout is marked uncertain. Global wave sizes for the Silencers and the exact occupancy and ejection durations for dream rooms are unspecified.
- Ship fares, exact landing positions and Sandomo's cluster exchange price and quantity limits are absent from the Global capture; Crystal's 100 gold per cluster is donor evidence. Global does not give the Terrorsleep repeat interval or any further First Mate continuation.
- The facts name items and achievements but omit numeric IDs and a full unique/action and tile-ordinal map. Native item names and pinned scripts supply the IDs identified above; unverified placements, achievement bindings, First Mate papers and other unsourced IDs need further evidence.

## PARTIAL

The generic cooking owner can check fluid subtype. The quest carried-recipe registry supports item pairs and fixed outputs without a subtype selector. A wet recipe needs admission through the shared conversion catalogue, with corresponding shared owner tests and counts.

Entry-stage and journal admission; gravel/chalk gathering and caves; wet mortar, delivery and lower contribution rewards; horns, shrines, plateau raid and Sight credit; all Upper Roshamuul task grants, achievements and repeat cycles; temporary Dark Grounds access, Umbral service and First Mate.
