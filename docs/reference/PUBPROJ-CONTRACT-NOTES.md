# PUBPROJ contract notes

Owner authorization: 2026-11-07, “1 tak, 2 zgoda”. Base: `d34807f5b`.
Branch: `claude/happy-davinci-ijz3wy-pubproj`; migration allocation: 1350–1359;
new numeric identifiers use next free on the base plus 50. Allocations are recorded below.
Owner extended the task on 2026-11-07 with G-12 world_daily and both G-8
statistics families; migrations 1360–1369 are additionally reserved if needed.

## Part A

The parser already has `PublicProjectionsConfig`, default-disabled gating,
purpose-separated credential/fence validation, and a production caller in
`node/serve/supervision.rs`. Install the disabled table in the Synology template
and node boot fixture. No gameplay rule changes or donor rules in this step.

The disabled table exposed eager certificate reads by Bazaar and T94. Those
reads now require public_projections.enabled, while configuration path-identity
validation remains active. The existing publisher itself was already gated.
The new missing/mistyped-field negative test passed (first build 9m14s;
execution 0.01s). Server check passed (4m15s); all-target server clippy passed.
Full changed-crate tests, pin and architecture checks remain pending in this
checkpoint. No claim of completed Part A qualification is made yet.

## Part B reconciliation

Read Platform main's
`docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md`
(revision 2). Its V2 paths replace the three candidate V1 families; the existing
V1 watermark enum stays closed. The owner additionally requires stable killer
identity: player CharacterId and native creature content key, historical names
for display only. This amendment must preserve §7 privacy and requires durable
cause capture before a creature identity can be emitted. Producers, acceptance
text, fixtures and execution evidence are still pending in this checkpoint.

### V2 wire checkpoint

The owner acceptance is recorded in the V1 status line and the separate V2
profile. V2 has typed player/creature killer identities, stable guild member
ids, and exact house status variants. The fixture generator is
`tools/contracts/generate_public_projection_fixtures.py`; it emits nine byte
fixtures. Five focused wire tests passed (build 14.58s, tests <0.01s).
V2 acknowledgements are version-bound; the closed V1 watermark stays unchanged.

This checkpoint compiles, but B is not complete: the durable store methods,
source capture migrations, PostgreSQL cases and resource-limit registration
are pending. The production store defaults to unavailable for the V2 methods,
so it cannot publish fabricated facts before those source paths are installed.
Recent historical receipts without immutable classification must withhold
freshness, rather than become guessed PvE or a known empty death history.
C and D implementation has not started.

### Durable capture checkpoint

Reserved migrations:1350 (V2 heads/outbox/source seam),1351 (Guild/house),1352
(death capture). Resource ordinals10069–10076: base maximum10018, next10019
plus lane offset50. No command ids, capability bits or mechanic ordinals added.

The reconciled durable read/ACK/facts adapter and native death capture compile
with the server (4m08s). Native admission-source tests:151 passed,0 failed,
0 ignored (0.14s); native all-target clippy passed (16s). Source migration PG
execution, house catalogue boot integration, delayed-condition cause provenance
and full changed-crate validation remain pending. This is a compiling WIP.

A's full server tests exposed a pre-existing stale reward-claim golden: actual
311 versus expected308. ITEMHOLD-Q commit ca5a91b59 admitted exactly
ornamentedshield, powerring2 and triangletower; see
`docs/architecture/reviews/ITEMHOLD-Q.md:651–659`. The correction preserves the
original308 cohort and additionally requires each of those three exact ids.
No content or binder rule changed. Its focused rerun remains pending.

Pin-refresh, pin-check and architecture-check completed successfully. Physical
node-boot qualification reached the exact Platform checkout but failed on the
host prerequisites: Docker buildx tried a read-only home path, and cleanup
reported `sudo: command not found`. No node-boot PASS is claimed.

The new ADMINAN obligation reserves creature keys player and unknown. Future
admin player-death analytics uses World × primary killer key × UTC day, with
all player killers collapsed to player and unattributed causes to unknown;
private G-4 CharacterIds never enter that admin feed. Known invalid source
metadata must remain quarantined, not be relabelled unknown. C/D source and
retention implementation is in progress; no ADMINAN producer is implemented.

### House boot checkpoint and executed PG red point

Active checked-map catalogue capture is wired before listener readiness; the
server compiles with the native Guild profile join and catalogue source
(2m57s). Focused PostgreSQL source cases executed with the configured route
(8.54s target run), rather than route-less success, and exposed the resync
coalescing defect: unchanged acknowledged Guild/house snapshots do not get
re-enqueued. The exact red case is
`pubproj_house_catalogue_source_revision_withdrawal_pending_freshness_and_resync`,
assertion at `pubproj_guild_house.rs:112`: `next_public_v2(...).is_some()`.
The fix needs a forced resync enqueue separate from normal content coalescing.
Neither PG case is claimed green in this checkpoint.

Effective reset calendar is Europe/Berlin06:00 after migration0562; the original
0148 Berlin10:00 declaration was superseded. C follows the persisted calendar
and its revision, including DST, rather than a fixed86400-second day.

### Resync repair: executed green

A permanent private force-touch helper now advances revision/outbox for operator
resync even when normal source capture coalesces unchanged content. Guild and
house source derivation now serializes before reading its snapshot cut, including
catalogue-sync versus owner/name changes. Privacy-dependent death subjects are
visited in stable UUID order.

The two previously red Guild/house PG cases now pass:2 passed,0 failed,0 ignored,
10.81s execution (2m05s build;137s command). Each printed
`PUBPROJ_GUILD_HOUSE_PG_EXECUTED` with elapsed_ms=10797. Old ACK versus newer source
revision, explicit catalogue withdrawal, unknown-source freshness and operator
resync were exercised. Other B validation and C/D implementation remain pending.

### C and online-source integration checkpoint

C and character_online_sessions are integrated into the V2 typed codec, durable
reader, operator family allowlist and purpose-separated V2 ACK path. Their
publication flags default false beneath the disabled master flag. Online source
capture/retention remains active independently; a bounded32-row/1-second node
maintenance loop purges source payloads even while publication is disabled.

Server check passed (2m57s); native-source suite passed159 tests,0 failures,
0 ignored (0.22s). C/online PG cases and complete changed-crate qualification
remain pending. Resources10077–10082 cover C;10083–10086 cover online intervals.
Migrations1353 and1354 are reserved/implemented;1355 and10087–10092 remain for
physical-creature kill statistics. No additional command/capability ids exist.

B now carries frozen native cause through canonical bounded condition lifetime,
including source disappearance, with logout/restore scrubbing. Private primary
killer_key supports future admin aggregation without exposing CharacterIds in
that feed; player and unknown are reserved creature-key exclusions. This source
change compiles, but its focused PG/source tests still require execution.

### B historical-window recovery checkpoint

An indexed32-subject maintenance pass rechecks unavailable death snapshots only
after the actual source window changes; it cannot invent old PvE classification.
Known/hidden subjects clear their pending deadline. Resource10093 is allocated.
The production house metadata parser now has a995-record source-shape test.
The combined server check passed (2m54s); the new expiry PG proof remains pending.

### C compiling checkpoint

World daily source capture, V2 wire, default-off publication flag and operator
family are implemented under migration1353. Reset intervals derive from the
persisted Europe/Berlin06:00 calendar, including23h/25h DST days; historical
coverage and committed fixed-slot classification remain explicit. Resources
10077–10082 are allocated. Server compilation and native golden tests pass;
C PostgreSQL execution is pending after fixing test-only facade type imports.
No activation or completed C qualification is claimed.

### Owner routing decision 2026-11-07

G-12 World daily is removed from PUBPROJ in 7a8a04358 and preserved in the separate owner-managed lane. Prior C notes are superseded. Migration 1353 and resource ids 10077–10082 remain free on this branch. D migrations 1354/1355 retain their numbers.

### Statistics durability and lifecycle evidence

Migration 1354 captures authoritative committed ONLINE transitions as durable,
owner-only intervals with statistics_id. Deferred triggers observe final committed
state. Unknown starts and unsafe clocks withhold coverage; crash/lease closure
uses authoritative boundaries. Payload retention is 30 days; bounded maintenance
runs independently of publication. Source metadata fences survive payload erasure.

Migration 1355 records physical creature death occurrences, deduplicated across
Channels before transactional aggregate/revision/outbox updates. Combat enqueues
in memory; autonomous batches contain at most 256 occurrences and the buffer holds
4096. Flush cadence is 1000 ms. Durable aggregates retain World, UTC day, creature
content key and disjoint boss classification for later admin analytics; the public
wire exposes only totals. Retention is 400 days after UTC day end.

Shutdown drains listeners and source owners before closing the kill source.
Instance retirement joins its local owners and flushes before unregistering native
state. Final replay accepts only immutable already-recorded facts under current
authority and preserves the original endpoint; new facts cannot reopen a clean
source. Retained actor pruning is bounded to 32 actors per background pass.

Player-death admin keys are a native creature content key, literal player for any
player killer, or unknown for no attributable killer. The native creature loader
rejects player and unknown, with a guard test for both. Public player-killer identity
remains CharacterId under the existing PvP/privacy rules. No admin producer is added.

Resource ordinals 10083–10086 cover online request bytes, retention, maintenance
batch and cadence. Ordinals 10087–10092 cover kill request bytes, retention, batch,
pending buffer, cadence and creature key bytes. No command/capability IDs are added.

Donor evidence: Canary 99902524e052f37574194466c2949c576e4ab269,
src/creatures/creature.cpp:579–600 provides one physical onDeath boundary;
657–669 separately iterates credited killers. Credited-character multiplicity
therefore cannot count physical deaths. Canary src/io/functions/iologindata_save_player.cpp:
320,359–360 and Crystal ff7ede src/io/functions/iologindata_save_player.cpp:
200–206,364–378 provide persisted login/logout evidence. Authoritative intervals,
UTC buckets, retention, source gaps and reserved keys follow the owner-approved
Platform statistics rev 2 contract rather than inferred donor behavior.

### Focused qualification repair

Owner replaced full reverse-dependency nextest with own mechanic cases plus
workspace all-target compilation and changed-crate clippy. The interrupted run
exposed a null-family resync validation regression and shared advisory-lock
exhaustion in the existing 10,000-Character resync fixture. Each wrapper now
preserves SQLSTATE 22004 for null arguments. Online capture serializes exact
Character identities with private tuple locks, retaining the same lock order
without one shared lock-manager slot per Character. This changes serialization
storage, not the owner-approved interval rule or donor login/logout semantics.

The interrupted full run also reported Bazaar Transient results and an unrelated
item-mint Ambiguous result under concurrent test/build load. These were collected
without changing tests or deadlines; full-suite CI remains the owner's gate.

The same tuple-lock primitive covers G-4 and World/day kill subjects, preserving
all previous lock namespaces and canonical subject order. This also prevents a
large all-family resync from exhausting the shared lock table after online
capture has completed. Fence rows contain only canonical subject identity, no
interval payload, ownership, creature dimension or counters; retention still
erases the retained projection/source payload on its existing deadlines.

Focused unit execution found two additional issues: the summonable creature
fixture lacked its required mana-cost declaration, and an empty final-source
retry regressed a proven same-millisecond cut after acknowledgement. The fixture
now satisfies native Content admission; quiesced retries retain the last proven
cut while the existing unsafe-clock checks remain unchanged. No assertion or
source deadline is removed or relaxed.

The legacy resync negative probe used Guild as an unknown family. Guild is now
owner-accepted, so the probe uses an explicitly unsupported family instead;
the SQLSTATE 22023 assertion remains intact. Dedicated Guild PG coverage still
requires successful resync and its revision/outbox behavior. The large resync
case passed with stock PostgreSQL lock settings after tuple serialization.

### Focused green execution evidence

The retained PUBPROJ unit selection passed 78/78 in 19.279 seconds, including
wire goldens, stable player/creature/PvP-hidden identities, reserved-key admission,
condition provenance, joined-owner final cuts and config negatives. Its two
Character-death PostgreSQL cases emitted execution markers at 9382 ms and
7264 ms. The dedicated 11-case PostgreSQL selection passed 11/11 in 94.583
seconds, including the 10,000-Character resync regression, null/unknown-family
rejections, G-4 revision/outbox/ACK/resync, online retention/ownership/1000-interval
resync, and physical-death deduplication/final replay. No assertions or deadlines
were weakened and no sleeps were introduced for timing evidence.

Final allocation is migrations 1350–1352 and 1354–1356, resources
10069–10076 and 10083–10093. Migration 1353 and resources 10077–10082 stay free
in PUBPROJ. No command IDs or capability bits were allocated.

Final workspace all-target check passed in 9m43s; changed-crate all-target
clippy with warnings denied passed in 8m01s. Formatting, duplicate migrations
and architecture checks passed. Repository fixture generators ran; World
pin-refresh and pin-check passed after G-12 removal, with their inputs unchanged
by the subsequent SQL/runtime/test repairs. Full-suite results are delegated
to CI under the owner's replacement speed rule.

### PR 3031 main integration

Merged main 1138720b1 after owner request. Main has no migration in 1350–1355;
PUBPROJ retains 1350–1352 and 1354–1355 with 1353 free. QENGINE2/QBASE use
0999–1008; no 1200–1205 copies are present. Duplicate migration checking passed.

### PR 3031 failure repair

Migration 1356 preserves the custody digest from migration 0996 and excludes
only `game_character_online_intervals` and `game_character_online_pending`.
These G-8 observation rows change during quarantine; they are not transferable
Character/item custody. Including them made CANCEL fail its custody comparison
before restoring the public highscore. Migration 1353 remains free.

The equipped field-rune PostgreSQL scenario now holds its per-owner simulation
clock while SQL and wire turns run, then retains its existing explicit
application-plus-11-second residual evaluation. This prevents database latency
from injecting an unintended nine-second fire pulse into the replay assertion;
all damage, charge, deadline and replay assertions remain intact. Real transport
pacing and database timestamps still use their original clocks.

Merged main 47db34f09d760b2f435860c2f8d4ddf12a893e05, including the VIP
replay fix and the upstream callback integration repair. Main's single
`apply_input_rows` helper is retained unchanged; both callers use it through
their existing paths. This lane adds no separate VIP or royal costume behavior
change. These failure repairs introduce no new donor-derived gameplay rules.

Focused SQL regression evidence: `PUBPROJ_CUSTODY_STATISTICS_SEPARATION_PG`
passed in 2.089 seconds against an isolated clone of this lane's full migrated
PostgreSQL database. It applied migration 1356, inserted interval history and
changed pending coverage, verified the custody hash stayed identical, then
changed the canonical Character revision and verified the hash changed. The
transaction rolled back and the isolated database was dropped.

On repaired main, durability's focused unit selection passed 616/616 tests in
153.121 seconds (the 96 PostgreSQL-labelled cases were outside that unit run).
The workspace all-target compile passed in 4m30s and architecture checking
passed. Item-audit tags and quest binding evidence regenerated without changes.

PREPUSH-2 regeneration reran the prey model (15 cases, 776 creatures), then
native registration, item-audit tags and quest binding evidence. Only the prey
model input fingerprints changed; calculated simulation and slot values are
identical. The generated balance reports and JSON were produced by repository
tooling. The repaired main's callback helper is not edited by this lane.

World pin-refresh completed with the bundle digest unchanged and the client
placement pin already current. Dispatch and durability all-target clippy passed
with warnings denied in 11m06s. The transport unit/PG reruns are queued behind
the final all-target check; their completion is not claimed by this evidence.
Pin-check also passed against the regenerated index, proving the unchanged
32,839,850-byte World bundle. Item/Mount/content-tree equivalence and duplicate
migration checks passed; 1353 remains free and 1356 has no collision with main.
Final post-regeneration workspace all-target checking passed in 16m40s,
including build-lock wait. Formatting passed. At this cadence push, the
combined dispatch/durability unit selection and the account-deletion,
field-rune and upstream VIP PostgreSQL reruns are still building under the
owner's PREPUSH-2 pending-tests exception; they are not claimed green.

### Quest collector main merge (2026-11-08)

Merged main 78294c081a099c96a57948573d5dbf0caf398abf as requested for PR 3031.
Before the merge and after index reconciliation, the PUBPROJ diff against main
was identical in size: 131 files, 7,581 insertions and 49 deletions. Accepted
producers, configuration, fixtures and migrations 1350–1352/1354–1356 remain;
1353 remains unallocated by PUBPROJ. The merged-index migration check passed.

The four conflicts were generated prey-model fingerprints/reports. Repository
simulation and registration regenerated them; calculated model values match
main. World pins refreshed and checked against main's unchanged bundle digest.

The sole reported afda5a518 CI failure was trade admission. Incoming main
preserves typed unavailable-holder errors through offer/accept preflight and
retries those reads within the existing admission budget, retaining classified
business refusals. Before the fixture readiness repair, the original two-real-sessions
atomic/replay/cancellation PostgreSQL scenario passed in 218.263 seconds.
The incoming contention case exposed a one-shot source-map setup refusal;
existing diagnostics reached the owner SQL boundary before returning false.
The positive fixture now uses the existing bounded admission readiness retry.
The contention case additionally requires the held root to prevent the first
map commit, then requires the real committed map and accepted trade after
release. It passed in 127.294 seconds. No admission window, production guard,
or command-result assertion was relaxed.

After the readiness repair, forced contention passed in 127.294 seconds,
the original trade scenario in 133.988 seconds, inspection in 86.158 seconds,
account deletion in 15.430 seconds and upstream VIP replay in 16.354 seconds.
Final workspace all-target checking passed in 11m27s; formatting passed;
changed-crate all-target clippy with warnings denied passed in 9m21s.
Content's 322 unit tests and game-server's unit library passed. The remaining
broad unit run is still in progress and is not claimed complete here.

The complete focused PostgreSQL rerun passed 20/20 cases in 1551.003s
combined execution time (individual durations 6.840–1036.811s).
This includes forced/original trade, inspection, account deletion, VIP,
field runes and all fourteen selected authority cases. The broad unit run
remains pending under the owner's PREPUSH-2 cadence exception; content and
game-server unit libraries are green. No new PUBPROJ numbers were allocated.

## P3 Part A — accepted G-4 revision 3 wire

Base: `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`. Authority: Platform
`f5f91a6e81bc8b89b51cbb77517253cf549dc4fa`, accepted revision 3 (#1854).
Adds skills (public), achievements/equipment/outfit (owner-only) wire records,
canonical encoders and independent default-off activation fields. Missing
activation fields deserialize as false. No environment variable is invented.
The shared watermark vocabulary has exactly ten values. Main has five snapshot
families and no G-12 world_daily payload; its watermark value is represented
without inventing a snapshot, operation, route, source or producer loop.

The fixture generator preserves the in-scope revision 3 examples. New fixture
files have one terminal LF as requested; tests strip that file delimiter before
comparing to the transmitted encoder bytes, which have no terminal LF. Existing
fixtures and v1 bytes are unchanged. Resource ordinals 10094–10097 follow main's
maximum 10093; generated entries register the four exact request byte bounds.
The numbering helper was run; its reported clashes belong to unrelated open PR
migration allocations, and it does not enumerate resource-limit ordinals.

Part A proves structural bounds, ordered encoding and golden bytes. Catalogue
resolution (including synthetic fixture catalogue evidence), current ownership
and its revision floor, committed custody/DisplayedLook and suppression on
unknown source evidence remain Part B/C source-reader responsibilities. Merely
constructing a wire record does not prove those facts or authorize disclosure.
There is no inbound snapshot decoder in the existing Game v2 producer. Invalid
raw request fixtures are retained for Platform conformance; typed record
validation is exercised here. The equal-key changed-equipment example is valid
wire and is a sequence conflict, never a malformed schema; deciding 409 remains
the Platform ingest responsibility.

The supervisor authorized only mechanical integration compatibility edits:
config parses four independent default-false flags without new environment
variables; serve copies them into Activation. Wire-only families are excluded
from publisher selection even if enabled, and durability returns no snapshot
and refuses completeness facts for them until Part B provides the sources.

The pasted death example's `"killers":["Beren","Mira"]` differs from main's
stable-identity records. Per supervisor decision, deaths are outside P3: neither
the existing encoder nor death fixtures change; Platform receives the gap.
