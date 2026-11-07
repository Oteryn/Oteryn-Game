# WEVENTS — timed World event qualification evidence

Task branch: `claude/happy-davinci-ijz3wy-wevents`. Initial remote head:
`3722c495cb866d83b9cb6835b1bb5ba51f757267`. The owner authorizes direct
fast-forward batch pushes and conservative documented rules where source facts
cannot be recovered. This record does not replace other lanes' WORLDCHG-Q rows.

## Shared owner foundation

`world_event::changes::timed` extends the existing reset owner. Migration 1120
stores bounded event state, per-Character participation/cooldowns and immutable
input-bound retry receipts. Current epoch admission precedes every state read or
write; the existing raid scheduler polls after node, Channel generation and map
binding validation. Seasonal cycles survive daily resets. Daily cycles use the
reset epoch as identity, including when activation timestamps happen to be equal.

`interact` joins a caller-owned transaction and rechecks recovery, session,
Character revision, lease, node and scope generation. Its result is pending until
that transaction commits. Object custody, reward delivery and map effects must
join their existing owners; a stored counter is not a physical spawn or item grant.
No transport accepts client-supplied time, random draws or kill attribution.

Flower Month's source is Crystal
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/scripts/raids/world_changes/flower_month.lua:1-65`, SHA256
`dc1607c195712ed166dcda95423fba91821d0ea1b22b85a249fc9aab8d74b1bb`.
The donor's four areas rotate indefinitely, 25 Dryads per wave with 250 placement
attempts, every 900000 ms during June. This wins over the wiki five-wave account
by the owner's explicit decision. Missed interval slots are not replayed as a burst after downtime.
This foundation rule is superseded by the native binding below: the committed June
reset snapshot starts an indefinite cycle that lasts through the next Berlin
06:00 server save, including the June 30 overnight tail. This follows donor startup
month admission and its unbounded scheduleWave recursion.

Mummy source: the same Crystal revision,
`data-global/scripts/actions/worldchanges/the_mummys_curse/horestis_jars.lua:18-122`,
SHA256 `6e47be40083c0ff174c454940382f891f96d7c304c999457e5165ab98d24751a`.
AIDs 50006–50009 advance in order with 1/10 success and 30-minute failed-attempt
cooldown; 50010 completes the sequence without another draw. Lines 105–106
contain two spawn calls; the owner's explicit **single boss** instruction removes
the duplicate admission. Line 108 restores item 1603 after **277000 ms**;
lines 109–112 restore item 3514 after **1200000 ms**. Those executable delays
win over the script's inaccurate announcement. Daily state resets with the epoch.

## Validation record

The initial focused rules test failed with E0432 (missing `timed`) before the
implementation, captured in `/tmp/wevents-generic-red.log`.
The real PostgreSQL case uses the runtime role, production `poll`/`interact`,
transaction rollback, exact receipt replay, conflicting input, stale session and
epoch denial, four-area rotation, seasonal failure, single boss admission, both
wall deadlines, and equal-timestamp daily reset separation. It prints
`WEVENTS PG EXECUTED` and an elapsed duration only after successful execution.
Session-local logs are `/tmp/wevents-generic-*.log`.

Foundation validation: focused rules **1 passed**; focused PostgreSQL **1 passed**,
**5.22 s**, marker duration **5.210922522 s**. `cargo check --locked` passed for
world-change, durability and game-server. `cargo clippy --locked --all-targets
-- -D warnings` passed for all three changed crates. `cargo fmt --all --check`,
explicit mounted-test rustfmt and `git diff --check` passed. Tool-generated
`pin-refresh` and `pin-check` both passed, with unchanged inputs
`ebae5fa901f10f1711a101fc00c19584b7704e3beccf626c8afa4c1b2b1b8f87`
and World digest
`3af394d0f4e68a8485d4655e3109ceaf867320c5ca053290a29e8d297faf7afd`
(31101773 bytes). No full nextest or full PostgreSQL suite ran. Native title
binding qualification is recorded in the individual WORLDCHG-Q rows as it lands.

## Native binding batch: Flower Month, Mummy’s Curse, Chyllfroest

Reused qengine2 `5a6de28c142de185594217ce1e5cbc7f4b6e6e4e` through merge
`89ac1cece`: generic reward exchanges, encounter admission and Channel projections.
The startup selector generator verifies Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/startup/tables/item.lua:3087-3126`, SHA256
`677bffd2482d16a5c73633e3f7324c4dc16cf1e00fbb5368e56ce4b6d60402fd`.
It restores metadata at eleven bridge grounds and five ornate jars without changing
map binaries. `native_event_bindings.py --check --map-check` verifies all sixteen
selectors against both the retained donor and current native B3 placements.

Native map reconciliation: the current B3 tomb places item 1603 at all four
source wall coordinates; the donor removes 3514 and creates 1603 later. The
projection owns both source wall appearances at those exact coordinates, hides
preplaced 1603 during the open interval, and restores the donor delays. This
explicit map/source conflict is resolved in favor of the executable event cycle.

Native binding slice validation before publication: focused rules 4 passed;
native map tests 3 passed (0.01 s); catalogue generator 15 passed (14.991 s).
Real owner PostgreSQL: 2 tests passed, 23.44 s, with execution markers for
Chyllfroest 7.467046836 s, Flower 8.323957080 s, Mummy 7.641205679 s and the
foundation 7.518263334 s. The Mummy case also holds an uncommitted first state
insert while reconciliation waits, then verifies the committed receipt.
Real physical actor PostgreSQL: 2 passed, 70.73 s, with markers
`wevents_flower_map` 70.683130456 s (25 Dryads) and `wevents_mummy_map`
70.727587693 s (one Horestis). No suite-wide PG run was used.
Nine affected/reused crates passed `cargo check --locked --all-features`
(45.52 s). Formatting and explicit mounted-test formatting passed. Pin refresh
and pin check passed: inputs `bdf720e5cce527a9ca45b09d196d878ec12a1a6de47a3d28ddaaf6a7400f7726`,
World `d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`,
31101950 bytes. Strict clippy/content checks continue after this immediate
owner-requested publication; their final result will be recorded separately.

Qualification follow-up for the first three rows: all nine production libraries
passed strict `cargo clippy --locked --all-features --lib -- -D warnings`
(3m52s). The earlier broader all-target attempt exhausted the workspace disk;
unused compiler caches were removed and the required changed-library check
completed. Focused content checks passed: 123 selected integration content tests
(4.77 s) and 5 World-package tests (76.65 s). These are scoped content targets,
not a workspace test run. No full nextest or full PG suite was run.

Spring native membership scan found exactly six current B3 item14098 dragon
nests. Candidate coordinates and region SHA256 values are retained in the
session scratch record `/tmp/wevents-native-spring-nests.json`; authored seasonal
policy will bind those existing objects rather than infer empty map positions.

Final first-batch native rerun: 2 passed, 78.70 s; execution markers
`wevents_flower_map` 72.133108776 s and `wevents_mummy_map` 78.699636997 s.

Shared-object foundation: red E0432 for absent `objects`, then 2 Cargo tests
passed for hatch boundary/one-time claim, shared units/fire, reset and atomic
rejection of invalid times/overflow. `cargo check`, strict library clippy and
package fmt check passed for world-change. This is a reusable primitive; native
bindings for the remaining titles are recorded when their complete slices land.
Reused worldchg `318ab59d200ad2030ca7c829566b88175cb7e2dc` through merge
`fd1421e2b`: native death capture and World-owned credit authority/receipts.
Its focused real PG case passed (7.27 s), printing `WORLDCHG-Q PG EXECUTED`
with elapsed 7.262177514 s after concurrent writers, dedupe, rollback, stale
scope/epoch and reset checks. No full PostgreSQL suite ran.

## Shared seasonal lifecycle slice

Migration 1122 admits all nine event identities, bounds the lifecycle state to
32 KiB, adds per-Character reward readiness and an immutable raid-origin relation.
Old daily snapshots and timed states decode with inactive/default new fields.
New daily rolls remain part of the committed reset snapshot. The lifecycle
model covers Spring timers, synchronized Devovorga preparation/incarnations/final
boss, hourly Kingsday waves, consumed hourly robbery draws and shared bucket/fire
restoration. Policy choices and public nest exclusions are in CONTRACT-NOTES.md.
The declarative Quest/NPC binding schema is additive; payments and grants still
need the existing custody/MINT transaction and native selector admission.

The new model test initially failed because fire extinguishing did not spend
water (`/tmp/wevents-water-red.log`). Both that defect and hourly celebration
continuation after victory are covered by the focused regression. The earlier
seasonal API red test is `/tmp/wevents-seasons-red.log`.

Real PostgreSQL execution of
`cargo test --locked -p oteryn-game-server --test character_authority_postgres wevents_postgres_timed_owner -- --nocapture`
passed **1 test in 12.00 s**, with markers `WEVENTS PG EXECUTED: six lifecycle
kinds, shared water/fire, serialization, runtime role, rollback;
duration=5.481294ms` and `WEVENTS PG EXECUTED: timed owner, rollback, replay,
calendar, jars, reset; duration=11.995856195s`.
The six-kind portion writes and reloads actual PostgreSQL JSON under
`oteryn_game_runtime`, validates the restored state and proves transaction
rollback. It does not represent native spawn acknowledgement, paid encounter
admission or physical reward delivery; those remain subsequent integration
checks, and this slice promotes no additional title rows.

Frozen-tree rerun after removing generated compiler caches: the same focused
PostgreSQL test passed **1 test in 5.10 s**, with six-kind marker duration
**7.512804 ms** and owner-case duration **5.096840617 s**
(`/tmp/wevents-lifecycle-pg-retry.log`). The first frozen-tree link attempt
failed with `No space left on device (os error 28)` and linker signal 7; deleting
only old incremental compiler caches restored space. No source or test was
removed or weakened.

Final focused model checks passed **13 tests** across `wevents_objects`,
`wevents_seasons` and `worldchg_owner`; the existing Quest reward loader bounds
check passed **1 test**. `cargo check --locked` passed for world-change, quests
and durability. Strict clippy `--lib -- -D warnings` passed for world-change,
quests, durability, dispatch and game-server. Workspace `fmt --check`, explicit
rustfmt of the two mounted test files, and `git diff --check` passed.
Tool-driven `pin-refresh` and `pin-check` passed with unchanged World digest
`d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`.
The diff review checked enum/schema correspondence, receipt immutability,
current-epoch/session guards, failed hourly draw consumption, payment policy
bounds and old snapshot defaults. No newly enabled value source is published in
this schema slice. No full nextest or full PostgreSQL suite was run.

## Transactional event/reward composition slice

Native selectors with item rewards now enter a dedicated RevisionSlot method
that supplies the existing native generation and wheel-aware MINT capacity
lookup. Fresh USE and interrupted USE reconciliation share this path. The Quest
writer resolves the content-bound event action, checks the Character's locked
level and the current World epoch, and reserves admission in the same
transaction as custody inputs, MINT, storage and the Quest receipt. NPC exchanges
use the same admission composition. Historical Quest receipts return before
fresh effects. The new writer is registered with the existing structural gate.

A storage-free event callback would skip the durable writer in dispatch. The
production loader now rejects this shape. Its focused test failed before the
fix (`/tmp/wevents-binding-red.log`) and passed afterwards
(`/tmp/wevents-binding-green.log`, **1 test**). The unmodified structural gate,
compiled directly from its std-only source, passed **3 tests in 2.81 s** and
scanned production callers (`/tmp/wevents-binding-writer-tests.log`).

Real PostgreSQL `wevents_postgres_timed_owner` passed **1 test in 12.58 s**.
Its `WEVENTS PG EXECUTED` marker explicitly includes reward admission, rollback,
zero participation and duplicate denial; duration **12.543221213 s**. The
six-kind runtime-role persistence marker reports **9.487184 ms**. These new
assertions drive production `interact`, reserve once, replay the same key,
roll the reservation back, refuse zero participation and refuse a second key
following a successful claim. Item delivery remains delegated to MINT; this
owner test does not manufacture a reward Item or claim a newly qualified title.

`cargo check --locked` passed for durability and dispatch; strict clippy
`--lib -- -D warnings` passed for quests, durability, dispatch and game-server.
Workspace `fmt --check`, explicit mounted-source rustfmt (without traversing
unrelated child modules), and `git diff --check` passed. `pin-refresh` and
`pin-check` passed with unchanged World digest
`d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`.
The diff review traced fresh and historical callers, admission failure rollback,
static source resolution, Character sequencing, and bounded input validation.

## Native firing provenance and participation slice

Migration 1123 adds immutable native-death receipts. A unique constraint on the
firing, Channel, wave, entry and actor index prevents another death ID from
crediting the same actor twice. Main and the fresh wvar, wcontrib, worldchg and
qengine2 refs contain no migration in 1120–1129; the numbering tool also checked
open PR reservations (`/tmp/wevents-native-credit-number-check.log`).

The existing admitted Flower/Mummy firings now seal their event origin in the
scheduler transaction. Durable wave plans carry the marker through the existing
physical-effect equality guard into the native actor specification. Ordinary
raid actors remain unmarked. The lethal producer freezes the marked actor's
exact firing identity, sealed death occurrence and positive damage contributors;
this includes the proven lethal principal when the sixteen-entry damage ledger
could not retain another contributor. The list is bounded to seventeen.

The World writer independently rechecks recovery, current node/scope, reset,
target generation, placement acknowledgement, origin and cycle. It locks the
participant Character FK roots before the World cycle, preserving the Quest
writer's lock order. A native death preceding its placement ACK stays queued;
an acknowledged failed placement earns no credit. The death receipt and all
participant updates commit together. Exact replay returns the recorded result,
while a second death ID for the same actor and stale-reset sources are refused.

The focused native test failed with the missing marker/source APIs before their
implementation (`/tmp/wevents-native-source-red.log`). The actual carrier raid
module then passed **8 tests in 0.01 s**, including live actors, marked/unmarked
identity, death retention, replay, failed placement and cleanup
(`/tmp/wevents-native-source-final.log`). An initial additional filter matched
zero tests; it was corrected to `raid_spawn::tests` and the eight named tests
were read before reporting this result.

Real PostgreSQL `wevents_postgres_first_title_bindings` passed **1 test in
24.06 s**, with executed markers: Chyllfroest **8.979521477 s**, Flower
**6.105882617 s**, Mummy **8.972222328 s**. The Mummy case drives the production
scheduler, sealed origin, wave plan, placement ACK and native-credit writer;
it verifies pending ACK, successful credit, replay, alternate death-ID denial,
empty-participant denial and reset refusal. Five jar contributions plus one
accepted native-source contribution produce exactly six credits. Actual actor
realization is separately exercised by the carrier tests; the PostgreSQL owner
case supplies placement outcomes through the existing physical-owner API.

`cargo check --locked` and strict clippy `--lib -- -D warnings` passed for
entities, durability, dispatch and game-server. Workspace `fmt --check`, explicit
mounted-source rustfmt and `git diff --check` passed. Tool-generated
`pin-refresh` and `pin-check` passed with unchanged World digest
`d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`.
The diff review traced source tagging, every corpse-admission branch, transient
ACK ordering, actor identity/replay, participant bounds and transaction lock
order. Title qualification remains tied to each title's native integration
proof; this shared slice changes no additional WORLDCHG-Q status rows.

## Essence price policy correction

The focused test reproduced the placeholder currency mismatch: slot zero
returned `(3043,1)` instead of `(11666,2)`
(`/tmp/wevents-cost-red.log`). The five slots now require 2/5/8/12/16 tentacle
pieces, with retained-recipe and pinned-wiki citations in CONTRACT-NOTES.md.
The focused test passed **1 test**, strict clippy passed for the changed library
and test target, and the crate's `fmt --check` passed. Logs are
`/tmp/wevents-price-{clippy,test,fmt}.log`. The price policy publishes no new
event selector; its payment and encounter path receives separate title-level
PostgreSQL verification. No qualification status changes in this slice.

## Seasonal expiry and clock proof

The PostgreSQL regression first failed because an old annual cycle remained
active (`/tmp/wevents-expiry-red.log`). The frozen focused run then passed
1 test in **20.29 s**, with `WEVENTS PG EXECUTED` markers for Chyllfroest
**6.979974967 s**, Flower **6.706093949 s**, and Mummy **6.600754529 s**
(`/tmp/wevents-calendar-pg-frozen.log`). It exercises annual expiry, isolated
Root test time, and a held old-cycle lock that must not block a current jar.
Four-crate cargo check and strict library clippy passed in **30.41 s** and
**44.38 s** respectively. Workspace fmt, explicit mounted-source rustfmt,
diff whitespace checks, and tooling pin refresh/check passed; content pins
remain unchanged. Migration 1124 adds the bounded expiry index. This shared
slice changes no title qualification rows.

## Spring integration contract

The two public item-14098 nests are bound by authored IDs 52601/52602;
`native_event_bindings.py --map-check` verifies source-region SHA256, object
membership and absence of house ownership. The other four nest objects are
excluded. The raid generator adds one Dragon Hatchling per public nest, using
existing native creature policy and bounded area placement. Timers, admission
and retry cadence are the explicit policy in CONTRACT-NOTES.md, while surprise
nest contents retain the donor's eleven equally selected categories.

The scheduler seals the event/member/revision with the immutable firing and
records its planned version. Physical publication and ACK acquire the event
cycle before firing rows, matching scheduler lock order. Successful physical
ACK rearms the shared object once; all-Channel placement failure retries after
an hour. Superseded persistent actors use the existing LOST cleanup. Personal
hatch eligibility survives joining another nest. The NPC reward reserves the
once-per-season claim in the same sequenced transaction as MINT, and the paid
purchase debits its source-bound crystal coin through the existing reward owner.

## Shared hatch owner preparation proof

The focused PostgreSQL owner run passed **1 test / 26.30 s**, with executed
markers: Chyllfroest **6.926932154 s**, Flower **6.804642492 s**, Mummy
**6.530311934 s**, and Spring **6.026340056 s**
(`/tmp/wevents-spring-reset-pg-final.log`). Spring exercises the one-hour
boundary, no early reward, sealed native member, successful ACK, all-target
placement failure/retry, preserved terminal firing history, LOST cleanup and
reset recovery on the raid scheduler's next due check. The earlier retirement
attempt failed against the existing immutable-terminal trigger; migration 1125
adds a separate immutable retirement receipt instead of weakening that trigger.

Actual native hatchling realization passed **1 PostgreSQL test / 80.38 s**
(`/tmp/wevents-spring-native-pg.log`). Literal native nest USE passed with an
executed marker in **94.10603899 s**, and native surprise-nest opening passed in
**172.22974216 s**; logs are `/tmp/wevents-spring-three-pg-green.log` and
`/tmp/wevents-spring-native-four-pg.log` (those runs also retain the merchant
failures that led to adding Rosemarie's donor conversation registry entry).
The focused item-owner test reaches all eleven gift categories and inclusive
quantity endpoints: **1 passed / 0.25 s**. The generator's **15 tests**, native
map provenance test, reward schema test and **12** lifecycle/catalogue tests
passed. Seven-crate cargo check passed in **18.76 s**, strict library clippy in
**40.64 s**; fmt and tooling pin refresh/check passed, with unchanged World pin.

This shared-engine checkpoint keeps Spring unpublished in the committed owner
catalogue. Its native merchant verification is running separately; no new
WORLDCHG-Q title row is claimed here. Declared event raid bindings remain
reserved while unpublished, so they cannot fall through to ordinary calendar
admission. The focused owner case includes Spring whenever publication is on.

## Spring qualification

Spring into Life is now **QUALIFIED**. The source-bound Rosemarie conversation
resolved the real `NoConversation` regression. Both production NPC-dispatch
PostgreSQL cases passed, with executed markers: paid purchase **127.065023255 s**,
free participation reward **129.857878653 s**, overall **2 passed / 129.86 s**
(`/tmp/wevents-spring-merchant-pg-green.log`). They exercise real native MINT,
coin debit, filled-backpack refusal/rollback, literal command replay and repeated
claim refusal. These complete the independently executed nest USE, actual
hatchling realization, surprise-nest opening/weights, shared-clock ACK and
reset/retirement proofs recorded above. No seeded merchant prerequisite is
claimed as proof of physical hatching; those paths have separate executed cases.

Final strict clippy passed all seven changed libraries in **48.98 s**;
publication model checks passed **12 tests**, and fmt passed. Logs:
`/tmp/wevents-spring-publication-{clippy,model,fmt}.log`.
Only the Spring row changes in WORLDCHG-Q.md. The owner catalogue publishes its
closed timed binding, and the focused PostgreSQL owner run includes this title
whenever that publication is enabled. The two declared event raids remain
reserved if publication is later disabled.

## Kingsday qualification

Kingsday is **QUALIFIED** through synchronized celebration state, hourly native
exhibition raids and a participation-bound royal achievement. Respectful donor
greetings open King Tibianus's conversation; asking about the celebration
records attendance and reserves the Loyal Subject claim in the same transaction.
Attendance does not require killing an exhibition creature. Waves continue after
attendance victory and retire their previous population. The next inactive
reset closes admission. The explicit fallback chance, population and verified
public arena are recorded in `CONTRACT-NOTES.md`; they are Oteryn policy rather
than invented donor facts.

Red proof: `wevents_kingsday_royal_greeting_is_participation_without_arena_combat`
failed to compile before `Attend` existed. The native NPC test then found a real
wire-helper regression (`hi` instead of the donor's respectful phrase); that
failure is preserved in `/tmp/wevents-kingsday-native-pg.log`.
Green native PostgreSQL markers in `/tmp/wevents-kingsday-native-pg-final.log`:
`wevents_kingsday_arena` **148.26777236 s** and
`qengine-rewards-kingsday` **204.488350726 s**, **2 passed**. These execute actual
native creature placement and NPC wire/achievement writes, replay and repeat
refusal. The model has a separate zero-credit reward refusal case.

The generator's 15 focused tests and reproducibility check passed. Six-crate
cargo check passed in 4m22s. Tooling pin refresh and pin-check passed with
unchanged World pin `d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`
(31,101,950 bytes). Source dialogue file:line+SHA and wiki disagreement boundaries
are recorded in the contract notes.

The production scheduler/owner PostgreSQL case also passed: **1 passed / 34.13 s**,
with executed markers for Chyllfroest **6.669275162 s**, Flower **6.808789985 s**,
Mummy **7.333247266 s**, Spring **6.785818476 s** and Kingsday **6.529069867 s**
(`/tmp/wevents-kingsday-owner-pg.log`). Kingsday covers no first-hour firing,
hourly wave/replay, continued exhibitions after attendance, real placement ACK,
previous-wave LOST cleanup, claimed-participant refusal and inactive reset.
Strict six-crate clippy passed with `-D warnings` in **6m07s**; the focused model
run passed **13 tests**. Workspace fmt, mounted-source rustfmt and diff checks
passed. Logs: `/tmp/wevents-kingsday-{clippy-final,model,fmt,mounted-fmt}.log`.

## Bank shared owner checkpoint

The reusable bank-service gate, server hourly draw, recovery cost contract,
reset-scoped banker projection, duplicate recovery refusal, native actor
retirement/retry and exact unsigned death-key encoder are implemented. The
activation remains off in this checkpoint; native loot/reward verification
continues in the next slice. No new WORLDCHG-Q title row is claimed here.

Red: the recovery contract required no bag and repeated recovery errored; both
focused model tests now pass. The real PostgreSQL owner case exposed numeric
division rounding at maximum u64; migration 1126 now uses exact PostgreSQL
`div`, preserving every generation bit. Its executed Bank marker is
**8.205666004 s**, with **1 passed / 57.14 s** across all six selected owner
cases (`/tmp/wevents-bank-owner-pg-final.log`). This covers the production
hourly draw, no reset-time robbery, replay, Jesse's admitted raid plan/ACK,
Naji's closed service, unrelated banker admission, maximum-u64 encoding and
next-reset reopening. The case enables the candidate binding in its working
fixture; committed publication remains disabled until native reward proof.

Cargo check passed six crates in **42.00 s**; strict six-crate clippy passed
with `-D warnings` in **4m46s**. Focused cost/lifecycle and catalogue model tests
passed; fmt and mounted-source formatting passed. Migrations 1120–1129 were
checked against freshly fetched main, WVAR, WCONTRIB, WORLDCHG and QENGINE2;
none of those branches uses this range.

## Bank native Item/loot checkpoint

The stolen-gold bag and Jesse's four-stage corpse now materialize through
source-derived native profiles and the existing loot/deadline owners. The
20/20/10-slot chain and timers, bag weight conflict, source SHA/lines and
registered NPC bank callback are recorded in CONTRACT-NOTES.md. Historical
reference-only Item declarations are preserved. The native loot admission
generator consumes the exact admitted profiles, including corpse capacities.

The production NPC PostgreSQL case passed with executed marker
`qengine-rewards-bank` **135.149640147 s**, exercising actual native MINT and
hand-in, ordinary bag refusal, missing native kill proof refusal/rollback,
current sealed raid kill admission/replay, achievement and coin reward,
reopening and repeated/literal claim refusal. Log:
`/tmp/wevents-bank-native-pg-complete.log`. Its companion actor case exposed a
fixture assertion equating boss identity with end policy; that correction is
being independently verified and is not included in this Item checkpoint.
Bank's publication row and callbacks remain outside this commit.

Source Item/corpse tests: **2 passed / 5.402 s**. Native loot class: **4 passed /
19.044 s**. The dependent Prey model was actually regenerated with the unchanged
Rust combat tool (**15 cases / 776 creatures**) against the complete current
loot admission; generated reports/runtime values and registration were refreshed
through tooling. Final six-crate cargo check and strict clippy each passed in
**2m34s**. Formatting and tooling pin refresh/check passed; World pin unchanged.

## Bank Robbery qualification

Bank Robbery is **QUALIFIED**. The final native actor case passed with executed
marker `wevents_bank_robber` **65.333512195 s**, **1 passed**
(`/tmp/wevents-bank-actor-pg-green.log`). It loads Jesse's admitted native reward
row and guaranteed bag, places the real native boss at the declared public
source area, checks the admitted boss marker and commits actual HP death.
The prior fixture failure compared that marker against an end-policy switch;
the assertion now derives expected identity from the admitted boss-entry set.
The successful NPC case is independently recorded above (**135.149640147 s**);
it uses a sealed native source/placement and frozen kill proof fixture, not a
claim that its input MINT fixture substitutes for the physical actor test.

Together with the production hourly/service PG proof (**8.205666004 s**), this
closes the native activation, current-epoch service denial, actual actor/loot
admission, source-bound recovery, reward/achievement, replay and reset paths.
The public Thais escape alley, 10‰ hourly draw and first-success-per-day scope
are documented conservative policy where no complete donor scheduler survived.
Jesse's guaranteed stolen-gold bag, Naji's registered bank callback and donor
corpse chain have explicit file:line+SHA evidence. Source code wins the bag
weight conflict. Only this title's WORLDCHG-Q row changes.

Final cargo check and strict library clippy passed the six changed crates, each
in **2m34s**. Source Item/corpse tests and the four native loot tests passed;
fmt and mounted-source formatting passed. Final tooling pin refresh/check
passed with unchanged World pin
`d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`.

The native NPC greeting/service admission unit passed (**1 / 46.59 s**), the
focused World lifecycle/cost/catalogue run passed **15 tests**, and the full
content registration reproducibility check passed (**42 generated files**).
Logs: `/tmp/wevents-bank-{npc-unit,model-final,registration-check}.log`.

## Greenshore shared restoration and population binding

The existing daily World selection owns Insectoid Invasion. Two exact retained
Scout memberships are reserved before ordinary activation and admitted through
one event-owned raid. Synchronized 90-second slots replace superseded populations;
shared bucket revisions cannot create extra waves. Victory and reset retire the
population through the existing LOST cleanup path. The conservative restoration
rule and retained placement/spawn SHA256 evidence are in `CONTRACT-NOTES.md`.

Four native ground USE selectors bind the shared bucket and three fires.
Declarative projections resolve served item definitions independently of base
placements, preserve stable entry identities, and atomically remove only the
named owner's objects. A missing/house tile cannot publish partial state.
The existing Mummy projection uses separate ownership.

The focused model first failed both new cases (`wevents-insect-model-red.log`),
then passed all four cases (`wevents-insect-model-final.log`): full/completed
interaction no-ops, 90-second synchronization/replay, fresh reset, and exact
source membership reservation. The focused world projection test passed
(`wevents-insect-projection-test.log`, one test, 0.01 s), including overlapping
owners and removal of omitted objects. Raid producer tests: 15 passed in 20.593 s.
The selector tool verified all 22 bound native objects against B3 placements.
Compiler `pin-refresh` and `pin-check` both passed; source-world digest remains
`d5e52c7a0ef24bf6aa7b78926fd0806e43eb731ac69b10783f20c9f10dcd88e5`.

The focused real PostgreSQL restoration case passed: marker
`WEVENTS PG EXECUTED: insect restoration duration=5.948852685s`, one test,
5.95 s (`wevents-insect-owner-pg-green.log`). It runs current-session/runtime-role
World interactions, rejects extinguishing without water, fills to capacity,
refuses overflow, extinguishes all three fires, preserves completed-state no-ops,
reads committed Victory and refuses the next population slot. The preceding
combined run exposed a test-clock mismatch on its final read; all interaction
assertions had passed. The corrected case installs the existing owner test clock.

Checkpoint checks passed: locked crate check; strict `--all-features --lib`
clippy with `-D warnings` for world-change, durability, world, dispatch,
game-server and quests; workspace fmt check, explicit mounted-source rustfmt and
staged whitespace check. Logs: `wevents-insect-check-final.log`,
`wevents-insect-clippy.log`, `wevents-insect-quests-clippy-final.log` and
`wevents-insect-fmt-final.log`.
