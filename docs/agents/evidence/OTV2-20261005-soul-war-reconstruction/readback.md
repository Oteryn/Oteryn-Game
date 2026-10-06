# Soul War reconstruction readback — Crystal summer-update + YouTube

**Task:** `OTV2-20261005-soul-war-reconstruction`  
**Primary donor:** `zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f` (`summer-update`)  
**Cross-check:** `opentibiabr/canary@04b83b512114bfd888000d6e1433ed8ecaec7c5b`  
**Behavioral evidence:** YouTube playlist `PL2czNtPw97ZRaU9SIaoZF42p6K5BTu-qX`, six parts, visually reviewed.  
**Runtime authority:** none. The reconstruction is content/evidence only; it does not activate Quest or Encounter runtime.

## Result matrix

| Area | Reconstructed state | Evidence / decision | Admission state |
|---|---|---|---|
| Start / Flickering Soul | level 250, premium, `hi -> task -> yes`, teleport access | Crystal source | Quest DATA authored; Native lowering still held |
| Five branches | Claustrophobic Inferno, Mirrored Nightmare, Ebb and Flow, Rotten Wasteland, Furious Crater | Crystal + video | progression recorded |
| Taints | five ordered taints; first boss starts 14-day window; bosses may be done in any order | Crystal; accepted Oteryn taint decisions preserved | Quest/encounter runtime held |
| Claustrophobic Inferno | 3 raids; Crystal has 120 s survival, 20 s spawn cadence, 3 monster groups | Crystal + video; Canary conflict retained | three source-backed raid cores authored; 5 s player removal, entry-direction/10 s cooldown and one bad donor spawn token remain explicit hard holds |
| Malice | Soul Cage 23 s / 40 s timeout chain and white/safe-tile behavior recorded | Crystal + visual video window `ZJKtfyRCPjQ 04:30-06:50` | representable core authored; procedural white tiles / additive reflect stay explicit hard holds |
| Greed | Greedbeast/Soul chain, 5-kill vulnerability, Soul Sphere and 45 s window | Crystal + visual video `X_eftME4qkY 06:15-08:05` | representable core authored; moving Soul Sphere, Soulsnatcher area damage and max-health/reflect stack remain hard holds |
| Spite | Hazardous Phantom access, Searing Fire cycle, Weeping Soul corpse behavior, Fear evidence | Crystal + visual video `AR0TxuTv0rc 07:25-09:35` | representable fire core authored; per-player stomp cooldown/corpse step remain hard holds |
| Cruelty | 40/55/70 energy gates, Greedy Eye, Mortal Essence/Greedy Maw, 15 s defense escalation | Crystal + visual video `t3GcDyTQsPk 05:10-07:15` | representable core authored; per-player Maw cooldown, dynamic damage scaling and custom elemental transform remain hard holds |
| Hatred | 180 -> 135 -> 90 -> 45 Burning Hatred forms, Sorrow +10 s, +10 hatred empowerment | Crystal + visual video `oOgX3QQtJxg 04:20-06:30` | representable core authored; per-player Condensed Remorse / torment reset remains a hard hold |
| Megalomania | 4 Aspect deaths, 5 s Aspect respawn, Purple -> Green -> Blue -> Purple phase path, splinter/sanity evidence | Crystal + visual video `L8b-8qgQhuo 01:20-07:05` | representable phase core authored; player-local corpse/sanity/remains, white tiles, torment and custom elemental transform remain hard holds |
| Final completion | Megalomania kill, report to Flickering Soul for Revenant Outfit, one random item from 18 Soul rewards | Crystal + quest reconstruction | canonical Quest DATA authored; reward/runtime lowering held |

## Source conflicts and donor defects

1. **Claustrophobic Inferno cadence:** Crystal summer-update uses 20 s / 6 waves and three monster types; current Canary uses 10 s / 12 waves and Brachiodemon-only. Reconstruction selects Crystal as primary donor and retains the conflict.
2. **Crystal bad spawn token:** the second Claustrophobic Inferno phantom list contains a bare `Pos` token. It is not copied or guessed.
3. **Mirror Image:** Crystal behavior conflicts with accepted Oteryn Q5b. Oteryn's accepted 70% attacker vocation / 7.5% each other vocation / 1 HP floor remains authoritative.
4. **Soul Sphere cadence:** Crystal source is exactly 3000 ms. Video presentation is treated as non-numeric behavioral evidence.
5. **Rotten Wasteland shrines:** donor code stores only last shrine id + count and can be advanced by alternating ids. Reconstruction requires four distinct ids: 33019, 33021, 33022, 33024.
6. **Spite recovery:** donor constant is 56 s while reference-wiki prose describes 60 s. The exact donor value remains source data; the disagreement is not hidden.

## Architecture boundary

Accepted encounter admission E4 forbids admission of an encounter while its manifest contains `unresolved_semantics`. Therefore the six Crystal boss bundles intentionally remain source-backed authoring/evidence and are **not** promoted into `content/encounters/**` while those hard holds exist. This is not a silent omission: each blocked mechanic is listed in its manifest.

The canonical Quest DATA is separately materialized through the Quest authoring pipeline. It does not claim Native quest execution. Dynamic Ebb and Flow map replacement, boss-room lever lifecycle, party placement/reset, random reward transaction ownership, and encounter runtime remain their existing owner/runtime lanes.

## Machine artifacts

- `tools/content-schema/quest-authoring/samples/soul-war-reconstruction/reconstruction.json`
- `tools/content-schema/quest-authoring/samples/soul-war-reconstruction/recipe-followup.json`
- `tools/content-schema/quest-authoring/soul_war_reconstruction.py`
- `tools/content-schema/encounter-authoring/soul_war_crystal_reconstruction.py`
- `tools/content-schema/encounter-authoring/samples/soul-war-crystal-reconstruction/**`
- `tools/content-schema/encounter-authoring/test_soul_war_crystal_reconstruction.py`
- canonical Quest shard `content/quests/definitions/quests-00200-00299.json`

