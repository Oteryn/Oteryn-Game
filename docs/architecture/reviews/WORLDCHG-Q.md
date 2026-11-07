# WORLDCHG-Q — shared World owner and native bindings

Branch: `claude/happy-davinci-ijz3wy-worldchg`. Continues `3722c495c` through shared-counter batch `318ab59d200ad2030ca7c829566b88175cb7e2dc` and pushed native batch `bd380a5e4`. Fast-forward pushes only, authenticated as `blakinio`; no PR.

## Scope and qualification

Qualification covers the shared World binding and its native execution seams: committed daily decisions, active/stage queries, attributed progress, current-world callbacks, map sources and phased spawns. Quest, Item, XP, Achievement, Spawn and RAID owners keep their existing authority. WVAR, WCONTRIB and WEVENTS own the transferred titles. Their catalogue entries are preserved.

Authored inventory: **12 QUALIFIED, 0 READY, 31 HELD**. All eleven titles in this task’s final scope are QUALIFIED. The thirty-one HELD rows belong to the explicitly transferred WVAR, WCONTRIB and WEVENTS tasks; their catalogue bindings are unchanged.

## Durable owner and physical application

`world_event::changes` remains the one owner. Migration1130 adds its World counters/immutable source receipts/personal observers;1131 indexes the existing gobbler cooldown receipts;1132 adds Voice to the same counter table and its personal encounter-period field;1133 extends the existing Quest consumption proof for one closed Benevola handover. Migration numbers were checked against main91afbe0ba1 and open PRs; main max1095, open claims969/994/995/996/998. Existing migrations and immutable day rows are preserved.

Daily selection stays in the existing server-save transaction and purpose-separated seeded RNG. Save transitions lock the same sorted World rows. The native death queue captures the real wild creature, home cohort, epoch/generation and full death identity; receipt and progress commit together. Replay cannot substitute the principal. Voice requires an owned wave/home slot, never an ambient servant name. A global25-bit mask deduplicates a wave slot across Channels.

Physical changes use the existing Spawn home-point owner, warning/occupancy cadence and budgets. Suppressed points cannot respawn; administrative retirement produces no kill, corpse, XP or loot. The existing event supervision projects phases under current node/scope/reset authority and the same open World transaction, with a brief synchronous runtime try-lock after database authorization; a busy runtime retries without holding its mutex across database awaits. Deadline reads use the database clock even before the next physical tick. Epoch-zero and legacy decisions without counters stay inactive for new counter effects.

## Source evidence and explicit native choices

- `credited-kills.json` separates retained source facts from authored thresholds/geometry/composition. Deepling200points, deer1000/save, weighted wolf recovery100/save and Fire199permille are explicit native parameters. Serpent1000/2000, Mage2000/five minutes, Hive200points versus200actions and source save ages keep their distinct units.
- Existing native spawn points supply faction/Seacrest/Hive/volcano membership. Fury sea-serpent sources and Vandura Hive Outpost stay outside these projections. Four exact retained held boss placements are phase-bound through Spawn. One existing volcanic Fire Elemental home supplies the authored active Dragonling opportunity.
- Shared-object geometry is explicitly chosen and checked against retained B3 plus admitted terrain/item physics. Antenna33443,31310,7; digestion33446,31309,7; pores33449,31309,7; coral33640,31379,9 replace blocked candidate tiles. Guardian33553,31624,14 and its three adjacent orb sources replace void candidate coordinates. Navigator uses admitted adjacent33638,31377,9. Orockle/Benevola retain their sourced anchors.
- Hive uses full accepted mission objectives, including crawler8 AND hatchling8 for chitin. Native extraction/drum stations are execution adaptations; Orockle supplies tools through his sourced six-offer shop. Shared credit follows the completed NPC receipt, not individual objective clicks or raw Kollos kills. Fifty Favour and seven days are retained facts; per-floor eastern/western gate membership and entry sides are explicit native composition from the B3 gate/stair geometry. Outward travel bypasses only stage and scent, preserving current Character/node/map/content/reset proof and original source CAS.
- Twenty Miles wiki69583@1149608 supplies Mind Stone/Seacrest Pearl/Gloo and the three energies before Guardian NPC completion. Gloo stays present through the existing in-place item replacement proof. Guardian is a sourced NPC, not an invented boss. Receipt-backed energy entitlements and the fixed orb geometry are native adaptations. Yeo’s repeatable ten-Shrimp barter consumes actual custody through the existing NPC Trade owner. Three retained native drop sources and their exact corpse/decay closure supply Mind Stones, Shrimp and Seacrest Pearls; source probabilities and quantities stay unchanged.
- Grimvale deliberately uses the already-admitted XML61s Feroxa variant. The Lua5% startup/15+30min/20h sequence is a different recipe. The physical source is actual ordinal1 at33430,31537,z11; no absent B3 AID is fabricated. Source-aware floor resolution restores original1949 travel when the appearance is restored.
- Mage uses actual10840 ordinal1 at33143,31527,z2 and10842 ordinal2 at32942,31592,z8. Retained oramond movements13405/13406 supply destinations; B3 attributes remain unchanged. Only entry requires a current active Mage state; escape remains stateless.
- Voice retains100gobbles,25×25 servants,10/30/60 creature choice, three Mage choices, first-use1h restore, integer-second cooldown and20XP/use.25 personal uses qualify participation and do not cap extraction. Its native composition uses player-attributed kills, matching the shared credited-kill owner; the donor generic onDeath accepted any killer.
- The 41-item supplement reproduces pinned Crystal XML`c847293e…` and official appearance`2dfa943b…` through the existing normalizer/provider. Reserved sprite12064 is excluded. Source portability, stacking and ground speeds are preserved; fixed callback custody independently sets bound/no-pickup. No raw third-party assets are added.

## CALLBACKS FOR QUEST LANES

1. In the quest’s existing fenced transaction call `changes::load(tx, world, reset_epoch)`. `owner::is_active(&day, key)` requires the committed current World. Counter stages/generations/progress are live projections and are skipped by immutable daily serialization. `key@stage` is a closed counter-state prerequisite, used by the existing White Deer scheduler; an unselected Voice day stays inactive.
2. Native world callbacks declare `world_change_requirement(callback_key)`; `changes::callbacks::require` checks current state inside the same quest transaction. Stateless entry uses `observe_world_callback_gate` plus the existing actor/object CAS after the await. Mage escape and ordinary travel stay stateless.
3. Completed shared actions join `counters::authorize` / `counters::credit` in the existing receipt transaction. Native death adapters supply full death identity and captured cohort. Hive/deepling/Guardian adapters validate their closed completed receipt; Benevola debits one actual carried wolf before that transaction commits. A tally alone is no item custody or reward permission.
4. `counters::voice_eligible(tx, world, epoch, character)` observes the current encounter’s25-use entitlement after the caller’s Character fence. Old periods read false without a World-wide player reset scan.
5. Existing resource lanes keep `changes::contribute(tx, recovery, world, epoch, operation_id, kind, amount)` and `changes::tally(tx, world, epoch, kind)`; their own owner fences/debits coal, medicine or votes in that same transaction. No new contribution/DAILY/RAID owner is introduced.

Hive lanes use the native Orockle transition `hive_access::BUY` through the existing Quest writer; they never debit Favour directly or invent a World contribution for the personal scent. Entry uses the registered `hive_access::gate_key(index)` callback. The server alone derives an outward `/exit` observation key from its frozen actor position; quest input must not supply that exemption. Deepling boss callbacks retain the existing personal Crystal13 prerequisite in addition to World stage3.

## Per-title inventory

| Title | Status | Owner | Evidence / binding or exact hold |
|---|---|---|---|
| Awash World Change | **HELD** | WVAR | wiki54563@1149363, wiki54770@1149364; Pump/lever event membership, flooded/dry layer transition and shared reset rule are not bound. |
| Bank Robbery Mini World Change | **HELD** | WEVENTS | wiki76142@891480, wiki76139@1082154; Hourly robbery trigger/probability, banker-disable state and bag recovery/reset binding are absent; W 15 explicitly leaves the mid-epoch trigger outside v 1. |
| Bored Mini World Change | **HELD** | WVAR | wiki76210@845015, wiki76211@1135336; W 15 fixes 82 permille. Wyda and fake-spider components do not identify the active illusion-spider spawn variant or daily availability binding. |
| Chakoya Iceberg Mini World Change | **HELD** | WVAR | wiki76212@847008, wiki76213@887139; Selected iceberg geometry, creature-to-variant membership and activation rule are absent; base Chakoya spawns/storage names do not identify the active iceberg. |
| Chyllfroest Mini World Change | **HELD** | WEVENTS | wiki76214@797728, wiki76215@887140; Exact seasonal opening interval and native bridge/closed/open projection with spawn membership are not established. |
| Deeplings World Change | **QUALIFIED** | WORLDCHG-Q | wiki56899@845058, wiki56900@501316; Heart/net credits, save-time stages and five-save reset; explicit native coral-OR-stair stage-two composition, because retained donors do not establish a shared composition. Three phase-three boss portals retain personal Crystal 13. |
| Demon Wars World Change | **QUALIFIED** | WORLDCHG-Q | wiki54562@1090693, wiki54700@880733; World losses drive save-time faction changes; tested 100-kill steps and strict >400 balanced-to-prince boundaries; native faction spawns follow the phase. |
| Devovorga's Essence Mini World Change | **HELD** | WEVENTS | wiki76216@845419, wiki76217@944253; W 15 fixes 16 permille. Native Vengoth gate/destination, incarnation selection/cost debit and encounter admission are absent. |
| Down the Drain Mini World Change | **HELD** | WVAR | wiki76218@891481, wiki76219@887142; W 15 fixes 307 permille. Native flooded/dry river topology, active spawn membership and world-state projection are absent. |
| Fire from the Earth Mini World Change | **QUALIFIED** | WORLDCHG-Q | wiki76220@845085, wiki76221@887143; Seeded 199-permille native candidate; volcanic spawns and a Dragonling opportunity. Fifty personal credited kills grant the canonical achievement once in the counter transaction. |
| Flower Month | **HELD** | WEVENTS | ; June, four native donor rectangles, 25 dryads per wave, 15-minute rotation and 250 attempts are known. Raid catalogue explicitly holds wiki five waves/hour versus donor indefinite four-area cycle; accepted firing/end policy remains unresolved. |
| Fury Gates Mini World Change | **HELD** | WVAR | wiki76237@1099676, wiki76236@887144; 343 permille and ten equal cities; exact entry/return positions and Premium/promoted/level60 rules are sourced. The Furyosa scheduler precondition is bound and denies new firings while the title is HELD. Native W1 variant/layer-set composition, return AID restoration and eligible callbacks remain unimplemented. |
| Grimvale Mini World Change | **QUALIFIED** | WORLDCHG-Q | wiki76238@847053, wiki76239@1102848; Existing monthly selection and current-world entry gate; the admitted XML final wave at 61 seconds spawns Feroxa and opens the actual 1949-to-22761 portal source. |
| Hive Born World Change | **QUALIFIED** | WORLDCHG-Q | wiki56901@845113, wiki56902@930222; Accepted missions require full objective counts before Orockle turn-in. Personal cycle reset, one/two Favour and shared credit commit together; fifty Favour buys a seven-day scent. Forty-eight sourced gates enforce stage plus scent, while server-derived outward crossings preserve escape. Main-Hive population follows the shared phase. |
| Hive Outpost Mini World Change | **HELD** | WVAR | wiki76240@969491, wiki76241@887146; Native invasion variant membership, activation rule and optional-boss selection are absent. |
| Horse Station World Change | **HELD** | WCONTRIB | wiki54701@844780, wiki54702@502074; Startup horse anchors/counts and stable handler are known. Escape, three-hour/zero-horse raid activation/expiry and native stable transition are not composed; donor stable script references an unassigned waveEvent. |
| Insectoid Invasion World Change | **HELD** | WEVENTS | wiki54570@1106148, wiki55050@1160735; Greenshore invasion/restoration state, credited restoration threshold and active spawn membership are not bound. |
| Jungle Camp Mini World Change | **HELD** | WVAR | wiki76242@847067, wiki76243@887148; Camp-to-variant membership, daily variant trigger and rare-boss selection rules are not bound. |
| Kingsday Mini World Change | **HELD** | WEVENTS | wiki76244@845148, wiki76245@994341; W 15 fixes 24 permille. Celebration state, hourly arena raid binding and achievement eligibility are absent. |
| Lumberjack Mini World Change | **HELD** | WVAR | wiki76246@845163, wiki76247@893906; W 15 fixes 90 permille. Native felled-tree variant, Chip activation and report/reset mutation are absent. |
| Nightmare Isles Mini World Change | **HELD** | WVAR | wiki76248@891482, wiki76249@887167; 1000 permille and three equal entrances; exact shared-dungeon entry/return positions and base creatures are sourced. Native W1 variant/layer-set composition, missing return UID restoration and current-fenced entry/return callbacks remain unimplemented. |
| Nomads Mini World Change | **HELD** | WVAR | wiki76251@845176, wiki76252@1128611; W 15/D 1055 fixes 799 permille, four camps and equal weights 1. Native camp/spawn/chest membership and selection-to-map binding are absent; base Nomad spawns do not establish camp variants. |
| Noodles is Gone Mini World Change | **HELD** | WCONTRIB | wiki76253@891483, wiki76254@1122443; W 15 fixes 92 permille. Retained Noodles mutations are PostmanMission 06; Dog Collar escort, disappearance/return and next-save Dog Lady transition are absent. |
| Oriental Trader Mini World Change | **QUALIFIED** | WORLDCHG-Q | wiki76255@845183, wiki76256@887151; Existing three-town ship layers, real Yasir offers and ariki greeting; the selected deck is checked inside the trade transaction. Historical receipt replay precedes the live gate. |
| Overhunting World Change | **QUALIFIED** | WORLDCHG-Q | wiki54569@844823, wiki54864@1084984; Daily deer/wolf counters and save transitions; Benevola consumes one real captured wolf for ten recovery credits in the same transaction. Native population and the phase-one White Deer raid follow this state. |
| Poacher Caves Mini World Change | **HELD** | WVAR | wiki76257@1175804, wiki76258@1175805; Native three-state population transition, credited kills and active Game/poacher map/spawn membership are absent. |
| Rathleton Quest | **HELD** | WCONTRIB | wiki68050@846610, wiki68051@1141625; 50/150/300/500 spent-vote rank thresholds are known. Ballot identity, election period, eligibility/point debit, winner transition and native rank admission are absent. |
| Rise of Devovorga | **HELD** | WEVENTS | wiki48435@1076762, wiki48437@1204791; Boss spell/storage metadata exists; exact seasonal interval, synchronized phases, victory/failure and participation/reward admission are absent. |
| River Runs Deep Mini World Change | **HELD** | WVAR | wiki76259@845206, wiki76260@887153; W 15 fixes 269 permille and fishing callback exists. Flooded/dry river layer and fishing-object/spawn membership are not bound. |
| Spider Nest Mini World Change | **HELD** | WVAR | wiki76261@891484, wiki76262@887154; Mamma Longlegs spell/base creature exists; active nest spawn membership, entry barrier, daily activation rule and kill progression are not bound. |
| Spirit Grounds Mini World Change | **HELD** | WVAR | wiki76263@891485, wiki76264@887160; Named gate regions/storage metadata exist; rotation rule, source/destination pairing and spawn-variant membership are absent. |
| Spring into Life | **HELD** | WEVENTS | wiki58901@845338, wiki59009@1099118; Egg/nest/hatchling identities are described; exact seasonal dates, native nest membership, hatch timing, reward weights and merchant admission are absent. |
| Stampede Mini World Change | **QUALIFIED** | prior WORLDCHG-Q | wiki76265@845239, wiki76266@887155; Accepted W 15 daily 125-permille gate composed with the existing Stampede raid schedule, 16 Terrified Elephants and bound Tiquanda area; existing fifth-credited-kill achievement path. The raid keeps its own native interval/margin and until-reset creature lifetime. |
| Steamship World Change | **HELD** | WCONTRIB | wiki54705@891478, wiki54769@772277; 200 sharedCoal and Junkar destinations/prices are known. Coal custody/debit, shared unlock transition and transport gate are absent; savecoal dialogue is not a debit. |
| Swamp Fever World Change | **HELD** | WCONTRIB | wiki54559@844892, wiki55045@749214; Ottokar exchanges medicine 12517 for belongings 12413 as an individual task. Epidemic threshold, medicine drop/custody and shared disease transition are absent. |
| Thawing Mini World Change | **QUALIFIED** | WORLDCHG-Q | wiki76267@845260, wiki76268@890265; Fourteen actual Lua flower sources with stable Added provenance; current epoch and selection precede harvest CAS/mint. Native PG covers yield, replay, restart, regrowth and refusals. |
| The Fire-Feathered Serpent World Change | **QUALIFIED** | WORLDCHG-Q | wiki85784@891479, wiki85785@1078002; Immediate 1,000/2,000-kill milestones; three carried-item orb receipts and Guardian NPC completion; mission completion AND 2,000 Renegade deaths reset the event. Yeo barter and three native loot sources supply the inputs. |
| The Mage's Tower World Change | **QUALIFIED** | WORLDCHG-Q | wiki54564@844897, wiki55046@1095409; 2,000 Yielothax deaths plus Energized death; Raging death starts the five-minute collapse. The current DB clock gates entry; only source 10840 closes, while 10842 remains an escape. |
| The Mummy's Curse World Change | **HELD** | WEVENTS | wiki54567@844898, wiki55049@1084983; Jar actionids 50006-50010, chances and boss/wall coordinates exist. Native shared jar/world composition is absent; duplicate boss spawn and conflicting wall timing in retained donor are unresolved. |
| Their Master's Voice World Change | **QUALIFIED** | WORLDCHG-Q | wiki54568@1110635, wiki55047@1110638; 100 shared gobbles, 25 waves of 25 owned slots, Mad Mage and first-use one-hour restore. Actual gobbler custody, inclusive five-second cooldown, atomic 20 XP and current-period 25-use eligibility. |
| Thornfire World Change | **HELD** | WVAR | wiki54125@844903, wiki54126@1020842; Native global fire/spawn membership, water/action credit, release condition and reset transition are absent. |
| Twisted Waters World Change | **HELD** | WCONTRIB | wiki54198@844910, wiki55048@935150; 1000 worldcorpse deposits, save transitions and 20-hour personal restriction are known. Deposit debit, lake-state projection and catch eligibility are absent. |
| Warpath Mini World Change | **HELD** | WVAR | wiki76269@845669, wiki76270@944151; W 15 fixes 373 permille. Native camp/guard/Bibby kill-to-spawn progression, entrance and cooldown binding are absent. |

## Commands and executed results

Commands source `/workspace/oteryn-env/env.sh`. Only changed-crate lint and focused tests run; no full nextest or full PG suite.

- RED proofs: missing counter/phase APIs; Hive missing native Orockle receipt; missing Mage entry/physical method; Fire kill50 produced0 instead of1 achievement in real PG5.51s; unselected Voice phase query incorrectly returned active. All are retained in `/tmp/worldchg-*-red*.log`.
- Unit/map/content: `cargo test --locked -p oteryn-gameplay-world-change` (37); focused quest callbacks (6), phased Spawn (1), World map (4), NPC book (2), original corpse capacity guard (1), real native loot closure and execution budget (2). Python source/provenance/admission tests: 10 passed. Native item regeneration and `--check` passed with 41 scoped profiles.
- `cargo clippy --locked` for the eleven changed crates, `--all-targets --all-features -- -D warnings`: passed (12m27s); dispatch replay repair recheck passed (7m01s), and final wire-fixture recheck passed (1m00s). `cargo fmt --all --check` and `git diff --check` passed. Focused `worldchg_stampede_content` passed (1); `cargo test --locked -j 1 -p oteryn-game-server --test content_world_project_repository` passed (5,85.77s). Tool-driven pin refresh/check passed: `5051dde5232f5a1a1536e8c8e8bd73f53f362e870b78822ed3788fb0ca6a0cbd`.
- Owner PG: `cargo test --locked -p oteryn-game-server --test character_authority_postgres worldchg_postgres_ -- --nocapture`: 6 passed,22.15s. Literal `WORLDCHG-Q PG EXECUTED` markers include Hive14.573s, Serpent14.631s, eight concurrent counter writers14.643s, Fire14.673s and contribution persistence/eight writers. Native dispatch PG markers cover Thawing four cases100.10–115.71s, Grim final wave83.70s/current gate103.56s, Benevola recovery196.27s, Yasir145.63s and Yeo barter123.21s. Voice inactive72.338s and active/restart60.714s passed with execution markers; the latter proves the replay projection and missing-epoch admission refusal without a second receipt, XP award or gobble.
- Fixture repairs preserve production guards: bootstrap content activation precedes World reset creation; NPC talk uses the existing test owner-clock cadence; Feroxa uses actual native ground104. Disk exhaustion is resolved by removing only superseded completed task build caches; active working checkpoints/prepared dependencies remain.

## Final six-title validation

The focused admission test first failed on `deeplings: HELD != QUALIFIED`; the native Favour test first failed on the missing Favour effect. After implementing the callbacks and promoting the six bindings, all 38 world-change tests passed, native quest callbacks passed (6), Hive access tests passed (4), and map tests passed (4). The Hive tests cover one/two Favour per completed turn-in, replay refusal, a 50-point debit, the exact seven-day boundary, all 48 native gates and outward escape with expired scent or closed stage.

The phased-spawn membership/suppress/rearm test also passed (1). Final owner PG passed all six cases in16.93s, with literal `WORLDCHG-Q PG EXECUTED` markers: eight-writer credited-kill dedupe/reset9.756s; native Hive Favour/completion/purchase/expiry/replay9.798s; Fire personal49/50 achievement9.809s; Serpent Guardian/compound reset9.812s; Stampede7.097s; and persistence/eight concurrent resource writers/three resource resets on PostgreSQL18.6.

Final native dispatch PG also passed: `worldchg_hive_native_gate_closed_stage_and_missing_scent_allow_egress_only`, 53.11s overall, literal `WORLDCHG-Q PG EXECUTED Hive native USE` marker at53.009s. Actual wire USE commits outward movement from a closed-stage/no-scent gate, refuses reverse entry, and leaves both Quest and shared-credit receipt counts zero. All required focused checks are green; no full nextest or full PG suite ran.

Strict lint of the eleven changed crates passed (17m32s), followed by `cargo fmt --all --check` and `git diff --check`. Tool-driven regeneration and `--check` passed (42 files / 41 scoped profiles). Fresh pin-refresh and pin-check passed with inputs `b39a63c626aba4254114e41efe701b664151c8be1d0d69ddb490c6f0bb4ba37e`; the map remains 31,102,025 bytes with digest `5051dde5232f5a1a1536e8c8e8bd73f53f362e870b78822ed3788fb0ca6a0cbd`.

Fresh main/open-PR number check: main `9668e0a04e`, migration maximum1095 and open claim969; 1130–1134 have no collision. No open PR exists for this branch. Catalogue comparison against `318ab59d2` proves all eleven scoped titles QUALIFIED and all thirty-two other rows unchanged.

Executed final-batch commands (source `/workspace/oteryn-env/env.sh`):

```sh
cargo test --locked -p oteryn-gameplay-world-change
cargo test --locked -j 1 -p oteryn-gameplay-quests worldchg_
cargo test --locked -j 1 -p oteryn-gameplay-world --lib worldchg_
cargo test --locked -j 1 -p oteryn-gameplay-entities --all-features --lib worldchg_
cargo test --locked -j 1 -p oteryn-game-server --test character_authority_postgres worldchg_postgres_ -- --nocapture
cargo test --locked -j 1 -p oteryn-gameplay-dispatch --all-features --lib worldchg_hive_native_gate -- --nocapture
cargo clippy --locked -j 1 -p oteryn-game-server -p oteryn-gameplay-combat -p oteryn-gameplay-content -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-gameplay-entities -p oteryn-gameplay-items -p oteryn-gameplay-npc -p oteryn-gameplay-quests -p oteryn-gameplay-world -p oteryn-gameplay-world-change --all-targets --all-features -- -D warnings
cargo fmt --all --check
python3 tools/content-migration/register_spell_families.py
python3 tools/content-migration/register_spell_families.py --check
cargo run --locked -j 1 -p oteryn-world-bundle-compiler -- pin-refresh .
/workspace/oteryn-env/target/debug/oteryn-world-bundle-compiler pin-check . /tmp/worldchg-final-pincheck-actual
git diff --check
```

## Review

Whole-diff review covers current authority, row/physical lock order, bounded reads, receipts/rollback/replay, no source-value duplication, native geometry/provenance and map/spawn end paths. Independent exact-head review is recorded after final local validation. This report does not claim an independent verdict before that run.
