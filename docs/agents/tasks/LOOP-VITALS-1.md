# LOOP-VITALS-1

```yaml
task_id: LOOP-VITALS-1
title: "LOOP-VITALS-1: regeneration accumulator, food time, soul, stamina (skeleton)"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-vitals-1
issue: 1622
pr: 2196
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-VITALS-1 (rows 30, 32, 33); wave 2"
class: "hard (Character state)"
migration: yes
registry: no
owner: /root/vitals (family E; D1107 source authoring)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-17
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_vitals_soul.rs
  - apps/game-server/src/durability/character_vitals_stamina.rs
  - apps/game-server/src/durability/character_vitals_stamina_math.rs
  - apps/game-server/tests/loop_vitals_stamina.rs
  - docs/agents/tasks/LOOP-VITALS-1.md
slot_only_paths:
  - migration number
  - mod.rs (`tick_vitals` composition)
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "ITEM-USE-1c: MERGED #2082"
  - "CHAR-VITALS-1: MERGED #2045"
  - "LOOP-PROGRESS-1: BLOCKING: not merged (chain #3)"
  - "OFFLINE0 accepted on main e2da9805 via ACCEPT-BATCH; OFFLINE-1 integration remains pending"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Durable regeneration accumulator, food-time ingress, soul regen and soul orb, stamina.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-USE-1c | MERGED #2082 |
| CHAR-VITALS-1 | MERGED #2045 |
| LOOP-PROGRESS-1 | BLOCKING: not merged (chain #3) |
| OFFLINE0 (STAMINA-1) | ACCEPTED on main `e2da9805` via ACCEPT-BATCH; OFFLINE-1 integration pending |

**BLOCKING for runtime delivery:** LOOP-PROGRESS-1; OFFLINE-1 current marker/lease and build integration. Root has allocated the pure accepted arithmetic subset only. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/durability/character_vitals_soul.rs (CP-authorized bounded rebuild below)
- apps/game-server/src/durability/character_vitals_stamina.rs
- apps/game-server/src/durability/character_vitals_stamina_math.rs
- apps/game-server/tests/loop_vitals_stamina.rs
- docs/agents/tasks/LOOP-VITALS-1.md

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number
- mod.rs (`tick_vitals` composition)
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `tick_vitals` (l.782) / `observe_vitals` (l.727) via authority methods
- `gameplay_transport/last_position.rs` vitals projection
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] Regen survives restart; food time caps per Reference.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

DUR-02; CHAR-VITALS-0; Reference regen tables. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 LOOP-VITALS-1 (rows 30, 32, 33).

## Builder instructions

- Work on branch `skeleton/loop-vitals-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: pure stamina settlement arithmetic prepared; source-bound tests 8/0 and Clippy pass
status: authoring
branch: skeleton/loop-vitals-1
blocker: OFFLINE-1 current marker/lease and build integration; LOOP-PROGRESS-1; other full-task vitals work
next_action: root reviews local packet; CP qualifies and wires the owning settlement transaction
```

## Bounded authoring packet (2026-10-15, draft #2196)

Root allocated only the three exact paths above in `loop-vitals-1-allocation.json`.
Exclusive authoring checkout `/workspace/oteryn-prep/loop-vitals-1`, branch
`skeleton/loop-vitals-1`, unchanged initial HEAD
`bad34047869564dc00e23ddc75a3cdc20c377892`. No worker commit, push, lifecycle,
remote mutation or shared wiring edit is authorized.

### Accepted source and residue, before code

Read protected main `e2da9805d296a27b06b32335bf67394509d4910b` and its
ACCEPT-BATCH §1 item 5: OFFLINE-0 is accepted, superseding this skeleton's old
candidate gate. The exact arithmetic is OFFLINE-0 §3 (`stamina_minutes` 0..2520)
and §5 (offline duration capped 21 days; subtract 600 seconds; nothing below 180
remaining seconds; 180 seconds per minute up to 2340 and 360 thereafter).
Threshold wording applies to the post-delay duration, so the first possible
below-2340 gain is at total 780 seconds, and at/above 2340 at 960 seconds.
Crossing consumes the full 180-second periods below 2340 before 360-second
periods above it; no remainder is rounded up.

Main has no stamina arithmetic/runtime implementation (only a content rejection
fixture mentions stamina). Existing `spell/food_regeneration.rs` already owns
fed-tick regeneration and vocation rates. CHAR-VITALS-1 already persists HP/mana
in the last-position projection under CHAR-VITALS-0; neither is duplicated here.
CHAR-VITALS-0 does not accept durable food/regeneration accumulator storage.

### Approved implementation scope

A private arithmetic function validates the supplied stamina number and computes
one proposed stamina number from raw unsigned elapsed seconds, capped before
subtraction/division. It reads no clock, record, identity, database, actor or
account. The input is deliberately not a current-authority witness; output is
not a settlement receipt, write permission or proof that elapsed time qualifies.
Tests cover the delay/period boundaries, crossing 39 hours, 42-hour cap, invalid
stamina, maximum integer durations and an independent period-by-period oracle.

### cross_domain_findings / pending integration

The future OFFLINE-1 owner must independently qualify the latest build-carrying
logout marker of the immediately preceding lease, use database marker commit and
lease-acquisition times, clamp negative elapsed time to zero, and supply zero
after crash/skipped/old markers or first login. Same-GameSession reconnect settles
nothing. A single fenced/replayed marker transaction must apply this arithmetic
with current Character/session/lease/scope authority and the accepted build
revision/guard order before playability. None is reconstructed from these raw
numbers. World and Channel remain distinct.

OFFLINE-1, LOOP-PROGRESS-1, live hunting-clock consumption/PREY reads, checkpoint
flushes, training/pool settlement, migrations/guards, food ingress/accumulator
persistence, soul regeneration/orb, protocol and runtime composition remain
outside this preparation. No existing runtime or durable authority is activated.
The full-task restart acceptance criterion stays open. Root alone may publish
the reviewed local packet to the existing draft; CP owns eventual integration.

Durable source-bound harness/config/hashes/logs are preserved outside the
repository at `/workspace/oteryn-prep/loop-vitals-1-evidence/`.

### Actual local validation and limits

The tiny `loop-vitals-stamina-source-harness` Cargo package has no dependencies,
lib or bin. Its sole test target is the exact new root integration-test file,
which includes the exact arithmetic source by its real relative path. There is
no copied source, extracted facade, alternate implementation, stub authority or
server dependency. The root test imports no external/private library symbols;
normal Cargo test discovery is enabled in the server manifest. Future production
module registration remains untouched. This validates the real leaf sources,
not the original server package's complete build or runtime.

Actual isolated tests: **8 passed, 0 failed**. Harness all-target Clippy passed
with `--locked --offline --all-targets --quiet -- -D warnings` and the workspace
Rust/Clippy deny list. All Rust commands use Rust 1.94.0, the shared lock/target,
two jobs and `CARGO_INCREMENTAL=0`; no implicit server bin is linked. The exact
pre-test source/config hashes are preserved and verified unchanged after checks.
A period-by-period oracle independently consumes 180/360-second intervals rather
than repeating the closed formula. Table tests include first gains, boundary
neighbors, 39-hour crossing, exact 42-hour saturation, invalid state, cap/cap+1
and maximum integer durations. All valid starting values are checked for bounds
and monotonicity over increasing durations. Full recovery occurs before 21 days,
so cap-1/cap/cap+1 already saturate; the code explicitly caps raw duration before
arithmetic and maximum-integer tests verify safe bounded results.

Direct rustfmt checks of both owned Rust files and `cargo fmt --all --check`
passed. Governance validated 22 policy documents and 9 lanes; **124/124**
governance unit tests passed. Exact commands, final owned-source/unchanged-root
hashes and a reviewable
local patch are in `/workspace/oteryn-prep/loop-vitals-1-evidence/`. Full-server
checks, PostgreSQL, durable restart/replay and live gameplay E2E are not claimed.
No authority-consuming mutation exists in this subset: stale/mismatched leases,
marker provenance, reconnect/recovery and concurrent marker replay must be tested
at the future owning boundary; arithmetic fixtures cannot qualify them. No full
task completion, freeze, architect acceptance or publication is claimed.


## D1107 source authoring — 2026-10-16 (PARTIAL)

CP [D1107](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219643346)
allocates typed food/HPMP residuals with a receipt in the existing stamina leaf;
0094 is reserved after #2163. This supersedes the earlier pure-arithmetic-only
allocation/storage hold above; historical validation reports above retain their
original scope and do not qualify this material addition.

Exact active authoring paths are the four YAML paths. The additional
`character_vitals_stamina_math.rs` pure leaf was allocated by the family lead to
keep the existing eight standalone arithmetic tests compatible. The real native
owner re-exports that one arithmetic definition; no test crate includes an
inherent implementation of the foreign DurabilityRoot type. No shared
registration, migration, registry, admission/death caller or manifest was edited.

The real typed owner now provides actor-end residual save/read/reconcile,
current recovery/session/lease/scope/root checks, one CharacterRevision and
immutable v1 receipt in one transaction, exact/conflicting replay, all-three-field
storage and recovery-history verification. Remaining food time, HP fractional
regeneration time and mana fractional regeneration time restore together. The
native production module remains unregistered until its assigned composition
slot, so source authoring is not runtime delivery or proof of restart behavior.

D1108 owns the single physical build/marker/stamina writer and calls this leaf's
existing arithmetic inside that same fenced transaction, with independently
qualified FND clocks/predecessor facts. No second stamina table/receipt/clock
witness is introduced. Soul rate evidence is already available; soul/orb runtime
delivery remains open and is not falsely reclassified as a missing rate decision.

Prepared outside the repository: full 0094 DDL/guard union, exact authority
receipt UNION/history hook, death triple-reset hunk and private native PG test
proposal. 0094/source/shared hooks must form one reviewed integration bundle;
the authority writer coordinates with H/D1092, death reset follows #2193,
admission restore/capture follows ADMIT-CAP-1. The shared hooks are not installed.

Validation of this material candidate: Rust1.94 rustfmt and patch syntax only.
Native compile, Clippy, PG17.6, independent material review, CI and production
integration are pending. The earlier eight arithmetic PASS results do not count
as validation of the new durable owner. External evidence and exact frozen
working-tree manifest: `/workspace/oteryn-family/evidence/d1107-vitals/canonical-generation-2/`.

No commit, push, ready, merge, migration installation or complete-task claim.

## Lost Soul R5 payload rebuild — 2026-10-17 (NFQ / NOT RUN)

CP [6230877600](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6230877600)
authorizes rebuilding the missing unpublished Soul increment on existing draft
#2196. Base head is `9659ed8b4b0a9a5687e4ad5db1e9993f8da471f2`.
The lost source generation `72ed…` and its unexecuted native 18/15 recipe are
unavailable; no PASS for that generation is claimed. The existing published component's 11 PASS
results remain historical evidence of that component only.

New owned leaf `character_vitals_soul.rs` rebuilds only the bounded pure policy
and typed semantic kill-receipt participant. D1251 cap uses the real current
`PremiumGate` exclusively (100/200); the regeneration interval is 15 seconds
only with stored promotion and an open current gate, otherwise 120 seconds.
The accepted applied-kill facts produce no hook for zero XP, close the window
for positive XP below the calculation's level, and replace the window with 240
seconds for applied XP at least that level. Invalid level/negative XP are refused.
Facts and the resulting hook travel together as receipt participant data; neither
is a kill proof, current fence, Character binding or permission to write.

Five focused inline tests cover the premium/promotion combinations, expiry,
zero/sublevel/threshold/large XP and invalid inputs. They are **NOT RUN**;
this new leaf is deliberately unregistered, and no source qualification, native
compile, Clippy, PostgreSQL or runtime validation is claimed. No Cargo/heavy/native
or host work was run for this rebuild. Isolated Rust 1.94 leaf formatting/syntax,
diff whitespace and governance validation were checked; inline tests remain NOT RUN.

The full durable Soul window writer/schema, receipt integration, kill caller,
promotion caller, clock/scheduling, replay enforcement and runtime registration
remain explicitly **unrebuilt** and with CP's final shared slots. A historical
receipt must never reapply a window; the future owning transaction must bind and
enforce occurrence/Character/revision and independently prove current authority.
No new columns, migrations, clock producer or fake current-authority facts are
introduced. D1264 live stamina remains **NotWired x1**; no active hunting time is
invented. This source proposal is **NFQ**, not complete-task or runtime delivery.
Root reviews/publishes to the existing draft; worker made no commit or push.
