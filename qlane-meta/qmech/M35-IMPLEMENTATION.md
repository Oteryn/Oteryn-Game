# M35 — source qualification for independent treasure claims (PARTIAL)

Reference: `zimbadev/crystalserver` at `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/startup/tables/chest.lua` (`OTS_HYPOTHESIS_ONLY`). Each listed storage/KV
marker is an independent once-only claim. All groups are required, in any order.
Access-key claims are excluded from these treasure-room sets, as in the existing Bear Room rule.

The source placement UIDs, positions and appearances were checked against the pinned table.
The implementation binds each marker to its own `/chest` transition using the authoring generator; the existing
bundle chest dispatch creates a MINT obligation and refreshes the durable quest writer.
No fixed chosen-source stage prerequisites are imposed on these independent claims.

| Quest | Required source markers (table line) |
| --- | --- |
| alawar_s_vault_quest | `kv/alawars_vault1` (918); `kv/alawars_vault2` (925) |
| bear_room_quest | `kv/bear_chest1` (1458); `kv/bear_chest2` (1467); `kv/bear_chest3` (1474) |
| behemoth_quest | `kv/behemothquest1` (1150); `kv/behemothquest2` (970); `kv/behemothquest3` (977); `kv/behemothquest4` (984) |
| black_knight_quest | `kv/blackknight1` (933); `kv/blackknight2` (940) |
| circle_room_quest | `kv/circleroom1` (1526); `kv/circleroom2` (1533) |
| crystal_wand_quest | `kv/crystalwand1` (1541); `kv/crystalwand2` (1548) |
| demona_ring_quest | `kv/demonaring1` (1556); `kv/demonaring2` (1563) |
| double_hero_quest | `kv/doublehero1` (820); `kv/doublehero2` (827) |
| draconia_quest | `kv/draconia1` (748); `kv/draconia2` (757) |
| dragon_tower_quest | `quest/u7_1/dragon_tower/rewards/backpack1` (1131); `quest/u7_1/dragon_tower/rewards/backpack2` (1140) |
| edron_goblin_quest | `kv/edrongoblin1` (835); `kv/edrongoblin2` (842) |
| fire_axe_quest | `kv/fireaxe1` (1079); `kv/fireaxe2` (850) |
| ghoul_room_quest | `kv/ghoulroom1` (1636); `kv/ghoulroom2` (1644) |
| goblin_temple_quest | `kv/goblintemple` (1490); `kv/goblintemple2` (1499) |
| griffin_shield_quest | `kv/griffinshield1` (1873); `kv/griffinshield2` (1880); `kv/griffinshield3` (1887) |
| katana_quest | `kv/katanacorpse1` (1851); `kv/katanacorpse2` (1858) |
| longsword_quest | `kv/longsword1` (1713); `kv/longsword2` (1720); `kv/longsword3` (1727) |
| mad_mage_room_quest | `kv/madmageroom1` (1193); `kv/madmageroom2` (1200); `kv/madmageroom3` (1207) |
| minotaur_hell_quest | `kv/minohell1` (1509); `kv/minohell2` (1516); `kv/minohell3` (571) |

Dreamer's Challenge uses four independent storage groups (table lines 2116, 2124,
2131 and 2163). The four wine placements (UID6197–6200) share one claim/track,
so any one of them satisfies the wine group; they are not four required claims.

## Remaining M35 scope

Six quests are not certified by this slice: Elvenbane,
Emperor's Cookies, Fanfare, Ferumbras' Ascension, Forgotten Knowledge and Koshei the Deathless.
Their linked claims include shared placements, access keys or mission components; the source
claim list alone does not establish which required/alternative groups constitute the final reward set.
Elvenbane has no canonical RewardClaim association in its current quest definition.
These require explicit source-qualified group declarations and dispatch completion cases.
For example, Crystal `data-global/lib/core/quests.lua:5173-5176,5241-5247` starts
Forgotten Knowledge on tomes but names `LastLoreKilled` for its final fight; merely
claiming the linked tomes and mission objects does not prove that completion.

The existing alternative-group evaluator is retained. This slice adds no new alternative
group declaration and does not claim the complete 26-quest M35 row is DONE.

## Runtime regression evidence

The regression `m35_alawar_dispatch_completes_only_on_last_chest` drives literal wire USE for both
source chests in both orders, with duplicate replay, a new-command repeat and reconstructed
callers between chests, checking exact claim/receipt/consumed-obligation counts and durable
completion only on the final chest. The catalogue test exercises each declared group first
and last, in forward and reverse rotations. The authoring test retains an exact binding map.

The twenty-set slice is validated; the six remaining quests keep the full M35 row PARTIAL.

## Claim-set slice validation

- 20 sets / 49 claim tracks / 52 source placements checked against Crystal.
- 49 generated placement-binding additions; no other shard changes.
- Rust formatting and Clippy for quests, content and dispatch (all targets/features) passed.
- All 28 reward authoring checks passed.
- Quest suite: 403 passed; one pre-existing Ice Islands exact-hash baseline failure.
  The same `ca68476c…` versus `ce91fb47…` mismatch reproduces with the original
  one-set qmech input. Its assertion is unchanged.
- World pin refresh and `pin-check . /tmp/m35-pin-check` passed with the same pin digest:
  `4b6cb9bf12cf8b14b66da82168c23b53f89c45dfeb5c5d666829d5c562a4d7cb`.
- Local node boot was not run, as requested by the owner.
- Affected claim-set tests: 3 passed, 0 failed.
- Affected content `world_reward_claims` tests: 11 passed, 0 failed.
- Live dispatch regression: RED on the base's missing Alawar binding, then GREEN:
  1 PostgreSQL case passed in 150.61 seconds, exercising both chest orders.
  `M35_ALAWAR_POSTGRES_CASE_COMPLETED` confirms the database scenarios executed.
  The reconstructed caller must retain the Channel's command-ingress state while
  clearing its quest cache; a fresh ingress correctly refuses continuation at command 3.
