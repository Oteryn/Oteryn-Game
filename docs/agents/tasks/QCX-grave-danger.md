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
- The QUESTPC7 approach starts at Edron and chains twelve visits with teleports, then five Lich Knight encounters, Zelos, Cobra, and Scarlett. Global starts with Jack and lets the player visit the graves by travelling to their locations. The approximation treats the graves alike; the spoiler distinguishes boss encounters at Edron, Ghostlands, Cormaya, Darashia and old Thais, water use at Dark Cathedral, Femor Hills, Cobra Bastion and Kilmaresh, and already-defiled graves at Vengoth, Orclands and Folda. The two general world callbacks are position-based Cobra access and the Zelos lever-area teleport. Those callbacks retain Crystal positions. The QUESTPC7 alternate route uses placed appearances and synthetic arrival/exit paths in its own generator.
- The spoiler specifies repeatable boss admission every 20 hours, five players for the Lich Knights, ten for Zelos, and 24 minutes for the Zelos run. The selected route uses native death objectives and fixed guardians. It does not reproduce party admission, encounter phases, the four timed Zelos minibosses or the effects of ritual duration.
- The definition lists the basic outfit and achievement with no item or XP claims. The active QUESTPC7 reward is on armour after Scarlett. Jack's stage-four final report is held, and the global reward requires Jack after Zelos. The donor gives base outfits 1244 and 1243 and the achievement with stage 5. The existing QUESTPC7 world reward already issues a canonical base outfit and achievement, but the source NPC final still needs a binding to that reward and completion.
- No Grave Danger rows were found in the general world-callback held shards or reward-claim catalogue. The Jack source-only final is a separate record.

## Blockers

- BLOCKER King Zelos ritual encounter orchestration with time-dependent phases and group timeout `data-global/scripts/quests/grave_danger/grave_danger_king_zelos.lua:1`
- BLOCKER Lich Knight completion full encounter phase and party credit integration `data-global/scripts/quests/grave_danger/actions_bosses_killed.lua:1`

## OPEN-FACTS

- Exact global numeric IDs for the outfits, quest items and placement uniques/actions, holy-water price and consumption, exact travel costs and level gate locations, and a global resolution for the four-versus-six Kilmaresh puzzle wording. Donor-only numbers are identified as such.

## PARTIAL

- Global grave routing and progression, Cobra/Kilmaresh prerequisite binding, removal of mandatory Scarlett, final NPC reward, and encounter/cooldown parity.
