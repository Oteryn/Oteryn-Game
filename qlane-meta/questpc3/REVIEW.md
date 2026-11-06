# Active restart self review — Sinatuki and retained Rookgaard adapter

Reviewed the whole handwritten diff, the new migration1041, the renamed adapter,
all new Sinatuki code/tests, and the unchanged generated JSON/map delta inventory.
The latter inputs still match restart-next-inputs.json; this restart changes no
content or generator input. Checklist: /home/mole/oteryn-cache/CODEX-CHECKLIST.md.
This is local self review, not a frozen-head independent review or publication.

- Bounded reads: Sinatuki history LIMIT101, partial character index, at most99
  positive partial receipts plus one final; existing inventory bound510 retained.
- Roles: no new role/table/grant. New trigger function revokes PUBLIC execution;
  runtime-role real dispatch executed its current-row and deferred guards.
- Locks: existing current equipment fence locks Character root before inventory;
  cumulative trigger takes the same root. Item writes retain existing locks.
- Input integrity: both exact fish Item/revision identities, positive quantities,
  both access tracks, metadata shape and exact five-shard output are checked.
  Zero outputs are allowed only for this named recipe; original checks are kept.
- Idempotency: immutable command/intent/character receipt, whole-stack before-row
  proof, root-serialized cumulative/once check; no separate unfenced counter.
- Recovery: exact and changed-payload replay plus cold context tests; retained
  receipt reply cannot reopen a retired conversation. Final step uses literal wire.
- Migration: forward1041 within reservation; narrow existing guard-chain arms;
  guarded replacement fails if expected predecessor text is absent. No test guard
  weakened. Initial and final PG runs prove the migration/custody/outbox sequence.
- Deadlines: existing semantic pass and NPC spell lane retained; test wire uses
  existing bounded30s driver, with256MiB scenario stack.
- Relay/capabilities/fencing: existing command7 and selected NPC capability;
  exact actor/NPC/content/conversation/lease checks remain. No new protocol code.
- Wiring: accepts_input has a production durability caller and both species are
  exercised through real dispatch. NPC adapter uses the existing hook and resume
  chain; only its single sorted shared module declaration changed.
- Exhaustiveness: workspace constructor/caller searches retained in active-*
  caller/constructor-sweep.log. No new enum variant, struct field or trait method
  in this restart; receipt triple's only consumer updated. All-target/all-feature
  clippy compiled all five changed crates after the type-alias cleanup.
- Tests: four existing Rookgaard cases retain their inputs and all assertions;
  only the literal wire helper was shared. Two meaningful Sinatuki scenarios cover
  mixed99+1 and whole100+1, access denial, exact quantities, final achievement,
  once-only reward, payload conflict, cold replay and the real serve_admitted path.
- Formatting: explicit rustfmt on15 changed Rust paths is required because the
  durability include! mount is not traversed by cargo fmt; explicit check passed.
- Scope: no Cargo changes, no dependency download, no forbidden CI/repository path,
  Git write, credential read, external write or source upload. Retained38 blockers
  name actual shared engine/content-owner boundaries rather than absent tests.

No new P0/P1 defect found in this pass. Focused Rookgaard regression and final
unit/migration/clippy checks are still running/queued; their final outputs own any
passing claim. git diff --check outside .questpc3 passes. The full scratch-inclusive
check reports preserved whitespace in raw pinned wiki captures and saved literal
patch evidence; those source bytes remain unchanged for provenance.

Earlier review below is historical.

# questpc3 self-review — previous completed batch

## Current restart verification

Re-read the full authored Rust/SQL/Python/test delta against the owner checklist. No additional concrete code defect found. No source change was needed in this restart. The retained exchange receipt, root/custody locking, recovery/session fences, semantic deadline, capability gates and closed recipe checks remain intact. The real USE and NPC_TALK tests retain their once/repeatable, duplicate/conflict, cold replay, nested input, exact reward/text/charge and no-reopen assertions. No test was removed or weakened.

Refreshed workspace-wide searches are current-restart-all-sites.log and current-restart-case-sites.log: NpcQuestExchange has one production constructor; Exchange has four complete static rows; ChestCase has its complete constructor/destructure; the private NPC Case has four complete rows; the adapter State is default-constructed and both Step variants are handled. Every added public exchange/rule entry still has a production caller and literal-dispatch PG coverage; there is no new trait method. All86 non-record hashes match the prior tested snapshot. Fresh semantic comparison confirms all29 JSON change-path sets and exactly24 UID additions across17 B3 regions with every other map fact unchanged (current-restart-tree-proof.json).

All56 title records and41 canonical blocker pointers/values were rechecked. Existing missing implementations remain explicitly BLOCKED, with exact gap, file:line and attempted path; none is newly qualified by this audit. No owner decision is pending. Fresh PG qualification passed23 chest tests/24 markers and4 NPC tests/4 markers; clippy passed for all five affected crates. All12 selected current checks passed (current-restart-checks.json), including587 pure Rust tests, the one dispatch binding/coverage test, and both formatting checks. One existing content test remains ignored; this diff adds no ignore. Authored diff whitespace checks pass outside immutable source captures and the retained historical full-code.diff. This is a local self-review, without a frozen candidate, independent review or publication.

## Current terminal-status pass — 2026-11-06

Re-read the full hand-authored production, SQL, Python and PG-test delta against
the owner's `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`, then refreshed workspace
constructor/match/impl/caller searches (`owner-final-constructor-review.log`,
`owner-final-all-sites.log`, `owner-final-case-sites.log`). The five-field
NpcQuestExchange construction, eight-field Exchange rows, ten-field ChestCase
constructor/destructure, seven-field NPC Case rows, two-field State and both
Step variants are complete. Same-name private types in other modules are
unrelated. No new trait method exists. The public exchange owner and rule
methods have production callers and literal-dispatch PostgreSQL coverage.

Checklist 1–8: keyed or inventory-bounded reads, root/custody locks, immutable
receipt replay before consumed-source lookup, exact wire conflicts, retained
cold replies and the closed migration1040 recipe tuples remain intact. There
is no new table/role/grant or migration in this continuation. Checklist9–12:
the semantic deadline, active content, NPC identity/generation, physical
service, current lease and selected NPC capability checks remain in the real
dispatch path; no lock is held over the database await. Checklist13–19:
all production callers are wired, both mounted PG files retain their complete
once/replay/inventory/text/charge assertions, and no test was removed, skipped
or weakened. No command, capability, wire field, dependency or protected CI
file changed. Fresh formatting and all-target/all-feature clippy evidence is
recorded in owner-final-checks.json; all13 selected checks completed with exit0.

Fresh semantic comparison in owner-final-map-review.json proves exactly24
marker additions across17 B3 regions and no other map changes. The29 JSON
change-path inventories in owner-final-json-review.json match the earlier
review. All86 non-record input hashes match restart-next-inputs.json; this
continuation changes only task records and evidence. Existing captured
reference whitespace and the historical diff snapshot remain untouched.

No additional concrete code defect was found. The terminal status validator
checks all56 captured titles: every qualified row names a real passing PG
test with POSTGRES_CASE_COMPLETED/duration, and every blocked row records an
exact gap, file:line and attempted path. This is not full R–S gameplay
completion, independent frozen-head review or publication. The38 missing
implementations and retained Dawnport exclusion are explicit in
FINAL-BLOCKERS.md; no new implementation is claimed by this pass.

## Latest restart boundary sweep

Re-read the whole production/test/authoring diff against the owner's
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. No additional concrete code defect
was found. This continuation changes task records and evidence only; it does
not add a function, field, enum variant, trait method, migration, command or
capability. The earlier complete code review below remains applicable.

Refreshed constructor/match/caller searches across apps/, crates/ and tools/
are in restart-next-constructor-callers.log. The five-field NpcQuestExchange
has one production constructor, both Step variants are handled, and all four
Exchange/NPC Case constructors and the ChestCase constructor/destructure remain
complete. Every public exchange entry has a production caller and the retained
literal NPC_TALK/USE proof. CI auto-mounts the PG files through dispatch/build.rs.
No test, assertion, source/custody guard, deadline or session fence changed.

The independent semantic B3 comparison again proves exactly 24 marker additions
across 17 regions and unchanged remaining map facts. All 29 JSON change-path
inventories match the saved review. Fresh snapshots are restart-next-map-review.json,
restart-next-json-review.json and restart-next-inputs.json. No generated artifact
or source capture was rewritten. The authored diff whitespace check passes;
immutable wiki captures and the older saved full-code.diff retain their original
source whitespace.

The additional blocker inventory distinguishes existing endgame progression
from the explicit ContentUnavailable W02 admission handoff. World-change source
candidates and synthetic outfit packets do not establish production activation.
The 35 stale IN_PROGRESS rows now name their concrete boundaries; 17 qualified
paths, 38 unfinished titles and one Dawnport exclusion remain. Validation results
and exact commands belong to restart-next-checks.json. This is a local worker
review, not independent frozen-head review or a publication/merge claim.

Reviewed the complete code diff, every new source/test file and semantic JSON/B3 differences
against /home/mole/oteryn-cache/CODEX-CHECKLIST.md. This is a local worker review; it is not
an independent external review, a frozen candidate, or permission to publish.

- Persistence classes 1–8: no production SQL, table, migration or cause changes. Existing
  fenced claim/MINT transactions and receipts own every grant. New test reads are keyed or
  bounded (21 children for a 20-child maximum; one test database with one claim/receipt).
- Session classes 9–12: no production transport/fence/deadline/capability changes. Literal
  USE dispatch and its existing DB boundary are exercised, with exact command replay.
- Wiring 13–14: new public Rust functions: zero. New test struct ChestCase has eight fields;
  all constructor/destructuring sites were searched across apps/, crates/, tools/ and are
  confined to the new test module (`constructor-review.log`). No enum or trait changes.
  CI auto-mounts this dispatch --lib file through build.rs and the existing PostgreSQL lane.
- Tests 15: no deleted, ignored, skipped, weakened or disabled tests. Exact authoring totals
  increase from 336 to 337 / ready 220 to 221 for the new Rapier record. The historical 231
  plain and 105 variant records remain unchanged. A missing route URL still returns Ok in
  route-less CI as required; local qualification requires actual completion markers.
- Hygiene 16–19: manual rustfmt check passes for the auto-mounted file; final cargo fmt,
  clippy and PG results are tracked in RESUME.md. No command/capability allocation or wire
  changes; protected CI paths and migration files are unchanged.
- Generated artifacts: generated through authoring/map/registration/pin tools. B3 semantic
  review checks unchanged tiles, flags, houses, zones, palette/depth, all non-UID attributes,
  and contents. Only exact source startup UIDs are added. Existing 3566 native Item records
  are unchanged; the only native record addition is Banana3587 with existing canonical facts.
- Source parity: 112 fetched wiki pages retain revision/hash witnesses. Rapier's missing
  script claim uses exact UID14042/appearance2473/Item3272/count1. The 20 selected source
  chest stacks are read from the committed B3 map in PG scenarios, including ordinal0 and
  ordinal2 cases that the old simplified fixture did not represent. Skull of Ratha includes
  its third (Witch House) bag, because the pinned wiki title lists those rewards together.
- Completion scope: this is the 13-title reward batch. Short Sword, Studded Shield and
  Stealth Ring remain partial quests; other unimplemented R–S chains are not certified.

No outstanding code defect found in this pass. All listed validation completed with exit 0. Pin-check reproduces the generated
world digest; the 20 PG completion markers and individual durations are retained.

Final count/roundtrip repair: the legacy validator and test now include the scripted
source claim count and the same exact Rapier admission used by the materializer. The
Item roundtrip still compares every definition; no field or assertion is exempted.
The shard assertion names all three historical shards plus the new scripted shard.

Restart review and final-tree local verification are complete. The restart adds migration1040, NPC exchanges and a closed Small Sapphire container recipe; the earlier no-SQL/no-new-public-function statements above describe only the previous batch.

## Restart code review (2026-11-06)

Read every changed production Rust/Python/SQL hunk and both complete dispatch PG files, then re-read `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Semantic JSON paths are recorded in `final-json-review.json`; `final-map-review.json` proves exactly 24 marker additions and no other tile changes. Captured wiki/Lua are reference evidence, not runtime scripts. This remains a local worker review; no publication or independent-review claim is made.

- Persistence 1–8: new migration1040 is within the reserved range. It adds no role/grant, table or event code. The closed four NPC recipe tuples reuse craft source/custody locks, debit/mint, completion trigger, audit and command receipt. Existing ordinary craft receipts explicitly reject NPC receipt reuse. Receipt lookup is keyed by the unique session/command pair before inventory lookup; recovery and permit scope remain checked. Amber alone may debit the written notebook. Shared nested-source custody guards remain in place. SQL reads are keyed, bounded or over the already bounded carried inventory.
- Session 9–12: the adapter rechecks current NPC/context, physical service, actor life/spell state, active content digest and session-generation lease. Runtime/session guards drop before database awaits; the existing semantic deadline owns DB work. Retained replies do not reopen forgotten conversations. No command IDs, capability bits or wire fields were added.
- Wiring 13–14: all new public Rust entry points have production callers and literal NPC_TALK/USE PG coverage. `NpcQuestExchange` has five fields and one production construction site; `Exchange` has eight fields/four static rows; the adapter `State` has two fields, `Step` two variants and one exhaustive match; ChestCase has ten fields with one complete default constructor and one complete destructure. All names/match/constructor/impl sites were searched across apps/, crates/, tools/ (`restart-constructor-review.log`); unrelated same-name types were inspected separately. No trait method was added. Existing build.rs auto-mounts both test files in CI's dispatch library PG target.
- Tests 15: none removed, disabled, skipped or weakened. Authoring counts remain exact:231 historical plain records,105 variants and2 scripted additions; ready224, held item semantics9. Protection Amulet also legitimately admits the old Desert bag (native variants67->68, lowering errors38->37). New tests refuse changed source digest, position, marker, text and unadmitted children. Source text revision/fresh cursor corrections are limited to the new NPC fixture and documented in SHARED-FIXES.
- Hygiene 16–19: cargo fmt --all --check and explicit rustfmt --check of both auto-mounted PG files passed. Final clippy on all targets of all five changed crates, final filtered PG/unit/content tests and pin-check passed (final-checks.json). No protocol codes, public contracts, protected CI files, Cargo manifests or dependency versions changed.
- Generated material: authoring/map/materialization, source capture, native registration, raid registration, Prey simulation and pin tools produced the artifacts. Existing physical Item facts stay exact; six definitions gain bounded admission/classification changes (certificate637 also gains its proven nonstackable fact). Native profiles add banana3587, pan3466 and map2822; only existing certificate637 changes. Raid changes are only18 source-region hashes; Prey report changes are only input digests. The full strict legacy roundtrip and materialized-tree validator pass.
- Source parity: Small Sapphire keeps honest empty donor UIDs and an explicit native marker 64993, the four exact reward children, both text hashes and the accepted canonical Quest identity. The one-tile source/map discrepancy and certificate XML weight conflict are documented in the pinned packet. The older single-book recipe guard remains intact; the new closed recipe requires both written children.

No outstanding concrete code defect found in this pass. Final validation state belongs in `final-checks.json` and RESUME.md. Whole R–S completion is not claimed; reset-dependent/pot-triggered rewards and remaining storylines/outfits are unfinished.

Final verification: 15 recorded check commands completed with exit 0, including a second clippy/fmt pass and pin-check after the last metadata refresh. The final PG runs have 24 chest and 4 NPC completion markers (23+4 tests, no absent-route early return). Final pure Rust totals are 312 content, 24 NPC and 250 quest tests; the content crate has one pre-existing external-manifest ignore. All 5 world-repository and 123 content integration tests pass. Reward authoring 61, native Item 14 and overlay 3 Python tests pass. The repository coverage report now names tests for 34 claims (11 existing, 23 newly qualified; one of this lane's 24 was already qualified). Its supported set passes the actual binder comparison. Every named qualification test exists (`coverage-qualification-review.json`).

The final whole-diff/constructor review found no outstanding concrete code defect. Source captures retain their original whitespace/hash witnesses; `git diff --check` is clean outside the immutable `.questpc3/wiki/**` captures and the preserved older `.questpc3/full-code.diff` snapshot. No ignore/skip, weakened assertion, production role grant, dependency, command ID, capability bit, protected CI path or Git write was introduced. Publication and independent frozen-head review remain outside this local worker's scope.

## Continuation exchange review (2026-11-06)

Re-read the changed production code, SQL, authoring and PG fixture/template hunks
against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; the existing generated JSON/map
semantic review remains in final-json-review.json/final-map-review.json and those
artifacts were not changed in this continuation. Two reachable source-parity cases
were repaired as one class: separated/plural Lee'Delle keywords, and pending
confirmations that also contain the root offer keyword. Earlier root keywords
cannot initiate the barter. The four constant precedence lists were compared with
all 224 prior root/alias registrations in the locally pinned donor NPC files.

The production matching helpers are private and called by the existing public
exchange methods; no new public function, cause, protocol code, capability or
migration was added. All four Exchange keyword constructors now represent ordered
alternatives. The test Case adds `answer` at all four construction sites; searches
across apps/, crates/, tools/ are retained in continuation-constructor-review.log
and continuation-field-review.log. No enum or trait changed in this continuation.
All earlier NpcQuestExchange/State/Step/ChestCase sites remain accounted for.

No SQL, item custody guard, deadline, session-generation check or retained-receipt
route changed. The four renamed tests preserve the real wire, nested inputs,
written notebook, atomic repeatability, duplicate/conflicting payload, cold receipt
and no-reopen assertions. They add intercepted/reversed keyword probes and source
confirmation variants. The clock fix makes the guard probes reach production
selection without rate limiting; it does not weaken production or an assertion.
No test was removed, ignored, skipped or quarantined by the move. Fresh PG result:
four passed, zero failed, four completion markers, 226.61 s.

Final continuation checks: all five changed crates pass all-target/all-feature
clippy with -D warnings; 587 pure unit tests pass with one pre-existing content
ignore; both cargo and explicit mounted-file format checks pass. No production
code changed after the four-case PG proof. Fresh commands/results are in
continuation-checks.json. No independent frozen-head or publication review is
claimed for this uncommitted worker tree.

## Restart greeting repair review (2026-11-06)

Re-read the full production Rust/SQL/authoring/test delta against the owner's
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. The additional concrete defect was
first-focus processing: `FreshAdmissionAuthority::npc_talk` calls
`newhaven_talk`, whose ordinary dialogue opens Lee'Delle's conversation before
the exchange adapter examines the same `hi honey scented flowers` input. That
command consumed a flower and minted armor; the new shared guard failed on
`inventory_changed` (restart-greeting-red.log). All four source NPCs now require
the exact NPC identity/generation to be engaged before ordinary dialogue runs.
The receipt-first path remains before this guard, so cold committed replies do
not require reopening a conversation.

No SQL, lock, authority, semantic deadline, capability or session fence changed.
The added chat snapshot releases its existing lock before the dialogue await.
No public function, field, enum variant or trait method was added. Refreshed
searches across apps/, crates/ and tools/ cover all prior constructor/match/impl
sites and every production caller (restart-current-constructor-review.log,
restart-current-callers.log); the existing five-field NpcQuestExchange, eight-field
Exchange, two-field State/two-variant Step, ten-field ChestCase and seven-field
NPC Case remain complete. No test or assertion was removed, skipped or relaxed.
Each NPC case adds a mixed first greeting, source greeting retention and no
inventory change before the original keyword/confirmation/wire/replay probes.

Fresh production PG proof: 23 chest tests / 24 completion markers and 4 NPC tests / 4
completion markers, no absent-route return. The source rule unit test passes.
All five changed crates pass all-target/all-feature clippy `-D warnings`; cargo
fmt and explicit mounted-file rustfmt checks pass. All 61 reward-authoring and 3
map-overlay Python tests pass. The initial missing pytest module was handled by
running the repository's unittest cases, without installing anything.

Fresh whole-map comparison proves exactly 24 marker additions across 17 regions;
all other decoded facts are unchanged (restart-current-map-review.json). The 29
semantic JSON diffs have exactly the previously reviewed path inventory
(restart-current-json-review.json). Generated material and immutable source
captures were not edited in this continuation. Protected CI paths and manifests
are untouched. Diff whitespace checks pass outside the retained source captures
and historical full-code.diff snapshot, whose bytes stay intact.

No outstanding concrete checklist hit was found. No independent frozen-head or
publication review is claimed. Full R–S completion remains unfinished, with the
same 17 qualified paths and 38 remaining titles. Small Ruby's stale reset-finalizer
blocker is replaced with the traced B3/source-map pickup/remint boundary.

## Restart completion review

The active source/content snapshot has89 files and zero byte drift, including
Sinatuki's migration1041, durability conversion and dispatch tests. The complete
handwritten diff and these new files were checked against the owner's
CODEX-CHECKLIST.md. Restart constructor/caller sweep:
restart-constructor-caller-sweep.log. The source-backed Exchange table, serialized
NpcQuestExchange binding, conversation State and Step, and cohort constructors
retain all their callers and initializers. No new protocol/capability or test
suppression exists. Protected CI paths remain absent from the diff.

The prior review's seventeen-title total is superseded: Sinatuki is the eighteenth
qualified title, with two real NPC_TALK/serve_admitted PG cases, completion markers
and positive durations. Rookgaard final4/4 and chest23/23 (24 claim markers) logs
remain bound to unchanged inputs. The outstanding pure tests finished with
312 content passes/1 pre-existing ignored,24 NPC passes and251 quest passes.
Fresh five-crate clippy -D warnings, both format checks, dispatch check, durability
fluid/migration filters and six focused Python checks exit0. No additional
concrete checklist finding was identified; no independent publication review or
integration is claimed. Exact terminal gate:18 QUALIFIED plus38 BLOCKED=56 titles.
