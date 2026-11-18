# QCX65 Hidden Threats Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. The Global evidence is
`qcx-facts/hidden-threats.md` from `460e1d8c2c0e88c438ae5e54910db00822dc0129`,
captured 2026-11-17. [Quest page, curid 83384, oldid 849215](https://tibia.fandom.com/?curid=83384&oldid=849215) · [Spoiler, curid 83388, oldid 1200776](https://tibia.fandom.com/?curid=83388&oldid=1200776)

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies implementation
evidence. Global wins on the quest route and rewards. [Donor scripts](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/hidden_threats)

## Comparison

| Part | Global and Crystal evidence | Base behaviour and disposition |
| --- | --- | --- |
| Entry requirements | Level 0, free account, recommended level 50 and rope or equivalent for traversal | The definition already has 0 and free. The chosen entry refers to surveying an inner chamber. The actual cave-to-mine route and rope traversal still need map qualification. |
| Ratter and Servant access | Ratter `hi → help → yes`, then Servant `hi → decreased resources → defy`. The transcript and Crystal Servant line 111 use `decreasing resources`. | Ratter already writes the first access track. Servant only matches the transcript wording on base. `defy` already writes the second access track. |
| Three boxes | One Metal File, one rusted head and one rusted handle | Ready once-per-character claims hold 1×27270 at (33078,31980,13), 1×27262 at (33080,32014,13) and 1×27261 at (33031,32050,13), source uniques 6094, 6095 and 6096. The base also targets the head box with the chosen reward callback. The base map has appearance 2469 at the three claim cells and omits the source uniques there. The spoiler map links for the file and handle point to nearby tiles without a box, so the claim positions follow the actual boxes. |
| Forging | File both fragments, heat in Crucible lava, combine, hammer on the small anvil, then quench in water | No Hidden Threats forge callback is active or held in the world callback directories. Crystal `action-key_forge.lua:3–31` covers heating, combining, hammering and quenching. Crystal also accepts a pair of the same glowing fragment. Global calls for one head and one handle. Metal File target rules are separate from this script. |
| Workrooms and progress | Unlock all three doors with the forged key. Worker visits for gemstones are optional. | The NPC programme requires quest line 3 and eight rescue credits before the Servant offers the hand-in. Crystal Servant lines 65–85 also require rescues. The base lacks the source chest step-out progression and forged-key callbacks. The five base door bindings and placements already map actions 45972–45976 to Ratter, Servant and the three workrooms. The missing key-use callbacks leave the workroom access tracks without the Global trigger. |
| Worker gems | 1×3030 Small Ruby NW, 1×3032 Small Emerald NE, 1×3033 Small Amethyst SW | Workers 1 and 3 already issue ruby and emerald. Worker 5 uses 3029 Small Sapphire, matching Crystal `data-global/npc/corym_worker_05.lua:62`. Crystal places Worker 5 in the SW room at (33008,32051,12). The pinned item XML names 3029 and 3033. |
| Rare Earth | 20 Rare Earth from Stonerefiners traded to the Servant for 2×3040 Gold Nugget | The existing NPC exchange already consumes 20×27301 and grants 2×3040. The eight-rescue prerequisite differs from Global. The separate chosen chest grants the nuggets and all three gems after five Coryms without the hand-in. |
| Exit and completion | Unfinished exit goes back to the cave. After the hand-in, exit alone into the Cave Spider room with three Tarantulas and a five-minute deadline, then leave after killing the boss. | The (33040,32081,12) teleport points to (33056,32004,11). No Hidden Threats source route for the timed encounter is installed. The chosen entry, Corym death objectives and chest in `questpc7-*` award quest completion at the valuables box instead. |
| Quest log and other rewards | Tibia Tales. The two Global revisions list the gems and nuggets as rewards. | The base has a separate reward-only identity and a Hidden Threats mission under Tibia Tales. No XP, achievement or outfit grant was verified for this quest. |
| NPC presence | Crystal lists the Ratter, Servant and eight workers in the mines. | NPC definitions and quest rules are present. The published NPC placement shards contain no Corym quest NPC records. Qualifying their exact cells and the door route is pending. |

Sources for item IDs, NPC cells and donor differences. [Items](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml) · [NPC positions, lines 3038–3066](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world-npc.xml#L3038) · [Worker 5, line 62](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/corym_worker_05.lua#L62) · [Servant, lines 65–138](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/corym_servant.lua#L65)

## Corrections and evidence

- Corym Servant accepts both spoiler spellings. The walkthrough spelling now reaches `defy` and writes the second-floor access track. [Spoiler, oldid 1200776](https://tibia.fandom.com/?curid=83388&oldid=1200776)
- Worker 5 grants one Small Amethyst (3033), with the existing once-per-character rescue guard. [Spoiler, oldid 1200776](https://tibia.fandom.com/?curid=83388&oldid=1200776)
- The Tibia Tales predecessor registry retains the exact definition hash observed on the base NPC catalogue, `8d75517e41359a25abea653c8fd44fa5b4d2768c1470d9acaa9442b73f543065`. Only the Hidden Threats registration is added to the shared registry.
- PostgreSQL baseline with the pgsplit build files applied on `2ece2eb0da2efb38c28383058d0d6b5bbed377c9` and the completed local fixture. The prompt returned no quest reply in 49.44 s. The worker issued zero amethysts in 26.85 s. Both failed their intended assertions.

- PG via shard (pgsplit build files applied on `5b01ca8a5362b70b8f54057558eeece160511a90`) passed 2/2. Servant wording and durable access took 45.42 s. The worker reward and repeat greeting took 33.59 s. The cases dispatch against real PostgreSQL with fixture NPC placements and verify acceptance of the exact prior Tibia Tales definition.
- The same filter without the admin URL returned `Ok` for both cases in 0.00 s. These exercised the fallback only.
- Quest authoring passed all 44 check commands. `cargo fmt --all -- --check` and a direct format check of the auto-mounted test passed.
- Dispatch library Clippy passed with all features and `-D warnings`.
- World `pin-refresh` reproduced the existing Oteryn bundle digest. `pin-check` passed for the one world.

## Remaining work

`BLOCKER Cave Spider room atomic solo teleport and timed player ejection data-global/scripts/quests/hidden_threats/movement-cave_spider_room.lua:57`

Main’s personal STEP encounter timer removes its spawned actors. The callback admission contract forbids combining that timer with a teleport, and timeout does not move the player. [Admission](../../../crates/gameplay/quests/src/quest/world_callbacks/personal_trial.rs#L6) · [Timeout](../../../crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/timed_encounter.rs#L174)

Crystal changes the rescue track on entry. The spoiler requires boss completion
after a kill and exit. The source script has no kill handler to resolve the
durable credit or retry rule.

**OPEN-FACTS** — Exact Global action and unique IDs, mapped NPC positions,
forge target subtype and consumption/decay details, Rare Earth drop rates, boss kill-credit and retry
rules, and whether there are additional XP, achievement or outfit awards.

**PARTIAL** — Source NPC population, entrance and rope route, box progression,
forging, three forged-key doors, optional-rescue hand-in gate, boss and end
state, and retirement of the separate five-Corym approximation.
