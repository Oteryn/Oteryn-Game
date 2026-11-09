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

Three quests remain uncertified after the Fanfare caller slice: Elvenbane,
Ferumbras' Ascension and Forgotten Knowledge.
Their linked claims include shared placements, access keys or mission components; the source
claim list alone does not establish which required/alternative groups constitute the final reward set.
Elvenbane has no canonical RewardClaim association in its current quest definition.
These require explicit source-qualified group declarations and dispatch completion cases.
Fanfare's recipe (the same definition shard, record 34) declares only the Fanfare as
final loot and uses Key 3520 for access. The later Fanfare slice below binds that
single final group and proves that the access grant does not complete it.
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
Ferumbras' Ascension and Forgotten Knowledge; their physical final
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


## Fanfare final-reward caller

Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/startup/tables/chest.lua:1680-1687`, declares one final Fanfare
reward at UID 6142, (32390,31769,9), appearance 2472. Key 3520 at UID 5018,
(32376,31802,7), appearance 2473, is a separate access grant. The catalogue
now has 23 declarations: 22 multiple-required-group sets and this exact
single-final-group rule. The 22 existing multi-group assertions remain intact.
The final claim gets a generated quest transition; the key explicitly does not.

The native callback generator qualifies the exact bone-key claim, preserving
its numbered-key binding and source placement. It publishes canonical item
2973 through the normal provider; an independently counted one-key cohort
raises the native union to 5802. No source quantity or canonical item semantic
is replaced. The mounted Fanfare caller case executes the real access-key
MINT and final chest separately, checks key number 3520 and receipt linkage,
requires zero quest receipts/obligations for the access grant, and requires
canonical completion after the final chest. Both paths exercise duplicate
replay and refusal of a fresh duplicate command.

Further donor qualification: Ferumbras' actual final action is
`data-global/scripts/quests/ferumbras_ascension/actions_reward.lua`, UID 1035,
carrier 4073. It grants a brown bag containing 22731 x1 four times, 22737 x4,
22775 x1, 3043 x10 and 22516 x5, plus Hat Hunter and both Rift Warrior outfit
addon variants (852/846, addon mask 3). Its once marker is
`Storage.Quest.U10_90.FerumbrasAscension.Reward`. The three existing component
claims are not that reward. A complete source-faithful adapter must bind the
physical UID and all item/cosmetic effects atomically rather than treating
those three components as the final required set.

Fanfare validation before push:

- Formatter, quests/content/dispatch all-features check and all-target clippy
  with `-D warnings`: passed.
- Completion-set tests: 3 passed; focused content decoder tests: 12 passed.
- Source chest-authoring checks: 29 passed; callback generator tests: 14 passed.
- Native-only source generation and reproducibility: passed. This changes no
  World placement or terrain data; the dependent native registry pins are generated.

Forgotten Knowledge's exact final writer is Crystal 201189f
`data-global/scripts/quests/forgotten_knowledge/creaturescripts_bosses_kill.lua:11,18-29`:
it attributes the Last Lore Keeper death to damaging players and writes
`LastLoreKilled` with a 13-day, 20-hour deadline. Together with the final quest-log
mission in `data-global/lib/core/quests.lua:5241-5247`, this establishes a real
kill/time route; collecting the eight imported tome/component claims is not
sufficient final-completion evidence. Elvenbane still has no canonical claim
association, and its selected Spellbook reward remains an ambiguous item identity.
These are remaining native binding requirements, not completed claim-set routes.

Final profile review caught and repaired an admission-order regression: the new
Fanfare callback closure could otherwise shadow its later House source profile
with poorer canonical-only facts. The callback pass now seeds the exact retained,
pin-checked House profile for item 2955. A regression test requires all established
classification, container, charges, temporal, skill, equipment, presentation,
physical, stack, attributes, authoring and binding facts to survive. Only the
already-canonical known trade policy enriches its previously unknown group.
The generated profile diff was checked against the preceding pushed commit.

Live results: all three mounted M35 cases passed in 387.42 seconds, printing
`M35_ALAWAR_POSTGRES_CASE_COMPLETED`, `M35_WINE_ALTERNATIVES_POSTGRES_CASE_COMPLETED`
and `M35_FANFARE_POSTGRES_CASE_COMPLETED`. After restoring Fanfare's known native
facts, the focused Fanfare case passed again in 126.20 seconds, including the
real key MINT, final chest MINT, duplicate replay and canonical-state assertions.
Final formatting checks passed. Local node boot remains skipped per owner.
The earlier unrelated `quest::reward_callbacks::tests::published_ungated_quest_hash_baseline`
failure for Ice Islands remains recorded above; its assertion is unchanged.
The final exact production equipment/source-profile check also passed (1 test,
71.76 seconds) against the preserved profile and 5802-item native union.

## Koshei physical caller proof

Crystal 201189f8 `chest.lua:2459-2472` independently grants blue legs at
UID6240, (33261,32444,12), appearance2472, and 100 platinum coins at UID6241,
(33261,32448,12), appearance2480. The mounted source-map fixture now preserves
each chest's individual appearance and row. Existing callers retain their
single-appearance wrapper and original placement data.

The literal dispatch PostgreSQL case exercises both orders, reconstructs the
caller and reloads durable tracks between chests, requires incomplete state
after either first claim and completed state only after both, and verifies
exactly one blue-legs item and 100 platinum coins through MINT receipts.
Duplicate command replay and a fresh repeated USE grant no additional value.

Fresh local checks: dispatch all-features/all-target clippy with `-D warnings`
passed; formatting passed; live Koshei case passed in 134.91 seconds with
`M35_KOSHEI_POSTGRES_CASE_COMPLETED`; the unchanged Alawar case passed against
the generalized fixture in 138.11 seconds with its database-execution marker.
No production content, pins, migration or protocol number changed in this slice.

Additional source check: both pinned Crystal maps were scanned for unique ID
1035 under any item carrier. Neither contained it. The global map SHA256 is
`c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85`;
the Crystal custom map SHA256 is
`3021fa0cd15a0a34cb805571f783da0f852441a6bbe98fed99ad277787710817`.
The final Ferumbras action remains source evidence, but these maps do not yet
supply its physical reward-chest binding; no coordinate is fabricated.

## Black Knight physical caller proof

Crystal 201189f8 `chest.lua:932-947` declares two independent source KV rewards:
UID6047 at (32868,31955,11), appearance3634, Crown Armor3381 x1; UID6048 at
(32880,31955,11), appearance3634, Crown Shield3419 x1. The mounted literal USE
case drives both orders through dispatch, reloads durable tracks between chests,
requires incomplete state after either first chest, completion only after both,
exact MINT quantities, duplicate replay, and refusal of another fresh claim.

Fresh checks: dispatch all-features/all-target clippy with `-D warnings` passed;
formatting passed; the live PostgreSQL case passed in154.24 seconds and printed
`M35_BLACK_KNIGHT_POSTGRES_CASE_COMPLETED`. This slice changes no content or pins.
The expanded donor scan also checked every additional pinned OTBM (41 maps,
verifying each Git blob identity), finding no UID1035 under any carrier; together
with the two primary maps, all43 source maps lack that unique binding.

## Fire Axe complete charged bag admission

The new authoring regression first failed: UID6067 had no native projection.
Crystal201189f `chest.lua:1078-1087` grants a brown bag2853 containing Ring of
Healing3098 x1, Dragon Necklace3085 with200 charges, Small Diamonds3028 x7,
and Fire Axe3320 x1. The charge disposition already correctly normalizes the
necklace to one instance at its full200-charge definition; only its historical
canonical materialization hold prevented the complete bag from being served.

The exact `kv/fireaxe1` claim now reuses the established native charged-item
qualification. It requires agreeing canonical/native identity, binding, physical,
stack, charge and custody facts. Neither the canonical Item nor the immutable
source packet changes. Generated native recipes increase71->72, held shapes
34->33, and the exact bag-member closure140->142 (3085 and3320). The native
Item provider is byte-identical to the preceding head; no known facts are lost.

The mounted live dispatch case passed in60.54 seconds, printing
`M35_FIRE_AXE_BAG_POSTGRES_CASE_COMPLETED`. It checks the actual root MINT,
all four nested definitions/quantities, one necklace rather than200 instances,
exact duplicate replay, refusal of a fresh duplicate claim, consumed quest
obligation, and canonical completion=false while `fireaxe2` remains unclaimed.
This proves the repaired bag caller; the complete two-floor reward route still
needs its combined caller proof.

Fresh checks passed: formatter; content+dispatch all-features/all-target clippy
with `-D warnings`; content decoder12 tests; container authoring15 tests;
variant authoring13; callback generator14; existing chest authoring29; native-only
regeneration and its reproducibility check. Generated files came from their
existing generators. Local node boot remains skipped per owner.

## Fire Axe combined two-floor caller proof

The source fixture can now preserve individual floors as well as appearances:
UID6067's bag at(33078,31656,11), appearance2472, and UID6036's separate axe
at(33084,31650,12), appearance4024. A test-only runtime factory re-admits the
same fenced character at each source endpoint, retaining the command stream
and reloading quest state from PostgreSQL. This qualifies reward completion;
it does not claim to qualify travel between the endpoints.

The combined case exercises both orders: neither first chest completes; the
second does. It checks the bag's four exact children, the independent axe root,
both claimed tracks, exactly two consumed obligations/receipts, replay and fresh
repeat refusal. The earlier single-bag test retains its exact expectations.

Fresh dispatch all-features/all-target clippy with `-D warnings` and formatting
passed. Both live Fire Axe tests passed in184.12 seconds, with
`M35_FIRE_AXE_BAG_POSTGRES_CASE_COMPLETED` and
`M35_FIRE_AXE_TWO_FLOOR_POSTGRES_CASE_COMPLETED`. This closes the previously
recorded combined Fire Axe caller-proof gap. No production pins or data changed.

## Emperor's Cookies caller and container-key guard repair

The new literal caller case was RED: the cookie bag was rejected at commit.
The retained PostgreSQL log confirmed `game_item_key_proven` raised
'a key number is set only by the reward-claim MINT of its one item'. Migration
2150 replaced that guard and omitted the complete-container proof arm installed
by0762. The failed commit left a frozen pending claim, with no completed reward
in the RED run. The lane needs both the original container arm and M21's map arm.

New migration2601 restores the already-owned complete frozen-container arm by
inserting it into the installed function definition. It preserves every map,
inbox and root-key arm, the installed privileges and pinned search path. The
proof still requires the current physical transaction and its whole reserved
child plan. It adds no table, privilege or new value-grant API. Migration2601
was the next free number from next-free-numbers.py against origin/main7723ead5a
and origin/qmech856a420d6 plus open PR claims when authoring began. The final
scan saw a new unrelated peer2701, advancing its next suggestion to2702;2601
remained unused in both refs and peer PRs. Existing main peer-PR clashes do not
involve2601; no reference branch was merged.

The GREEN case drives both source endpoints in both orders: UID5004,
(32648,31905,3), appearance2472, bag2853 holding Key3801 (2970 x1) and27 cookies
(3598); UID5005, (32599,31923,6), appearance2472, Key3802 (2970 x1). It checks
exact roots, nested quantities, immutable key numbers, receipts, consumed
obligations and canonical completion only after both claims. Replay and a
fresh repeated USE add no reward. An adversarial insertion of Key3800 onto
the already-minted bag is rejected with SQLSTATE23514 and leaves two key rows.
The live guard retains all four installed provenance-owner references.

Dispatch all-features/all-target clippy with `-D warnings` and formatting pass.
The live case passes in126.35 seconds and prints
`M35_EMPEROR_COOKIES_POSTGRES_CASE_COMPLETED`. Temporary diagnostics were
removed. The forward migration and its real-caller/negative proof form the
coordinator's review packet for this persistence repair; no PR or review was
requested by this single-agent lane.

## Draconia complete charged-weapon bag and caller proof

The new authoring check was RED because UID6023's complete bag was held on the
canonical Ice Rapier3284 materialization flag. Its already-admitted native
profile has exactly one charge and matching physical, custody and weapon facts.
The scoped charged-item qualification now includes `kv/draconia1` and3284's
one-charge definition. It also checks agreeing weapon facts; the regression
rejects changed attack and charge count. The canonical Item and the entire
published native Item provider remain byte-identical to the preceding head.

The generated complete recipes increase72->73, held shapes33->32, and bag-member
closure142->144 (Ice Rapier3284 and Serpent Sword3297). No source quantity,
immutable capture, migration or protocol identity changes. The source roots are
UID6023 at(32803,31582,2) and UID6024 at(32804,31582,2), both appearance2472.
The actual caller grants bag2853 with3284+3297 and bag2853 with3081+3051,
all one instance each. The PostgreSQL case checks either bag first remains
incomplete, both together complete, exact roots/children, consumed obligations,
duplicate replay and refusal of a fresh duplicate, with durable tracks reloaded
between source claims.

Fresh checks passed: formatting; content+dispatch all-features/all-target clippy
with `-D warnings`; content decoder12; container authoring16 (RED before the fix,
GREEN after regeneration); variant authoring13; callback generator14; existing
authoring29; native-only generation/reproducibility. The live case passed
in128.99 seconds with `M35_DRACONIA_POSTGRES_CASE_COMPLETED`. No local node boot.


## Goblin Temple complete water bag and caller proof

The new authoring regression was RED because UID6119's complete bag was held
on its source WATER vial. The existing native fluid MINT now admits this exact
claim alongside Dreamer's already-qualified water claim. It requires UID6119,
`quest_kv/goblintemple2`, vial2874 count1 and subtype1; missing or substituted
subtypes remain rejected by the content decoder. The immutable donor capture
and canonical vial definition remain unchanged.

Crystal 201189f binds UID6118 at(31973,32209,12) and UID6119
at(31977,32209,12), both appearance2472. Their bag2853 contents are sandals3551
x1, small stones1781 x5 and gold3031 x50; and pan3466 x1, snowballs2992 x4
and WATER vial2874 x1. The caller case exercises both orders and checks all
six quantities, root bags, fluid subtype and same-root/same-child transaction
proofs, completion only after both claims, replay, duplicate refusal and
track reload between chests.

Complete native recipes increase73->74, held shapes32->31 and callback child
closure144->146. The native provider adds only pan3466 (5802->5803 identities).
The earlier callback generator now seeds Snowball2992 from its retained
seasonal profile, preserving every existing known semantic and attribute;
its canonical trade policy becomes known through the existing admission pass.
No migration, protocol or source quantity changes.

Fresh checks passed: formatting; content+dispatch all-features/all-target clippy
with `-D warnings`; content decoder12; container authoring17 (RED before the
fix, GREEN after regeneration); variant authoring13; callback generator15;
existing authoring29; native-only generation and reproducibility. The live
caller passed in134.43 seconds and printed
`M35_GOBLIN_TEMPLE_POSTGRES_CASE_COMPLETED`. Local node boot remains skipped
as instructed. The previously recorded final-binding, partial-charge and
remaining caller-proof gaps still prevent a complete26-quest DONE claim.
The full pinned-production equipment/attribute check also passed in76.86
seconds with the exact5803 profile union. Self-review covered all handwritten
changes and semantic generated deltas; no significant finding. Independent
coordinator review and CI are not claimed by this lane.

## Crystal Wand real-caller qualification

Crystal 201189f8 `chest.lua:1540-1554` binds UID6124 at(32479,31611,15)
and UID6125 at(32481,31611,15), both appearance2472. The independent rewards
are item3068 x1 and item11609 x1. The mounted PostgreSQL case uses those
literal endpoints and the production wire USE/RewardClaim/quest-obligation
path. In each order it requires the first reward to leave the quest incomplete
and the second to complete it, with exact persisted rewards, consumed
obligations, duplicate replay, fresh-repeat refusal and track reload between
claims. Existing production group declarations and grants remain unchanged.

Fresh validation: formatting and dispatch all-features/all-target clippy with
`-D warnings` passed. The live PostgreSQL case passed in135.59 seconds and
printed `M35_CRYSTAL_WAND_POSTGRES_CASE_COMPLETED`. Self-review found no
significant issue; existing assertions are retained and exact-reward checks
are extended. No generated content changed. The full M35 scope remains PARTIAL
for the source-binding, partial-charge and remaining caller-proof gaps above.

## Circle Room real-caller qualification

Crystal 201189f8 `chest.lua:1525-1539` binds UID6122 at(32495,31992,14)
and UID6123 at(32497,31992,14), both appearance2469. The independent rewards
are war hammer3279 x1 and dwarven axe3323 x1. The mounted case drives literal
wire USE through production dispatch, durable MINT and quest obligations.
Both chest orders require incomplete state after the first claim and complete
state only after the second. It checks exact reward identities and quantities,
receipts, consumed obligations, replay and fresh-repeat refusal, with a quest
state reload between claims. No production declaration or grant is changed.

Fresh validation passed: formatting; dispatch all-features/all-target clippy
with `-D warnings`; live PostgreSQL18.6 caller case in147.08 seconds, with
`M35_CIRCLE_ROOM_POSTGRES_CASE_COMPLETED`. The full handwritten diff was
reviewed against the pinned donor endpoints and quantities; no significant
finding. No assertion was removed or loosened, and no generated file changed.
M35 remains PARTIAL for the previously documented final-source binding,
partial-charge and remaining real-caller proof requirements. Node boot was
skipped as instructed.

## Demona Ring and Double Hero real-caller qualification

Crystal 201189f8 `chest.lua:1555-1569` binds Demona Ring UID6126
at(32504,31596,14) and UID6127 at(32515,31596,14), both appearance2472,
with stealth ring3049 x1 and energy ring3051 x1. `chest.lua:819-833`
binds Double Hero UID6032 at(33109,31679,13) and UID6033
at(33110,31679,13), both appearance2469, with red gem3039 x1 and club
ring3093 x1. Each mounted PostgreSQL case drives both source orders through
literal wire USE, production RewardClaim MINT and durable quest obligations.
The existing completion, replay, fresh-repeat refusal, receipt and obligation
checks remain strict; exact persisted reward assertions cover each new pair.
Neither first chest may complete its quest, and state is reloaded between
claims. No production content or grant behaviour changes.

Fresh validation passed: formatting; dispatch all-features/all-target clippy
with `-D warnings`; both live PostgreSQL cases (2 passed, 0 failed) in274.78
seconds, printing `M35_DEMONA_RING_POSTGRES_CASE_COMPLETED` and
`M35_DOUBLE_HERO_POSTGRES_CASE_COMPLETED`. Self-review covered the complete
handwritten delta and donor source records; no significant finding. No
assertion was removed or loosened and no generated file changed. Node boot
was skipped as instructed. The documented final-binding, partial-charge and
remaining caller-proof requirements still leave the complete M35 row PARTIAL.

## Katana corpse-pair real-caller qualification

Crystal 201189f8 `chest.lua:1850-1864` binds UID6162 at(32175,32145,11)
and UID6163 at(32174,32149,11), both appearance4240. The independent
rewards are item3367 x1 and item3300 x1. The mounted test preserves both
literal positions through the existing source-world helper and drives actual
wire USE through production RewardClaim MINT and quest obligations in both
orders. It checks exact durable rewards, incomplete state after the first
claim, complete state after the second, receipt and consumed-obligation
counts, replay and fresh-repeat refusal, and state reload between claims.
No production declaration, reward data or generated pin is changed.

Fresh validation passed: formatting; dispatch all-features/all-target clippy
with `-D warnings`; live PostgreSQL case in138.05 seconds, printing
`M35_KATANA_POSTGRES_CASE_COMPLETED`. Self-review covered all handwritten
changes against donor endpoints and quantities; no significant finding. No
existing assertion was removed or loosened. Local node boot was skipped as
instructed. The complete M35 row remains PARTIAL for the final-source binding,
partial-charge and remaining caller-proof requirements documented above.
