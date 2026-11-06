# QUEST-FINISH — QUESTPC, 2026-11-06

Status: **PARTIAL / validation blocked; not integration-ready**. Task: QUESTPC
(PARITY-QUEST-CALLBACKS-1 + W15b). Base: `1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`,
detached worktree. No candidate is frozen, no PR is created, and no commit,
push, reset, checkout, stash, index mutation or external publication was performed.
The coordinator owns review and integration of this uncommitted patch.

## Scope and disposition

DONE below qualifies the named bounded artifact only. No newly completed whole
quest or deployed World is certified. Historical Cloud runs do not qualify this tree.

| Quest / package | Disposition | Evidence and remaining requirement |
| --- | --- | --- |
| Transfer conflicts | DONE | Both listed files resolved by retaining current main: `apps/client/src/features/0199-cyclopedia-character/tests/play.rs` and `crates/client-ui/src/features/0199-cyclopedia-character/protocol.rs`. The transferred duplicate test mount in the adjacent `mod.rs` was also withdrawn. Main's newer bounded decoder and tests are preserved. |
| Source and generated content | PARTIAL | Owning generators reproduced 19 handover snapshot hashes exactly; the documented stale Quest index is now the documented regenerated `c16f9777cc4ee30ee96937533ed4c83536f6d7be6aeeabbf5322242b16489969`. The twentieth retained file is that index; the excluded Newhaven capsule was withdrawn. Dependent binding packets, exports, state provenance and reports also require regeneration, rather than hand-editing hashes. Final checks are recorded below. |
| Exact full-definition charges | PARTIAL | Seven exact admitted definitions: 3054=200, 3056=200, 3082=50, 3083=150, 3084=250, 3085=200, 3284=1. Pinned Canary 04b83b5 `data/items/items.xml:6946,6966,7255,7272,7283,7294,8525`; placement-qualified `source_charge_evidence.json` sha256 `aced9b5e52f472f2a1bc80e7192093036f513c182a6542933ae1abe7f613cf71`. Eight plain leaves and five container recipes become supported. Source diagnostics and partial-charge conflicts remain held. Fresh Rust/dispatch/PG evidence is blocked. |
| Medusa reward slice | PARTIAL | Fresh read of Crystal 201189f `data-global/startup/tables/chest.lua:2017-2024`: bag 2853, children 3436/3567/3324, one each. Crystal `data-global/scripts/actions/system/quest_reward_common.lua:42-49` assigns fox text to 6179/book 2815; Canary 47dfd `data-otservbr-global/scripts/actions/system/quest_reward_common.lua:40-47,119-123` instead stamps Medusa children. The exact Crystal-only native projection retains the original SOURCE hold. Focused Python tests pass; focused real USE/PG test is authored but blocked by diskguard. |
| Medusa complete access path | STOPPED | Inherited Wiki page 3793/revid 771196 and spoiler 4919/revid 748935 corroborate the three named rewards. The spoiler warns its Drefia maps are outdated. No fresh modern-map/level-60/access-path proof or full-quest dispatch execution; no new access route was invented. |
| Native filled-fluid mint slice | PARTIAL | Complete root/child fluid plan and initialization use the existing once-claim transaction and fluid state owner. Migration `0969_quest_finish.sql` is new and 0969 is absent on HEAD; no merged migration changed. New PG case checks replay after draining, changed intent, missing initialization and substituted subtype, and emits `POSTGRES_CASE_COMPLETED` only after successful cleanup. Fresh Rust and PG qualification is blocked. No liquid recipe is admitted by this slice. |
| Goblin Temple | STOPPED | Crystal chest table:1499-1506 says vial of milk but encodes `{2874,1}`; inherited Wiki page 3574/revid 683865 and spoiler 8013/revid 748709 identify milk, not its numeric subtype. No explicit reconciled native fluid recipe or complete access-path test; the held recipe remains unmounted. |
| Dreamer's Challenge | STOPPED | Crystal chest table:2131-2161 gives four wine reward placements with raw vial argument 1. Inherited Wiki page 12354/revid 869211 and spoiler 12355/revid 891457 identify wine, bread and certificate. No explicit reconciled numeric fluid recipe, book USE/READ route, puzzle/access/faction execution. Existing supported authored text rewards are preserved. |
| Present | STOPPED | Crystal chest table:1448-1455 gives jug/present/cup/plate with raw argument 1. Inherited Wiki page 3214/revid 1044890 and spoiler 5480/revid 749066 do not establish liquid contents. No guessed fill or empty-vessel rewrite, Seymour exchange or new runtime claim. |
| Deeper Fibula; Queen of Banshees stone-skin; Targuna silver amulet; two Ornamented Shield claims | STOPPED | Exact raw-1/full-default conflicts remain. Crystal chest table:660,1000,1042-1058,3487 supplies the held placements. Crystal 201189f `src/lua/functions/creatures/player/player_functions.cpp:2815-2843`, container functions:177-183 and item.cpp:335-339 are inherited evidence that raw subtype is actual charges. Missing current reward charge quantities; no automatic full-charge grant. |
| Secret Library / Falcon Bastion six repeat/random chests | STOPPED | Preserve accepted PR2770 Q3/A 24-hour ruling. Fresh Crystal chest table:1383-1430 declares the six random pools and `time = 24`; a timer field alone does not prove handler or repeat-cycle behavior. Missing qualified per-cycle native RNG/receipt execution and exact source reward reconciliation. No once-only substitution or new runtime mount. |
| Newhaven / Corruptor | STOPPED | Inherited named-gear reconciliation agrees for Ranger's Cloak 3571 (Wiki revid 1186981), Magician's Robe 7991 (1186986), Harmony Amulet 50195 (1194222), Refined Bow 51760 (1148726). Staff conflicts: Wiki page 108393/revid 1186982 says 51166; pinned donor/client identifies portable named 51119. Missing version/namespace resolution and full Viola/arming/kill/loot/take/equip path proof. Withdrawn: changed Rust gear IDs/admission, unrun dispatch fixture, unused generator/hooks and its tests/capsule. Existing characters' historical kit compatibility is unchanged. No starter kit invented. |
| NPC catalogue, 13 programs / 1123 rules / 78 NPCs | STOPPED | Source catalogue remains test-only as on main. Missing integrated exact SOURCE-qualified default-minus-one refinement and union bounds, complete native effects, and real dispatch/recovery proof. Withdrawn the unmounted adapter, provisional writer/sequencer/recovery methods, extra owner arguments, planner extensions and their draft fixtures. No source tracks/hashes or native overlays overwritten. |
| Father's Burden | STOPPED | Source acceptance needs eleven tracks in one write; accepted QuestState cap remains eight. Missing accepted scoped atomic eleven-track operation. No split writes or cap increase. The provisional NPC write path is withdrawn. |
| Generic NPC fee cause/tag 18 | STOPPED | Missing executable fee helper, receipt/custody proof, SQL owner extension and canonical audit qualification. Entire unfinished cause/tag/module/test additions withdrawn; cause 15 remains Rotten Blood only. No new fee code is allocated by this patch. |
| Book window / capability 289 | STOPPED | Missing server/client production USE/READ route and accepted bounded wire integration. The test-only missing implementation and module mount are withdrawn together. No capability or command registration was added; 289 and 360-369 / 620-639 remain unused here. |
| Blue Valley / Cheesy / Stag callbacks | STOPPED | Seven literal object-selector drafts had no activated production projection or executed real-caller proof. Two Stag guard-only callbacks still require a genuine read-only owner. Their draft fixture changes are withdrawn; no `ADD(0)` substitute. |
| Dawnport | PARTIAL (owner-excluded) | Runtime QuestState/callback/reward binding and NPC offer exclusions retained; SOURCE captures remain intact. Positive donor-transcription tests now parse the SOURCE explicitly; production tests require no Dawnport mutations. Fresh focused dispatch qualification remains blocked. |
| OT-only features; Prey / Hunting Tasks P23 | STOPPED (owner-excluded) | No new runtime admission or implementation. |
| Client quest log / tracker | STOPPED for new qualification | Existing decoder/reducer/UI implementation remains on main. No new client feature or Win32 claim. Conflicting unrelated Cyclopedia transfer changes were withdrawn. |
| Remaining catalogue / Soul War | STOPPED for new whole-quest certification | Historical 373 titles are not a fresh current-Tibia census; continuation `0|64408` remains incomplete. Existing bespoke implementations are preserved. No new whole-quest completion or deployed-World proof. |

## Source freshness

The two chest tables and both reward handlers above were fetched read-only from
`raw.githubusercontent.com` at the exact donor commits supplied by the owner.
Crystal handler sha256: `9b38b43e800e05bd0262753198ec3b9adcb64a523ff3b085bf2e79fba05b95ad`;
Canary handler: `3394214ca803640b0148854f3a2cb8dd8e4ecd3749268800d4a3f8c50330f1cd`.
Donor behavior is `OTS_HYPOTHESIS_ONLY`; selecting the exact Crystal recipe does
not prove modern Global map access. Wiki revids above are inherited owner-supplied
Cloud evidence, not new successful API fetches in QUESTPC.

No video was opened or verified in QUESTPC. The inherited Soul War windows remain
unverified: Malice 04:30-06:50 (ZJKtfyRCPjQ); Greed 06:15-08:05 (X_eftME4qkY);
Spite 07:25-09:35 (AR0TxuTv0rc); Cruelty 05:10-07:15 (t3GcDyTQsPk);
Hatred 04:20-06:30 (oOgX3QQtJxg); Megalomania 01:20-07:05 (L8b-8qgQhuo).
These timestamps establish nothing new in this report.

## Validation and blockers

The required absolute cargo wrapper refused both compilation and a filtered PG
run with `[diskguard] refusing build: D_free=744G root=95%`. No bypass, alternate
cargo test runner, dependency download, Docker operation or credential access was
attempted. Some wrapper subcommands return zero despite refusal; zero is not proof
of execution. No current-tree PG test produced `POSTGRES_CASE_COMPLETED`.
The owner was asked asynchronously to free shared root filesystem space.

The offline donor-map unit module also fails because `g++` cannot find `zlib.h`.
No test was skipped, weakened or quarantined to hide that environment failure.
Withdrawn tests belonged solely to withdrawn unfinished drafts; existing main tests
remain, with scope assertions updated for the owner's Dawnport exclusion.

Detailed final commands, timings, changed paths and review notes follow.
