# QUEST-FINISH handover — owner stopped authoring for transfer to Codex CLI

No commit or push. Working tree is deliberately unfinished and currently has known compile blockers.

## Base and transfer

HEAD/base commit: f52f71f7857b399dc8f6d70ff51c23f5b779f89d.
Uncommitted merge: origin/main a9db6a76a1ba375cb29104ec4a814310651c35fb.
Exact git merge-base of those commits: c475b511ba82c20597fbefa3e179c4be5b015217.
Both exported diffs include the incoming-main file changes. A patch does not recreate MERGE_HEAD/index merge metadata.
The tree immediately after the main merge, before task changes, was cf02deefb793053b9b2aa976cf11de1450b7317a.
That is a tree object, NOT a commit or the git merge-base.

To recreate the stopped file contents: checkout f52; apply source.diff; regenerate excluded JSON in the documented order and check every SHA. Do not also merge main first: its file changes are already in the patch. Alternatively merge the exact main commit then apply a task-only patch regenerated against the integrated baseline tree (not this source.diff).
All tracked and untracked source files are included, including intentionally unfinished drafts. Main's incoming changes touch Quick Loot, Store and Cyclopedia client code; those are merge changes, not quest work.

## DONE versus PARTIAL

No newly completed whole quest can honestly be certified DONE. Existing reward-grant tests are not full access-path or whole-quest tests.

DONE capability slices with executed tests:
- Native fluid root/child reward mint: one atomic transaction initializes exact optional fluid subtype, freezes the separate fluid plan, refuses missing/substituted state, and does not refill on replay. Actual PostgreSQL test passed in11.48s on180006. This does NOT admit the three source-held fluid recipes yet.
- Exact full-definition charge admission for seven Items (3054,3056,3082,3083,3084,3085,3284), plus complete native item closure for ready non-Dawnport rewards. Python charge and container tests passed. Partial-charge donor/default conflicts stay held.
- Runtime Dawnport exclusion: two focused content/quest tests passed. SOURCE records remain; client NPC routes do not offer Dawnport. Later changed classic PG expectations were not rerun.
- Medusa native source reconciliation: Python red→green and15 container-native tests passed. Current fetched Wiki corroborates the three rewards. Rust admission/test additions and catalogue PG path are NOT qualified after the latest edit.

PARTIAL quests / packages:
- Medusa: exact Crystal bag2853 +3436/3567/3324 recipe authored; original Canary text-carrier hold preserved. Need Rust test/real source-selector PG execution and modern map/access confirmation (Wiki spoiler says maps outdated).
- Goblin Temple: Wiki confirms milk; fluid capability works, but source recipe subtype authoring/native admission and full quest execution missing.
- Dreamer's Challenge: Wiki confirms all four boxes give wine,bread,certificate; recipe lowering still held, book READ/window not implemented, puzzles/access/faction progression not fully executed.
- Present: Wiki lists jug/plate/cup/present in backpack, without liquid contents; source fluid tuple interpretation unresolved; Seymour exchange and complete quest execution not proved.
- Deeper Fibula, Queen of Banshees stone-skin reward, Targuna silver amulet, two Ornamented Shield claims: donor raw1-charge versus full Item default conflicts remain held; do not silently grant full charges.
- Secret Library/Falcon Bastion: six repeat/random chests remain held. Preserve prior accepted24h ruling despite missing donor timer; implement real repeat-cycle RNG receipts and exact rewards, not once-only reuse.
- Newhaven/Corruptor: source-correct named gear IDs drafted, admission refuses incomplete native gear before consuming claim, real kill→loot→take→equip test authored for all five vocations. No actual dispatch test completed. Do not integrate the draft capsule: current Wiki Simple Jo Staff51166 conflicts with pinned donor/target51119.
- NPC catalogue:13programs/1123rules/78NPCs. Domain evaluation plus fenced storage/time/topic durable owner authored, adapter UNMOUNTED. Full effects (item give/take,money,XP,outfit,teleport,map marks,FX,end/native RNG) remain unsupported whole-refused. Achievements and inventory snapshot helpers are unregistered drafts. NPC real-caller RED never ran.
- Father's Burden: source acceptance writes11 tracks atomically. Accepted QuestState cap8 retained. Owner question proposing scoped11-track NPC operation was unanswered. Never split this into multiple writes or weaken cap silently.
- Blue Valley/Cheesy/Stag: seven literal USE object-selector regressions authored, no runtime projection activation and no tests run. Two additional Stag guard-only callbacks cannot be implemented using ADD(0): accepted decision explicitly rejects that substitute.
- All other catalogued quests: no new whole-quest completion certification. Existing bespoke functionality is not erased by historical rollout/preflight flags.
- Client quest log/tracker: real decoder/reducer/UI paths and recorded client test exist in base; identified, not run in this task. Win32 presentation untested here.

## Immediate broken/unfinished drafts — read before building

1. Fee18: durability/item_fee_burn.rs declares `pub mod npc_program`, but item_fee_burn/npc_program.rs WAS NOT CREATED. This is a production compile blocker. Child stopped midway. Typed audit cause18/inner prost tag18, manifest rows and tests are partial; no executable fee helper, receipt lookup/custody checks or SQL changes exist. Historical manifest digests were reconstructed in Python and remain unverified against Rust canonicalization. Cause15 remains the fixed Rotten Blood fee; never reuse15 for generic NPC fees.
2. Book codec: protocol-oteryn/src/item_text.rs contains only new tests referring to missing ItemTextView, encode/decode functions and MAX_ITEM_TEXT_BYTES. It is mounted in lib.rs. Protocol lib tests will not compile until completed or intentionally removed for a separate continuation. No book server/client route or capability fragment exists. Cap289 ITEM_TEXT_V1 was RESERVED ONLY; no new wire command was created. Existing USE command is2 (not11); LOOK30 is separate. Command502, mechanic632 and offer629 remain unused.
3. NPC runtime loading:132 conflicting SOURCE/program tracks need source-qualified default−1 refinement, union of existing/program bounds, and final standard catalogue extension. Only an unrun loader regression exists. Latest compatible-range helper/test in npc_rules.rs is also unrun. Preserve SOURCE parse counters, historical cores and existing broader bounds; never override a native overlay or raw definition hash.
4. Whole-catalogue chest harness: first attempt failed before PG with `Crystal source selector`; now changed to explicit negative refusal for one Canary-only Wrath placement. Latest variant NOT rerun. Harness covers rewards at synthetic exact source selectors, NOT quest access routes or deployed-world qualification.
5. Newhaven draft still chooses51119, which is not current-Wiki corroborated. Native profiles/generator hooks have deliberately NOT been integrated.
6. Position-callback fixture tests expect still-held callbacks; expected RED unexecuted. No production callback change was made.
7. Current authoring checks FAIL: test_variant_stack_serialization.py:40 still expects38 remaining native errors; current correct observed count32 after five charge containers+Medusa. One earlier companion expectation was updated, this one was not. Do not claim checks pass. Further checks after this failure have not run.
8. Final fmt, Clippy and changed-crate suites have NOT qualified this frozen tree. It is not integration-ready.

## Inventory before and after (source / implementation / proof are distinct)

Before: historical373titles; Canary324implemented+9partial+40absent, Crystal328+13+32.
Donor journals52Canary/59Crystal; sourcegroups114/129; scripts978/1133.
Canonical352definitions=284donor+68authored. All373 titles resolve:278direct+27family+68authored; zero missing canonical title refs.
Rollout373partial/needs-runtime; runtime_enabled0/smoke_verified0 are metadata flags, not bespoke runtime census.
Generic preflight0nativebindings/0deliveries,1887unboundstages; historical sourcegaps4111=3499bespoke+467ownerpending+145shared.
Before chest336=11PGqualified+297supported-unqualified+28notplayable.
After chest336=11PGqualified+309supported-unqualified+16notplayable (14real holds+2owner-excluded Dawnport).
After supported in-scope320; no additional per-World qualification claimed. QUEST-QUAL ledger309 entries blocked on WORLD_BINDING_AND_E2E_EVIDENCE_MISSING.
Native exact recipes67→73; plain ready220→228/231; full-quest proof remains0recorded.
Static callback source256 across51quests:167step+89use; runtime excludes one Dawnport callback, retains dynamic Rotten Blood altar.
NPC13programs1123rules78NPCs; production mount remains incomplete.
Current inventory is PARTIAL: owner fetched first500 transcluding pages with continuation0|64408 (not500quests).
Current Wiki checks: Medusa reward trio corroborated; milk/wine named rewards corroborated; four gear IDs agree, SimpleJoStaff conflicts. Whole-quest current verification remains unproved.

## Donor conflicts and decisions taken

- Owner exclusions applied: Dawnport/OT-only/Prey tasks; Newhaven gear, not an invented starter kit. Existing StarterTemplate main is backpack2854 only.
- Medusa: select complete Crystal201189f chest.lua:2017-2024 bag+threeitems; Crystal handler42-49 fox text belongs6179/book2815. Canary47dfd handler40-47 gives6183 fox text and container helper119-123 stamps every nonkey child. Native tokenCRYSTAL_MEDUSA_WITHOUT_CANARY_TEXT; retain original SOURCE hold and never invent a book/armor-text carrier. Fetched base3793rev771196/spoiler4919rev748935 supports three items, not a text carrier or exact modern coordinates.
- Charges: admit only exact source/full-default matches. Crystal player_functions.cpp2815-2843/container_functions.cpp177-183 forward raw subtype; item.cpp335-339 makes nonzero actual charges. Raw1 is not evidence for full default. Preserve sibling holds and existing charge-state ownership.
- Text scroll5952: native portable weight120 from pinned Canary XML16088-16091; canonical UNKNOWN remains historical source fact. No generic unknown-weight default.
- Source baselines: generated SOURCE42ready/242waiting/68waitingbindings and28 reviewed source cores are historical evidence. Native admission is a separate projection. Used repo amend_monk_source_receipt.py to refresh permitted compiler fingerprint, preserving all reviewed cores; no hand pin edits.
- Newhaven donor script uses stale/mismatched numeric IDs. Named XML correction drafted:10321→3571Ranger;48843→51760RefinedBow;8871→7991Magician;48847→50195Harmony;48845→51119SimpleJo. Wiki corroborates first four named identities but staff current51166 conflicts with donor/pinned target. HOLD the entire draft admission pending version/namespace reconciliation.
- NPC default: accepted QuestState decision §3 already allows source−1. Ordinary missing storage−1 proven Crystal player.cpp1341-45; setter−1 erases1327-37. Scope any refinement to exact original SOURCE declaration/sameowner and preserve union bounds. i64 bounds are Oteryn-declared representation, not proven donor KV numeric width. No cap8 change authorized.
- Canary-only Wrath placement UID14092 at33074,31170,8 must refuse on Crystal; four other placements may be valid. Do not fabricate a Crystal UID or silently skip the negative case.
- No new live protocol capability/command actually allocated. Fee cause18/inner tag18 is unfinished;289 is a reservation only. Only migration0969 added.

## Open source questions — no more fetches were made after STOP

Exact known MediaWiki requests are in remaining-api-urls.txt. API-only for Wiki; no UA spoofing/captcha bypass/CipSoft. Direct GitHub raw sources worked; guessed-repository404 was corrected to zimbadev/crystalserver.
- Complete current inventory continuation0|64408 and compare new/removed/renamed titles against historical373; follow any further API continuation. Then inspect current base/spoiler bodies (15 known batches in file). Current metadata alone cannot certify rules.
- Simple Jo Staff: MediaWiki page108393 currentrev1186982 says51166; donorXML201189f and pinned target2dfa identify portable named51119. Need current client Item ID namespace/version, relevant current appearances mapping, and `The Corruptor` reward facts; do not substitute an immovable nameless object or mint an invented canonical51166.
- Newhaven other base-kit facts: Brass Armor,Longsword,Soldier Helmet,Snakebite Rod,Wand of Vortex; combined exact API URL in file. Need complete Viola book/arming/Corruptor access composition, not only reward test.
- Medusa modern coordinates/level60 gate/access: supplied spoiler warns maps outdated. Review current Drefia map evidence and current quest video if API text cannot establish path; no exact new video was selected/fetched in this task.
- GoblinTemple milk and Dreamer wine: current named liquids are now evidenced. Numeric fluid subtype must come from existing source fluid enum/owner and be explicitly authored; no guessed default. Present jug/cup contents remain unstated in supplied Wiki; inspect actual donor tuple semantics, current Present facts/video if still ambiguous.
- Five source charge/default conflicts (DeeperFibula, QueenBanshees stone-skin, Targuna, OrnamentedShield two claims): compare current reward charge quantities, not just item names; current quest/base+spoiler batches include these pages.
- Six Falcon repeat/random chests: current SecretLibrary/Falcon facts and acceptedPR2770Q3/A24h, exact RNG/reward semantics; retain owner decision until a real differing rule is presented.
- Stag boat-to-Ada/marked-wall guard-only callbacks: exact donor scripts isle_of_ada.lua/bounac_cathedral.lua and accepted noADD(0) decision; implement a genuine read-only owner, not a fake storage mutation.
- Soul War inherited source questions still present: see reconstruction.json/readback for exact source-rule holds. Existing video windows to recheck on owner's PC:
  https://www.youtube.com/watch?v=ZJKtfyRCPjQ&t=270s — Malice04:30-06:50, white/safe tiles and SoulCage; raid spawn/entry donor conflicts require exact sources.
  https://www.youtube.com/watch?v=X_eftME4qkY&t=375s — Greed06:15-08:05, SoulSphere motion/phase.
  https://www.youtube.com/watch?v=AR0TxuTv0rc&t=445s — Spite07:25-09:35, stomp/corpse effects; video cannot establish exact cooldown alone.
  https://www.youtube.com/watch?v=t3GcDyTQsPk&t=310s — Cruelty05:10-07:15, Maw/dynamic damage/transform.
  https://www.youtube.com/watch?v=oOgX3QQtJxg&t=260s — Hatred04:20-06:30, CondensedRemorse/torment reset.
  https://www.youtube.com/watch?v=L8b-8qgQhuo&t=80s — Megalomania01:20-07:05, corpse/sanity/remains, tiles/transform.
No video was fetched or newly verified during this task. These are inherited evidence references, not new successful accesses.

## Validation actually run (all Cargo shells sourced env.sh)

Startup: source /workspace/oteryn-env/env.sh && bash /workspace/oteryn-env/start.sh — PASS once. No tools installed.
PASS results below qualify only their tested checkpoint, not the final stopped tree.

`cargo test --locked -p oteryn-gameplay-dispatch quest_container_chests --lib -- --nocapture` —12passed,325.81s actualPG, build11m52; explicit POSTGRES_CASE_COMPLETED markers in quest-finish-chests-baseline.log.
`cargo test --locked -p oteryn-gameplay-dispatch quest_catalogue_chests --lib -- --nocapture` —1FAILED0.05s beforePG at Crystal source selector. Corrected negative-case code unrun.
`cargo test --locked -p oteryn-gameplay-content owner_scope --lib -- --nocapture` and `cargo test --locked -p oteryn-gameplay-quests owner_scope --lib -- --nocapture` —1+1passed0.05s/1.05s.
`cargo test -p oteryn-gameplay-quests npc_rules -- --nocapture` —25passed0.58s; latest range-compatible helper and26th test unrun.
`cargo test --locked -p oteryn-gameplay-quests tutorial_corruptor_rewards_use_source_items_and_base_vocations --lib -- --nocapture` — initial stale Paladin ID red, then1passed (before fetched51166 conflict).
`cargo test --locked -p oteryn-gameplay-durability filled_fluid_rewards_bind_subtypes_and_refuse_ambiguous_shapes --lib` —1passed0.01s, build3m43.
`cargo test --locked -p oteryn-game-server --test character_authority_postgres filled_fluid_claims_commit_root_and_children_and_replay_without_refilling -- --nocapture` —1passed11.48s, build8m32, PG180006; exact database marker in quest-fluid-pg.log.
Prepared migration proof applied297 migrations and exercised shape/runtime-role checks in7.91s; quest-fluid-migration.log retained. No fullPGsuite run.
`cargo test --locked -p oteryn-gameplay-dispatch newhaven_corruptor_named_gear_all_vocations_kill_take_equip_once_postgres --lib -- --nocapture --test-threads=1` —CANCELLED130 during compilation by ownerSTOP; NO actualRED/runtime result.
`cargo fmt --all -- --check` —earlier checkpoint passed (quest-finish-fmt.log); later fluid global fmt check reported drift, scoped files formatted; final current whole-tree fmt NOT rerun.
No final Clippy -Dwarnings, changed-crate fulltests, client recorded tests or independent final review executed. No fullnextest/fullPGsuite.

Python checkpoints:
`python3 -m unittest discover -s tools/content-migration -p test_quest_reward_item_semantics.py` —30passed after7caseRED,0.4s.
`PYTHONPATH=tools/content-migration python3 -m unittest test_quest_callback_profiles.QuestCallbackProfiles.test_every_ready_reward_leaf_has_its_exact_native_item_profile` —RED92missing thenGREEN.
Combined migration Item/profile tests44cases —1stale closure-countfailure130vs138; focused corrected closure testGREEN; entire44notrerun afterward.
`python3 -m unittest discover -s tools/content-schema/reward-claim-authoring -p test_container_native.py` —15passed8.668s after Medusa; fullcharge leaves previously red8 then13passed.
`PYTHONPATH=tools/content-schema/reward-claim-authoring python3 -m unittest test_container_native.ContainerNativeTests.test_medusa_uses_complete_crystal_reward_without_canary_fox_text test_container_native.ContainerNativeTests.test_medusa_reconciliation_cannot_clear_a_sibling_hold_or_invent_a_carrier` —2passed0.803s after RED.
`PYTHONPATH=tools/content-migration python3 -m unittest test_quest_finish_newhaven_items` —8passed; sourcecapture --check passed; capsule now source-conflicted with fetched51166 and deliberately unintegrated.
`python3 tools/content-schema/quest-authoring/amend_monk_source_receipt.py` —PASS28exactreviewedcores unchanged.
`python3 tools/content-schema/quest-authoring/quest_tree_authoring.py content` —regenerated,4shards, sourcebaseline preserved.
`python3 tools/content-schema/quest-authoring/run_checks.py --part checks` —latest FAIL at test_variant_stack_serialization.py:40,expected38observed32. Prior run failed same stale count in companiontest, nowfixed. Verify-schema269/269, SoulWar reconstruction check and7unitcases, variantauthoring13cases passed before latestfailure.

Logs and source snapshots are included in the handover archive. Environment credentials/env.sh are NOT included.

Transfer-proof result: JSON-path restoration verified all21files in a detached f52scratch worktree. Running current generator commands then reproduced20/21snapshot hashes; quest definition index is stale in the frozen tree (two pre-Medusa input hashes). Exact stopped hash and expected regenerated hash are in REGENERATE.md. No task-source repair was made afterSTOP.

Earlier command logs do not retain every original shell redirection/auxiliary flag; commands above preserve known crate/filter/invocation, and logs are the authority for actual result/duration. The standalone297-migration7.91s proof was a prepared SQL harness, not a cargo suite command; do not invent a reproducible command from its result alone.
