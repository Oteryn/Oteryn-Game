# questpc3 round L4 — AUTHORING checkpoint

Task_id: questpc3-L4. Packet `.questpc3/LANE-L4.json`. Current directory only;
single agent; no Git writes or publication; no frozen SHA or PR. Git reports
no branch name; the operator owns snapshots. Read-only L4 base cfbefef85;
inherited multi-stack work is outside this round's native diff.

Result: 1 QUALIFIED /21 UNFINISHED /0 BLOCKED. Stampede is the qualified entry.
The full 22-entry request is not complete, and no 3–6-entry batch completed.
Every remaining canonical and its exact gap is in `QUEST-STATUS.json.round_L4`
and `L4-REPORT.md`. No source entry was QUALIFIED on the latest bounded reads
of questpc2/questpc4 status. No entry is claimed qualified-elsewhere.

Owned native paths and SHA-256 hashes: `l4-native-tree-final.json` (19 files).
Concrete changed-file list: `L4-REPORT.md`. Changes cover World daily roll,
server raid selection, native raid scheduling and both terminal clock paths,
quest catalogue extension, existing fenced Quest writer, native death queue,
and two mounted dispatch PostgreSQL cases. No new migration, dependency,
protocol code, enum variant or trait method. DailyWorldChanges.stampede is the
one added field; all workspace constructors/consumers were searched and updated.
No qengine2 piece was needed or implemented by this Stampede candidate.

Implemented Stampede:
- Independent 125-permille draw per committed World/reset (accepted W10), stable
  across repeated checks, restart and Channel choice; no minute reroll.
- Existing source-bound raid owns sixteen elephants, announcement, source area,
  placement replay and reset cleanup; exactly one firing per reset.
- A busy legacy raid does not suppress the day event. Stampede does not set the
  legacy busy flag or advance either terminal legacy delay-clock path.
- Source most-damage principal enters the native queue even when ordinary corpse
  admission is unavailable. Existing ordinary Item/loot/XP admission stays intact;
  the fallback adds only quest credit, not fabricated ordinary rewards.
- Existing fenced Quest writer increments once per native death, caps at five,
  and grants Trail of the Ape God atomically with the fifth increment.
- Duplicate captures, repeated drain and retained occurrence replay do not count
  twice. Wrong USE cause and stale captured lease write no quest progress.

Final evidence: `L4-CHECKS.json` contains exact commands, logs and exit codes.
All six changed packages passed all-target Clippy -D warnings; durability and
dispatch passed again after the narrow legacy-interference repair. Eight pure
unit cases passed. Both final PG cases passed in 36.20s, with completion markers:
native scheduler/spawn 34988ms; personal counter/achievement 36202ms
(`l4-stampede-independent-green.log`). Cargo fmt check, explicit included/new
helper rustfmt check, git diff whitespace check and final native hash readback
passed. No cargo command remains running.

RED evidence:
- `l4-stampede-red.log`: native deaths were dropped before quest-only admission,
  1 failed /29.89s. The initial compile attempt wrongly cloned NativeEntryContentPin;
  corrected. Two redirects shared this log, so late compile text/NUL holes remain
  alongside the real named PG failure. Raw evidence is preserved.
- First candidate committed the first count, then its two-actor fixture ran out
  of slots. Fixture now has one player plus six death slots. No production limit
  or assertion was changed. A missed fixture argument was corrected afterward.
- `l4-stampede-independent-red.log`: current World/epoch admission was proven,
  then the busy legacy raid suppressed Stampede: 1 failed /32.00s. Repaired all
  matching busy/clock predicates; strengthened scheduler PG is green afterward.

Whole L4 native diff reviewed against the owner CODEX-CHECKLIST, including
fences, immutable occurrence replay, atomic reward, bounded reads, caller wiring,
all new field sites and both clock-update paths. No known unresolved finding in
the implemented Stampede change. Independent review/publication were not performed.

Remaining work is the 21 UNFINISHED entries, not an external terminal blocker.
Keep the round in AUTHORING and preserve the exact per-entry gaps. The complete
round is not ready to be reported DONE or archived.

## Previous checkpoint (retained)

# questpc3 continuation checkpoint — 2026-11-06

Task questpc3, AUTHORING. HEAD 5cfc696a3 (previous run committed); no frozen
candidate or PR. Current owner forbids all Git writes and publication. Single
agent; current worktree only; absolute cargo wrapper, focused PG filters, no
external writes or dependency downloads. Migrations 1040–1059 remain the lane
allocation; this continuation adds no migration or protocol/capability ID.

The current owner instruction supersedes the historical terminal-blocker claim.
Available inventory: 56 titles, 18 previously QUALIFIED, 37 IN_PROGRESS, one
BLOCKED owner exclusion (Sanctuary of the Lizard God Quest: Dawnport).
MECHANISM-BATCHES.json groups the 38 formerly blocked titles. Game-owned gaps
remain unfinished implementation, never terminal BLOCKED. Ten world-change
entries have candidate owner WORLDCHG, but no in-flight allocation record was
found here, so external BLOCKED is not asserted. No ITEMHOLD allocation inferred.

Pending owner questions (asked together asynchronously): where is the requested
94-blocked inventory; which task records confirm WORLDCHG/ITEMHOLD ownership?
OWNER-DECISIONS.md was absent and now records only this session's direct rules.
The other 27 Game quests can proceed independently of those answers.

Batch 1: shared atomic multi-stack carried consumption, not a quest qualification.
Production change is quest_world_callback.rs: read bounded positive stacks in
stable Item order, resolve the exact full debit before any write, then use the
existing atomic per-stack consumption proofs, Item guards and Quest receipt.
No new public API, variant, field, trait method, schema CHECK or privilege.
The test uses the existing source-pinned three-tear callback (actions_tears.lua:
1–16) as mechanism evidence; it does not qualify the Gravedigger or an R–S quest.

RED: multi-stack-handin-red.log, genuine PG execution 33.62s, valid split hand-in
Rejected instead of Committed. Earlier fixture attempts exposed pickup auto-
stacking and a retained spell-lane permit; corrected via the real split owner
and explicit permit release. These attempts are not qualification evidence.
GREEN: multi-stack-handin-final-pg.log, POSTGRES_CASE_COMPLETED duration_ms=33893.
Proves atomic shortfall, three units across two stacks, one surviving unit,
transport replay and durable receipt replay using the original world context.
Regression: mechanism-altar-regression.log, 1/1, 37.15s, completion marker;
Rotten Blood's possession-only phase and exact item burns/replay are preserved.
Pure callback tests passed; see CONTINUATION-CHECKS.json for exact commands.
Both changed crates passed check and all-target clippy -D warnings. Workspace
fmt and explicit included-file rustfmt checks passed. All six commands in
CONTINUATION-CHECKS.json exited 0; no cargo command remains running.

Owned code: durability/quest_world_callback.rs; dispatch postgres_tests/
quest_world_callbacks.rs and quest_world_callbacks/{scenario,multi_stack_handin}.rs.
All 18 previous quest qualifications remain evidence from the prior run; no new
quest is claimed complete. No 5–10-quest qualification batch has completed.
Next: compose atomic NPC Items plus canonical QuestState, beginning with the
npc_atomic_items_and_progress group. Do not treat that shared writer gap as a
reason to defer the Game quests. SHARED-FIXES.md records fixture/integration facts.
