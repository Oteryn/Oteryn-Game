# GREEN main #3020 integration; publishing this merge

All39focused mechanics PASS390.313s, actualPG180006 observed+completed receipts.
Elvenbane source treasure+earned exit+actual journal list/line PASS150.647s;
Annihilator128.124s, Desert128.457s, native goals99.327s, Monk109.471s,
XP/objective atomic84.188s, stale-capture82.298s, killobjective7.563s.
NPC source conversation test PASS32.742s (all original assertions).
Final workspace --all-targets PASS after NPC dedupe; changed31crate clippy
--all-targets -Dwarnings PASS. Fmt/dup/diff checks PASS. Canonical regen/pin
refresh completed via repo tooling; final staged pin-check PASS, digest
 d65e38a4c120cfb927b5ee9915f7615ae1fd559fb2518f2f2312ccdf2362772e.
Proof: BATCH-main-3020-integration-2026-11-07.json. Main0999–1008 byteexact,
no1200–1205 copies. Own1240–1244. Commit owneridentity+requiredtrailers,
fetchbranch, merge movement, fast-forwardpush immediately. No PR/main/rebase.
Next: Elvenbane canonical authoring/native admission metadata reconciliation;
actual production runtime+journal PG alreadygreen. Status still2QUALIFIED until
that reconciliation; actual3inprogress Elvenbane/RottenBlood/SoulWar,24notstarted,
6excludedothers. Earlier27inprogress count was overbroad; corrected ownerreply.
Only5active-owned displaynames sent to coordinator for dedup. Await notification
of not-started ownership transfers. No bigenginework: two ENGINE-REQUESTs already
sent (arena sequencing; atomic creature capture). wpkg collectsbranch, no ownPR.

# COLLECTOR19:17: wpkg collects green lane batches into ONE PR

Keep pushing only this lane branch; no PR. wpkg pulls5–8quest batches. Fix any
collector failure list together in ownbranch and repush. Merge origin/main after
coordinator says collector PR merged. Current mainmergeverification continues.

# OWNER OVERRIDE19:16: shared engines now belong to qengine2

New owner rule supersedes prior implement-all-shared-features instruction.
Do not build big generic engines in this lane. Request missing generic features
with ENGINE-REQUEST, then move to another owned quest. Only local helpers under
~50lines in NEWfiles allowed going forward. Two requests already sent:
- participant-bound arena fight sequencing/tier settlement: Barbarian Arena Quest;
  crates/gameplay/encounters.
- atomic tool consumption/live-creature capture/captured-item custody: Goblin
  Merchant Quest; crates/gameplay/dispatch.
Keep working Elvenbane completion using existing mechanisms. No expansion beyond
packet. Push ready tested work now, then batches5–8 finishedquests. Coordinator
opens PR/heavyCI. Merge main promptly on coordinator merge notice. Generate matrix,
pins/derived files once immediately before batchpush; do not regen perquest.
Current merge's canonical regen alreadycompleted before this rule; don't repeat.
Nothing green enough for push yet: focus39 rerun compiling native dispatch,
log/tmp/questpc2-3020-focused-green-nextest.log, session45566. Clippy52643 final and
workspace7597 afterNPCdedupe queued. ExistingNPC conversation28062 queued.
Last actualpush26e5a8f0b. After mandatoryGREEN commitmainmergeandpushimmediately.

# LIVE19:05: all focused failures collected and fixed; final rerun queued

Focused catalogue-fix run finished39:37PASS2FAIL, runtime385.650s. Both failures
are diagonal pacing receipt assertion: process rollout defaults off. Added explicit
cfg(test) composed-owner receipt override; route fixtures set Some(true), production
uses unchanged process switch. No assertion weakened. Main10migrations unchanged.
RED log /tmp/questpc2-3020-focused-nextest.log. New fullsame39case rerun queued
session45566 /tmp/questpc2-3020-focused-green-nextest.log, report
/tmp/questpc2-3020-focused-green-run.json. Final all-target check24061 compiling;
clippy retry4 session36204 queued /tmp/questpc2-3020-clippy-round4.log after
removing one remaining orphandoc in worldquestcallbacks. Inspect ALLfailures.
Canonical regeneration already COMPLETED. No full-suite run. Root5GBfree after
retiring obsolete18:38 native incremental snapshot; keep18:56 warm snapshot.
Push currentmainmerge immediately after finalmandatoryGREEN, thencontinue27.

# LIVE: main #3020 merge final verification

Last successful push26e5a8f0b; current uncommitted merge1ae39c7fb.
Latest owner speed rule applies: own mechanics/actualPG only; workspace
all-target check, changed-crate clippy, fmt, dup, canonical regeneration.
Canonical regeneration COMPLETED, including stale NPC shard cleanup and pins:
/tmp/questpc2-3020-final-regeneration.log. All main0999–1008 SQL byte-identical;
no1200–1205 copies. Own1240–1244 retained. No full nextest running.
Focused first run39cases:14PASS25FAIL, all same duplicate catalogue reward key.
Removed second reward_callbacks::extend in quest loader, preserving main once
and owned native/kill extensions. RED archived /tmp/red-catalogue-dup-questpc2-
3020-focused-nextest.log. Focused rerun /tmp/questpc2-3020-focused-nextest.log,
report /tmp/questpc2-3020-focused-run.json; must finish all39 and inspect failures.
Disk exhaustion on rebuild recovered by retiring obsolete lane caches; jobs1.
Clippy fixes removed orphan doc/allow and blank doc separator only. Retry3 log
/tmp/questpc2-3020-clippy-round3.log. Final workspace check queued log
/tmp/questpc2-3020-workspace-final.log. Previous all-target round3 GREEN171s.
Do not repeat finished generation or launch full reverse-dependency nextest.
After mandatory checks GREEN, record actualPG proof, commit with owner identity
and trailers, sourceenv, fetch branch, merge movement, fast-forward push at once.
Then continue remaining27 owned entries. No test weakening, no PR/main push.

# LIVE18:37: focused mechanic run compiling; workspace ALL TARGETS GREEN

Mainmerge1ae remains uncommitted; lastpush26e5a8f0b. Latest speed rule ONLY own
mechanic/PG, workspace --all-targets, changed-crate clippy, fmt, dup and regen.
Workspace all-target check GREEN round3,171s after two merge fixture issues:
QuestWorldContext.party requires Vec::new(); byte-identical duplicated NPCtest
was deduplicated with every assertion retained. Logs
/tmp/questpc2-3020-workspace-all-targets-round3.log. Normal31check passed31.37s.
Own12author scripts:11PASS; container-count57->58 exact merge-union drift fixed,
13container tests nowPASS4.091s. OriginalRED preserved. Fmt+dup+diffchecks PASS.
No full nextest running. Focused own run session60417, log
/tmp/questpc2-3020-focused-nextest.log; script/tmp/questpc2-run-focused-3020.py
selects8own PGcases and focused quest/world/corpse units,5packages,4testthreads,
1buildjob. Success output retained for PG markers/durations. Native coldlibtest
compile currently~4m36s/9.7GBRSS,no newOOM (baseline30events13kills). Clippy31
changed crates queued session58514 /tmp/questpc2-3020-clippy.log. Must be GREEN.
Generator mainstage finished, fulltree validation passed. Stale oldNPC37shard
beside main56shard caused NPC_PLACEMENT_STRAY_FILE; ran repo convert_placements
(default cleans stale outputs), --check and materializedtreevalidatorPASS. Added
converter to normal regen pipeline beforematerialization. Resume remaining checks
and finalpin via/tmp/questpc2-3020-finish-regeneration.py, active session94929,
log/tmp/questpc2-3020-final-regeneration.log; currently waiting cargo lock for
content_world_project_repository, then stage and pin-refresh. Do not repeat
10+minute completed tree/prey/materialization stages. Oldgenerator session52259
endedfailure; sourceproof/log/tmp/questpc2-3020-regeneration-round2.log retained.
Focused PG started only after all content generation complete, immutable sources.
Watch rootdisk:~5GBfree, tmp2GBused. Helperretire-completed-incremental excludes
activecompiler and preserves binaries ifcapacityneeded. No binaries committed.
After allGREEN update status/resume with actual PGproof, commit asowner+trailers,
fetchownbranch+merge movement, push fast-forward immediately. No PR/main/rebase.
Continue remaining27owned entries afterpush; no finalsummary while scope remains.

# CURRENT OWNER OVERRIDE: speed-first; NO full reverse-dependency nextest

Latest owner replaces prior full46 test rule. Before push ONLY own mechanic tests
RED/GREEN incl actual PG marker+duration, cargo check --workspace --all-targets,
clippy changed crates, fmt, duplicate migrations, repo pin/generated regeneration.
Finish current main1ae merge and push immediately after those checks. CI full
results arrive from owner; fix all together in one push. No fullrun active.
Normal31crate cargo check passed31.37s; warning was duplicated reward callback
preparation, old duplicate block now removed retaining main's prepare/checks.
Canonical regen round2 active session52259, log
/tmp/questpc2-3020-regeneration-round2.log, materializer runtimePID450846. Do NOT
repeat finished materialization stages if later generator check fails; resume repo
stages appropriately. Need full workspace all-target check after regeneration,
focused ownPG (Elvenbane full route+journal and relevant earned/source regressions),
clippy31changed crates, fmt, dupcheck, commit required trailers, fetch+merge+push.
Last successful push26e5a8f0b. Owner ETA said30-40min around17:48.

# LIVE: merge main1ae39c7fb (#3020) in progress; do not push until prevention rule green

Owner corrected registry: Main owns0999-1008 (QBASE folded into1006-1008),
NO1200-1205 on main. Branch has no1200 copies. Own1240-1244 unchanged;
1245-1249 free. Duplicate migration check passed after merge. Preserve main SQL
bytes. Latest actual successful push26e5a8f0b (compiling WIP) before newmain.
Current MERGE_HEAD1ae39c7fb9bef09c3cd50d16410374f4134b1676, uncommitted.
Author/committer blakinio75369544+blakinio@users.noreply.github.com plus required
Co-Authored-By and Claude-Session trailers from owner. Sourceenv before fetch/push.
Fetch ownbranch before push, merge any movement, never force/rebase/main/PR.
Latest owner explicitly requests ONEpush after full prevention validation; continue
owned scope afterward. No agents. ReasoningMEDIUM. No credentials/test weakening.

## Immediate active work

Canonical regen /tmp/questpc2-3020-regeneration.log stopped on duplicate entity
method inserted by additive merge driver. Those and five other duplicated shared
methods were resolved with main authoritative APIs, preserving distinct lane
extensions. Initial31crate check exposed duplicatedeffects module; fixed. Round2
exposed duplicatequest module/re-export and duplicate reward_admission parameter;
fixed, and all8 quest-inner call argument lists now exactly main. Round3 active
session60500 /tmp/questpc2-3020-check-round3.log. Poll it, collect ALL errors,
fix grouped integration issues, then rerun canonical regeneration with JOBS1:
python3 tools/content-migration/regenerate_content.py --resolve
(use /workspace/oteryn-env/env.sh). Native cache pre3020 is obsolete; retired.
Generator previous session55898 ended failure, not active. Rust fmt parsed after
shared-copy cleanup. Need final fmt/check/clippy -D warnings on31changed crates.
No hand edits .github/workflows/tools/repository; incoming main changes preserved.

Authoritative shared files added on both sides listed
/tmp/questpc2-3020-shared-added-paths.txt. They cannot be insert-concatenated!
Main shared reward code/captures/schema/tests used, including44 additional Item
proofs. Main qengine_access kept with tiny hook into newquestpc2_access.py for
owned rewarddoor/Desert rite/Elvenbane callbacks and pinned verifier functions.
Three lane-only source/rejection tests relocated intact to
test_questpc2_effect_authoring.py; main's tests retained. Main worldeffects merged
against original sharedbe6db5edd retaining quest_completion, party_transfer,
reward_door. No duplicate implementations. Helper scripts in/tmp document edits.
Main NPCscope53 includes all36old NPCs; selected cells unchanged. Main access
binder retained incl syntheticcapture handling, with own sourceUID30030 added.
Main access tests retained. Two conflicting B3regions proven identical geometry,
only lane source-backed UIDs5001,6085-6088 and doorAID1100 differ; kept HEAD
regions/index/summary with those attributes. All17 changedindexrows correspond
lane chest/access overlays; repo chest check passed35attributes, access checkPASS.
Never change geometry or fake health/kill credit to make route pass.

## Known failures fixed before next full run (validation still required)

Old mainbbb3 full run compiled all46 targets7m17s, then owner requested immediate
main1ae merge. Interrupted old run (not full success):1189/10722tests run,
1137pass,51functionalFAIL,1SIGINTcase,44existingignored. Logs preserved under
/tmp/questpc2-before-3020-questpc2-main-dependent-nextest-*.
29authoring scripts ran23pass6fail, original outputs preserved
/tmp/questpc2-authoring-before-fixes/. Do not overwrite proof or claim qualification.
Known roots:
- Default client caps ended383,384,371 -> strict invalid capabilityset (33direct
  failures,12more admission failures likelysame). Moved371 before381; no codec
  validation relaxed.
- Client placement index pin stale (5related failures). Extended repo generator
  repin_rust_inventory() to refresh only clientassets World pin from real index.
  Must regenerate through that tool, never hand-rehash tracked pins.
- Corpse rejection test rightly refused31: restored creature_reward::row ceiling
  to1..=30 per existing Decision20 tests, not u16::MAX. Tests untouched.
- Authoring completion effect unknown: quest_batch4 validates bounded distinct
  claim keys and advancing nonreversible phase with no incompatible reward/operation.
  Main effect tests4PASS8.026s and owned3PASS7.751s afterextension.
- Ledger only disposition_totals drift; verified generated claims exactly equal
  every existing record; ran existing repo generator. Validator now283PASS.
- Merge union exact countpins updated from actual RED outputs: chest children139
  (was133), native container derives71(was70), rejected variants34(was36).
  All rejection/materialization assertions retained; need rerun full authoring.
- Main qengine_rewards.py regenerated its authoritative updated cosmetics/grants;
  do not recreate old competing cosmetic catalogue.

## Elvenbane finishing slice (not yet QUALIFIED)

Whole route PG133.72s beforemain proved Native6 realwire kills, own counters1/1/1/2,
all4source claims+filled bags/root/child same-tx custody/replay, zero/partial3claim
completion refusal, canonicalexit32597,31642,z7, ownphase4. Exact final RED was
journal unavailable (fixture had quest_log_content=None). Log
/tmp/questpc2-completion-safe-landing-green-pg.log. Keep all those assertions.
Now authored kill_objectives::journal(), loaded alongside Monk journal; fixture
loads real QuestLogLines and selects capability16 for ElvenbaneComplete. Fullroute
now additionally sends actual command22 list+line over productionconnection and
asserts completed Elvenbane listentry, completed line, all5missions done and return
text. Code added but not yet compiled/tested after3020merge.
Test: gameplay_transport::monk_save::postgres_tests::elvenbane_treasure::
elvenbane_source_treasure_exit_and_journal_via_wire (actual path may wrapped).
Need PG RED->GREEN proof with marker+duration, no 0.00 skip. No whole qualification
until journal fullsource route and fulldependent set are green; canonical recipe
metadata still waiting may need proper authoring update, not paper qualification.

All2qualified/27unfinished/6otherlane skips remain unchanged. Follow LANE-L3,
QSPLIT-PLAN and ownership, not other lanes/worldchange scopes. Next after full
Elvenbane proof: continue remaining owned entries, do not end with summary.
Important sourcefacts/contracts and previous finished mechanics remain in
CONTRACT-NOTES.md and BATCH*.json, old commits retained, do not redo them.
Source chest corpus9f5a72... chest.lua2551-2580 sha0ca1a7ad... exactly4claims:
bp1/bp2(container2853 with3/2children), shield3425,morningstar3282;9physicalitems
in4roots+5children, not7roots. Stair re-ascent requires East89,42,z3 after ladder;
flatBFS must not walk on down stair88,43. Source sixactor/player heights/XP untouched.
Generic completion guards durable exact once-claim identities in same transaction;
callback binding serializes fullcallback, preserves fences/replay. NativeXP and own
kill objectives stay one revision/physical transaction; stale deaths read-only no
regrant, callbacks immediately update copy+capture caches. All proven tests retained.

## Prevention set / resources

/tmp/questpc2-3020-dependent-test-set.json =31changed46normalreverse dependents,
derived by exact cargo tree -i ... -e normal --workspace --prefix none for each.
No new dependents beyond previous46. Must run ALL with nextest+PG --no-fail-fast,
collect all failures, groupedfixes, rerunFULLset, then ONEpush. Need prepare new
runner/log filenames3020; old /tmp/questpc2-run-dependent-tests.py reads oldspec.
Author rerun helper reads original29script list withoutMERGE_HEAD dependency;
add newtest_questpc2_effect_authoring.py to the next authoring run.
Use buildjobs1: initial2job builds OOMed; no blocker. BaselineOOMevents30/kills13.
Oldpre3020 156test executables13.78GB and warmNative2.8GB cache retired afteroldrun
stopped, audit/tmp/questpc2-3020-capacity-reset.json. Root15GBfree/tmp2GBused at17:32.
Never restore old cache archives, allpreMainABIobsolete. No active oldnextest.
Helper/tmp/questpc2-retire-completed-incremental.py retires only inactive completed
incremental caches (preserves binaries/activecompiler). Retain current binaries
until fullrun ends. Sourceenv exposes PG180006, do not print/readcredentials.
Oldrun DBbaseline/tmp/questpc2-full-run-initial-databases.txt captured before PG;
only retire THISlane's idle old databases when needed, never active/otherlane DBs.
