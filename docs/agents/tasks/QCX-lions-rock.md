# QCX-83 · Lion's Rock Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Quest identity
`oteryn:quest.lions_rock`. Runtime source callbacks use the Tibia Tales
quest owner. Global sources are the captured [quest page, curid 69459,
oldid 846852](https://tibia.fandom.com/?curid=69459&oldid=846852) and
[spoiler, curid 69463, oldid 1041971](https://tibia.fandom.com/?curid=69463&oldid=1041971).
Crystal fallback is `201189f871d23597b82caf88fd611454a2ddefb8`.
The [earlier import disposition](../evidence/quests-4-20261102/lions_rock.json)
records placement and donor-script evidence. Later M07 and M44 runtime
work now covers several of its held steps. No Lion's Rock record is present
in the current `world-callbacks-held` catalogue. Numeric item cross-references use
[Crystal items.xml](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml)
and the checked-in global own-item-ID index under
`imports/tibiawiki/source-evidence/stack-default-successor8/`.

| Area | Global and donor comparison against main |
| --- | --- |
| Requirements | Global requires premium, has no mandatory level (70 recommended), and identifies a Cobra Tongue for the mission and four small gems for the daily task. The chosen definition has premium and minimum 0. M07 trials and pickers check premium. The Mystic Flame gate checks the progress counter, and the M44 flower and amphora callbacks have no local premium guard. The spoiler gives a quest-wide premium requirement without per-callback rules for lapsed accounts. No NPC starts or NPC offers are specified by the spoiler or the two donor quest scripts. The quest dialogue source has zero entries, and no quest-owned NPC offer or callback was found. |
| Chosen quest definition | Its eight-stage approximation sequences Strength, Beauty and Tears, names only one inscription and ends at a chosen Lion's Heart cache. Global allows any trial order, uses four inscriptions and grants its random reward from a shared, repeatable fountain. The active Tibia Tales callbacks and the non-admitted chosen stage graph remain separate runtime concerns. |
| Above ground | The admitted well at (33137, 32351, 6) awards one Ewer with Holy Water (21466) per use, matching the donor's unconditional repeatability but differing from the spoiler's short lockout. Two M07 and six M44 bindings handle renewable flower sources (21388 → 21935 and back after the donor’s 60 seconds), each granting one portable flower (21389). The spoiler does not give flower regrowth duration. |
| Dead Explorer | The spoiler calls for an Old Parchment (Brown) (21413) from the corpse. Crystal identifies Dead Explorer as 21412. The expected floor-8 region contains no placement of 21412. No Lion's Rock reward claim or corpse selector was identified. Crystal's two quest scripts do not implement this pickup. |
| Three trials | M07 consumes one Cobra Tongue (9634), one flower (21389), or one ewer (21466) at the corresponding native statue, basin, or pedestal, in any order. The once-per-character flags advance the shared quest counter and reset the lit pagoda (2074 → 2075) using the donor's 15-second reset. The Tears feedback had donor typos and punctuation. The authored M07 callback and the quest-specific Rust binding now use the global text verbatim, including the doubled space before “enter”. |
| Sanctum access | Existing Mystic Flame STEP (33128, 32308, 8) requires questline ≥4, sends the source success text, and teleports the player to (33122, 32308, 8). It preserves progress for repeat entry and returns a refused player to their origin. Donor teleport and refusal visual effects are absent. Separately, the chosen M01B area-entry callback records a visit and does not control sanctum access. |
| Scroll | M44 handles eight STEP tiles around the amphora (33119, 32247, 9), one Translation Scroll (21467), questline 4→5 and the donor’s 15-second reset. The four follow-up inscription readers are absent. Global wording differs from the donor's two-message “he tooks” split. |
| Daily sockets | No active or held Lion's Rock socket callbacks were found. Global maps ruby (3030) to northwest/hyaena, sapphire (3029) to southwest/nest, topaz (9057) to northeast/snake, and amethyst (3033) to southeast/scorpion. Players can insert gems while their own reward is on cooldown. The fourth sacrifice reveals a shared fountain that another player can use. The Crystal movement and action files omit the socket handlers. |
| Fountain and mount | The authored definition describes a once-only chosen approximation with achievement and fixed Gold Ingot, Yellow Gem and unresolved Red Gem intents. There is no admitted Lion's Rock reward claim for the daily fountain. Global lists one random item (count 1) from six options, with a 24-hour repeat, excludes the Heart for Noble Lion owners and awards Lion's Den Explorer (`oteryn:achievement/lion_s_den_explorer`). Crystal additionally lists Green Gem (3038) and uses an unqualified pearl name. IDs confirmed by Crystal `data/items/items.xml` are Lion's Heart 21439, Gold Ingot 9058, Yellow Gem 3037, Red Gem 3039 and Emerald Bangle 3010. Crystal's pearl name covers 281 and 282 in the XML. The repository's pinned global item observations identify Brown as 282 (page 64268, revision 1011763), and the Brown parchment as 21413 (page 69846, revision 1014182). The initial rock (1852) at (33073, 32300, 9) is intentional. The fountain (6389) is a shared transformation. Global describes taming the Noble Lion with the Heart, matching the admitted pair in `content/cosmetics/taming-native.json`. The temple arrival message already has a once-only callback. Neither global page lists a direct XP or outfit reward. |
| Placements | The active trials and pickers use native position, appearance, and ordinal selectors. The gate, amphora ring and temple intro use movement positions. The gate tile has floor 19275 and flame 1959. The temple intro has floor 499 and prop 6381. Both positions lack unique/action attributes, and position-only STEP selectors bind the floor ordinal. The decoded floor-9 room has socket tiles 537/538/539/540 at (33072, 32299), (33074, 32299), (33072, 32301), (33074, 32301) respectively, with no unique/action attributes. The daily room and the explorer still lack executable binding coverage. |

## Blockers and open facts

The paths below refer to the pinned Crystal donor. Main supports elapsed-time
gates for fixed item rewards. The well remains open pending a confirmed
duration, and the timed fixed-grant path alone cannot implement the fountain
pool. The shared `present` predicate can check a carried Translation Scroll
without consuming it on STEP. The inscription callbacks remain partial.
The spoiler identifies the objects and gives their messages. Its map markers
do not distinguish exact trigger tiles from the inscription objects, and
its text does not specify the state changes Crystal applies on each reading.
Crystal sets one private flag per sign and advances the questline by one
when the reader is carrying item 21467. Its STEP witnesses on floor 9 are
snake (33095, 32244), lizard (33130, 32300), scorpion (33109, 32329), and
hyaena (33127, 32340). [Crystal movements_lions_rock.lua:1–126](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/lions_rock/movements_lions_rock.lua#L1-L126)

```text
BLOCKER daily sockets shared four-offering window and room transformation data-global/scripts/quests/lions_rock/actions_lions_rock.lua:115-118
BLOCKER daily fountain atomic repeatable random claim with mount-filtered pool, 24h clock, achievement and shared reset data-global/scripts/quests/lions_rock/actions_lions_rock.lua:115-139
```

OPEN-FACTS — holy-water lockout length (the spoiler says “1 day?”), flower
and amphora global reset lengths, socket flame durations and exact native issue/revert rules, reward weights,
explorer corpse selector, exact inscription STEP tiles and state changes,
per-callback premium behavior on lapse, and whether donor-only magic effects match global. The spoiler gives one
Tears message and does not specify alternate feedback for a different trial
order. Crystal sends the Tears line whenever the Tears offering succeeds.
Donor IDs and counts are cross-reference evidence where the captured wiki
names the entity without its native ID.

PARTIAL — fountain reward/claim and its chosen definition approximation,
gems and room reveal, four inscription readers, well lockout, explorer
pickup, and source visual effects.

## Verification

- The pre-fix PostgreSQL case reached the native pedestal USE on PostgreSQL
  18.6 and observed the original Tears feedback. The new assertion failed in
  45.39 s. The authoring assertion also failed on the original text.
- PG via shard (pgsplit build files applied on
  `308631d0a28dca8257195633a876f2f0367ca1ee`). The Tears case passed in
  38.06 s, with 1 passed and 0 failed. It covers the committed feedback,
  exact item consumption, progress and pagoda reset, and refusal of a second
  offering.
- The same filter without the PostgreSQL URL returned `Ok(())`.
  One test passed in 0.00 s through the route-less guard.
- The focused Lion Tears authoring module passed all 3 tests.
  `cargo fmt --all -- --check` passed.
- `oteryn-world-bundle-compiler pin-refresh .` retained the existing pin.
  `oteryn-world-bundle-compiler pin-check . /tmp/qcx83-world-pin-check`
  passed for the single Oteryn bundle. This compiler requires an output
  directory for `pin-check`.
- The requested quest-authoring check completed all 44 commands successfully.
- Dispatch library Clippy passed with all features and `-D warnings`.
