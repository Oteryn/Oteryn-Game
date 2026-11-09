# M35 — source qualification for independent treasure claims (PARTIAL)

Reference: `zimbadev/crystalserver` at `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/startup/tables/chest.lua` (`OTS_HYPOTHESIS_ONLY`). Each listed storage/KV
marker is an independent once-only claim. All groups are required, in any order.
Access keys and mission components are excluded from final-loot sets, as in the Bear Room rule.
Recipe-declared reward keys, such as Emperor's Cookies Key 3802, remain required.

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
| emperor_s_cookies_quest | `quest/u6_1/emperors_cookies/rewards/cookies` (402); `quest/key/id3802` (413) |
| fire_axe_quest | `kv/fireaxe1` (1079); `kv/fireaxe2` (850) |
| ghoul_room_quest | `kv/ghoulroom1` (1636); `kv/ghoulroom2` (1644) |
| goblin_temple_quest | `kv/goblintemple` (1490); `kv/goblintemple2` (1499) |
| griffin_shield_quest | `kv/griffinshield1` (1873); `kv/griffinshield2` (1880); `kv/griffinshield3` (1887) |
| katana_quest | `kv/katanacorpse1` (1851); `kv/katanacorpse2` (1858) |
| koshei_the_deathless_quest | `quest/u8_1/koshei_the_deathless/koshei_the_deathless_legs` (2459); `quest/u8_1/koshei_the_deathless/koshei_the_deathless_platinum` (2466) |
| longsword_quest | `kv/longsword1` (1713); `kv/longsword2` (1720); `kv/longsword3` (1727) |
| mad_mage_room_quest | `kv/madmageroom1` (1193); `kv/madmageroom2` (1200); `kv/madmageroom3` (1207) |
| minotaur_hell_quest | `kv/minohell1` (1509); `kv/minohell2` (1516); `kv/minohell3` (571) |

Dreamer's Challenge uses four independent storage groups (table lines 2116, 2124,
2131 and 2163). The four wine placements (UID6197–6200) share one claim/track,
so any one of them satisfies the wine group; they are not four required claims.

Emperor's Cookies and Koshei use final reward membership from the existing authored
recipe (`content/quests/definitions/quests-00000-00099.json`, records 32 and 52,
`definition/oteryn_recipe/payload/recipe/reward_intents`) and exact donor claim identities.
Key 3800 and the four Koshei amulet parts are access/components; the incidental gold
claim is not a declared final reward. Existing source payout quantities are unchanged.

## Remaining M35 scope

Four quests are not certified by this slice: Elvenbane, Fanfare,
Ferumbras' Ascension and Forgotten Knowledge.
Their linked claims include shared placements, access keys or mission components; the source
claim list alone does not establish which required/alternative groups constitute the final reward set.
Elvenbane has no canonical RewardClaim association in its current quest definition.
These require explicit source-qualified group declarations and dispatch completion cases.
Fanfare's recipe (the same definition shard, record 34) declares only the Fanfare as
final loot and uses Key 3520 for access. It does not establish a second required final
reward group; one must not be invented merely to prevent first-chest completion.
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

The slice with 22 claim sets remains PARTIAL for the full M35 row: four quests are unresolved.

## Claim-set slice validation

- 22 sets / 53 claim tracks / 56 source placements checked against Crystal.
- 53 generated placement-binding additions; no other shard changes.
- Rust formatting passed. Follow-up Clippy for quests/content (all targets/features)
  passed; the unchanged dispatch caller was also linted in the preceding slice.
- All 29 reward authoring checks passed.
- Prior twenty-set full quest suite: 403 passed; one pre-existing Ice Islands exact-hash baseline failure.
  The same `ca68476c…` versus `ce91fb47…` mismatch reproduces with the original
  one-set qmech input. Its assertion is unchanged.
- World pin refresh and `pin-check . /tmp/m35-more-pin-check` passed with the same pin digest:
  `4b6cb9bf12cf8b14b66da82168c23b53f89c45dfeb5c5d666829d5c562a4d7cb`.
- Local node boot was not run, as requested by the owner.
- Affected claim-set tests: 3 passed, 0 failed.
- Affected content `world_reward_claims` tests: 11 passed, 0 failed.
- Live dispatch regression: RED on the base's missing Alawar binding, then GREEN:
  1 PostgreSQL case passed against the 22-set catalogue in 162.49 seconds, exercising both chest orders.
  `M35_ALAWAR_POSTGRES_CASE_COMPLETED` confirms the database scenarios executed.
  The reconstructed caller must retain the Channel's command-ingress state while
  clearing its quest cache; a fresh ingress correctly refuses continuation at command 3.

## Dreamer alternative caller qualification

The live alternative case exposed two missing production connections: the
native container projection discarded the source fluid subtype, and the
native item provider did not publish the reward's certificate and bread.
The bridge now qualifies only the existing exact native empty-vial profile
for Dreamer's four source placements (UIDs 6197–6200). It preserves the
canonical definition and immutable source snapshot, decodes the child subtype,
and passes it through the bundle-chest caller to the existing atomic child
fluid MINT owner. The source packet proves raw subtype 1 = WATER; the quest
name is not evidence for wine. Missing, zero and subtype 15 are rejected.

The callback generator closes the full declared reward item set and adds five
source-qualified identities: blue legs 645, star amulet 3014, hat of the mad
3210, brown bread 3602 and certificate 6498. Its equipment cohort remains
independently counted; the generator also corrects the pre-existing engine
packet count from 63 to the retained packet's actual 64. Native-only
regeneration updates dependent digests rather than hand-editing them.
The callback test fixture now replays the exact pre-callback admission packet,
before later rendering attributes are attached; all refusal assertions remain.
The exact complete-container closure increases from 137 to 140 identities,
and the exact native recipe count from 70 to 71 (held shapes 35 to 34).

The mounted PostgreSQL regression uses all four physical wine placements in
both traversal orders, reloads quest tracks between commands, and requires
one root receipt, three children and one WATER fluid row atomically linked to
that receipt. Every alternative after the first must refuse another grant;
canonical Dreamer completion must remain false because its other three
required groups are unclaimed. Alawar still exercises both required-chest
orders and completion only at the last chest.

The declaration count of 22 is not proof that every cohort's complete physical
reward route is admitted. Remaining undeclared final groups are Elvenbane,
Fanfare, Ferumbras' Ascension and Forgotten Knowledge; their physical final
claim identities still require source qualification. Other declared sets
retain source item admission holds (including Draconia, Edron Goblin, Fire Axe,
Ghoul Room and Goblin Temple); this scoped WATER bridge does not admit them.

Bridge validation (live retained PostgreSQL 18.6, two Cargo build jobs):

- `cargo fmt --all` plus rustfmt for the mounted PostgreSQL case: passed.
- Content + dispatch `cargo check --locked --all-features`: passed.
- Content + dispatch clippy, all features/targets, `-D warnings`: passed.
- Content `world_reward_claims`: 12 passed.
- Dispatch `m35_`: 2 passed in 272.10 seconds; both database completion
  markers were printed, with both traversal orders exercised per case.
- Dispatch exact production equipment source/profile test: 1 passed.
- Container authoring: 14 passed after native regeneration; callback profiles:
  13 passed; variant authoring: 13 passed; source snapshot: 9 passed;
  existing reward authoring script: 29 passed.
- Native-only regeneration and reproducibility check: passed, 44 generated files.
- Compiler `pin-refresh` and `pin-check`: passed. Map digest remains
  `4b6cb9bf12cf8b14b66da82168c23b53f89c45dfeb5c5d666829d5c562a4d7cb`;
  refreshed inputs digest is
  `f60dc45c615b91527b32f9c304cdc17b965d81cc85564221ecf2e37c096b2e61`.
