# Active L3 encounter-progression authoring — 2026-11-06

Single agent, current worktree only. User forbids Git mutations/publication,
credentials, Docker, dependency downloads and subagents. Compare this continuation
against 8e24f1d88; operator snapshots may advance HEAD. No frozen SHA or PR.

In progress: one shared native encounter-death mechanism for Soul War, Rotten
Blood and Primal (the latter is an inherited N–Q entry). Sourced encounter XP is
now captured separately from unavailable corpse/loot admission and uses the
existing native queue and atomic party XP/progression owner. Tagged Instance
keys are retained by the three progression consumers; migration1023 extends only
the Soul/Rotten receipt key shapes. No new loot admission or whole-quest claim.

RED: l3_instance_endgame three cases actually executed and failed because no XP
occurrence survived native boss deaths (200.25s). First implementation compile
found four ordinary Rust issues, corrected. Four-case subsequent PG run reached
XP/progression but all four rejected at deferred COMMIT (237.41s). Root cause is
being diagnosed, not treated as contention or an external blocker. Temporary
[L3DBG] commit metadata logging exists in durability/db.rs and MUST be removed.
Current diagnostic command is the absolute queued cargo test --locked --offline
-p oteryn-gameplay-dispatch --lib
l3_instance_endgame_soul_war_native_death_progresses_once -- --nocapture
--test-threads=1; log l3-instance-diagnostic.log. No successful new test claim yet.

New cases share boss_activity/endgame.rs after real wire admission and native
lethal projection; pending final filter l3_instance_endgame covers five cases
including late-death refusal and Primal hazard. New scope unit under durability
scoped_death_tests also awaits execution. Late-room entitlement guard, final
checks, checklist/caller sweep and ledger shrink remain active implementation
steps. Finish diagnosis first; do not weaken any constraint/test. Temporary
intermediate compile/PG logs are preserved under .questpc2/l3-instance-*.

---

# Round L3 active checkpoint — 2026-11-06

Task questpc2-L3. AUTHORING, single agent, current worktree only; no Git
mutation commands, frozen SHA or PR. Operator snapshots can advance HEAD;
review this round against 7aa809818, which contains the inherited Present work.
Scope: the 35 entries in LANE-L3.json. Source status files were checked for all
from_lanes, including questpc3's `quest` field; none matched QUALIFIED. All 35
canonical definitions were located (L3-CANONICAL-AUDIT.json).

First admission slice implemented: W02 rechecks source-native Soul War and
Rotten Blood level/access/final-chain predicates for every participant under
its existing admission transaction and Character root locks. Native creature
identity controls the gate, including catalogue aliases. Soul taints expire at
the existing fourteen-day boundary. The W11 preview now continues to the real
shared owner; Primal behaviour is unchanged. No migration, dependency, command,
capability, content pin, source map or existing fixture policy changed.

Three real-wire PG scenarios cover Spite, initial purple Megalomania and
Bakragore. They refuse low level, missing access, incomplete/expired final
flags without handoffs/cooldowns, then use valid imported snapshots to cross
the real scope barrier and replay the retained handoff. Imported snapshots and
isolated room geometry prove admission only; no whole quest is qualified.

RED executed 169.32s: 0 passed / 3 failed because below-level actors were
actually admitted. Expanded run exposed a fixture-only revision-zero track
CHECK; fixed its import to advance root/progression revisions together. Final
three-case run passed 3/0/0 in 212.44s with completed markers: Bakragore73861ms,
Spite69044ms, purple Megalomania69007ms (l3-endgame-final.log). Existing endgame
refusal regression passed1/0 in4.22s; ordinary W02 entry/return/replay regression
passed1/0 in70.34s. Both have completed PG markers. Six pure endgame units pass.
All checks use the absolute queued cargo, offline/locked, serial filtered PG;
no skipped case counts as evidence.

Validation process completed successfully: all7 checks in l3-verification.json
PASS. PG regressions2/0, pure units6/0, new PG3/0; total11 passed,0 failed or
ignored. Changed durability+dispatch crates passed all-target/all-feature clippy
-Dwarnings (231.497s); cargo fmt/check, explicit new-module rustfmt and whitespace
checks passed. No product edit after the corrected final PG run. Full product
checklist/caller review: L3-REVIEW.md and l3-final-caller-search.txt. Exact commands:
L3-TEST-FILTERS.md. All8 Rust paths: L3-CHANGED-FILES.txt, including the new helper
boss_room/quest_entry.rs that the operator must include with its parent mount.

No frozen SHA, PR, independent review or readiness claim. No Git mutation
commands, subagents, Docker, credential reads, dependency downloads or external
writes. No owner questions. The task is unfinished; the source-native room/map/
template and encounter/corpse/reward path is still missing for these two quests,
and the other33 L3 source mechanisms remain unimplemented here. No external
owner dependency is asserted. Exact gaps for every entry: L3-BLOCKERS.md.
Next operation: author the missing source-native encounter and reward paths
listed there; preserve the verified admission checks and the inherited N–Q
ledger under QUEST-STATUS.json /prior_rounds. L3-REPORT.md is the current report.

Whole-task counts remain
0 qualified / 35 unfinished / 0 external blocked. Exact remaining source paths
are in L3-BLOCKERS.md and QUEST-STATUS.json /quests. L3 owns their mechanisms;
historical qengine allocation language is not an external blocker.


---

# Active restart checkpoint — 2026-11-06 (supersedes history below)

Task_id questpc2. AUTHORING, single agent, current directory only. Detached HEAD
bab350d65 (observed, not changed); no frozen SHA, branch mutation or PR. No Git
add/commit/push/checkout/reset/stash, subagents, Docker, credential access,
dependency download or external write. Absolute queued cargo, offline, jobs4,
serial own-mechanic PG filters only.

Available ledger:39 N–Q titles,14 inherited scoped QUALIFIED and25 actionable
IN_PROGRESS,0 confirmed external BLOCKED. Historical BLOCKED labels are
superseded by the latest NO DEFERRING instruction; source gaps are preserved.
No current in-flight ITEMHOLD/WORLDCHG assignment is present locally. Pending
batched owner question: location of the requested94-blocker ledger and absent
OWNER-DECISIONS.md, plus exact current external-owner assignments. Do not invent
missing69 rows or external ownership. MECHANISM-GROUPS.json groups all25 known
gaps with their saved source evidence:10 outfit,5 world-change,3 exact-container/
hand-in,2 kill/mission and5 individual mechanisms. All remain actionable.

One shared mechanism slice completed: Seymour empty Present2856 -> Legion
Helmet3374, on existing CRAFT-1 C8/NPC item writer. Source: saved Present Quest
Spoiler transcript; captured items.xml:5953 and9797. New migration1022 extends
closed recipe/consistency guards; no old migration or shared fixture changed.
Al Dee shares its existing one-item planner/audit shape. The touched lane module
is renamed npc_item_exchanges per owner naming rule, retaining one root mod/call.

Red PG actually ran59.21s, missing admitted Seymour dialogue. Final own exchange
filter passed4/0, with timed markers: Uzgod56578ms, Al Dee71409ms, Seymour72566ms,
filled-present preservation60804ms. Yasir Sell All regression passed62903ms.
All31 focused tests passed (9 trade units,17 NPC units,5 actual PG scenarios),
0 failed/ignored. All9 verify-present-exchange.sh stages PASS: focused tests,
check and all-target clippy/-Dwarnings for durability/npc/dispatch/game-server,
fmt, explicit mounted-file rustfmt and product whitespace. No code edit after
these checks. Exact commands/results: TEST_FILTERS.md and
present-exchange-verification.tsv. Constructor/caller sweep and whole-diff
checklist review: present-exchange-caller-search.txt and REVIEW.md. No new enum
variant, struct field or trait method. No independent-review/readiness claim.

Present stays IN_PROGRESS: its exact source chest still needs Item2881/2882
semantics/admission and actual acquisition-to-exchange proof. A carried exchange
alone does not qualify that full quest. No new full-quest qualification was
claimed. Next: reconcile the94-quest input/current ownership, continue shared
Game admission and mission mechanisms from MECHANISM-GROUPS.json, then qualify
complete paths in the requested5–10-quest batches. Do not treat these25 gaps as
final blockers. Check1022 allocation with the coordinator before publication;
no publication is authorized here. Inherited Item2816 Python assertion is only
a historical unfinished report, not rerun (no native profile changed).

Changed files: CHANGED-FILES.txt (81 product paths including inherited work),
PRESENT-CHANGED-FILES.txt (13 current delta paths including the rename pair).

---

# Current final checkpoint — 2026-11-06

AUTHORING; questpc2 N..Q; HEAD b2cfa798e3def61002d39a6da8b9de5df5aa2fd8.
Current directory only, no Git mutations, frozen SHA or PR. Initial absolute-
wrapper dispatch check passed. QUEST-STATUS.json now contains every39 title:
14 explicitly scoped QUALIFIED rows and25 canonical/engine BLOCKED rows, each
with exact evidence, attempts and missing pieces. Reward-only scopes retain
all access/story limits; no full-source completion is inferred from them.

New scoped qualification: Obsidian Knife Quest via the source Uzgod direct
carried steel+lance exchange. Current real-dispatch PG test
obsidian_knife_uzgod_atomic_exchange_cooldown_and_replay passed with
POSTGRES_CASE_COMPLETED/61480ms. Pick regression63180ms; Yasir Sell All/ariki
regression67591ms. No route-less return. All28 named test executions passed:
8 durability units,17 NPC units and3 actual PG scenarios,0 failed/ignored.

All10 rows of item-exchanges-verification.tsv PASS: the focused tests,
server check, all-target/all-feature clippy -Dwarnings for content/world/npc/
durability/dispatch and game-server, fmt, explicit mounted-file rustfmt and
product whitespace. Commands are in TEST_FILTERS.md and
verify-item-exchanges.sh. No feature code changed after the current PG run.

Shared-file repair: NPC routing now adds exactly one root mod and one dispatch
call; Oriental Trader is a child of questpc2_item_exchanges. Existing ordinary
conversation/window and durable owners remain in use. Migrations1020/1021 are
new and unmerged;1022..1039 free. No command id, capability bit, shared fixture,
lease, replay, fresh-admission or old migration was changed. The new schema
guards preserve existing Buy/Sell behaviour, proved by Yasir's real Sell All.

Whole product diff reread against owner and repository checklists.79 product
paths are listed in CHANGED_FILES.md; raw reference/scratch artifacts are not
publication files.385 whole-workspace constructor/caller/enum references are
in final-constructor-search.txt. Current semantic audit proves13 native
additions/no removals/three prior fact fills;23 additive selectors in15 regions;
only Poison Daggers canonical reward corrected, immutable source preserved;
17 proved claims removed from the ledger with all280 other records unchanged
and336 source claims conserved. See REVIEW.md/final-semantic-review.json.

Unfinished: all25 exact blockers in QUEST-STATUS.json (canonical Item/Outfit/
Service admission and excluded native World bindings), each qualified row's
explicit access/story limits, and the inherited unrelated Item2816 Python
negative assertion at test_writable_item_profiles.py:95. No missing-test-only
blocker, owner question, contention exemption or30-minute failed-test overrun.
No dependency download, external write, credential/Docker access, Git mutation,
paid review or independent-review/merge-ready claim. Return this local worker
packet to the coordinator; qualification remains scoped as recorded.

---

# Pick exchange continuation — 2026-11-06 (current)

AUTHORING; questpc2 N..Q, HEAD b2cfa798e3def61002d39a6da8b9de5df5aa2fd8;
no frozen SHA/PR or Git mutation. Initial restart check and all required notes read.
Pick is now QUALIFIED for its source Al Dee exchange using a carried small axe.
New own module questpc2_rookgaard.rs plus one additive root module/call use the
existing NPC trade writer, a closed internal Exchange side and migration1020.
No public wire enum, command or capability changed. Accepted contract: CRAFT-1
C8 and DUR-03 amendment; no craft bypass or independent consume/grant writes.

PG first red: missing Al Dee dialogue. Then own fixture required hi after each
disconnect; no production admission guard changed. SQL diagnostic exposed a
broad token replacement affecting Sell All alias l. Migration1020 now replaces
only its first intended per-line clause. Diagnostics/constraint forcing removed.
Final Pick PG1/0 POSTGRES_CASE_COMPLETED/54592ms; source/quote cancellation,
no unconfirmed mint, atomic swap, live duplicate, cold production-handler
receipt replay and no-axe refusal. Oriental Trader Sell All regression1/0
marker55948ms. No route-less qualification. Pure durability7/0 and NPC dialogue17/0.
All six stages of verify-pick-exchange.sh PASS; clippy/all targets/all features
-Dwarnings on content/world/npc/durability/dispatch, fmt and whitespace pass.

QUALIFIED: Pick Quest - pick_exchange_atomic_confirmation_refusal_and_replay (54592ms).

QUEST-STATUS.json has15 dispositions: Pick QUALIFIED and14 canonical/data
BLOCKED quests. Other24 titles are not final: do not end the task. Next Phase B:
Uzgod steel5889+lance3313 -> knife5908 with20-hour cooldown. All three Native
profiles and canonical materials are ready. External Uzgod research absent; use
saved Obsidian Knife Wiki transcript. Dragon Shield3416 for the optional Sweaty
Cyclops route is canonically held and outside this lane's quest ownership.
Migrations1021..1039 remain free; no new command/capability. Keep one cargo at
a time, jobs4, absolute queued wrapper, filtered PG and offline dependencies.

Source review, files/caller sweep and final full-task verification are still due
before final report. Existing Item2816 Python negative assertion remains the
inherited Unfinished item. No independent-review/merge-ready claim.

---

# Panpipe access continuation — 2026-11-06 (current)

AUTHORING; questpc2, N–Q only; HEAD unchanged, no frozen SHA or PR. Read the
restart notes and bound immutable policy; initial dispatch check passed in
current-restart-check.log. Panpipe's missing source action4055 at
(32643,32128,8) and native locked1631 are now restored by their generators.
No new migration, command, capability, shared engine or NPC hook.

New real-dispatch PG test quest_reward_claims_panpipe_locked_door_to_reward_and_replay
passed1/0 with POSTGRES_CASE_COMPLETED/27802ms in
panpipe-access-pg-replay-fix.log. The same test first reached PG and failed on
None versus Some(4055) before the map fix. Hollow-stone Key4055 separately passed
with marker26567ms in panpipe-access-pg-green.log. That earlier two-case run
then exposed a lane-private stale replay cursor; carry the returned continuity
as the existing M06 case does. No production or shared fixture guard changed.
The case seeds carried key custody and tests direct locked USE, wrong key,
matching key/replay, paced STEP through the door, exact bag contents and repeat
refusal. The long approach, shovel/ladders/rope journey and exit remain unqualified.

Final serialized validation passed all15 stages in verify-panpipe-access.sh;
panpipe-access-verification.tsv and individual logs retain the evidence. The
final repaired Panpipe pair passed2/0 in65.52s (positive PG markers), after
which no feature code changed. Final named Rust commands passed170 executions,
including five actual PG scenarios. Fmt, clippy/-Dwarnings, server content and
World pin-check all pass. Native provider has only one additional
Item1631 versus the prior checkpoint; map has exactly one additional action.
Fresh semantic/metadata and caller/constructor sweeps are saved under
panpipe-access-*. No five-new-quest checkpoint is due; this extends an existing
qualified reward set, so totals remain11 sets/18 claims plus Oriental Trader.

Unfinished: an overbroad Python profile discovery found the unrelated existing
negative assertion in test_writable_item_profiles.WritableProfilesTests.
test_native_round_trip_is_idempotent_and_does_not_grant_mint_authority: line95
assertNotIn Item2816 fails. Item2816 exists identically in HEAD;108 other tests
passed. See panpipe-access-unrelated-python-failure.md. No test was weakened.
All existing Phase B/C and held-item limits in PHASE-STATUS.md still apply.

---

# Oriental Trader continuation — 2026-11-06 (current)

AUTHORING; questpc2, N–Q only. HEAD remains
`b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`; no frozen SHA or PR.
All four restart notes read; initial dispatch check passed. The appended
16:20 routing grant resolves the previous owner question: do not ask again.
Phase B's item exchanges are blocked on their atomic durable owner, not routing.
See PHASE-STATUS.md for all39 dispositions.

Implemented Oriental Trader using existing owners, without a new shared engine.
Yasir was omitted from the talk book because his source Dialogue has no static
GREET. The real-catalogue unit was red (`Yasir conversation missing`), as was the
PG case (`committed Yasir dialogue is not admitted`). Admit only his captured
literal greeting/trade/farewell from the existing npc-br snapshot, revid433385,
and version the talk catalogue. The shared NPC file adds one sorted module and
one call; the mechanic-named module handles only Yasir's ariki window alias.
Existing NPC_SHOP and P11 own the complete sale and once-only achievement.
No migration, command, capability, inventory writer or appearance grant added.

First real dispatch PG run passed1/0, POSTGRES_CASE_COMPLETED/46850ms
(`oriental-trader-pg-green.log`). NPC dialogue units passed17/0; server ship
placement/absence units passed2/0, covering all three ports; reset determinism
passed1/0. Final content/world/npc/dispatch all-targets/all-features clippy with
-Dwarnings and fmt passed. The initial clippy type-complexity finding required
only a private test return-type alias; it changed no runtime behaviour.

Final filtered PG rerun passed1/0, POSTGRES_CASE_COMPLETED/61158ms
(`oriental-trader-pg-final.log`). No route-less return. The final validation
commands cover21 tests:17 NPC dialogue units,3 server port/reset units and1 real
PG scenario,0 failures. Clippy/all targets/all features/-Dwarnings, cargo fmt,
explicit auto-mounted-test rustfmt and product whitespace all passed. No feature
edit followed the first successful PG run; only the private test type alias
preceded the final run. Full self-review and workspace caller/constructor sweep
are recorded in REVIEW.md; CHANGED_FILES.md contains all66 product paths.

QUALIFIED: Oriental Trader Mini World Change - oriental_trader_catalogue_ariki_sale_and_achievement_once
Paired with server
`yasir` port/absence tests and the fixed reset-epoch roll test. P11 deliberately
grants Si, Ariki! on a committed trade, not opening the window. This qualifies
one actual admitted creature-product sale/replay and the existing three-port
world placement; individual product Item holds remain with their owning lane.
No new five-quest batch checkpoint is due: only one new title qualified here.

Old reward totals remain11 sets/18 claims; Oriental Trader adds no RewardClaim
selector or ledger entry. Canonical Power Ring/Ornamented Shield, remaining NPC
item exchanges, storylines/outfits and recorded access limits remain unfinished.
No Git mutation, dependency download, credentials, Docker, external writes or
new source collection. Migrations1020–1039 remain unused. All cargo calls were
serialized through the absolute wrapper, offline, CARGO_BUILD_JOBS=4.

---

# Restart routing audit — 2026-11-06

AUTHORING; questpc2, N–Q only. Detached HEAD remains
`b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`; no frozen SHA or PR.
Read RESUME and all four notes from `.questpc2/` (the three short NOTE names
are not repository-root paths), the bound immutable META policy and required
repository skills. Initial absolute-wrapper dispatch check passed (exit0,
current-dispatch-check.log). No unfinished Rust edit was found.

No production code or generated content changed in this continuation. Fresh
content/world/dispatch all-targets/all-features clippy with -Dwarnings, cargo
fmt --all --check and product git diff --check passed. Both dispatch source
selector units passed (1+1); Python source selectors10, Poison Daggers4 and
qualification-ledger7 passed. All cargo calls were serialized with four jobs
and offline mode through the absolute wrapper.

Full product-diff checklist review and constructor/caller search were repeated.
Evidence:current-constructor-search.txt, current-semantic-review.json,
current-metadata-review.json. The15 regions contain exactly22 additive source
selectors with all other facts unchanged. Native profiles have12 additions,
no removals and the same three UNKNOWN-to-KNOWN fills. Only the canonical
Poison Daggers reward differs; source_variant is unchanged. The ledger removes
exactly17 proved claims; all other280 records are unchanged,336 total conserved.
No new enum, struct field, trait method or public function was introduced.
No unresolved finding was identified in the implemented reward scope.

Historical PG logs were checked for actual completion markers, positive
durations and final zero-failure results. These remain prior qualification
evidence, not fresh PG runs. No unchanged heavy PG cohort was rerun. The total
remains11 distinct quest reward sets/18 exact claims; no new qualification or
five-quest checkpoint is due.

BLOCKED: Phase B production routing remains unresolved. NOTE-coordination.md
reserves npc_command.rs to questpc and permits only one module declaration.
The current production NPC route explicitly calls each mechanic; there is no
lane extension call. A batched owner question is pending: questpc supplies the
routing hook, or the owner authorizes the minimal hook beside the declaration.
No shared NPC file or uncalled public NPC API was added.

BLOCKED: Pick Quest exchange - existing NPC_TRADE is coin-priced buy/sell;
quest_dialogue only commits QuestState transitions; item_craft is restricted
to its admitted conversions and USE payloads. Al Dee needs an atomic,
receipt-keyed small-axe-for-pick NPC transaction. Do not simulate it with
independent consume/grant writes or build a replacement shared engine here.
This is additional to the routing dependency, not missing reference evidence.
Saved Wiki Pick transcript is the working source; no new research was collected.

Unfinished: Power Ring and Ornamented Shield canonical Item/charge holds,
Phase B NPC exchanges/trades, and Phase C storylines/outfits/world changes and
previously recorded access limits. Full39-title disposition remains in
PHASE-STATUS.md. Migrations1020–1039 remain unused; no command/capability change,
Git mutation, external write, dependency download or credential access.
Next: resolve the NPC routing dependency and obtain the admitted exchange
primitive before implementing a value-moving NPC handler; retain all holds.

---

# Current restart verification — 2026-11-06

AUTHORING; questpc2, N–Q only. Detached HEAD remains
`b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`; no frozen SHA or PR.
Read RESUME and all four restart notes, bound immutable META policy and required
repository skills. Initial absolute-wrapper dispatch cargo check passed, exit0
(current tool transcript,76.0s). No incomplete Rust edit was found. No product
code or generated content changed in this continuation.

Fresh content/world/dispatch all-targets/all-features clippy with -Dwarnings
and cargo fmt --all --check passed. The three `quest_reward_claims_batch_two_`
PG tests passed,0 failures,122.21s: four actual claim-completion markers with
positive durations (Parchment36261ms, Poison Daggers27764ms, Power Bolts29914ms
and28216ms). The Naginata full mechanic journey also passed,1 test,0 failures,
POSTGRES_CASE_COMPLETED/40407ms. No route-less return, test weakening, unrelated
failure or30-minute failed-test investigation occurred.

Focused Rust units also passed: two dispatch selectors,12 World facts and6
reward-content tests. Together with4 executed PG scenarios this continuation
passed24 Rust tests,0 failures. Python passed54 tests:10 map/selector,13 native
reward,4 Poison Daggers boundary,20 Item profile and7 ledger guards. Exact unit
commands/results are retained in continuation-units.tsv and TEST_FILTERS.md.

Fresh semantic self-review reproduces all22 exact additive source selectors in
15 map regions,12 native additions/no removals/three established fact fills,
and exactly one changed canonical reward (Poison Daggers), retaining its
source_variant. Evidence:continuation-semantic-review.json. Workspace-wide
constructor/caller search is in continuation-constructor-search.txt; the two
public map functions have production terrain-tool callers and the fresh
Naginata e2e case. No new enum variant, struct field, trait method or public
function was added in this continuation.

PHASE-STATUS.md now covers all39 scoped titles exactly once, distinguishing
11 qualified reward sets/18 exact claims from unfinished whole-quest journeys.
These are unchanged totals; no repeated test is counted as a new qualification.
No new QUALIFIED checkpoint is due. Cached source material was used; no new
reference collection, external mutation, Git mutation, dependency download,
Docker or credential access occurred. Migrations1020–1039 remain unused.

Unfinished: canonical Power Ring and Ornamented Shield Item/charge holds,
existing access limits, Phase B NPC exchanges/trades and Phase C storylines and
outfits. Shared npc_command.rs remains untouched. Pending owner question:
questpc must provide the production routing call, or authorize questpc2's
minimal call in addition to its permitted module declaration. No uncalled
public NPC API or substitute shared engine was built. Resume at that routing
dependency; do not repeat unchanged heavy reward runs merely for a checkpoint.

---

# questpc2 current checkpoint — 2026-11-06

Continuation in AUTHORING: startup dispatch check passed again
(`resume-dispatch-check.log`). Panpipe Key4055 now passes its real production
dispatch case:1 passed,0 failed,POSTGRES_CASE_COMPLETED/26039ms
(`panpipe-key-pg-green.log`). Regression first failed with ItemReference
(`panpipe-key-pg-red.log`); the authoring selector regression was also red first.
The runtime already supported this complete key-only variant. The bounded fix
selects it and supplies exact source stone1777 physics; no new engine feature,
source research, migration, command or capability. Key number4055 and its mint
transaction are atomic, with stale-fence refusal and live/cold replay retained.
Source capture: `samples/quest-chest-bindings/panpipe-key.json`.

Fresh content/world/dispatch all-targets/all-features clippy/-Dwarnings passed
(`panpipe-key-clippy.log`). Ten selector/overlay Python tests,15 profile tests,
7 ledger guards and129 governance tests passed; content/world/governance
validators passed. Ledger now removes only the proved Key4055 record (281->280);
every other280 record is unchanged, with336-total and28/28/280 conservation.
All11 final verification rows PASS (`panpipe-key-verification.tsv`): Panpipe
bag PG (25998ms), both selector units,12 World units,6 reward-content units,
123 server content tests,5 World repository tests, pin-check, fmt and whitespace.
Together with the new key PG case,150 Rust tests passed,0 failed. Both PG cases
have actual positive-duration completion markers; no route-less return. The
regenerated World pin is eaf464f35b521b0dac01b07afe055100a1a173b93e57d5e57a13912ff3e0e2d9.
No feature-code edit followed final clippy and routed PG execution. Existing
qualifications below remain historical evidence; no unrelated heavy PG rerun.

QUALIFIED: Panpipe Quest - quest_reward_claims_panpipe_key_4055_once_and_replay
(Key4055 acquisition only; existing bag separately qualified; actual dungeon
door and approach remain unqualified). Total exact captured claims:18 across
the same11 distinct quest reward sets, not12 completed quests.

Phase B's minimal shared NPC routing call remains an unanswered owner question;
no shared NPC edit has been made. Power Ring and Ornamented Shield retain their
canonical Item/charge holds. Phase C storylines/outfits remain unfinished;
item-cosmetic source rules alone cannot grant outfits while the canonical
`content/cosmetics/outfits/index.json` is READY_UNPOPULATED.

AUTHORING. Detached HEAD `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`;
no frozen SHA or PR. Local edits only; no Git mutation, dependency download,
Docker, credential access or external publication. Scope N–Q; migrations1020–1039
unused; no new commands/capability bits. Shared npc_command.rs remains untouched.

Phase A reward qualification:11 distinct quest reward sets /17 exact captured
claims. Fresh actual PG runs:5+3+7 tests,zero failures,25 positive-duration
completion markers; no route-less return. Batch checkpoints and per-quest
QUALIFIED lines remain in the history below. Naginata's full mechanic journey
is exercised with reduced approach corridors; other reward proofs do not
certify whole access paths/storylines.

This restart completed the mechanic-named fixture rename, retained all14 original
cases and added eight per-quest wrappers. Fixed Poison Daggers' real target-count
bug: two independent quantity1 daggers and30 arrows, exact source placement,
atomic four-instance audit and replay. Immutable donor/source_variant retained.
Fixed the stale qualification ledger:16 newly proved N–Q selectors removed from
its unqualified set, all other281 records unchanged. Exact336-total and27/28/281
conservation asserted; all negative ledger guards retained.

All final checks PASS: fmt; content/world/dispatch all-targets/all-features
clippy/-Dwarnings; two selector units;12 World facts;6 reward-content units;
123 server content tests;5 World repository tests; final World pin-check;
product whitespace. Nine final TSV rows PASS. Python suites29/30/28/7 pass;
source capture, deterministic content and ledger checks pass. No feature-code
edit followed final clippy and routed PG tests. World pin remains55d4d6bf...e93d7.

Unfinished: Power Ring target/source and canonical Item3056 charged-reward
admission; Ornamented Shield's Item3083/source-charge holds and partner access;
Newhaven prerequisites; Nightmare activation; Noble Armor NPC/key route;
Panpipe key/access; Ghostship travel; Parchment trap/access; Power Bolts sword
and carried-book READ. Phase B NPC exchanges/trades and Phase C storylines/outfits
remain incomplete. Native profiles alone do not qualify held canonical claims.
No unrelated/load-contended test failure was observed; no CI-only case to record.

Pending owner question: which production routing hook may the lane-owned NPC
submodule use? NOTE-coordination reserves all other npc_command.rs edits for
questpc, allowing only one mod line. Await questpc's call or explicit authority
for the minimal routing call. No unused public NPC API or permission expansion.

Handoff: CHANGED_FILES.md lists60 product paths; TEST_FILTERS.md gives all exact
commands; REVIEW.md records the full self-review, resolved findings and limits;
SHARED-FIXES.md gives the minimal shared authoring/ledger coordination changes.
Fresh proof:restart-batch-one-pg.log (543.57s), restart-batch-two-pg.log (156.19s),
restart-n-regression.log (530.34s), restart-verification.tsv (nine PASS rows).
Product raw/reference files and scratch .otwb bundles are not publication artifacts.

Next: route Phase B through an authorized production hook and qualify each
exchange through real dispatch. This task is not whole-quest/merge ready;
no independent-review or background-continuation claim.

---

# questpc2 checkpoint

Task: questpc2. Worktree HEAD: detached at `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8` (no branch mutation authorized). State: AUTHORING; no frozen SHA or PR.
Owner restrictions: local edits only; no git add/commit/push/checkout/reset/stash, no dependency downloads, no Docker or credentials. Migration allocation 1020–1039 remains unused. No command/capability allocation.

Scope: N–Q only, 39 pinned titles (owner NOTE-split.md). A–M belongs to questpc; R–S to questpc3; T–Z and digits/other characters to questpc4. Dawnport, OT-only and Prey Hunting Tasks excluded. Newhaven is not inferred to be Dawnport merely because a historical claim contains `legiondawnport`.

## Active alphabetical batch: N (14 titles)

Batch N remains open: 1 quest DONE (Naginata); 13 N titles still unfinished. Three reward/access slices are implemented; qualification is running. No STOPPED dispositions. Full source journeys must still be completed before any quest receives DONE.

| Quest | State and next action |
|---|---|
| Naginata Quest | DONE: source pick6/hazard/removal, rope ascent, two descents, level40 door, exact Item3314 chest and completion/replay run through production wire dispatch. Production `crates/gameplay/dispatch/src/gameplay_transport/item_craft/terrain.rs:222` and `crates/gameplay/world/src/map/facts/quest_callbacks.rs:81`; PG `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc2_rewards.rs:177` (`naginata-journey-complete.log`, POSTGRES_CASE_COMPLETED/47533ms). Approach corridors are reduced, source coordinates/selectors and every mechanic step retained. |
| Newhaven Legion Helmet Quest | UNFINISHED: UID6006 source chest identified. Wiki prerequisites name Gustavo and Viola; qualify tutorial prerequisite and helmet together. |
| Newhaven Quest | UNFINISHED: production tutorial owners exist; trace pinned Gustavo/Viola missions and boss/reward path rather than assuming tutorial-islet coverage proves the whole quest. |
| Nightmare Isles Mini World Change | UNFINISHED: identify active-island source/world-state callbacks and prove entrance-to-teddy path. |
| Nightmare Outfits Quest | UNFINISHED: trace Dream Challenge ranks, NPC tokens and outfit/mount reward consumers. |
| Nightmare Teddy Quest | IN_PROGRESS: restored Crystal UID6305 (Canary6306), appearance2433, and admitted the exact existing canonical Item21982 into the native provider (weight590 from items.xml). Real USE/replay/fence PG case added. Island activation/access still needs implementation/qualification. |
| Nimmersatt's Lair | UNFINISHED: use pinned all373 wiki source and Crystal boss source to implement access/kill/reward journey. |
| No Rest for the Wicked Quest | UNFINISHED: source-only NPC rules at content/dialogues/quest-batches/quests-5/no_rest_for_the_wicked.json are not production execution. Wire NPC/start, bosses and final reward. |
| Noble Armor Quest | IN_PROGRESS: restored UID6132 Item3380, UID6133 Item3385, and level35 door action1035. Independent real USE/replay/fence and level34/35 PG cases added. Skjaar1000gp quiz and key3142 access still need execution proof. |
| Nobleman Outfits Quest | UNFINISHED: trace exact NPC purchase/addon payment and outfit owner. |
| Nomads Land Quest | UNFINISHED: read structured wiki and Crystal nomad access script, implement complete progress and reward. |
| Nomads Mini World Change | UNFINISHED: trace timed world-state source and Nomad NPC entrance. |
| Noodles is Gone Mini World Change | UNFINISHED: source NPC and world-state cycle need production wiring and PG proof. |
| Norseman Outfits Quest | UNFINISHED: family source binding points at outfit_and_addon_quests; qualify full NPC exchange/outfit grants. |

## Evidence and commands

- Bound META policy3.1.0 resolved at immutable1bfb5ff98c8aa156e73669a14e083a1d464c29fb.
- Source capture: `tools/content-schema/world-authoring/samples/quest-chest-bindings/questpc2-n.json`; Crystal201189f chest/level tables and startup loadLuaMapUnique are pinned by digest. The capture replays with `quest_chest_bindings.py --capture .questpc2/reference --claims oteryn:reward-claim.kv.naginata oteryn:reward-claim.kv.nightmare_teddy_quest oteryn:reward-claim.kv.noblearmor1 oteryn:reward-claim.kv.noblearmor2 --output tools/content-schema/world-authoring/samples/quest-chest-bindings/questpc2-n.json --check`.
- Fandom MediaWiki API successfully re-fetched N–Q titles and available spoilers on2026-11-06 using the owner helper. Captures are in `.questpc2/wiki/`; the earlier403 was a User-Agent/raw-page route issue and is resolved.
- `map-semantic-diff.json`: exactly four chest unique IDs, two level-door actions and Naginata rock action40031 added; all other map tile/item facts equal HEAD.
- Native provider semantic diff: only Items1791,21982,6264,6265 added; no existing Item record changed or removed (`native-semantic-diff.json`). Generated registry/pin changes are in `generated-diff-paths.json`; the Raid catalogue changes only the affected region digest.
- Red evidence: `committed-map-red.log` (missing source UID), `nightmare-admission-red.log` (missing native reward), initial routed `batch-n-tests.log` (Teddy NothingToUse). A later fixture corridor bound error was corrected before final validation.
- PASS: Python callback profile suite14; overlay suite7; world-base suite57; Edron build4; minimap build3; full committed map validator.
- PASS: `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture` (6 tests).
- PASS: `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test integration_rest content_ --quiet` (123 tests).
- PASS: `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet` (5 tests).
- Initial dispatch clippy all targets/all features -D warnings PASS. Final rerun pending after the last fixture adjustment; log `dispatch-clippy-final.log`.
- Final routed run pending: `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib questpc2_rewards -- --nocapture --test-threads=1`; log `batch-n-final.log`. Require seven POSTGRES_CASE_COMPLETED markers with duration; an early return is not qualification.
- World pin-check caught a stale digest from access-door additions. Compiler pin-refresh and final pin-check pending; never hand-edit the digest.
- Full diff re-read against owner CODEX-CHECKLIST. Constructor search across apps/, crates/, tools/ found all four private Chest and two private Gate fixtures. No new enum variants, trait methods or pub Rust functions. No test was removed, skipped or weakened. Staging index remains empty.

## Next step

Finish the in-flight checks; fix any own-case failure, update exact results and file:line references here. Keep the batch open until the full N journeys are executable. Then continue the remaining N titles, followed by O/P in alphabetical batches of10–25. No task completion or merge readiness is claimed.

## Remaining titles in this lane

The active N batch above remains open. Naginata is DONE; the other13 N quests and all25 O/P quests remain unfinished. After it, process these25 O/P titles (there are no pinned Q-initial titles):

- Obsidian Knife Quest
- Opticording Sphere Quest
- Oramond Quest
- Orc Fortress Quest
- Orc Shaman Quest
- Orcsoberfest
- Oriental Outfits Quest
- Oriental Trader Mini World Change
- Ornamented Shield Quest
- Overhunting World Change
- Panpipe Quest
- Parchment Room Quest
- Pick Quest
- Pilgrimage of Ashes Quest
- Pirate Outfits Quest
- Plate Armor Quest
- Poacher Caves Mini World Change
- Podzilla Quest
- Poison Daggers Quest
- Poltergeist Outfits Quest
- Power Bolts Quest
- Power Ring Quest
- Present Quest
- Primal Ordeal Quest
- Purple Tome Quest

## Current continuation (2026-11-06)

- Immutable META3.1.0 was refreshed from its bound commit into `.questpc2/META_POLICY.md`; no publication is authorized.
- Native source files (Canary99902524, verified by the existing generator against canary-ground-proof.json) re-fetched into `.questpc2/reference/native-items/`. They are research inputs, not redistributable game assets.
- Found and repaired a real production gap: Noble Armor's level door6264 and open6265 were absent from native Item profiles. Added them to the existing bounded key-door generator/admission set and regenerated its output and native providers. Red evidence: `noble-door-admission-red.log`. Native profile tests PASS16 (`native-door-profile-tests-final.log`).
- Corrected the PG map fixture to retain Item classification and native physics instead of synthesizing Terrain for source doors/chests. The real door test exposed `Rejected` at the wire boundary after fixing its earlier `NothingToUse`; qualification continues. Capabilities now use the same selected set as the existing placed-door PG case. Do not weaken the Locked/Committed assertions.
- The owner-requested OTBM decoder now compiles with zlib; its11 structural tests PASS (`otbm-nodes-tests.log`).
- Newly fetched Wiki says Nightmare Teddy requires participation in killing all six Terrorsleeps. The current mint/replay test proves only the reward slice and does NOT establish that eligibility. Implement the eligibility and source island entry before claiming the quest complete.
- Naginata source rock: `(32356,32074,10)`, Item1791 over ground385. Crystal startup/tables/item.lua:2612-2616 assigns action40031; register_actions.lua:751-774 counts six shared picks, removes the rock on the sixth and inflicts31–39 physical damage on each pick. The action40031 map selector and native1791 profile are now regenerated. The production pick route and expanded pick/rope/door/chest PG journey are implemented; final qualification is pending. Note a later duplicate40031 table field at item.lua:4679: source must be disambiguated by exact position/appearance, never by action alone. Local source captures are under `.questpc2/reference/`.
- No new migration, command, capability, enum variant or trait method. New private NaginataPick fields (session/command/SHA256 binding) and CallbackStates.naginata_picks have one constructor/default initialization; all sites searched across apps/, crates/, tools/. Both new public map methods have production callers in item_craft/terrain.rs and are exercised by the filtered Naginata PG case.

Next: finish the filtered gate/reward PG run, repair its actual root cause, then implement Naginata's source rock interaction and qualify its pick/rope/access/reward path. Continue the remaining N titles before starting O/P. No missing-evidence stopping disposition is used.

### Qualification in progress

- Source overlay suite8 and combined World authoring suites106 PASS (`n-pick-world-tests.log`); native profiles19 PASS (`n-pick-profile-tests.log`).
- Naginata level39/40 wire case PASS with POSTGRES_CASE_COMPLETED/64277ms (`gate-item-wire-diagnosis.log`). Root cause of the typed-door wire refusal was the fixture's missing offered/accepted Item/map capabilities; it now uses the existing placed-door fixture selection.
- First rock PG run executed all six source hits and retained the pick, then failed because the replay fixture reopened with the old next-command cursor. Repaired the test to retain current continuity while replaying the original CommandRef; no production guard/assertion was weakened.
- The final Naginata journey uses carried Pick and Rope, real paced STEP/floor changes, the level40 door and final chest over production wire commands. It keeps source coordinates/selectors and reduces only approach corridors. No post-start direct actor relocation or fixture teleport is used.
- `n-pick-clippy-final.log` and the next focused pick rerun are pending. The initial clippy exposed the two distinct SemanticTimeMicros wrappers; the damage-owner call now uses its required foundation wrapper.

- First extended journey run was red on its first paced STEP (`naginata-journey-pg.log`): the fixture derived its movement cells from a different content generation than its runtime pin. Corrected the fixture to use the active generation's qualified room and the bundle's real frame, including the SQL activation. No production movement guard was altered. Focused rerun: `naginata-journey-pg-final.log`.
- Content validators PASS: native/world structural tree (`n-pick-content-validation.log`), materialized97/97 (`n-pick-tree-validation.log`), committed World base (`n-pick-world-validation.log`).
- Clippy of world+dispatch/all targets/all features returned0; repeat after the final fixture change is pending. No whole quest is marked DONE until its complete journey has a completion marker.

- The full journey reached command35 and the exact reward before a fixture replay failed with CommandOutcomeExpired. A reconnect starts an empty protocol response cache even when it retains the next-command cursor. Duplicate testing now sends both identical commands in one live connection; late cold replay separately proves the map-owned payload binding. Focused final rerun: `naginata-journey-complete.log`. No protocol cache/limit/guard was altered.

### Naginata qualification complete

`/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib questpc2_rewards_naginata_pick_six_shared_hits_and_replay -- --nocapture`: PASS1, POSTGRES_CASE_COMPLETED/47533ms. The same connection receives Accepted then DuplicateReplay for the first pick, with damage only once; the sixth shared hit removes the rock. Carried Rope, paced STEP/floor changes, level40 gate, source chest, one exact Naginata, cold replay and changed-payload refusal all execute.

Final authoring-tree verification is running in `.questpc2/verify-final.sh`; statuses in `.questpc2/verification-results.tsv`, transcript `.questpc2/verification-run.log`. No further Rust/content feature edits are planned until these checks complete. Next quest after qualification: Newhaven Legion Helmet Quest, including Gustavo/Viola prerequisites; keep working N before O/P.

## Restart: owner Phase A restructure (2026-11-06)

The latest owner order supersedes the alphabetical/full-journey order above: Phase A bulk chest/claim/lever rewards, then Phase B NPC exchanges/trades, then Phase C storylines/outfits. Scope N–Q; migrations1020–1039; no new command/capability allocations. Current AUTHORING, detached HEAD unchanged; no freeze/PR/publication.

- Read RESUME, NOTE-coordination, NOTE-memory and NOTE-browser (the latter two are inside `.questpc2/`, not repository root); refreshed bound immutable META policy via GitHub read-only tool.
- Fresh restart `cargo check --locked -p oteryn-gameplay-dispatch --quiet` PASS (`restart-dispatch-check.log`). Prior interrupted verification script finished, with all12 rows PASS in `verification-results.tsv`; earlier evidence is retained separately.
- Added one shared bulk reward scenario with leaf, container and authored-book cohorts. Selected12 claims across Orc Fortress, Orc Shaman, Panpipe, Plate Armor, Poison Daggers, Power Bolts and Purple Tome; retains ordered children, exact source selectors, stale-fence refusal, command and cold replay, atomic audit, authored-text digest. One test per cohort and per-claim completion markers/durations. Orc Fortress level39/40 test added using the same existing gate template.
- Red evidence `phase-a-selectors-red.log`: committed Orc Fortress source chest UID absent (`None` vs6020). Captured `questpc2-phase-a.json`, restored12 UIDs plus source level40 gate at32981,31760,9. Source normalization does not change held RewardClaim dispositions or introduce a new reward owner.
- World-authoring Python suite PASS214 (`phase-a-world-authoring-tests.log`); native-profile tests and content regeneration in progress. No new shared fixture code change.

### Unfinished / source decisions

- Power Ring Quest: both pinned Crystal and Canary chest tables give Item3031/gold instead of the Wiki Power Ring for UID6151, and UID6152 carries200 charges for bronze amulet. Do not qualify or silently mint the incorrect reward. Exact source conflict remains to be resolved through normalized authoring with current target evidence; no claim was changed.
- Poison Daggers: canonical/pinned donors give one Item3299, whereas current Wiki says two Poison Daggers. Bulk test qualifies the exact existing canonical reward only; full target parity remains unfinished.
- Plate Armor: reward-only qualification does not prove Ghostship travel routing. Purple Tome reward qualification does not prove its entire Demona access route. Power Bolts authored book digest qualification does not implement a carried-book READ transport.
- Newhaven/tutorial, Nightmare eligibility, Noble Armor NPC/key access and every outfit/storyline remain Phase B/C work; no missing-evidence STOPPED dispositions.

Next: await content regeneration, run fmt, filtered Phase A PG cohorts (absolute cargo wrapper, CARGO_BUILD_JOBS=4, one cargo at a time), repair own cases within30minutes each, record first5 qualified quest reward sets, regenerate coverage only after actual PG markers, clippy changed crates and required content checks. Never claim full-quest DONE from reward-only evidence.

### Phase A first PG run / carrier repair

`phase-a-pg.log` is the first routed PG run. Real completion markers: Power Bolts authored book, Orc Shaman bag, Panpipe bag, Power Bolts ammunition bag, and all three Orc Fortress rewards. Poison Daggers and Parchment Room failed with exact error `ItemReference` during map boot. Investigation traced this to native map policies absent for Item2473/2474; Purple Tome's bookcases2436/2438 have the same absent profile class. Do not substitute Terrain fixtures or relax the boot guard.

Added a bounded source-derived carrier overlay using the existing pinned native importer unchanged: `build_quest_chest_profiles.py`, `quest-chest-item-profiles.json`, and `quest_callback_profiles.admit_map_carriers`. Source XML/appearances/binding digests are verified by the reused importer. Box2473 is portable/nonblocking in the source; the two bookcases and coffin are nonpickupable/solid. Retain those distinct physical facts. Native-only regeneration and routed rerun pending, with30minute repair budget beginning at approximately12:27UTC. This is production admission, not a shared fixture relaxation.

Index observation at12:36UTC: `git status` now reports scratch/reference files as worktree ` A` (intent-to-add), while `git diff --cached --stat` is empty. This session ran no git add/reset/commit or other index mutation; leave those entries alone under the owner's hard rules. Product whitespace validation excludes `.questpc2/**`, whose immutable raw sources contain donor/Wiki whitespace. Do not publish reference assets or the generated scratch World bundles.

QUALIFIED: Orc Fortress Quest, Orc Shaman Quest, Panpipe Quest, Power Bolts Quest, Parchment Room Quest — reward paths only; tests questpc2_rewards_phase_a_leaf_cohort, questpc2_rewards_phase_a_container_cohort, questpc2_rewards_phase_a_book_cohort, questpc2_rewards_phase_a_key_container_cohort (phase-a-pg-final.log; exact per-claim POSTGRES_CASE_COMPLETED durations). Full access/storyline completion is not claimed.

### Phase A routed qualification complete (reward scope only)

`CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib questpc2_rewards_phase_a -- --nocapture --test-threads=1`: PASS5, zero failures,683.69s (`phase-a-pg-final.log`). All13 captured claims have their own POSTGRES_CASE_COMPLETED marker/duration, plus four cohort markers and the Orc Fortress level-gate marker. No route-less early return. The four source-carrier repairs resolved the ItemReference class within the owner's30minute budget; no assertion or production guard was relaxed.

Additional qualified reward paths: Plate Armor Quest and all three Purple Tome Quest claims (`questpc2_rewards_phase_a_leaf_cohort`). Poison Daggers' existing one-dagger canonical/source recipe is PG-qualified (`questpc2_rewards_phase_a_container_cohort`), but target two-dagger reward parity remains unfinished and is explicitly marked in the coverage inventory. The first-five QUALIFIED line above excludes that source conflict.

Fresh structural native/content validation PASS (`phase-a-content-validation-final.log`); source carrier replay PASS (`phase-a-carrier-source-check.log`); carrier profile tests PASS15 (`phase-a-carrier-profile-tests-final.log`); overlay tests PASS9 (`phase-a-overlay-tests-final.log`). Coverage generation/check is being refreshed from the pinned donors only after actual PG proofs.

Final batch checks are running sequentially in `.questpc2/verify-phase-a.sh`; statuses `.questpc2/phase-a-verification.tsv`, transcript `.questpc2/phase-a-verification-run.log`. Includes fmt (auto-mounted fixture too), changed-crate clippy/all targets/all features/-Dwarnings, exact selector units, the seven resumed N PG regressions, World/content units, required server content tests and final World pin-check. No new Rust/native content edit is planned while these run.

Pending owner/coordination question (asked asynchronously during final checks): Phase B's lane-owned `npc_command/questpc2_*.rs` needs a production routing hook. `.questpc2/NOTE-coordination.md` reserves every other part of `npc_command.rs` for questpc and permits at most one module line. Current routes have no generic questpc2 extension hook. Is questpc providing the routing call, or should questpc2 use a named existing hook? No shared NPC file has been edited, and no uncalled public NPC API has been added while this is unresolved. Continue independent Phase A checks/source work while waiting; do not infer an authorization expansion.


## Restart continuation: per-quest reward qualification and target correction

AUTHORING; HEAD unchanged, no frozen SHA or PR. Read all four restart notes and
bound immutable META policy. Fresh absolute-wrapper dispatch check passed
(`restart-current-dispatch-check.log`). No Git mutation or dependency download.

Renamed the auto-mounted fixture to `postgres_tests/quest_reward_claims.rs` and
its test prefix to `quest_reward_claims_`, retaining every existing test and
assertion. Five quest-specific wrappers reuse the shared scenario:
`quest_reward_claims_batch_one_` (Orc Fortress including its level gate, Orc
Shaman, Panpipe, Plate Armor and Purple Tome). Current PG run:
`restart-batch-one-pg.log`; do not infer success until its final result.
Focused Python selector/profile checks passed29 (`restart-python-profiles.log`).

Poison Daggers target correction is in AUTHORING. The explicit two-dagger
regression failed on the old canonical recipe (`poison-daggers-authoring-red.log`).
Saved Wiki quest/spoiler and both donor comments agree on two daggers. New
`reward-claim-authoring/poison_daggers.py` restricts correction to the exact
UID6068, position, backpack and original full recipe. It serializes two separate
quantity1 Item3299 children through the existing container owner, preserving
immutable donor evidence. Four correction-boundary units pass. In-memory
regeneration changes only Poison Daggers and preserves its source_variant;
all105 generated variants retain the strict schema. Canonical content and the
PG fixture still have the old one-dagger recipe until this batch ends: do NOT
claim the target correction qualified yet. Next: finish batch one, fmt/clippy,
write the five QUALIFIED lines, then generate corrected content, change the
exact PG expectation to two child instances and execute its real dispatch case.

The Phase B production hook question remains pending. No npc_command.rs edit
or unused public NPC API. Current shared-file restriction comes from
NOTE-coordination; no permission expansion inferred. Source/reference collection
is stopped per its latest owner note. Existing full-quest limits remain explicit.


Batch-one PG finished:5 tests,0 failures,543.57s;12 completion markers with
positive durations (nine claims, the Orc Fortress gate and two cohorts).
`restart-batch-one-pg.log` contains the full routed run; no route-less return.
Fresh formatting checks pass; clippy is still running in
`restart-batch-one-clippy.log`. Do not begin the next qualification batch before
recording the per-quest lines after that result.

BLOCKED: Power Ring Quest - UID6151's donor gold is contrary to the target ring;
UID6152's bronze-amulet charged-instance path has no qualified native grant.
BLOCKED: Ornamented Shield Quest - both reward sets are needed; Item3083 source
charge mismatch/Item hold remains, and the partner-operated stalagmite access
is unqualified. Do not lower those holds or certify the whole quest.
DEFERRED TO PHASE B: Present Quest and Newhaven Legion Helmet Quest - they are
not chest-only journeys: Seymour exchange, respectively Gustavo/Viola mission
prerequisites, need production NPC wiring. Present also retains real Item holds.
BLOCKED: Power Bolts Quest full completion - the two existing claims do not
cover the sword under the bones, and carried-book READ is absent. Its existing
ammunition/book grants may be qualified separately.

Fresh semantic diff review:21 source UID/action additions in14 map regions,
with every other tile/item field identical to HEAD;11 native Item additions,
no removals, three existing UNKNOWN-to-KNOWN fills. See
`restart-semantic-review.log`. Workspace-wide constructor/caller search retained
in `constructor-search-current.txt`: no new Rust enum or trait method; all
private Chest/Gate literals and NaginataPick construction accounted for, both
resumed public map functions have real production terrain-tool callers.


### Batch one qualified (reward scope only)

Fresh fmt/check and world+dispatch/all-targets/all-features clippy/-Dwarnings
passed (`restart-batch-one-fmt.log`, `restart-batch-one-clippy.log`). All five
quest-specific PG tests passed with executed markers; full access/storyline
limits still apply where stated above.

QUALIFIED: Orc Fortress Quest - quest_reward_claims_batch_one_orc_fortress_all_rewards_and_level_gate
QUALIFIED: Orc Shaman Quest - quest_reward_claims_batch_one_orc_shaman_bag_exact_contents
QUALIFIED: Panpipe Quest - quest_reward_claims_batch_one_panpipe_bag_exact_contents
QUALIFIED: Plate Armor Quest - quest_reward_claims_batch_one_plate_armor_coffin_once_and_replay
QUALIFIED: Purple Tome Quest - quest_reward_claims_batch_one_purple_tome_all_three_bookcases
QUALIFIED: Orc Fortress Quest, Orc Shaman Quest, Panpipe Quest, Plate Armor Quest, Purple Tome Quest — exact reward sets; tests quest_reward_claims_batch_one_ (restart-batch-one-pg.log,5 passed,543.57s).

Next batch: Poison Daggers target correction, Power Bolts ammunition/book and
Parchment Room key bag; at most three quests. Do not claim full Power Bolts or
Parchment Room access/trap completion from the reward tests.


Poison Daggers canonical generation now complete: exactly one reward record
changed; no registry, schema, Item profile, command, capability or migration was
changed by this correction. Source_variant is identical to HEAD. The exact
source capture replays unchanged. The new PG expectation contains two distinct
quantity1 dagger children; atomic audit requires all four instances (backpack,
arrows and both daggers) in one transaction and remains strict through replay.
Current filtered run: `quest_reward_claims_batch_two_`, transcript
`restart-batch-two-pg.log`. Authoring/native/correction units pass30
(`poison-daggers-authoring-green.log`). Real dispatch remains pending; no target
qualification until its actual marker is present.


### Batch two qualified (reward scope only)

`quest_reward_claims_batch_two_`:3 passed,0 failed,156.19s. All four claims
have actual positive-duration PG completion markers; no route-less return.
Final content+world+dispatch/all-targets/all-features clippy/-Dwarnings and fmt
checks passed (`restart-batch-two-clippy.log`, `restart-batch-two-fmt.log`).

QUALIFIED: Poison Daggers Quest - quest_reward_claims_batch_two_poison_daggers_two_instances_once_and_replay
QUALIFIED: Power Bolts Quest - quest_reward_claims_batch_two_power_bolts_book_and_ammunition (existing ammunition/book claims only; sword/READ remain blocked)
QUALIFIED: Parchment Room Quest - quest_reward_claims_batch_two_parchment_room_key_bag (reward only; access/trap remains unqualified)

The Poison Daggers count conflict above is resolved: Wiki target two instances,
30 arrows, immutable donor preserved; marker39427ms. Self-review also reproduced
and fixed a stale unqualified ledger (297->281,16 newly proved N--Q selectors).
Every other281 record is unchanged. All7 ledger guard tests pass; full336-claim
conservation and27/28/281 counts are asserted without relaxing drift checks.

Next batch: replay the seven N scenarios (three quests) using the renamed
mechanic prefix, then finish affected owner/content/server/pin checks. Phase B's
shared NPC production-hook question remains pending; no shared NPC edit.


N restart checkpoint: the Naginata gate, full pick/rope/floor/gate/reward/replay
journey and simple reward replay have each passed with actual PG markers. The
Nightmare Teddy grant also passed. Remaining Noble Armor cases are still running;
no result is inferred for them. Rust code is unchanged since final fmt/clippy.

QUALIFIED: Naginata Quest - quest_reward_claims_naginata_pick_six_shared_hits_and_replay (full mechanic journey, reduced approach corridors)
QUALIFIED: Nightmare Teddy Quest - quest_reward_claims_nightmare_teddy_use_once_and_replay (reward only; island activation/access remains unqualified)
QUALIFIED: Poison Daggers Quest, Power Bolts Quest, Parchment Room Quest, Naginata Quest, Nightmare Teddy Quest — tests quest_reward_claims_batch_two_ and quest_reward_claims_naginata_pick_six_shared_hits_and_replay / quest_reward_claims_nightmare_teddy_use_once_and_replay; reward/access limits above remain binding.


### N regression / final quest checkpoint

`quest_reward_claims_n`:7 passed,0 failed,530.34s, seven positive-duration
POSTGRES_CASE_COMPLETED markers; no route-less return. Naginata's full mechanic
journey, its once/replay grant and level gate, Nightmare Teddy's grant and both
Noble Armor grants/level35 gate all executed on the corrected content tree.

QUALIFIED: Noble Armor Quest - quest_reward_claims_noble_armor_use_once_and_replay, quest_reward_claims_noble_armor_crown_helmet_use_once_and_replay, quest_reward_claims_noble_armor_level_gate_requires_35 (reward/gate only; Skjaar quiz/payment/key path remains Phase B)

Total distinct quest reward sets qualified in this restart:11; exact captured
claims:17. The original27-qualified coverage count includes16 newly proved N--Q
selectors plus eleven earlier qualified selectors (Power Bolts book was already
qualified). No full-quest count is inferred from the reward-set count.

Current final verification: N PG, committed selectors, negative bindings,
World facts and reward-content owner units PASS. Server content/world and final
pin-check still running in `restart-verification-run.log`; TSV is authoritative.
Rust/content feature code is unchanged since final clippy and PG execution.


### Final restart validation

All9 rows of restart-verification.tsv PASS. Server content123 and World
repository5 passed; final pin-check retains55d4d6bf...e93d7. Cross-checked actual
PG markers against the two committed captures:17/17 exact claims,25 timed
markers,15 scenario tests. Total Rust tests across the named commands:163,
zero failures. Final source/metadata checks and product whitespace also pass.
Full diff and constructors reviewed under the owner checklist; no unresolved
finding in the implemented reward scope. Future quest paths remain as above.

### Panpipe access qualified

Final content/world/npc/dispatch all-targets/all-features clippy/-Dwarnings
passed (panpipe-access-clippy.log). The separate Panpipe bag regression also
passed. The final validation TSV continues with Naginata, selectors, owner
units, NPC regression, content/server and pin-check; no further feature edit.

QUALIFIED: Panpipe Quest - quest_reward_claims_panpipe_locked_door_to_reward_and_replay (locked-door-to-exact-bag slice only; carried key fixture, acquisition independently proved by quest_reward_claims_panpipe_key_4055_once_and_replay; long dungeon approach/exit remain unfinished).

This extends one existing qualified quest reward set; no additional distinct
quest or reward claim, and no new five-quest checkpoint, is counted.

### Final Panpipe access validation

All15 serialized verification stages PASS. The repaired final Panpipe pair
passed2/0 in65.52s, with the locked-door marker32777ms. Across the final named
commands:170 Rust test executions,0 failures,0 ignored; five actual positive-
duration PG markers. Source selector/world/reward/NPC units, Panpipe bag and
Naginata/Oriental Trader PG regressions,123 server content and5 World repository
tests pass. Fmt, all-targets/all-features clippy/-Dwarnings, native and raid
checks, product/record whitespace and final pin-check pass. Pin84deca54...78f20.
Exact commands:TEST_FILTERS.md and verify-panpipe-access.sh; fresh counted
evidence:panpipe-access-final-summary.json. Focused Python24 passed. Broader
profile discovery remains108 passed/1 unrelated Item2816 failure, explicitly
Unfinished rather than green. No load-contention or CI-only test exemption.

Full product checklist review and constructor/caller sweep found no unresolved
issue in this implemented slice. All66 product paths are in CHANGED_FILES.md.
No further code edit followed final validation, no Git mutation or publication,
no outside-directory write, credentials, Docker or dependency download. Migrations
1020–1039 remain unused. Task is PARTIAL: existing held Items, atomic NPC item
exchanges, outfit catalogue and shared-engine paths in PHASE-STATUS.md remain
blocked/unfinished; Panpipe's long approach/exit is not qualified. Next: consume
the owning lanes' admitted implementations when available; do not repeat
unchanged heavy cohorts solely for another checkpoint.


### Owner stop-rule continuation — 2026-11-06

AUTHORING; task questpc2, current directory only; no Git mutation. Read all five
restart notes, local bound META policy and required process skills. Initial
absolute-wrapper dispatch check completed with exit0 (restart-owner-check.log).
Existing11 reward sets are partial scope, not automatically full-quest statuses.
Next: finish Pick's fixed atomic NPC exchange by composing the existing craft
transaction, rather than retaining the obsolete missing-exchange blocker.
New real-dispatch regression pick_exchange_atomic_confirmation_refusal_and_replay
is compiling/running red first; pick-exchange-red.log is authoritative. No
production edit yet. All39 titles must receive a final status before exit.

Pick exchange implementation (in progress): first PG red reached the database
and failed `committed Al Dee dialogue is not admitted` after66.12s. The accepted
CRAFT-1 C8 / DUR-03 contract requires the NPC trade owner; the initially considered
craft shortcut was rejected before any production craft edit. Implemented the
closed internal Exchange side on the existing NPC trade writer, audit and replay
readers, a lane-owned command7 hook and Al Dee's source greeting. Migration1020
extends the current consistency guard, retaining prior mechanisms. No client
wire enum, command or capability changed. Dispatch check is now exit0. New PG
run pick-exchange-green-attempt.log is compiling/running. Still AUTHORING.
New public API exchange_small_axe_for_pick has the command7 production caller
and the named dispatch regression; remaining operator/negative checks and clippy
are due after green. Migration1020 is the only number used.

Pick SQL diagnostic: root refusal was CommitRejected. Temporary immediate-
constraint evaluation exposed `missing FROM-clause entry for table "l"`.
Migration1020's broad replacement had also touched Sell All's inventory query.
The repair replaces only the first exact definition clause in the live guard,
leaving Sell All unchanged. Both temporary diagnostic blocks were removed.
Current test: pick-exchange-guard-fix.log (running); the regression now also
checks durable replay through a fresh production command-handler owner without
conversation/cache and verifies that counts stay0 axes/1 pick/1 trade/1 audit.
All six Outfit titles have final canonical-population blockers in QUEST-STATUS.json;
other statuses are not final yet. Do not end after Pick; continue N..Q.

Pick PG is green: pick-exchange-guard-fix.log,1 passed/0 failed, actual
POSTGRES_CASE_COMPLETED/56367ms. No route-less return. It includes the real
command7 confirmation path and a cold production-handler receipt replay.
Fmt applied; cargo fmt --all --check initially requested only two wraps, then
cargo fmt --all applied them. All-targets/all-features clippy/-Dwarnings passed
for content/world/npc/durability/dispatch. The queued wrapper is now running
only pure NPC-trade durability units (pick-exchange-durability-units.log).
Next: NPC units, final Pick+Oriental Trader PG regression, fmt/whitespace and
checklist/caller sweep; then mark Pick QUALIFIED and continue Uzgod/remaining
N..Q. QUEST-STATUS.json currently has13 canonical/data BLOCKED dispositions,
including Present's explicit Item2881/2882 holds; it does not yet cover all39.
Migrations:1020 used,1021..1039 free. Public writer method is
exchange_small_axe_for_pick; production route questpc2_rookgaard::talk.
No Git mutation or external write/upload. HEAD remains unchanged.

### Uzgod implementation checkpoint

Red first obsidian_knife_uzgod_atomic_exchange_cooldown_and_replay reached PG
and failed `committed exchange NPC dialogue is not admitted` (47.27s). Native
steel5889/lance3313/knife5908 all admitted and canonical materializable. Saved
Wiki explicitly permits the Uzgod hand-in without Sweaty Cyclops completion;
use that direct carried-input path, without importing Friends and Traders from
the non-runtime chosen approximation. The optional Dragon Shield3416 route is
held; do not lower its canonical hold. External NPC Uzgod research was absent.

Implemented in npc_trade/obsidian_knife.rs, its audit companion and1021. Root
NPC hook renamed questpc2_item_exchanges.rs and handles both concrete recipes;
there remains exactly one new root mod/call for this continuation. No public
wire enum/command/capability change. Both APIs share the existing NPC transaction.
1021 indexes committed_at and rejects another Uzgod exchange in20h under the
Character lock and at deferred commit; replay reads before the live/cooldown gate.
Steel can be a partial stack; lance retires; outputs and the clock start commit
with all burns and the one audit event. Native capacity helper now accepts the
two declared takes. No old migration or shared fixture changed.

Dispatch check is exit0. Current PG run: obsidian-knife-green-attempt.log. The
shared real-dispatch template now has Pick and Uzgod cases; it checks exact
remaining quantities per input, not just their sum. Uzgod seeds enough for two
recipes, so the fresh-command refusal proves cooldown rather than exhaustion.
New pure audit/plan unit is in npc_trade/tests.rs. Still AUTHORING; no qualification
for Uzgod yet. New Recipe and ExchangeCase fields require a final whole-workspace
constructor sweep; caller/enum sweep and clippy/fmt/PG pair are due after green.
QUEST-STATUS.json has23 dispositions;16 titles are not final. Do not stop.


QUALIFIED: Obsidian Knife Quest - obsidian_knife_uzgod_atomic_exchange_cooldown_and_replay (POSTGRES_CASE_COMPLETED elapsed_ms=61480; item-exchanges-pg-final.log). This adds one new scoped quest qualification; Pick regression also passed63180ms.
