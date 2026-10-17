# LOOP-PROGRESS-1

```yaml
task_id: LOOP-PROGRESS-1
title: "LOOP-PROGRESS-1: level-up vitals/capacity, vocations, skill gain, magic level (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-progress-1
issue: 1622
pr: 2192
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-PROGRESS-1 (rows 37, 38, 40, 41); wave 1 (migration chain #3)"
class: "hard (Character writes)"
migration: yes
registry: no
owner: house_runtime_1a (owned preparation)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-16
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/attack.rs
  - apps/game-server/src/domain/progression.rs
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/gameplay_transport/character_progression_binding.rs
  - apps/game-server/tests/loop_progress_*.rs (new)
slot_only_paths:
  - migration number (chain #3)
  - mod.rs
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "GAME-CHAR-CMD-0: MERGED (CHAR-CMD-ACCEPT-1 #1980)"
  - "CHAR-BUILD-1: MERGED"
  - "W2b: MERGED #2109"
  - "DAWNPORT-1: BLOCKING: not merged (chain #2)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Level-up updates vitals and capacity per vocation; vocations end to end; skill gain on hits; magic level. Production training stays disabled until DEATH-ML-LOSS-1 merges.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| GAME-CHAR-CMD-0 | MERGED (CHAR-CMD-ACCEPT-1 #1980) |
| CHAR-BUILD-1 | MERGED |
| W2b | MERGED #2109 |
| DAWNPORT-1 | BLOCKING: not merged (chain #2) |

**BLOCKING:** DAWNPORT-1. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/domain/progression.rs`
- `apps/game-server/src/durability/character_progression.rs`
- `apps/game-server/src/gameplay_transport/character_progression_binding.rs`
- apps/game-server/tests/loop_progress_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (chain #3)
- mod.rs
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/character_progression_binding.rs`
- `durability/character_progression.rs`
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] Level-up vitals/capacity match Reference per vocation.
- [ ] Training flag default off in production.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

A13; SKILLS-0; Reference vocation tables. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 LOOP-PROGRESS-1 (rows 37, 38, 40, 41).

## Builder instructions

- Work on branch `skeleton/loop-progress-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: native committed-XP level-up maxima adapter prepared locally
status: waiting
branch: skeleton/loop-progress-1
blocker: DAWNPORT-1
next_action: native qualification, then CP/architect review and dependency integration slot
```


## Owned preparation (2026-10-15)

Residue checked against protected accepted main `e2da9805d296a27b06b32335bf67394509d4910b`
and this draft's exact base `69bb29ef934a09c4f953f5c9773c12d0a925e5a7` before implementation.
Existing `domain/progression.rs` owns XP-to-level calculation; `character_progression.rs`
already commits/reconciles XP with one Character revision. `character_build.rs` owns build
conversion and W2b's `spell/mana_training.rs` / `spell/hit_training.rs` own training and
checkpoints. Those writers and production training flags are unchanged. The bounded residue
is effect preparation for a committed level increase, identified by plan row 37.
Accepted A13 Character Build State §§4.1–4.6 (effective through ACCEPT-BATCH #2209) does not
transfer build authority to this adapter; SPELL-D5's existing native cast facts remain the
vocation/HP/mana projection owner. Later protected main `3ef900c1` adds shared logout/admission
changes; this draft is not merged/rebased and CP must revalidate those consumers at integration.

`domain/progression.rs` adds only concrete `LevelUpVitalMaxima` data (level/maxHP/maxMP)
and checked maximum differences. These values do not qualify a source or select a curve.
`character_progression_binding.rs` binds the existing native `ExperienceCommitOutcome` to
independently supplied pre/current fences and an owner-projected after revision/maxima pair.
`AlreadyCommitted` emits no effect; a fresh same-level award requires no after projection.
A level increase yields checked gains and expected receipt/fence bindings. Current pools
are never refilled, durable state is never written and no authority is granted.
No XP formula, vocation curve, build conversion, skill/magic accumulation or writer is copied.

The pre-award maxima/fence must be retained from the independently admitted actor;
the current fence must be independently refreshed. Neither may be reconstructed from a
receipt. The binder checks Character identity, session, connection generation, Character
lease, distinct WorldId/ChannelId, scope generation and exact successor Character revision.
The supplied after revision and level must match the receipt, and maxima cannot decrease.
These are binding/arithmetic checks, not proof of the supplied maxima's provenance.

**CP hook still required:** `spell_character_facts::load_character_cast_facts` brackets existing
build/progression reads with root revisions. The caller must project its Ready result to
`LevelUpVitalMaxima`, retaining its returned revision and checking unchanged vocation,
magic level and soul maximum against the admitted actor. Unavailable/no-vocation/mixed facts
must refuse that hookup. The binder itself cannot independently make those checks.
An attempted direct private-loader import into the binding was removed before qualification:
`tests/support/character_progression_admission_postgres_cases.rs` path-includes that entire
binding with public durable/domain owners only. Preserving that real consumer rules out
private spell imports here. No cfg hiding, replacement types, new module or root wiring is used.
The future installer must atomically revalidate session/lease/scope/Character authority at
mutation; a prepared plan cannot substitute for current liveness.

Capacity remains pending its owning accepted native source/unit and absolute capacity facts;
the parallel ITEM-LOOP preparation likewise admits no unit conversion/base/vocation formula.
This patch does not promote candidate capacity totals or create an authoritative progression
snapshot. Unclassed actors, vocation transitions, native-loader qualification, durable vitals
adoption and capacity integration remain owning work; this subset does not establish LOOP DONE.

Eight inline native-library unit tests exercise actual receipt/fence binding, each replaced
fence dimension, replay, same-level awards, missing/wrong-revision after data, level/maxima
mismatch and malformed/overflowing receipts. Four external tests import the actual public
library and cover checked differences without copied source. Their maxima are arithmetic
fixtures, not curve/current-authority qualification. The existing standalone progression
PostgreSQL wrapper must compile against the whole changed binding. No production mutation
boundary is added: time, lock/concurrency and PostgreSQL commit qualification remain with the
future integration and existing writers, rather than being claimed by these pure tests.

Evidence is retained outside the repository in `/workspace/oteryn-prep/loop-progress-1-evidence`.
The first native library attempt encountered 34 missing sparse-checkout include errors;
that preserved setup failure is not product RED or a test pass. Root restored exact-HEAD
inputs before the corrected native run. Final results/source hashes are recorded in the packet.
Full acceptance remains pending except the completed pre-code residue check. DAWNPORT/chain #3,
capacity ownership, native loader hookup, atomic application and durable vitals/training
integration, exact-head CP/architect review and slot-only wiring remain required.

Observed local validation: native library filter `level_up_tests` passed 8 tests; the actual
`--test loop_progress_level_up` target passed 4 using the public native library. Existing
`cargo check --lib --test character_progression_postgres` passed, preserving the standalone
consumer; native `cargo clippy --lib --test loop_progress_level_up -- -D warnings` passed.
The executions used the retained pre-annotation binding. Its only final Rust delta is four
scoped `dead_code` attributes explaining the pending CP slot, proven by an exact byte-level
strip comparison; final native Clippy checked that annotated source. Both source manifests
and the execution artifacts are retained; this is not a claim that the earlier test binary
was rebuilt from the final annotated source. No PostgreSQL execution is claimed.


## D1102 independent activity handoff (2026-10-16)

CP material assignment2131/comment6219643346 and the direct E–D handoff authorize
E as the sole native attack/heal activity source author. This increment contains only
`gameplay_transport/attack.rs` and this task record, isolated from the unpublished
whole-kill settlement and actor-facts proposals. The other local source preparations
are not part of this increment. The existing draft remains partial.

The native `ChannelAttackStates` API supplies
`prepare_shared_experience_record(runtime, actor, session) -> bool`,
`shared_experience_active(runtime, actor, session, now: SemanticTimeMicros) -> bool`,
and `record_committed_shared_experience_batch(runtime, content, batch, receipt, party) -> bool`.
The last method takes actual native `OwnerCombatBatch`/`CombatBatchReceipt` and optional
transaction-qualified `SourcePartyWorld`; it does not mint membership authority. Native
positive attacks against a currently qualified aggressive non-summoned creature record
activity. Spell/heal installation must call the committed-receipt method under the same
runtime/states/attack guards; its foreign caller hunk remains an explicit proposal.
Current native owner/session/Character lease and control facts qualify reads. Backward or
future times refuse; activity expires after120 seconds. Actual membership/range/level and
leader-PZ qualification stay with D at the kill. No whole-kill XP transaction, journal,
claim table, migration, capacity/maxima column, registry or shared wiring is published here.

D's actual read-only `PartyXpActivity::recorded_activity` source is
#2172@b856170ed896ab9b8d8cdbbd554a9c1aa49d3850, file SHA256
`e85a7d6d54fdf8a5870bb88c7c7273687ca136853dff8a38da59bca8ea019238`.
It copies retained activity data and adds no current authority or clock renewal. D's getter
is delivered; remaining whole-kill producer/FND/activation ports are separate.

Native independent-candidate qualification and the exact published head are recorded with
the handoff on PR2192 after checks. Two exact native activity tests cover native lease/session
turnover, the120-second boundary, future refusal and out-of-order events. They do not claim
a complete E–D gameplay/combat/heal or whole-kill settlement qualification.

## Recovery rebuild for draft backup (2026-10-17)

CP #6230877600 authorizes rebuilding the missing Family E activity increment from
this draft's current GitHub head, `8b79b5d31b20d8143705785838786bfcc1d8340a`,
after confirming that the original source copies no longer exist. The owner
explicitly requested backup to this existing draft. This increment is new source,
**NFQ / unqualified WIP**. It does not reproduce the lost 25 postimages or claim
tree `baa19543958c9cf8d55c51f5c5c67ed77178f868`. The recovered R6 archive contains
recipe metadata only; its historical tests, approvals and PASS evidence do not
qualify this rebuild. Earlier observations above remain historical evidence only.

The rebuilt `OwnerCombatBatch.now_us` carries an optional original native semantic
time captured before millisecond conversion. Native producer paths and due timer
callbacks supply their actual semantic event clock; clone/retry paths preserve it.
Physical canonical command identity and deferred provenance bind its presence and
exact value. A supplied value inconsistent with `now_ms` refuses staging; absence
remains compatible with existing physical callers but grants no shared-XP activity.
Native paid-cast finalization checks the exact semantic time as well as milliseconds.

The player-vitals preflight records each actual heal's staged before/after HP,
including capped/zero transitions. The sealed proof copies these facts into the
applied physical receipt and its retained replay record before installation, with
no new allocation in phase 2. The native cast path preallocates attack state before
SQL and records activity from the actual applied installation receipt under its
existing runtime/player/attack guards, using its retained batch and qualified party
capture. Replay receipts remain unapplied and cannot renew event time. Positive
actual HP gain, rather than requested magnitude, determines heal eligibility.

The typed activity observation exposes `original_event_at` and `observed_at` only
after current runtime, exact actor/session, control and Character lease checks.
Reading never renews the event. Future observations refuse; the existing 120-second
activity predicate uses the retained microsecond event. This data gives no current
party membership, kill eligibility, whole-kill producer or settlement authority.
No missing D producer, whole-kill transaction, migrations, shared registration,
contracts, unrelated Family code or private source/fixture overlays are added.

New focused tests cover submillisecond command substitution, missing timestamp
substitution, retained exact replay, inconsistent millisecond binding and deferred
provenance substitution. Existing native activity tests additionally assert repeated
observations retain the event time after expiration; player-owner tests assert
actual HP receipt facts survive unapplied replay. These tests are authored but
**not executed**. Only the isolated official Rust 1.94 `rustfmt` parser/format check
and `git diff --check` have run locally. No Cargo, native, PostgreSQL or heavy gate
execution is claimed. Independent review and all required exact-head native
qualification remain pending; the PR must remain draft and must not merge.
