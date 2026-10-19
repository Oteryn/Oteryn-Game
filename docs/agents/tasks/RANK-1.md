# RANK-1 — highscores and analysers (skeleton task record)

```yaml
task_id: RANK-1
title: "RANK-1: highscores and analysers"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row RANK-1 (rows 48, 128); §7 'HUNT-1, RANK-1' HS-* and ANALYSER-* rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HIGHSCORES0_WORLD_HIGHSCORES_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ANALYSERS0_HUNTING_ANALYSERS_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/rank-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Highscore snapshot tables, per-World snapshot job, categories, ranks and filters (HS-1) with `HIGHSCORES_V1` query (HS-WIRE-1); analyser fact emission at commit points (ANALYSER-EMIT-1). Exports are normalized Game data Atlas consumes.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

HS-1, HS-WIRE-1 (Reg), ANALYSER-EMIT-1. ANALYSER-WIRE-1 MERGED; ANALYSER-CLIENT-1 is client lane.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-PROGRESS-1 (family entry gate) | BLOCKING |
| CHAR-BUILD-1b | MERGED |
| CHARM-1 | MERGED |
| account achievements 0021 | MERGED |
| ANALYSER-WIRE-1 | MERGED |
| ATTACK-1b | MERGED |
| D3 corpse mint | MERGED (#1213) |
| acceptance of HIGHSCORES0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |
| acceptance of ANALYSERS0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/highscores/`
- `apps/game-server/src/durability/highscore_snapshot.rs`
- `apps/game-server/src/durability/highscore_snapshot_tests.rs`
- `apps/game-server/src/gameplay_transport/highscores.rs`
- `crates/protocol-oteryn/src/highscores.rs`
- `crates/protocol-oteryn/src/highscores_tests.rs`
- `apps/game-server/src/analyser_emit.rs`
- `apps/game-server/tests/highscores_postgres.rs` (removed; its cases moved to the support file below)
- `docs/agents/tasks/RANK-1.md` (this record: status and notes only)

Widened by CP D1441 (integration slot, 2026-10-18):

- `apps/game-server/migrations/0129_world_highscores.sql` (0097 until the CP D1453 wave numbering)
- `apps/game-server/src/highscores/reply.rs` (the command type 14 handler core)
- `apps/game-server/src/gameplay_transport/highscores_tests.rs`
- `apps/game-server/src/gameplay_transport/capabilities.rs`, `capabilities_tests.rs`,
  `connection.rs`, `mod.rs` (capability 9 offer, `GATED` row, dispatch, authority)
- `apps/game-server/src/node/serve.rs` (the per-World snapshot job in the serve loop)
- `apps/game-server/src/durability/mod.rs`, `apps/game-server/src/lib.rs` (module registration)
- `apps/game-server/tests/character_authority_postgres.rs`,
  `apps/game-server/tests/support/highscores_postgres_cases.rs`,
  `apps/game-server/tests/support/public_projections_postgres_cases.rs`
- `crates/protocol-oteryn/src/lib.rs` (`pub mod highscores`, capability 9, errors 1200/1201)
- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`,
  `docs/contracts/protocol-oteryn/v1/highscores_v1.proto`

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- crates/protocol-oteryn/src/analyser.rs (capability 10, domain 15; existing)
- commit points in combat/item lanes for ANALYSER-EMIT-1
- node job list for snapshot job
- capabilities.rs `GATED` (capability 9 `HIGHSCORES_V1`, command type 14)

## Inseparable bundles (never split across PRs)

- migration + snapshot job + durability
- wire + registry + capability + handler + tests (HS-WIRE-1)

## Acceptance criteria

- [ ] Snapshot deterministic per World and epoch; ranks stable on ties.
- [ ] Paged query bounded.
- [ ] Analyser facts emitted only at commit, deterministic replay.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server rank
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HIGHSCORES0_WORLD_HIGHSCORES_DECISION_2026-10-03.md §3-§6
- docs/architecture/reviews/OTERYN_GAME_ANALYSERS0_HUNTING_ANALYSERS_DECISION_2026-10-03.md §4-§6
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation — HS-1 ranking rules (2026-10-15, unpublished)

- Existing draft #2168, branch `skeleton/rank-1`, allocated head
  `e2fb944ced21bcc9905889cb2f4427c95290ad11`. Only
  `highscores/ranking.rs`, `ranking_tests.rs` and this record are allocated.
  No commit, publication, ready flag, schema, registry, content or shared wiring.
- The lead confirmed effective HIGHSCORES0 acceptance through merged #2209.
  Owning text was read at accepted main
  `e2da9805d296a27b06b32335bf67394509d4910b`. Its §5 and D309 clarification
  govern this preparation; historical CANDIDATE dependency rows above no longer
  describe that acceptance. LOOP-PROGRESS-1 still gates family integration, not
  these separately accepted pure ranking rules.
- Prepared callable ordering over a complete, already eligible committed cut for
  one World/category: descending full value, standard competition ranks
  (1, 2, 2, 4), positions tied by raw Unicode code point name then native
  `CharacterId`. There is no case folding, Unicode normalization, locale or SQL
  collation. The caller's scalar or `(level, progress)` tuple is compared as-is;
  no category, points formula, achievement aggregation or Charm score is created.
- Each base-vocation list preserves its rows' global ranks and applies its own
  1,000-row cap **after** filtering the uncapped eligible cut. Promotions reuse
  the existing `character_wheel::base_vocation` mapping. An invalid/non-base
  filter or duplicate CharacterId produces a local typed preparation error,
  not a new wire/error-registry code. Source rows are borrowed and unchanged.
- Native CharacterId is the real protocol/foundation type. Durable vocation keys
  remain strings, as in the existing build state; no parallel vocation enum is
  introduced. Snapshot name text is borrowed rather than minted as a Character
  name: the current ASCII-only creation policy remains separate from the owning
  highscore contract's explicit Unicode ordering rule.

### Remaining indivisible integration work (REPORT_ONLY)

No production module or consumer is registered. The future snapshot owner must
select the complete eligible live-root cut for one World/category, exclude `none`
for categories 1–9 and 11, choose each account category's representative and
source-native values, and supply all rows before capping. These are source
selection/eligibility responsibilities, not facts that this pure comparator can
authenticate. Snapshot persistence, repeatable-read source transaction, job
lease/restart/retention, wire paging and snapshot tokens, registries, protocol,
Atlas exports and analyser emission remain outside this local scope. Ranking
rows grant no session, World, Channel or gameplay authority.

### Validation and self-review

- Source-bound isolated harness: **11 passed, 0 failed**. Cases cover competition
  gaps, raw Unicode/non-BMP and normalization/case boundaries, CharacterId tie
  order independent of input, full level/progress tuple values, global ranks
  across vocation ties, all five promotion families, a filtered list whose first
  member lies below the global cap, truncation inside a tie, empty/`none` inputs,
  invalid filters/duplicate sources, and u64 extremes without score arithmetic.
- Isolated all-target Clippy with the native root lint table: passed. Owned-file
  rustfmt and `cargo fmt --all --check`: passed. The harness imports the real
  protocol crate and the literal existing vocation helper; no native type is
  recreated. Full-server/package, persistence, wire and live-authority
  qualification are not claimed; all remain pending their actual consumers.
- Actual logs, harness and source/hash binding are retained outside the repository
  in `/workspace/oteryn-prep/rank-1-evidence`. Rust commands used the shared flock,
  target and two-job limit. No test binary was copied.
- Governance validator passed (22 policy documents, 9 lanes); governance unit
  tests passed (124, 0 failures). Logs are retained in that evidence directory.
- Whole-diff self-review: rank equality depends on the full value alone; tie
  position ordering cannot change rank; filtering cannot renumber or inherit the
  all-list cap; native types and promotion rules are reused. No authority-bearing
  mutation exists here, so current-session/replay-fence qualification is not
  applicable to this callable subset. Those checks remain required at future
  snapshot/wire consumer boundaries.

## Owned snapshot preparation, generation 2 (authoring checkpoint; 2026-10-15)

The prior pure packet was published to existing draft #2168 at
`b9fb7d9cd135bc3f91c36fa26c5a0c342c660bc6`. The new exact six-path allocation
prepares the actual HS-1 snapshot storage component without merging main or
changing shared registrations, migration numbers, registries, grants or pins.
HIGHSCORES0 acceptance is effective through ACCEPT-BATCH #2209, row 5; the
historical candidate/entry-gate table is not current acceptance evidence.

Authored `durability/highscore_snapshot.rs` reuses the published ranking kernel
and native `DurableBuildState` validation. It selects the complete committed
World cut from actual `0009` XP, `0030` build and `0022` root names. No-build
means native vocation `none`, excluded; a non-none eligible build missing its
required XP source fails rather than silently shrinking a snapshot. Categories
1–9 preserve scalar XP and complete magic/skill `(level, progress)` values.
All six vocation lists retain global ranks and each applies its own 1,000 cap.
No runtime state, speculative exclusion workflow, new score formula, Account
achievement source or Charm source is supplied.

The existing native Root issues the actual semantic pass. Its ordinary
transaction opener performs a SELECT before returning, so the owned projection
opener uses SQLx `begin_with` to establish REPEATABLE READ first, retaining the
same actual remaining transaction/statement/lock timeouts and native commit
deadline. No larger pass budget or separate raw administrative connection is
introduced in the product code. Projection publication is not a Character
write or gameplay permission and consumes no caller-supplied authority boolean.

One private publication-head row per World/category serializes replacement under
RR. A stale concurrent publisher returns typed retry/unavailable and never
retries automatically. One World source cut publishes all nine categories in
one physical transaction. Bounded per-category JSON bulk inserts avoid per-row
network roundtrips. New heads and current/previous retention pruning commit
together; failure preserves the old generation. Private candidate constraints
and triggers prevent retained-header removal, row changes, late append or
direct row deletion. A prior snapshot's rows prune by header cascade only.

The actual reader starts RR, resolves a token only within its exact
World/category and retained current/previous set, then reads at most 50 rows
with stable positions and an own row only when in that capped selected list.
Page 0 and invalid base filters refuse; pages beyond the list are empty. A
retired or foreign token returns the local `SnapshotExpired` preparation error.
These private result types/filter storage codes allocate no public wire values.
The future authenticated handler must independently supply the session's World
and Character; these query arguments and snapshots confer no authority.

### Historical authoring qualification state and remaining gates

Seven genuine PostgreSQL case bodies are authored for native source tuples/no
source writes, pinned pages/three generations, global-rank/filter beyond 1,000,
atomic failure and immutability, actual repeatable-read/source and stale
publisher behavior, World/category/token isolation and two real Root publishers.
The inherited 11 ranking cases remain unchanged. **No new test execution is
claimed at this authoring stage.** Heavy Rust qualification is held by the lead's
storage budget; evidence will record actual failures, execution counts and final
source bindings before a candidate is frozen.

The fixture uses real installed migrations and one genuine current-fenced native
build writer to establish its source template. Larger datasets are explicitly
administrative committed source DATA in the actual owning tables with native
SQL guards enabled; they do not claim gameplay initialization/admission for
synthetic Characters. `SNAPSHOT_SCHEMA_PROPOSAL` is uninstalled. The native Root
supplies a connection/deadline, not a grant to operate the snapshot job. The CP
must bind the dedicated projection role and source SELECT/projection DML/runtime
SELECT-only permissions. Production credential/grant qualification is pending.

Shared module/Cargo/canonical-PG registration, migration number/install/grants,
the 30-minute and planned-reset job, categories 10/11 and unsupported later
categories, actual HS-WIRE-1/session admission, capability/registry/profile,
exports and analyser emission remain open. The standalone target is explicitly
inactive until CP registration; an inactive zero-test target is not a runtime
pass. Validation will use exact outside-tree native registration/body bridges,
not source edits to shared files or substitutes for native types/Root APIs.

### Generation 2 executed qualification and frozen boundary

The authoring execution hold above is superseded by actual attempt 6. The exact
native bodies passed **18 tests: 11 inherited ranking and 7 genuine PostgreSQL
17.6 tests, 0 failed, 0 ignored, 0 filtered**, in 17.35 seconds. Each PostgreSQL
case emitted its actual `server_version_num=170006`. The 1,002-eligible-character
fixture published nine categories and six lists in 801 ms under the unchanged
two-second Root pass; this fixture measurement is not full-World throughput
qualification. Source hashes before and after execution are identical.

Original production strict metadata/Clippy and the enabled original standalone
private-Root consumer strict `--test` Clippy passed. The selected native
`cfg(test)` body-context Clippy, real native rlib codegen and tiny test driver
also passed. The original standalone target was checked with only its inactive
gate removed and the literal native module registered outside the draft. The
execution bridge changes only selected test attributes/parent dispatch
visibility; the genuine Root, SQL guards, fixtures and owned test bodies remain
unchanged. Baseline Rust unused-helper warnings in the normal `cfg(test)` rlib
are visible; configured Clippy denials remain enforced. No whole-server global
unit harness or full-workspace runtime pass is claimed.

The executed cases cover real committed native source tuples without source
writes, complete values/ties/Unicode order, capped filters with global rank
1,002, stable 50-row pages across publication and current/previous expiry,
atomic rollback and exact immutable-membership trigger errors, an actual RR
source cut/stale publisher conflict, World/category/token isolation, and two
independent genuine Root connections publishing coherently or returning typed
retry. They do not qualify production role credentials or imply a projection
is current session/World authority. Concurrent calls use separately connected
Roots; the explicit stale RR transaction separately qualifies a deterministic
PostgreSQL snapshot conflict rather than relying on scheduler timing.

Evidence is retained at
`/workspace/oteryn-prep/rank-1-snapshot-native-2-evidence`: attempt-6 commands,
source/extern/locator bindings, raw logs, resource counters and artifact hashes.
Earlier sparse-input failures are setup failures; Cargo capture exit 101 is the
intentional pre-link capture stop. Attempt 3 found a needless-borrow lint, fixed
without policy changes. Attempt 5 refused actual dependency-byte drift; attempt
6 captured and qualified a fresh coherent Cargo graph rather than substituting
externs or linking stale metadata. No intentional missing-file failure is
presented as behavioral regression evidence. All Rust commands used the shared
flock, two-job limit, zero debug/incremental setting and actual Root budget.

Whole-diff self-review preserves one native ranking implementation, complete
World source cut, all-nine atomic publication, bounded bulk writes, immutable
retained snapshots, same-cut reader pages and typed conflict failure without
automatic retry. Source tables and CharacterRevision remain untouched. The
private schema, production grants/job role, 30-minute/reset scheduling,
categories 10/11, shared registration, migration installation, authenticated
wire integration and the original family acceptance checklist remain OPEN.
Independent exact-head review, repository CI and the CP integration/ready/queue
slot remain required. This packet neither completes the full mechanic nor
authorizes ready, merge, shared-file integration or publication by the worker.

Final governance validation passed (22 policy documents, 9 lanes), and all
124 governance unit tests passed. Owned-file formatting and diff/scope checks
passed. Only five of the six leased paths change; the inherited ranking test
source is byte-identical to the base and actual execution manifest. Completed
private native rlib/driver evidence is retained losslessly as gzip with raw and
stream SHA-256 plus exact restore arguments; shared dependencies are untouched.

## Owned HS-1 completion preparation (2026-10-16; unpublished)

This authoring allocation starts at existing draft #2168 head
`8aaac7410d0fa53c5e2a0dd564b22a55e1e3cb8b`. It adds the following owned native
consumer work without changing shared registrations, manifests, registries,
migrations, grants or capability activation:

- Full eleven-category publication reads account facts and the owning Achievement
  catalogue (including retired/unknown-key behavior); one representative per
  account is the highest-level live World Character, then lowest CharacterId,
  selected before the vocation filter. All account facts count regardless of
  earning Character or World. Missing representative progression fails closed.
- Earned Charm Points read the actual Bestiary counters in one set-based World
  read, use the owning BestiaryRace completion predicate and content points, and
  ignore spent currency. The actual Charm source owner validates the catalogue
  keys, duplicates and bounds. Both additional categories share the original
  repeatable-read source cut and atomic all-category publication.
- The owned World job invokes actual Root publication every 30 minutes and after
  composition-root reset notifications. Publication locks serialize the durable
  due check; a restart consults committed timestamps. Failed passes retain the
  previous generation. No caller-provided score, raw administrative pool or
  gameplay authority is introduced. The existing nine-category entry point is
  retained for the original private native fixtures; production uses the full
  catalogue-bound refresh method.
- The exact `AFTER INSERT` public producer trigger attachment is an executable
  separate SQL proposal requiring the real owning migration
  `0091_public_projections.sql` from protected main@24fc332f256638e3b784bff1b3c85b18e2665a76.
  The CP must append it after installing that migration and the private snapshot
  schema. It fails installation when its owner is absent; no stub or silent
  attachment skip is supplied. Snapshot, public revision and outbox share the
  same transaction, including rollback after a later category fails.

Three additional genuine PostgreSQL case bodies cover account representative
selection/points, real fenced Bestiary completion, durable due checks with a
second Root and planned reset, plus exact public-trigger/outbox rollback. The
public case reads the actual migration at runtime and fails explicitly when
that executor prerequisite is absent. There are now ten PostgreSQL bodies and
11 inherited ranking cases; **none of the new bodies has executed**. The original
standalone gate remains honest: enabling `cfg(any())` without the shared native
module registration would run zero highscore cases.

Validation in this allocation: owned-file rustfmt and diff whitespace checks
passed; governance validation passed (22 documents, nine lanes), and all 124
Python governance tests passed. Rust/SQL execution remains blocked by the lead's
shared heavy-resource slot. Historical generation-2 runtime evidence above does
not qualify these new source bytes or the added categories/job/trigger.

Remaining REPORT_ONLY integration dependencies: dedicated projection-role Root
credentials and grants (SELECT native roots/progression/build/account facts and
Bestiary counters; projection DML and identity sequence access; runtime snapshot
SELECT only), durability module registration, canonical PostgreSQL target/Cargo
registration, actual per-World node task/reset notifications and content-generation
catalogue binding. The migration slot must install both schema and public-trigger
attachment after 0091. Qualification requires PostgreSQL 17.6 and the unchanged
Root budget; representative full-World performance is unqualified.

HS-WIRE-1 remains blocked on an accepted concrete `.proto` layout, error allocation
and the shared registry/capability/connection slot: the accepted prose does not
assign field numbers/types or the vocation representation. No wire schema was
invented. ANALYSER-EMIT-1 remains blocked outside owned paths: existing death loot
errors retain the corpse but discard the already committed entry list, successful
loot collapses `Committed` and `AlreadyCommitted`, and actual combat/item/session
commit hooks live in unallocated owners. A standalone emitter could not repair
those real consumers, so none is presented as completed emission. No commit,
push, ready transition, review trigger, merge or shared-path write was performed.

### Reviewed scheduler repair (successor authoring, 2026-10-16)

Independent static review of `df034dd61746996912e7f72997c1a80028812d81`
accepted one P2: a skipped startup check scheduled another full interval from
process restart and could postpone an already due durable refresh. This batch
explicitly returned to AUTHORING; the old commit/review/bundle remain retained.
The locked RR publication check now returns the remaining interval from the
oldest retained current category timestamp and database time. The actual job
anchors its next check conservatively before the pass, so connection/processing
delay can cause an early recheck but cannot postpone expiry. A successful reset
uses its returned durable interval; a failed reset preserves the pending
periodic deadline. A failed periodic pass retains the prior cut and backs off
one interval, avoiding an immediate retry loop.

Two added native PostgreSQL bodies exercise the actual `HighscoreWorldJob::run`
loop with genuine Root passes and real timers: startup near a retained cut's
expiry followed by a real SQL reset failure still publishes within the bounded
observation window; a failed periodic publication retains the previous cut and
does not spin after its failure trigger is removed. These seed only valid derived
headers under the unchanged SQL guards. A test-only completion counter exposes
pass completion, while actual durable generation advancement decides success;
conservatively early skipped passes are allowed. No fake clock, Root or Tokio
feature/manifests change is used. These regression bodies are authored but
unexecuted because the PC-global heavy-resource slot remains absent.

The same review batch expands actual-source evidence: native Charm unlock spends
five of 25 earned points while the highscore remains 25; account representatives
cover two accounts, different levels and absent-build vocation `none`; eleven
categories now have a separate two-real-Root concurrency/retention case; a missing
active Achievement catalogue key refuses the full cut and preserves every prior
category. The original nine-category concurrency fixture remains unchanged.
There are now 15 PostgreSQL case bodies plus 11 inherited ranking cases, without
new execution claims. Rust/PG qualification and all recorded shared integration,
role, concrete wire-layout and real analyser commit-point blockers remain open.


## Native hsextra fixture repair — 2026-10-16

The frozen `963c6ae8b181bfedaa91bb420c4d1e29bc28110d` native run passed
cases 00–06 and failed case 07
`account_representatives_and_earned_charm_points_use_native_sources` on
PostgreSQL 17.6 with `Unavailable(Unavailable)` (retained native case-07 failure log). This
repair explicitly returns the local candidate to AUTHORING; qualification of
the predecessor is not carried forward.

Source inspection identifies a fixture-ordering defect: `dataset` inserted two
administrative ranking roots without creation receipts before
`open_character_authority`. The latter runs the global
`verify_character_integrity` check, whose missing-creation-receipt predicate
returns exactly `CharacterAuthorityError::Unavailable(DurabilityError::Unavailable)`.
These source fixtures were never intended to prove admission or gameplay
authority. The repair completes the genuinely fenced Bestiary and Charm writes
before adding these administrative rows. Production code and integrity guards
are unchanged.

The same existing native case retains all eleven-category, common-cut, account
representative/filter/own-row and earned-Charm highscore assertions. It adds a
native `read_character_charm_progression` read at revision six asserting 25
points earned, five spent and 20 available; committed Bestiary receipt count
three and root revision six; and the exact authority rejection after projection
reads, proving the synthetic roots still cannot acquire authority. Moving the
source dataset later remains coherent: kill/Charm writes leave level and XP
unchanged, while the dataset explicitly restores each clone's root/progression
revision two and copies the unchanged native build state/receipt at revision
two. No artificial creation receipt or guard bypass is introduced.

Validation at authoring was source review and `git diff --check` only. No Cargo, build,
PostgreSQL run, PC operation, commit or publication was performed in this worker;
all added assertions remain unexecuted pending a new frozen candidate and fresh
native qualification. The root control plane owns independent review and gates.


### CP D1122/D1155: publishable implementation checkpoint

AUTHORING resumed for this task record; the Rust bytes of frozen runtime
candidate b83207bcc055290f3f46feaeb40d53abb524e837 are unchanged.
The concrete Unavailable(Unavailable) test failure was a fixture binding
error, repaired in b83207bc. Its separate source-bound evidence retains
26/26 selected native PASS (15 PostgreSQL17.6 and 11 ranking), the repaired
case07 PASS and independent source/runtime review PASS. No selected suite
is rerun or attributed to unexecuted package checks.

NOT_FULLY_QUALIFIED under CP D1122: remaining unfiltered literal-candidate
all-target Clippy, duplicate-source guard and complete package tests are
NOT_RUN. A bounded admission ended HOLD/postgres at 20:53:34Z without
creating attempt5. CP D1155 allows one new bounded attempt5, using the same
frozen b832 source and reviewed canonical V2/PG/stop-and-reap guards; no
new runner or hidden restart is authorized by this record.

CP6224947337 assigns migration0097 with roles/grants and snapshot spawn
to this PR. Test bodies must move to `tests/support/highscores_postgres_cases.rs`
and the canonical `character_authority_postgres.rs` root after #2198;
no Cargo target and no cfg(any()) in that final integration. Reset
notifications come from #2167. Shared file order remains
#2163 -> #2197 -> #2167 -> #2198 -> #2168 -> #2151 -> #2149.

HS-WIRE-1 gets cap9/cmd14, no state domain, subject to the required live
open-draft collision check before publication. Its layout/tags and
SNAPSHOT_EXPIRED registration remain to implement. ANALYSER consumes
existing receipts under cap10 without inventing a hook or statistics owner.
This checkpoint neither completes RANK-1 nor requests ready/merge.


## Current-source and recovered native evidence — 2026-10-17

This appendix preserves the historical record above and corrects its now-stale
execution statements. Current source is
`cfd881842c2593e2687aab22984761275ee1dc86`. It changes the pagination condition
and associated tests after historical
`b83207bcc055290f3f46feaeb40d53abb524e837`: a page greater than one with snapshot
token zero is refused. The historical statement that the b832 Rust bytes are
unchanged belongs to that earlier checkpoint.

### Two original current-head pagination regressions

The coordinator ran two original selected tests on PostgreSQL 17.6 through normal
Cargo and the existing `character_authority_postgres` target:

| Original selected case under `durability::highscore_snapshot::tests` | Observed result |
|---|---|
| `world_category_token_isolation_empty_world_and_query_boundaries` | PASS 1/1; 1.41 seconds, including the current page-2/token-0 refusal |
| `filtered_list_is_capped_after_filtering_and_keeps_global_rank_below_all_cap` | PASS 1/1; 3.67 seconds, including subsequent pages pinned to the captured snapshot token |

Each run selected exactly one original case, with zero failures or ignored
tests. `cargo fmt --all -- --check` also passed.

The isolated exact-head checkout added only
`pub(crate) mod highscore_snapshot;` to
`apps/game-server/src/durability/mod.rs`, whose original Git blob was
`dbae55905fa58f164b36f18d46c7661e218bdefa`. This was a **validation-only shared
module registration**, not a published runtime integration change. The original
production/test bodies, canonical fixture, Cargo configuration and migrations
were unchanged. A fresh dedicated Cargo target directory kept the source graph
coherent, and the coordinator verified source hashes and the one-line-only
overlay before and after. No forwarding runner, copied rlib, test-body rewrite,
schema overlay or stack-size override was used.

```sh
cargo test --locked -p oteryn-game-server --test character_authority_postgres durability::highscore_snapshot::tests::world_category_token_isolation_empty_world_and_query_boundaries --quiet -- --exact --test-threads=1
cargo test --locked -p oteryn-game-server --test character_authority_postgres durability::highscore_snapshot::tests::filtered_list_is_capped_after_filtering_and_keeps_global_rank_below_all_cap --quiet -- --exact --test-threads=1
cargo fmt --all -- --check
```

The native fixture used `OTERYN_TEST_POSTGRES_ADMIN_URL` with its unchanged
admin-URL guard on PostgreSQL 17.6. Current evidence is retained on Molehill-PC under
`/root/oteryn-six-pr-20261017/evidence-rank/`.

These results qualify the current pagination delta in that canonical Cargo
context. They do not qualify every RANK case, current-head all-target Clippy,
full package execution or runtime integration. The original public-outbox case
requires `apps/game-server/migrations/0091_public_projections.sql`, absent at
this exact head, and was outside the two-case selection. The inactive
`tests/highscores_postgres.rs` target is not counted as an executed suite.

### Historical attempt 5 is terminal

The retained 26 selected native passes (15 PostgreSQL 17.6 and 11 ranking cases)
belong to historical source
`b83207bcc055290f3f46feaeb40d53abb524e837`. They are not transferred to the
changed current source.

Recovered CP D1155 attempt-5 evidence supersedes the earlier statement that no
attempt 5 was created:

| Historical attempt-5 stage | Observed result |
|---|---|
| Literal and assembled source prechecks | PASS, exit 0 |
| Formatting and strict all-target Clippy | PASS, exit 0 |
| Duplicate-source guard | PASS, exit 0; 56 harness units and 11 allowlisted targets |
| Full package tests | Terminated with exit -15 under a resource Hold |
| Terminal outcome and source postflight | FAILED; postflight `NOT_QUALIFIED` |

The recorded Hold reason is `held peer owner PID absent`. The runner reaped
both children and ended at **2026-10-16T22:17:48Z**. The log stops during a
1,491-test group; no complete package PASS exists. The 26 selected results were
retained earlier evidence, not a new selected-suite execution in attempt 5.

Historical evidence remains under
`/root/oteryn-codex-20261016/rank-2168-963c6ae8/evidence/attempt-5/`.

| Evidence binding | SHA-256 |
|---|---|
| Runner manifest | `45279fbc91af649f2f9bdbae38726d65a515110764c94374679635057e20f909` |
| Strict Clippy log | `03273410add1753ba27bc82677a6eb937b6eeb453a35613a436b9fbd325173d2` |
| Interrupted full-package log | `edc9ef4ec0c3966b67368d99aaf205207ebcc73d504c1adba4836f9899102aa4` |

Attempt 5 is terminal. The earlier prospective allowance must not be interpreted
as permission to restart that historical runner or modify its guards or locks.
Any future broad qualification needs its own current candidate and applicable
resource/allocation gates.

### Remaining integration boundary

The one-line native validation overlay does not complete the CP D1155 shared
slot. Migration 0129, projection-role credentials/grants and snapshot spawn
remain allocated integration work. After #2198, the PostgreSQL bodies must use
`tests/support/highscores_postgres_cases.rs` and the canonical
`character_authority_postgres.rs` root; final integration must remove the
inactive `cfg(any())` arrangement. Reset notifications come from #2167.

The assigned order remains #2163 -> #2197 -> #2167 -> #2198 -> #2168 -> #2151 ->
#2149. HS-WIRE-1 still needs concrete layout/tags, `SNAPSHOT_EXPIRED`
registration and authenticated integration under capability 9 / command 14,
with no state domain and the required live collision check. Analyser integration
must consume the existing receipts under capability 10.

Current-head all-target Clippy, full package tests and complete native
runtime/E2E qualification remain open. This evidence correction leaves
`NOT_FULLY_QUALIFIED` and CP-only integration/merge authority unchanged.


## HS-WIRE-1 owned codec and native Rust qualification — 2026-10-17

This authoring batch starts at draft head
`14babd8f0ae95c55dd0395d63a3bbb5dd82d31d7`. It supplies the concrete layout and
tags owned by RANK under CP D1155 and accepted HIGHSCORES0 §6. The earlier
statements that the owned layout itself was blocked describe the historical
checkpoint; shared activation still requires the integration slot.

The two new owned files are `crates/protocol-oteryn/src/highscores.rs` and
`highscores_tests.rs`. The query carries category, base-vocation filter,
one-based page and opaque snapshot ID. The result carries category, snapshot
ID, persisted computation time, page, continuation flag, at most 50 rows and an
optional own row. A row carries global rank, Character name, the actual durable
vocation key, level and primary value. World and asking Character remain
session-derived. Skill/magic progress used to break ties stays private.

The codec validates the same semantic and size bounds on encoding and decoding,
including malformed/duplicate/unknown fields, numeric overflow, bounded strings,
and the page > 1/token 0 refusal. A filtered list is capped at 1,000 members;
global ranks and an own row outside the current page are retained. The owning
handler will return NOT_SUPPORTED and SNAPSHOT_EXPIRED through the existing
FND-02 error result, without a duplicate status field inside a success payload.

### Exact tested assembly

Normal Cargo qualification used an isolated checkout of the base above plus
the two owned files and one validation-only `pub mod highscores;` line in the
canonical protocol crate. That shared registration is held; it is not part of
this draft source publication. Original dependency sources, Cargo manifests,
`Cargo.lock`, test attributes and the ordinary crate test runner were retained.
No process-wide stack override was used.

| Tested source | SHA-256 |
|---|---|
| Owned codec | `4ff4acb418918648e0b1402f8364d9d68dfe0120404f18bef1a66e9bcf40e679` |
| Owned test leaf | `db618bd384884aaeed4256fcbc092f6bd865280e9b4600d50bd74fad83b4008e` |
| Validation-only protocol module root | `632ea8bda27a1503b5b6a97cd8852ee4956ce93eac590a84ab31da86238571d8` |
| Unchanged Cargo.lock | `a3b9800ad3199130b2ddc363b6b3972b1ecf119d22e595c243701fcd00f5a35e` |

```sh
cargo test --locked -p oteryn-protocol-oteryn --lib highscores::tests --quiet -- --test-threads=1
cargo clippy --locked -p oteryn-protocol-oteryn --all-targets --quiet -- -D warnings
cargo fmt --all -- --check
```

All **12 selected original Rust tests passed**, with zero failed/ignored and
242 unrelated cases filtered, at **2026-10-17T10:49:27Z**. Strict all-target
Clippy for the protocol package passed at **10:49:34Z**; workspace format check
passed at **10:50:05Z**. Source/build-input hashes matched before and after.
The tests include independent literal protobuf byte vectors, default and field
order equivalence, rejection paths, own-row/global-rank behavior and the maximum
constructed 9,874-byte result within the 9,878-byte leaf bound.

Evidence is retained on Molehill-PC under
`/root/oteryn-six-pr-20261017/evidence-continuation-reviewed-1017/`:
`results.json`, `rank-source-before.json`, `rank-source-after.json`,
`rank-wire-tests.log`, `rank-wire-clippy.log` and `rank-wire-fmt.log`.
The test log SHA-256 is
`df90274bb74ec76c20d4cfc7c7b9ff6080219af2088ec2f1ac6be4212d7886ef`.
Independent source review accepted the concrete codec and byte vectors before
execution. Formatting was the only subsequent source transformation.

### Integration still required

The matching `highscores_v1.proto` proposal and module mount remain an explicit
held integration packet. CP owns the shared IDL/registry/error/capability rows
and authenticated session handler slot under capability 9 / command 14. The
existing database qualification, migration 0129, projection-role grants,
snapshot spawn and reset notifications retain their separate gates. The
protocol checks above do not qualify the game-server all-target graph, every
ranking/PostgreSQL case or complete runtime/E2E delivery.

This draft remains partial, with no ready or merge action requested.


## Integration slot — 2026-10-18 (CP D1441, CP D1155)

CP D1441 opened the shared slot on draft #2168 after merging main. It installs the integration
the sections above held:

- [x] Migration 0129: snapshot tables, the NOLOGIN job role and its grants, the AFTER INSERT
  public-outbox snapshot trigger (0091 §11).
- [x] Snapshot job: one `HighscoreWorldJob` per World in the node serve loop
  (`node/serve.rs::highscores_loop`); immediate first check, then every 30 minutes from durable
  database time; a planned reset forces one complete cut.
- [x] Categories 1 to 11: nine native skill/experience lists, category 10 from the activated
  Achievement catalogue (each account's highest initialized Character; a bootstrap-only
  Character without a progression row never represents it), category 11 from the Charm point
  owner only once a per-race Charm point source is activated.
- [x] HS-WIRE-1: capability 9 offered, command type 14, errors 1200 `NOT_SUPPORTED` and 1201
  `SNAPSHOT_EXPIRED`, `HIGHSCORES0-RL-01..03`, `highscores_v1.proto`; the handler reads the
  admitted session's World and Character only. Live collision check: no open draft uses
  capability 9, command type 14 or errors 1200/1201.
- [x] PostgreSQL cases run in the canonical `character_authority_postgres` lane; the old
  `cfg(any())` target is removed.
- [x] End-to-end case `experience_gain_reaches_the_client_through_the_snapshot_job`: a real
  session-fenced experience award, the actual job (immediate pass, then a planned reset), and the
  client's encoded query answered by the dispatched handler with the ordered page and own row.
- [ ] ANALYSER-EMIT-1 (ANALYSERS0 is ACCEPTED). Not shipped: emission needs hooks in the combat,
  loot-settlement, progression and item-use commit paths (the loot settlement must return its
  committed partial list), which are owned outside RANK-1. Capability 10 stays registered and
  not offered, so the seam fails closed. This is the one remaining RANK-1 child.

Remaining dependencies (REPORT_ONLY):

- Planned World reset notifications (#2167, WORLD-RESET): the job's reset channel is held open
  with no sender until that owner lands; the 30-minute schedule runs meanwhile.
- Per-race Charm points in production (CHARM-6): no production source exists, so the job
  publishes categories 1 to 10 only and a category 11 query is `NOT_SUPPORTED` (1200) until
  `activated_charm_points` returns that source.

Status stays `NOT_FULLY_QUALIFIED` until ANALYSER-EMIT-1 lands; CP owns ready and merge.

2026-10-19 wiring cleanup: the legacy nine-category path and test-only wrappers are gone;
fixture entry points are `#[cfg(test)]`. Session resolution lives in
`highscore_snapshot::reply::answer_session_highscores`, with the transport delegating to it.
The new PostgreSQL case is
`session_resolution_rejects_another_world_an_unknown_session_and_an_unadmitted_session`.
The committed-session positive path through `answer_session_highscores` needs a real fresh
admission fixture (`postgres::fresh::Source`), which exists only in the `durability_postgres`
target, outside owned paths. Status remains `NOT_FULLY_QUALIFIED`.

2026-10-19 public projection alignment: the stored `filter` now uses the public §4.0 ids
(`OTERYN_GAME_PUBLIC_PROJECTIONS_V1` §11: 1 druid, 2 knight, 3 monk, 4 paladin, 5 sorcerer);
the HS-WIRE-1 query keeps its own numbering (`reply::QUERY_VOCATIONS`), so no wire change.
Open: HIGHSCORES-0 §5 keeps the global rank in every filtered list, while the public producer's
`encode_highscores` accepts only list-relative competition ranks, so filters 1..5 refuse to
publish. The fix belongs to the producer or the contract text, outside owned paths (owner
question).
