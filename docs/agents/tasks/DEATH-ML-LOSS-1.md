# DEATH-ML-LOSS-1

```yaml
task_id: DEATH-ML-LOSS-1
title: "DEATH-ML-LOSS-1: flush before death; death receipt build fields (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/death-ml-loss-1
issue: 1622
pr: 2193
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 DEATH-ML-LOSS-1 (rows 40, 41); wave 1 (migration chain #4)"
class: "hard (persistence)"
migration: yes
registry: no
owner: "Codex family lead: Rozwój postaci i trening; domain worker /root/death_review"
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-16
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_death.rs
  - apps/game-server/src/domain/death.rs
  - apps/game-server/tests/death_ml_loss_*.rs (new)
slot_only_paths:
  - migration number (chain #4, nullable build columns on the death receipt)
public_contracts: []
depends_on:
  - "CHAR-BUILD-1: MERGED"
  - "DEATH-1: MERGED #1278"
  - "LOOP-PROGRESS-1: BLOCKING: chain #3 not merged"
  - "A13 ACCEPTED: ACCEPT-BATCH-2026-10-15 #2209 integrated (control-plane verified)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Training/build progress is flushed before a death and the death receipt carries the build fields (A13 §4.6); this unlocks production training.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CHAR-BUILD-1 | MERGED |
| DEATH-1 | MERGED #1278 |
| LOOP-PROGRESS-1 | BLOCKING: chain #3 not merged |
| A13 ACCEPTED | ACCEPTED: ACCEPT-BATCH-2026-10-15 §1, #2209 main `e2da9805d296a27b06b32335bf67394509d4910b` (control-plane verified) |

**BLOCKING for full integration:** LOOP-PROGRESS-1. A13 acceptance is effective; owned choreography preparation is allocated. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/durability/character_death.rs`
- `apps/game-server/src/domain/death.rs`
- apps/game-server/tests/death_ml_loss_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (chain #4, nullable build columns on the death receipt)
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `durability/character_death.rs` `commit_character_death`
- `gameplay_transport/training_save.rs` (flush)
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] Death after training loses the Reference share; receipt carries the build fields; reconcile after restart.
- [x] Builder confirms the residue against the allocated base before code; exact findings below.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

A13 §4.6; DEATH-0. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 DEATH-ML-LOSS-1 (rows 40, 41).

## Builder instructions

- Work on branch `skeleton/death-ml-loss-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: accepted SKILLS-0 domain loss calculation prepared; durable writer remains pending
status: waiting
branch: skeleton/death-ml-loss-1
blocker: CP shared character_death.rs author/preimage with DEATH-COMPLETE-1; LOOP-PROGRESS-1; runtime integration
next_action: qualify the domain candidate; CP allocates the shared writer and native caller slot
```


## Local choreography preparation (2026-10-15, draft #2193)

- Exclusive allocation: `skeleton/death-ml-loss-1` at
  `b0e2210fb1ff2017a3116e616aecbb28303cd566`; actual writes are this record
  and `durability/character_death.rs` only. Deterministic tests live beside the
  private implementation in the owned source, avoiding a public API solely for tests.
  No domain, shared build/sequencer, wiring, migration, registry or content edits.
- Effective A13 §4.6: pending training uses `commit_character_build` in an earlier
  transaction; ambiguous flush reconciles before death takes its revision/binding;
  death has exactly one death receipt and no build receipt in its transaction.
  This preparation implements ordering, not the full loss/production training gate.
- Existing native seams reused: `RevisionSlot::commit_build`,
  `DurabilityRoot::reconcile_character_build`, the existing Character revision
  sequencer and death writer. The old `settle_player_death` delegates its unchanged
  settlement/fallback body to a shared in-slot method. A new callable
  `settle_player_death_with_training` holds that same slot across flush and death.
- A retained death receipt skips training and follows the existing exact replay
  binding. A new death validates the owner-prepared checkpoint's character/session,
  connection/lease/scope generation and formula digest. The prepared build revision
  is preserved even if the slot cursor has moved: stale checkpoints fail, never
  silently rebase. Receipt identity, intended before/after/cause/occurrence/prune,
  original revision and exactly-one successor revision are checked as history.
- Unknown build outcomes reconcile by occurrence and must then exact-replay through
  the real writer to compare the complete formula/revision command binding and
  advance the slot cursor. Unresolved, stale, conflicting or failed flushes stop
  before death. The death fence's revision is selected only afterward; its other
  fields remain supplied intent, with independently current DB fences enforced by
  the existing writers. No receipt or equality helper is current authority proof.

### Exact integration findings (REPORT_ONLY)

1. **Do not duplicate schema.** Migration
   `0030_character_build_state.sql` already has all 33 nullable death-build columns,
   all-or-none, eight-family direction/bounds checks, build-chain/row guards and
   revision integrity support. The skeleton's assumption that the columns still
   need adding is stale. Reuse and verify those actual guards; any additional
   OFFLINE-0 stamina/pool/offline-skill fields require their owner's integrated
   schema, not invented columns here. Existing migrations remain immutable.
2. **Loss and durable codec remain unimplemented.** The death calculation still
   passes `skill_progression: ()` and `magic_progression: ()`; generic progression
   copies them unchanged. `CommittedCharacterDeath`, receipt SELECT/decode and
   INSERT/binding still carry no build-loss payload. Domain/death provides XP/item
   loss, not a qualified all-family cumulative build-loss calculation. No loss
   share, formula or authority was invented in this slice.
3. **Inseparable writer bundle still needed.** With the owning loss calculation,
   load the latest durable build under the death root lock, write before/after
   into the one death receipt, update the existing build row (never insert at
   death), preserve vocation, and use the death occurrence/revision in the same
   transaction. Extend decode/reconcile and use a new binding version for non-NULL
   fields while retaining exact v1 bytes/replay for old/NULL receipts. No build
   receipt is allowed in that transaction. Verify against the actual 0030 guards
   and PostgreSQL 17.6 restart/replay/rollback qualification at integration.
4. **Runtime owner must capture/drain, not supply a bool as authority.**
   `gameplay_transport/mod.rs::commit_death` still invokes the old settlement path.
   `training_save.rs::save_spell_training` and
   `spell/mana_training.rs::RetainedTrainingCheckpoint` own pending checkpoint,
   formula digest, receipt application and successor deltas. Adapt those actual
   structures, independently refresh session/actor/lease/scope, force/drain all
   pending mana/tries (and accepted OFFLINE-0 state when available), and retain
   unknown deltas through retries. The new seam accepts one prepared checkpoint;
   `None` is only the caller's no-pending claim and does not qualify a drained actor.
   It does not admit a supplied checkpoint as independently current authority.
5. **Lifecycle/revision edge cases need actual owner integration.** A build flush
   requires initialized progression and refuses an earlier unconsumed respawn.
   Existing death settlement initializes/consumes those only when committing death;
   the producer must settle these prerequisites before preparing a training flush.
   A stale checkpoint stops and returns to existing training retry/reprepare rules.
   LOOP-PROGRESS must integrate force-save/death ordering without acquiring the
   same sequencer slot twice or allowing new live deltas between drain and death.
   This slice does not wire a runtime consumer or enable production training.
6. **Acceptance scope:** A13 and OFFLINE-0 are accepted through #2209. The readonly
   SKILLS-0 brief still has a CANDIDATE header; this change reuses the already
   present eight-family native build types and migration without asserting a new
   skill-loss amendment acceptance or changing any numerical rule. The control
   plane must resolve the effective skill-loss authority in its integration packet.

### Validation and evidence limits

- Whole registered native source: `cargo check --locked -p oteryn-game-server
  --lib --quiet` (no bin link); all Rust compilation uses the shared lock/target,
  jobs=2 and incremental=0. A pre-existing vendored Tokio missing-docs warning is
  recorded separately from package validation.
- Twelve deterministic source tests cover prepared identity/provenance mutations,
  stale revisions, exact retained replay, ambiguous commit/reconcile/replay,
  unresolved/error paths, binding conflict and committed-revision mismatch.
  Their actual helper/test sections are extracted with line/hash manifests and
  compiled against real native API types; no DB/authority implementations are stubbed.
- A focused negative control disables only prepared-revision matching in an evidence
  copy. That mutant fails; unchanged production helpers pass. It proves that the
  stale-binding regression detects that missing check, not PostgreSQL/authority.
- These tests do not execute the real death SQL transaction, qualify current session
  evidence, prove restart durability, or prove runtime death replay skips live
  capture. Those boundaries await actual PostgreSQL/runtime integration.
- Durable commands, raw logs, original harness-setup failure, source hashes and final
  exact counts are under `/workspace/oteryn-prep/death-ml-loss-1-evidence/`.
  The initial harness compile used an unavailable Tokio test macro; tests were
  corrected to ordinary immediately-ready future polling before final validation.
  No commits, pushes, external review, ready/merge or queue action were performed.

- CP freshness note: protected main advanced to `3ef900c1` (LOGOUT-WIRE #2127 and
  U-CC9 documentation) during preparation; the allocated draft is not merged or
  rebased. Revalidate force-save/death/terminal-release ordering and all shared
  consumers against then-current protected main at the slot. In particular, the
  owner must retire/apply the retained checkpoint without later replaying live
  deltas on top of death-lowered state; this one-checkpoint seam does not perform
  that actor-state lifecycle operation.

## Domain loss increment (2026-10-16, family handoff)

- Admission: live draft #2193 on `skeleton/death-ml-loss-1` was refreshed at
  `a224a280517aff801aebd65060c6d3df1d36535a`; protected main was
  `24fc332f256638e3b784bff1b3c85b18e2665a76`. The direct owner handoff assigns
  this family; there is no competing branch writer. Actual changed files are
  `src/domain/death.rs` under this app and this task record. Shared
  `durability/character_death.rs` is held pending one CP author/preimage with
  #2205; its broad historical owned-path entry is not an independent write grant.
- SKILLS-0 is ACCEPTED on that main through ACCEPT-BATCH-2026-10-15 §6
  (#2161, `b62a5595b6be426dd4c9cc016c9b3883e9193c9d`). This supersedes the
  CANDIDATE statement in historical finding 6 above. A13 acceptance also remains
  effective. No additional owner-approved loss rate is needed for this increment.
- `calculate_death_build_loss` implements A13 §4.6 and SKILLS-0 §3.6 using
  the actual capped XP loss divided by XP before death. It reuses the native
  `BuildFormula`, cumulative-progress saturation and re-levelling; keeps vocation;
  validates loss bounds and normalized reachable progress; and keeps exact values
  for zero XP, zero loss and rounded-zero loss. It neither resolves current
  authority nor writes a receipt. Formula/content currentness stays with the writer.
- Eight new native inline tests exercise all families, per-family flooring and
  de-levelling, seed floors, the actual capped death XP outcome, invalid inputs,
  invalid requirements, saturation and signed-storage limits. They are arithmetic
  fixtures, not additional Reference rates or PostgreSQL/restart evidence.
- Historical finding 2 is narrowed: the domain calculation now exists; the live
  writer still supplies `skill_progression: ()` and `magic_progression: ()`.
  Before/after build payload, binding-version compatibility, INSERT/decode/reconcile,
  the same-transaction build-row update and PG 17.6 death-after-training,
  restart/replay/rollback tests are still missing. Existing 0030 nullable columns
  are reused; no new loss-column migration is proposed.
- Runtime capture/drain and `mod.rs::commit_death` integration remain with their
  allocated owners/CP slot. Production training stays disabled. This is a partial
  increment, not completion of the full accepted task or permission for ready,
  queue, merge or deployment. Re-entry is triggered by CP naming the shared-file
  author/preimage and supplying the runtime integration slot, plus the existing
  LOOP-PROGRESS chain condition. No background continuation is asserted.
