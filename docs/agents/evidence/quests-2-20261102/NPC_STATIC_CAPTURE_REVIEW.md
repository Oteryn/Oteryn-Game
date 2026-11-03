# Static NPC speech capture review — batch 2

Classification: **DERIVED**. This is a read-only capture audit, not a gameplay or Lua execution test. All donor references below expand to `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:path:line`.

## Current result

**No missing literal text remains in the scoped static-call audit.** The rerun covers 85 source files: the 67 NPCs in the emperor capture, 12 in the classic capture and six NPC source files referenced by the soul-lane inventory. It checks lexical `addKeyword`/`addChildKeyword` calls using `StdModule.say`, plus `addGreetKeyword`.

| Result | Calls | Meaning |
| --- | ---: | --- |
| Literal text in reply, greeting or explicit SOURCE_ONLY data | 36 | Text is retained; this is not proof of active native branch admission. |
| Literal text in complete source-string fragments | 413 | Exact source text is retained without activating its keyword or quest behavior. |
| Literal text absent from the selected captures | 0 | All 449 literal speech calls are represented. |
| Dynamic expression | 20 | Reported separately; no expression was evaluated or replaced with guessed speech. |
| **Total scoped speech calls** | **469** | **449 literal and 20 dynamic.** |

The first audit found 296 missing literal calls. The emperor source-capture update retained its 288 missing calls. The remaining eight were added through the Liquid Black generator to `content/dialogues/quests-2/liquid-black-source-keywords.json`: Scrutinon line83 and Spectulus lines463,467,471,475,479,483,487. Every new Liquid record is `native_binding: SOURCE_ONLY`; active selector order, travel, foreign quest transitions and Spectulus admission are unchanged.

The full Jack Springer static set (26 calls), Gnomadness greeting and Scrutinon name were already retained. Flickering Soul and Kesar have no matching static calls. Kesar has no dedicated full-file capture in these selected artifacts; this narrow static result does not establish that its other speech was imported.

## Per-file source evidence

The table lists every source file containing a matched static-speech registration. Files without these registration forms contribute zero calls. Counts describe source capture, not executable dialogue branches.

| Pinned Crystal source | Literal text retained | Dynamic expressions | Registration lines |
| --- | ---: | ---: | --- |
| [data-global/npc/angus.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/angus.lua#L656) | 6 | 0 | 656, 657, 658, 659, 660, 661 |
| [data-global/npc/brodrosch.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/brodrosch.lua#L94) | 2 | 2 | 94, 104, 106, 132 |
| [data-global/npc/captain_bluebear.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/captain_bluebear.lua#L62) | 26 | 1 | 62, 65, 67, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117 |
| [data-global/npc/captain_dreadnought.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/captain_dreadnought.lua#L319) | 12 | 3 | 319, 323, 327, 332, 337, 341, 349, 354, 359, 363, 369, 378, 384, 397, 408 |
| [data-global/npc/captain_seahorse.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/captain_seahorse.lua#L55) | 6 | 1 | 55, 57, 80, 81, 82, 83, 84 |
| [data-global/npc/chemar.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/chemar.lua#L68) | 4 | 1 | 68, 75, 82, 99, 124 |
| [data-global/npc/chief_grarkharok.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/chief_grarkharok.lua#L130) | 15 | 0 | 130, 134, 140, 141, 144, 145, 146, 148, 153, 154, 157, 158, 159, 162, 163 |
| [data-global/npc/dermot.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/dermot.lua#L92) | 7 | 0 | 92, 93, 94, 95, 96, 97, 98 |
| [data-global/npc/gewen.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/gewen.lua#L96) | 3 | 1 | 96, 99, 101, 118 |
| [data-global/npc/ghost_of_a_priest.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/ghost_of_a_priest.lua#L118) | 27 | 0 | 118, 119, 120, 121, 122, 123, 130, 131, 132, 133, 141, 142, 143, 150, 158, 159, 166, 167, 168, 169, 170, 171, 178, 179, 180, 181, 188 |
| [data-global/npc/gnomadness.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/gnomadness.lua#L102) | 1 | 0 | 102 |
| [data-global/npc/gurbasch.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/gurbasch.lua#L58) | 2 | 2 | 58, 68, 70, 91 |
| [data-global/npc/inigo.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/inigo.lua#L139) | 18 | 0 | 139, 144, 149, 154, 160, 165, 171, 177, 182, 187, 191, 196, 201, 206, 210, 216, 222, 228 |
| [data-global/npc/ivalisse.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/ivalisse.lua#L130) | 7 | 0 | 130, 131, 132, 133, 134, 135, 136 |
| [data-global/npc/iyad.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/iyad.lua#L64) | 19 | 1 | 64, 67, 69, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105 |
| [data-global/npc/jack_springer.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/jack_springer.lua#L155) | 26 | 0 | 155, 156, 157, 158, 159, 163, 167, 168, 176, 177, 178, 186, 194, 202, 203, 204, 205, 206, 210, 211, 212, 213, 214, 222, 229, 230 |
| [data-global/npc/jerom.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/jerom.lua#L89) | 24 | 0 | 89, 90, 98, 99, 100, 102, 103, 104, 107, 108, 111, 113, 114, 115, 116, 117, 118, 119, 120, 121, 123, 124, 126, 128 |
| [data-global/npc/king_tibianus.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/king_tibianus.lua#L235) | 57 | 0 | 235, 246, 253, 257, 261, 265, 269, 273, 277, 281, 285, 289, 293, 297, 301, 305, 309, 313, 317, 321, 325, 329, 333, 337, 341, 345, 349, 353, 357, 361, 365, 369, 373, 377, 381, 385, 389, 393, 397, 401, 405, 409, 413, 418, 424, 429, 436, 442, 447, 451, 456, 461, 466, 471, 476, 482, 486 |
| [data-global/npc/mortimer.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/mortimer.lua#L654) | 6 | 0 | 654, 655, 656, 657, 658, 659 |
| [data-global/npc/mr_morris.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/mr_morris.lua#L287) | 22 | 0 | 287, 292, 297, 302, 308, 312, 317, 322, 332, 337, 342, 347, 351, 356, 360, 364, 369, 375, 380, 385, 389, 393 |
| [data-global/npc/oressa.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/oressa.lua#L69) | 13 | 0 | 69, 73, 77, 81, 90, 95, 99, 103, 107, 111, 115, 123, 127 |
| [data-global/npc/pino.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/pino.lua#L63) | 3 | 1 | 63, 66, 68, 88 |
| [data-global/npc/queen_eloise.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/queen_eloise.lua#L171) | 51 | 0 | 171, 173, 175, 182, 183, 185, 188, 190, 191, 192, 193, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 207, 210, 211, 212, 214, 215, 217, 218, 219, 220, 222, 224, 226, 227, 229, 230, 231, 233, 234, 236, 238, 240, 241, 242, 243, 244, 245, 246, 247 |
| [data-global/npc/scrutinon.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/scrutinon.lua#L81) | 2 | 1 | 81, 83, 92 |
| [data-global/npc/spectulus.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/spectulus.lua#L463) | 7 | 0 | 463, 467, 471, 475, 479, 483, 487 |
| [data-global/npc/swolt.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/swolt.lua#L101) | 2 | 0 | 101, 102 |
| [data-global/npc/tanyt.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/tanyt.lua#L63) | 10 | 1 | 63, 66, 68, 91, 92, 93, 94, 95, 96, 97, 98 |
| [data-global/npc/the_queen_of_the_banshees.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/the_queen_of_the_banshees.lua#L179) | 11 | 0 | 179, 180, 181, 182, 186, 190, 194, 198, 202, 206, 210 |
| [data-global/npc/urks_the_mute.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/urks_the_mute.lua#L54) | 2 | 2 | 54, 64, 66, 71 |
| [data-global/npc/uzon.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/uzon.lua#L63) | 23 | 1 | 63, 66, 68, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113 |
| [data-global/npc/uzon_back.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/uzon_back.lua#L63) | 7 | 1 | 63, 66, 68, 74, 75, 76, 77, 78 |
| [data-global/npc/woblin.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/woblin.lua#L124) | 9 | 0 | 124, 128, 132, 136, 140, 144, 148, 152, 156 |
| [data-global/npc/zarifan.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/zarifan.lua#L87) | 1 | 0 | 87 |
| [data-global/npc/ziyad.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/ziyad.lua#L63) | 10 | 1 | 63, 66, 68, 91, 92, 93, 94, 95, 96, 97, 98 |
| [data-global/npc/zumtah.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/zumtah.lua#L131) | 8 | 0 | 131, 132, 135, 136, 137, 138, 139, 140 |

## Reviewed capture identities

These SHA-256 values identify the reviewed artifacts. A subsequent capture change requires a rerun of this audit. Frozen quest-step disposition counts are independent of this source-text completion.

| Capture artifact | SHA-256 |
| --- | --- |
| `content/dialogues/grave-danger.json` | `2ba8e276400da3d13b8bb6bf99e4412ce5fa3ecacb9d19fc09eded931adc265b` |
| `content/dialogues/liquid-black.json` | `1c703ff71f053e58c51a17370c6b6a56943c096357e8258b5f7dc41362063520` |
| `content/dialogues/primal-ordeal.json` | `32eccadfdddc6d3084614a3fd90cff84f26add267fc11ca36c4b9144e9b33fb1` |
| `content/dialogues/quests-2/classic-donor-dialogues.json` | `b4a5ae314ab0f96b484ea6fc0859b40a7f16aa88b2ce594c32d75133dad150d3` |
| `content/dialogues/quests-2/emperor-donor-dialogues.json` | `f3a1703247bdb536835a26ab473e5fdf86a0bfca65d56670b5bdb1cf0355f2bb` |
| `content/dialogues/quests-2/liquid-black-source-keywords.json` | `6981b9334ea7ca0b3731f031ca4ae47300dce6cb2d79ea4bbdafb92854531751` |
| `content/dialogues/soul-war.json` | `95dd8c3a21cfc9dd851d8d1ec87558939857927afaebde42e4084810b704458d` |

The audit retained exact donor file SHA-256 values, registration/text/end lines, keyword expressions, literal messages or explicit dynamic expressions, and matching artifacts. It does not certify complete Lua control flow, dynamic evaluation, physical NPC admission, voice scheduling or playable quest completion.
