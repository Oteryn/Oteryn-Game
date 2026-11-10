# M09: independent quest reward outputs

Checked 2026-11-08. TibiaWiki BR is the gameplay reference; the pinned Canary
and Crystal NPC scripts corroborate item identities and source state changes.
The coordinator's M09 assignment includes Rookie Guard, Treasure Hunt and Tower
Defence. The latest assignment supersedes qready2's old range: use 2100–2109.

| Quest / step | Checked source | Exact consequence |
|---|---|---|
| The Rookie Guard / Vascalir mission 11 acceptance | https://www.tibiawiki.com.br/index.php?title=The_Rookie_Guard_Quest&oldid=433575 ; https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/vascalir.lua (mission 11 acceptance) | Mission11 -1 → 1 after Mission10=3: silver amulet 3054 ×1 and empty quest flask 12785 ×1. The source also marks the wasp nest on the map; the item participant does not certify that separate map-mark effect. |
| The Rookie Guard / overall reward list | Same wiki page, reward summary and individual missions | The reported shield and seven small health potions are accumulated across different stages. They are **not** one source reward and must not be invented as one finale grant. Hyacinth's two potions and star ring are also separate source hand-ins. |
| Tower Defence / Bunny final password | https://www.tibiawiki.com.br/index.php?title=Tower_Defence_Quest&oldid=393503 ; https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/npc/bunny_bonecrusher.lua#L72-L89 ; https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/bunny_bonecrusher.lua#L79-L85 | Questline 4 → 5, topic 2, literal `password*`: arrows 762 ×50, 774 ×50, 763 ×50, 761 ×50. One atomic four-output reward; no reward container. |
| Treasure Hunt / treasure room | https://www.tibiawiki.com.br/index.php?title=Treasure_Hunt_Quest&oldid=431833#Recompensa | Telescope and three gold ingots are in **separate chests**. The 108 coins, two small amethysts and three small emeralds are daily floor respawns, not guaranteed personal quest rewards. Preserve separate chest claims; do not bundle daily floor loot or silently combine both chests. |

Reused implementation evidence: qengine2 commit
`f4b876061d6ca15ac87e707ff7aead5e191dbc12` introduced the sibling-item Rust plan.
This lane reuses its Rust implementation, preserves subsequent fluid/container
proofs, and writes its own forward migration instead of copying migration1007.
The questpc4-n2 branch's `61a1f24af` already qualifies the Treasure Hunt poem and
telescope profiles; reuse those facts instead of manufacturing another profile.

## Treasure chest allocation decision (M09)

The admitted region `content/world/placements/region-z13-x125-y128.b3`
contains the two appearance-2478 chests at `(32144,32968,13)` and
`(32139,32978,13)`. The wiki confirms separate caches but its reward-room
images were inaccessible (HTTP 403), so the item-to-cell allocation is not
source-verified. Selected Oteryn approximation: the northern chest grants the
telescope (9616 ×1), the southern chest the gold ingots (9058 ×3). Each has an
independent personal once track and reward receipt. This preserves both source
reward quantities and separate chest claims; it does not claim source-exact
allocation, daily floor-loot ownership, or completion of the other route steps.

Rookie mission 11 uses its canonical initial storage value -1, not an invented
zero alias. The source item XML names 12785 “special flask”, description
“Vascalir gave it to you to collect wasp poison”, weight 180. Its physical
profile must be qualified through the existing pinned importer before runtime
activation. The grant reply intentionally omits the unimplemented map-marker
claim; the separate source map-mark effect remains outside M09's item scope.

Vascalir's donor greeting is conditional (`greetCallback`); its catalogue has no
literal fallback. The reward adapter therefore admits the pinned NPC handler's
neutral default salutation/farewell/walkaway (Crystal 9f5a72c, npc_handler.lua
81–83, SHA256 8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a).
This is the selected Oteryn opening for the M09 reward adapter, not a claim that
the other conditional Rookie greetings or journey steps have been implemented.
The reward remains guarded by the canonical Mission10=3 and Mission11=-1 state.
