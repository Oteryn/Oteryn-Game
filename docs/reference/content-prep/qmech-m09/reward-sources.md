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
