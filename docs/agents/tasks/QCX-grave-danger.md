# QCX45 Grave Danger Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

- Quest facts were fetched from `qcx-facts/grave-danger.md`. The base page gives level 250, recommended level 300, premium, and the basic Hand of the Inquisition Outfit plus Inquisition's Hand. It lists no fixed item or XP reward. [Quest, curid 88736, oldid 993205](https://tibia.fandom.com/?curid=88736&oldid=993205)
- The spoiler gives Jack Springer's `late` start, the holy-water trade, twelve graves, the `report → ultimate → threat` sequence and Isle of the Kings access, and the final report to Jack. [Spoiler, curid 88737, oldid 1176165](https://tibia.fandom.com/?curid=88737&oldid=1176165)
- Crystal was checked at `201189f871d23597b82caf88fd611454a2ddefb8`. The captured Jack Springer source matches the existing import's digest. [Crystal NPC](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/jack_springer.lua)

## Corrected

- Jack's `late` used to refuse the entire start. The existing writer supports 16 atomic tracks on main, including the twelve imported grave tracks and the questline. The start now advances all thirteen in one receipt.
- Stage-one `report` used donor-authored missing-graves dialogue. The generator now uses the pinned global transcript's holy-water and grave-location reply.
- The existing Jack offer sells the Vial of Potent Holy Water as item 31612 at 100 gold according to the imported donor offer. The pinned wiki confirms the NPC and trade keyword. The existing Jack topic sequence for Zelos and the Isle teleport already match the source sequence and donor geometry.

## Remaining differences

- The definition has level 250 and premium. Its prerequisites are empty and it treats all Cobra/Scarlett steps as required. The spoiler lists travel money, a Cheesy Figurine and four Kilmaresh puzzles under Requirements, describes six puzzles on the actual route to the Kilmaresh grave, and uses the figurine specifically for the optional Cobra chess route. Gaffir grants basement access before the grave north of Custodian. Full Cobra access and Scarlett are optional.
- The QUESTPC7 approach starts at Edron and chains twelve visits with teleports, then five Lich Knight encounters, Zelos, Cobra, and Scarlett. Global starts with Jack and lets the player visit the graves by travelling to their locations. The approximation treats the graves alike; the spoiler distinguishes boss encounters at Edron, Ghostlands, Cormaya, Darashia and old Thais, water use at Dark Cathedral, Cobra Bastion and Kilmaresh, and already-defiled graves at Femor Hills, Vengoth, Orclands and Folda. The two general world callbacks are position-based Cobra access and the Zelos lever-area teleport. Those callbacks retain Crystal positions. The QUESTPC7 alternate route uses placed appearances and synthetic arrival/exit paths in its own generator.
- The spoiler specifies repeatable boss admission every 20 hours, five players for the Lich Knights, ten for Zelos, and 24 minutes for the Zelos run. The selected route uses native death objectives and fixed guardians. It does not reproduce party admission, encounter phases, the four timed Zelos minibosses or the effects of ritual duration.
- The definition lists the basic outfit and achievement with no item or XP claims. The active QUESTPC7 reward is on armour after Scarlett. Jack's stage-four final report is held, and the global reward requires Jack after Zelos. The donor gives base outfits 1244 and 1243 and the achievement with stage 5. The existing QUESTPC7 world reward issues the canonical base outfit and achievement. The cosmetics authoring maps the two donor look types to the same canonical outfit. The source NPC final still needs a binding to that reward and completion.
- Crystal's legacy access repair marks seven non-boss graves visited and promotes the quest after the five combat encounters. Global requires the grave route before Jack's report. Crystal's six-minute Zelos damage threshold also differs from the spoiler's approximate twelve-minute context. These donor shortcuts and tuning were not imported into the corrected Jack path. [Legacy access](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grave_danger/grave_danger_access.lua#L21) · [Zelos ritual](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grave_danger/grave_danger_king_zelos.lua#L105)
- No Grave Danger rows were found in the general world-callback held shards or reward-claim catalogue. The Jack source-only final is a separate record.

The B3 placement audit found these bindings. Coordinates use donor-positive floor numbers.

| Site | Imported placement facts | Difference |
| --- | --- | --- |
| Cobra approach `33384,32627,7` | Item 16489, no unique or action attribute | The Crystal step writes access from position. It supplies no Gaffir credit check. |
| Isle of the Kings `32173,31922,8` and `32174,31916,8` | Door 6260 has action 46890. Vortex 23482 carries destination `33492,31546,13`. Neither has a unique attribute. | Both match imported door and teleport evidence. The global report prerequisite still needs authentic grave progression. |
| Lich Knight and Zelos levers | All six positions from Crystal have item 8911 without unique or action attributes. | The selected route uses scripted positioning and separate death objectives in place of these lever registrations. |
| Scarlett `33395,32660,6` and `33398,32640,6` | Lever item 31484 and armour 31482 have no unique or action attributes. | Crystal positions the lever directly and registers armour and mirrors through action 40003. The selected armour placement uses position and appearance for quest completion, which differs from both Crystal and global. |

[Crystal Cobra movement](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grave_danger/cobra_bastion/movements_access_storage.lua#L3) · [Crystal Scarlett action](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grave_danger/cobra_bastion/actions_scarlett.lua#L72)

The following differences are in the spoiler's site and boss sections. The active selected route uses single-form guardians, scripted movement steps and death credit. [Spoiler, curid 88737, oldid 1176165](https://tibia.fandom.com/?curid=88737&oldid=1176165)

| Site | Global behavior absent from the selected route |
| --- | --- |
| Edron | Quest door and vortex to Count Vlarkorth's lever. Soulless Minions heal the boss and summons. The invulnerability phase summons a Dark creature per player vocation, including Monk, and ends through the matching corpses. |
| Ghostlands | Crypt vortex and Lord Azaram's soul phase. Tainted Splinters become Cleansed Splinters used to heal his soul three times. Condensed Sins and Lesser Hex affect the fight. |
| Cormaya | Graveyard hole and vortex. Earl Osam becomes an invulnerable Channeling Earl Osam. Spheres reaching him heal 80,000 HP each. Frozen Souls and teleporting adjacent players are part of the encounter. |
| Darashia | Five Sacred Statues and the Sundial open the stairs. Sir Nictros and Sir Baeloc alternate, then fight together with summons, Lesser Hex and healing tied to their relative health. |
| Old Thais temple | Grave vortex to Duke Krule. Water and Fire Elemental transformations rotate, and proximity between opposite transformations causes damage. Soul Scourges accompany the boss. |
| Cobra Bastion | Gaffir's death allows basement entry and the grave north of Custodian. The optional continuation includes Quaid, chess and Scarlett. |
| King Zelos | Four minibosses gate the north teleport. Rewar is vulnerable between Fetters, The Red Knight takes drowning damage from Vampiric Blood deaths, Magnor splits into health-sharing shards, and Nargol regenerates if its mass survives 30 seconds. Greater Hex reduces health to 40% and can be cleared at the vortex at the cost of spawning Unleashed Hex. Risen Soldiers respawn. |

The spoiler's opening summary groups six visits with holy water. Its Femor Hills, Vengoth, Orclands and Folda sections describe already-defiled graves with no further action. The detailed sections support the table above.

## Blockers

- BLOCKER King Zelos ritual encounter orchestration with time-dependent phases and group timeout `data-global/scripts/quests/grave_danger/grave_danger_king_zelos.lua:105`
- BLOCKER Lich Knight completion full encounter phase and party credit integration `data-global/scripts/quests/grave_danger/actions_bosses_killed.lua:11`

## Verification

- Quest authoring `--part checks` passed all 44 commands, including 269 schema cases. The quest dialogue authoring suite passed all 5 cases.
- `cargo fmt --all -- --check` passed. Pin regeneration and pin-check both verified the existing world digest `0676b1568f55a55fbe994f344a8e0c60debf766751d12849f56c19f60c93aae5`.
- PG via shard (pgsplit build files applied on `c44eb69f9165250f009c135a8e5e8fc1f4b87ff3`): `jack_starts_twelve_graves_and_answers_an_early_report` passed in 46.68 s with live PostgreSQL. The route-less Cargo run passed in 0.02 s. Dispatch Clippy passed with `-D warnings`.
- The earlier 42.83 s live run failed in fixture setup because the admitted player had no position. The fixture now initializes the pinned position before placing Jack.
- With the base quest runtime restored locally, the same live case failed in 38.46 s because `late` returned `Rejected`. The fixed dialogue fixture and the shard were held constant.
- With the base dialogue restored locally and the fixed start enabled, the live case failed in 56.72 s because `report` returned the donor reply instead of the pinned global transcript.

## OPEN-FACTS

- Exact global numeric IDs for the outfits, quest items and placement uniques/actions, holy-water price and consumption, exact travel costs and level gate locations, and a global resolution for the four-versus-six Kilmaresh puzzle wording. Donor-only numbers are identified as such.

## PARTIAL

- Global grave routing and progression, Cobra/Kilmaresh prerequisite binding, removal of mandatory Scarlett, final NPC reward, and encounter/cooldown parity.
