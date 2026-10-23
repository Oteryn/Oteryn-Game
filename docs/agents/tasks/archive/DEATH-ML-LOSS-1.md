# DEATH-ML-LOSS-1

```yaml
task_id: DEATH-ML-LOSS-1
title: "DEATH-ML-LOSS-1: flush before death; death receipt build fields (skeleton)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/death-ml-loss-1
issue: 1622
pr: 2193
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 DEATH-ML-LOSS-1 (rows 40, 41); wave 1 (migration chain #4)"
class: "hard (persistence)"
migration: no
registry: no
owner: "Codex family lead: Rozwój postaci i trening; D1105 source writer /root/death_review"
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-18
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/durability/src/durability/character_death.rs
  - crates/gameplay/domain/src/domain/death.rs
  - crates/gameplay/durability/src/durability/character_revision_sequencer.rs
  - apps/game-server/tests/death_ml_loss_*.rs (new)
  - crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs (CP D1440)
  - crates/gameplay/dispatch/src/gameplay_transport/training_save.rs (CP D1452: re-fence of a proven-uncommitted checkpoint)
slot_only_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/mod.rs (runtime caller; CP D1425-D1440 death hunks)
  - crates/gameplay/spell/src/spell/mana_training.rs (CP D1433 `rebase_after_death`; CP D1452 `refence_uncommitted_training_checkpoint`)
public_contracts: []
depends_on:
  - "CHAR-BUILD-1: MERGED"
  - "DEATH-1: MERGED #1278"
  - "LOOP-PROGRESS-1: MERGED #2192 (main 04e48240)"
  - "A13 ACCEPTED: ACCEPT-BATCH-2026-10-15 #2209 integrated (control-plane verified)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2193](https://github.com/Oteryn/Oteryn-Game/pull/2193). Training flush, death build-loss receipts and stale-fence retry/reconciliation repairs are merged. Evidence: `crates/gameplay/durability/src/durability/character_death.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

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
| LOOP-PROGRESS-1 | MERGED #2192 (CP D1439 check) |
| A13 ACCEPTED | ACCEPTED: ACCEPT-BATCH-2026-10-15 §1, #2209 main `e2da9805d296a27b06b32335bf67394509d4910b` (control-plane verified) |

**BLOCKING for full integration:** none. LOOP-PROGRESS-1 merged as #2192 (main `04e48240`); the single-slot drain it was to own is integrated in this PR (CP D1433-D1452 checkpoints below). A13 acceptance is effective; owned choreography preparation is allocated. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `crates/gameplay/durability/src/durability/character_death.rs`
- `crates/gameplay/domain/src/domain/death.rs`
- apps/game-server/tests/death_ml_loss_*.rs (new)
- `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs` (CP D1440: the composed-owner death case; D1440 names it under `spell/`, the file lives under `gameplay_transport/`)

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
last_progress: "CP D1452: a proven-uncommitted rejected flush checkpoint is re-fenced; live dispatch path verified end to end"
status: completed
branch: skeleton/death-ml-loss-1
merge_authority: CP_ONLY
blocker: null
next_action: "CP reviews the D1452 head and decides ready and merge"
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

## D1105 writer and sequencer handoff (2026-10-16)

This checkpoint supersedes the shared-writer blocker in the preceding domain-only
checkpoint. CP D1105 (comment `6219643346`, relayed by the family lead) selects E
#2193 as the sole physical author of `durability/character_death.rs`. F #2205 is a
read-only contributor until #2193 merges; its future hunk is not integrated here.
The authorized PR preimage is SHA256
`82e30eb2517c96f48444826521f9bc6b9a2f052febc237326178d59a4d6ff4e3`
(git blob `44379e7f59092e2ec1f4b1005fdd5f77baa1cb95`), identical at the original
PR head and the domain candidate `ec0a6978425e3aee50919a9a10b8385605f6c216`.
It is not the newer main preimage; main has not been merged without its slot.

The lead's explicit handoff and progression worker acknowledgement additionally
allocate only `RevisionSlot::commit_death_with_build` and its scoped argument-count
annotation in `character_revision_sequencer.rs`. Progression is paused as physical
writer while this hunk is authored. No progression atomic-batch hunk or broader
sequencer ownership is imported. Actual files changed in this increment are
`domain/death.rs`, `durability/character_death.rs`, this narrow sequencer hunk and
this task record; no module registration, manifest, migration, registry or content
pin is changed.

Implemented native behavior:

- The existing fenced death transaction locks and loads current durable build,
  applies the accepted ratio to all eight families, updates an existing build row
  only when a family decreases, and writes the existing 33 nullable receipt fields
  in that same transaction. Vocation is preserved. No build receipt is inserted
  inside death, and no build row is created at death.
- `CommittedCharacterDeath::build_loss`, SELECT/decode and reconcile carry the
  typed before/after payload. Partial NULLs, unsupported binding versions/sizes,
  increases and no-op non-NULL payloads fail closed.
- NULL/no-change receipts preserve exact v1 bytes. Non-NULL receipts use version2:
  version byte, formula digest32, digest32 of the existing v1 intent plus formula
  and complete build values. The schema is unchanged. Exact historical replay
  uses receipt values, never today's possibly later build state.
- `settle_player_death_with_training_and_build` holds one sequencer slot across
  the earlier training commit and the death; all initial/fallback death dispatches
  use the build-aware slot method. A retained death bypasses new checkpoint and
  current-formula processing. New occurrences require current content revision;
  legacy callers cannot silently skip loss on trained positive-loss deaths.
- The domain wrapper and the value-only family core share the same accepted
  native cumulative/re-level arithmetic. The core avoids durable-type identity
  substitution when existing aggregate tests compile native modules separately.

Final qualification evidence is kept outside the repository under
`/workspace/oteryn-family/evidence/d1105-death/freeze/`. The lead completed native
validation on the complete writer, sequencer and composed training/death candidate:

- Domain SHA256 `d32cebc93a9f5201bfdf3e204d35e2a6f410ffcc05d387fdda18b5573b3a4021`;
  writer SHA256 `74f1d1de4679dcd9d45f0eeabfc367cbe246c8c9c8cce81193b3f2eec34fcae1`;
  sequencer SHA256 `de49b1da8b2f3c86f918dc9f07f9b74bd46e4f630aabcb03b136361beb6aa88b`.
- `native-run/result.json`: six exact native tests passed, five actual PostgreSQL17.6
  cases and one binding case, zero skips, mandatory PG URL present. It binds server
  version170006 and ELF SHA256
  `1189d1c3998d4780a9122acb9b680d6d648ea9362891c3433c42f65918ba07bd`.
  Cases cover the sequenced training/death path, all-family receipt values,
  reopen/reconcile/replay, retained replay despite changed checkpoint/formula,
  legacy v1 adoption, receipt-insert rollback, current fences/content, conflicts
  and concurrent independent roots. Reopening a root is not an OS-process or
  PostgreSQL-server restart certificate.
- `full-package-result.json`: `cargo test --locked -p oteryn-game-server --quiet`
  passed, 57 reports, 9901 passed, zero failed, 37 ignored. This run had no PG URL;
  it does not qualify conditional PostgreSQL cases. The separate six-test native
  run above supplies the bounded PostgreSQL evidence.
- Formatting, strict all-target package Clippy, governance validation and all 124
  governance tests passed; their logs are retained beside these result manifests.
  Rust1.94 was used. Earlier pre-sequencer evidence remains preserved separately
  and is superseded for this candidate by the exact-source results above.

This final documentation update changes no Rust source or tested ELF. The final
card preimage/postimage and documentation diff are recorded separately under
`/workspace/oteryn-family/evidence/d1105-death/final-doc-update/`.

Remaining full-task boundary: the owning runtime must capture/drain actual live
pending deltas, retain/reprepare ambiguous checkpoints, and retire/apply its actor
state correctly, then call the new settlement from `mod.rs::commit_death` in its
allocated slot. This increment supplies a typed prepared-checkpoint seam, not
independent proof of actor drain. Production training is not enabled. LOOP-PROGRESS
ordering, final CP/CI and ready/queue/merge remain separate conditions. This task
is not archived or declared complete; no ready, merge or deployment is authorized.

## CP D1372 takeover checkpoint (2026-10-17)

CP took this draft over in D1372 (#2131 comment `6234726328`). The CP worker
merged protected main `ce7bccab973625d5d3d0d0c566426903229d2373` into the
branch with a merge commit (no rebase or force). Main changed no owned path, and
the merge had no conflicts. The PR had no review threads and no Codex review.
This checkpoint changes only this record. No Rust source changed after the D1105
validation.

**Owned-path state.** The writer, domain calculation, V2 binding, decode and
reconcile, the sequencer hunk and the inline PostgreSQL cases are done. No new
`tests/death_ml_loss_*.rs` target was added. The inline
`death_build_postgres` cases compile into the registered
`durability_postgres` target, which mounts `src/durability/mod.rs`. The
merge-queue PostgreSQL 17.6 lane runs that target with a database URL. A new
`tests/` target would run there only after it is registered in
`.github/workflows/merge-group-gate.yml`, which is not an owned path.

**Merge conditions found in review (not fixable in owned paths):**

1. *Legacy caller refuses trained deaths.* `prepare_death_build_loss` refuses
   (`InvalidInput`) a new death with no formula when XP loss is non-zero and the
   build is above the seed. Main's `gameplay_transport/mod.rs::commit_death`
   still calls `settle_player_death` without a formula. Main also enables owned
   training whenever the native generation has a `build_training` pin, and
   `content/spells.manifest.json` has one. After this PR merges, such a death
   maps to `DeathCommit::Unavailable`, then `SessionDeath::Retry`, and the
   player stays dead. The guard is deliberate: it fails closed instead of
   skipping loss. So the `mod.rs` caller must land in this PR's final slot
   commit, not later. The exact hunk is a PR comment titled "Shared-file
   proposal (NOT APPLIED)".
2. *Stale actor build after death.* A death that lowers the durable build leaves
   the actor's `LiveManaTraining` baseline (and its magic-level fact) at the
   pre-death values. The next prepared or retained checkpoint then carries a
   stale `before`. `save_spell_training` returns `Unknown`, and the actor-end
   release retries for ever. The training owner must re-base the live actor
   state from `CommittedCharacterDeath::build_loss.after`, and retire any
   pre-death checkpoint, before the caller in condition 1 is enabled. The files
   involved are `spell/mana_training.rs`, `gameplay_transport/training_save.rs`
   and the death caller. None of them is owned here.
3. *Drain before death.* The interim caller passes `training: None`. The
   runtime must force-save pending training (`save_spell_training(.., true)`
   not `Unknown`) before it settles the death. A single-slot drain plus death
   remains LOOP-PROGRESS-1 integration.

Acceptance criterion 1 is met at the writer level only (PostgreSQL cases). It
stays open until conditions 1-3 are integrated. Status stays `waiting`. This
record is not archived, and no ready, queue or merge action is taken.

## CP D1425 bounded checkpoint (2026-10-17)

MERGE_AUTHORITY: CP_ONLY. CP D1425 (#2131) granted one slot-only hunk in
`gameplay_transport/mod.rs` at the `commit_death` call site. The CP worker
merged protected main `f094e0320289975a220852ff60ca7baf6d234ff4` with a merge
commit (no conflicts, no rebase or force).

**Condition 1: done.** `commit_death` now calls the new owned
`DurabilityRoot::settle_runtime_player_death` with the active generation's
`native_gameplay().training_formula()`. When that formula's content revision
equals the request's progression content, the death settles through
`settle_player_death_with_training_and_build` (training `None`) and applies the
build loss. With no formula, or one for another content revision, it keeps the
legacy `settle_player_death`, which still refuses a trained death and never
settles it without its loss. This addresses Codex P1 thread
`PRRT_kwDOT8SzxM6thk84`. The D1105 test and every other test are unchanged.

New PostgreSQL case
`native_runtime_death_of_trained_character_settles_loss_and_respawns`
(inline `death_build_postgres`): it trains the Character, then shows the legacy
refusal with no formula or a stale one (writes nothing). It then settles through
the runtime entry with the current formula (build lowered to the seed, receipt
and pending respawn written), replays on retry, and consumes the pending respawn
under the next fence. It drives the durable entry `commit_death` calls, not the
composed Channel owner. A composed owner test (`respawn_after_death`) needs
unowned paths: a new gameplay_transport PostgreSQL fixture, its mount, and the
`bundle_gates_tests.rs` construction counts.

**Condition 2: blocked (unowned).** `PlayerSpellState` keeps `training`,
`training_checkpoint`, `training_payment`, `facts` and `owned_skills` as
`pub(super)` in `spell/cast.rs`. `respawned()` re-bases none of them. A new
spell-module method must reset `LiveManaTraining` and the magic-level fact from
`CommittedCharacterDeath::build_loss.after` and retire any pre-death checkpoint.
Until it lands, a trained actor's next training save after a lowering death
returns `Unknown` (`BuildStateMismatch`). Its death no longer blocks, but that
actor's later training saves and actor-end release retry.

**Condition 3: not applied.** A forced `save_spell_training(.., true)` before
the settle fits in `mod.rs`, but `settle_death` has no actor, and without
condition 2 a second death would hit the stale baseline and never settle. It
stays with the LOOP-PROGRESS-1 single-slot drain.

Validation on this head (Molehill-PC, shared PostgreSQL 17.6 via
`in-pg-netns.sh`): `cargo fmt --all --check` and strict all-target package
Clippy pass. Package lib tests without a PostgreSQL URL: 3399 passed, 0 failed,
29 ignored. `cargo test --lib death` with the URL: 128 passed, 0 failed,
3 ignored, including all six `death_build_postgres` cases.
Governance validation and its 124 unit tests pass.

Status stays `waiting`. The record is not archived. No ready, queue or merge
action is taken; CP alone decides merge.

## CP D1433 checkpoint (2026-10-18)

MERGE_AUTHORITY: CP_ONLY. CP D1433 granted one extra file,
`spell/mana_training.rs`, for one method. The CP worker merged protected main
`1f51f5df` with a merge commit (no conflicts, no rebase or force).

**Condition 2: done.** New `PlayerSpellState::rebase_after_death(&DurableBuildState)`
re-bases live training from the post-loss build, lowers the magic-level fact and
owned skills, and drops any pre-death checkpoint and payment.
`commit_death` returns `CommittedCharacterDeath::build_loss.after` (a new commit
or a replay); `respawn_after_death` rebases the actor with it before `respawn`
under the same lock. Codex P1 `PRRT_kwDOT8SzxM6tlZN0`.

**Condition 3: done.** `settle_death` now takes the actor. Inside the bound
session it first asks `reconcile_character_death`: an already-committed death
skips the flush (its pending respawn fences build writes). Otherwise it runs a
forced `save_spell_training(actor, session, true)`; only `Unknown` refuses the
death as `Unavailable` (retry). The flush and the settle take the sequencer slot
one after the other, never nested. Codex P1 `PRRT_kwDOT8SzxM6tlZNx`.

New PostgreSQL case
`gameplay_transport::death_ml_loss_postgres::train_lowering_death_respawn_then_train_save_commits`:
train and save, lowering death, respawn consumed, a stale (un-rebased) save is
refused and writes nothing, then after `rebase_after_death` the next training
save commits on the lowered build (never `Unknown`).

Validation on this head (Molehill-PC, shared PostgreSQL 17.6 via
`in-pg-netns.sh`, `src/lib.rs` touched before each run): `cargo fmt --all
--check` and strict all-target package Clippy pass. With the URL: the new case
passes; `--lib` filtered to `gameplay_transport spell:: durability::` 1520
passed, 0 failed, 12 ignored; `character_progression_postgres` 1218 passed;
`durability_postgres` 1312 passed, 0 failed.

Status stays `waiting`. The record is not archived. No ready, queue or merge
action is taken; CP alone decides merge.

## CP D1439 checkpoint (2026-10-18)

MERGE_AUTHORITY: CP_ONLY. CP D1439 settles skill-loss authority: server-side
`domain/death.rs` from the durable build, applied in the one death commit
transaction, before/after in the 0030 receipt columns under binding version 2,
formula from Reference (A13 §4.6, SKILLS-0 §3.6, Canary `deathLossPercent`).
The CP worker merged protected main `64d5e4da` with a merge commit (one
`mod.rs` tail conflict, both sides kept; no rebase or force).

Audit against the code:

- Build loss: done. `calculate_death_build_loss` takes
  `floor(cumulative * experience_lost / experience_before)` per family and
  re-levels from the floor (magic 0, skills 10); vocation unchanged.
- Receipt columns and binding version 2 with v1 replay: done (D1105).
- Live caller: done. `commit_death` settles through
  `settle_runtime_player_death` with the active training formula (D1425).
- Flush before death and rebase at respawn: done (D1433).
- Release path: done now. `settle_released_death` rebases a still-held actor
  on the death's post-loss build, so the release save and a resumed epoch
  never submit the pre-death build.
- Fenced-out flush (Codex P1 `PRRT_kwDOT8SzxM6to_WN`): done. A live death
  settles only on a `Saved` or `NotApplicable` flush; `FencedOut` keeps the
  actor dead like `Unknown`. Only the actor-end release, whose own training
  saves already accept a fenced-out session, settles past it from the durable
  build (`death_flush_permits_settle`, unit case
  `a_fenced_out_training_flush_refuses_a_live_death`).
- Blessings reduce the loss only through the held `game_character_blessings`
  rows. Promotion has no current owner (PREM-2b) and fails closed to
  `promoted_with_current_premium: false`.

New PostgreSQL case
`gameplay_transport::death_ml_loss_postgres::trained_death_lowers_magic_and_skills_and_both_survive_respawn_and_relog`:
a level 50 druid with magic 3 and sword 14 (and a flushed live try) dies
losing L = 112,900 of 3L experience; the committed build equals an
independent restatement of SKILLS-0 §3.6, the receipt row holds binding
version 2 and the magic and sword before/after columns, the respawn rebases
the actor, and a restarted root replays the same loss, admits the lowered
magic level and build, and commits the next training save on them.

Remaining: a composed-owner case through `respawn_after_death` needs the
private composed fixture in `monk_save_postgres_tests.rs` or a new `src` file
with a `bundle_gates_tests.rs` construction row; both are unowned.

Validation (Molehill-PC WSL, slot-run slot 1, `-j 4`, `touch
apps/game-server/src/lib.rs` before each cargo run, PostgreSQL through the
unchanged `in-pg-netns.sh` against the shared 17.6 server):
`cargo fetch --locked` ok; `cargo fmt --all --check` ok;
`cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings` ok;
`cargo test --lib -- death_ml_loss_postgres a_fenced_out_training_flush_refuses_a_live_death`
3 passed; `cargo test --lib -- gameplay_transport spell:: durability:: domain::`
1696 passed, 0 failed, 12 ignored;
`cargo test --lib -- boundary_unknown_monk_save_keeps_lease_until_store_recovers`
1 passed; `cargo test --test durability_postgres --test character_progression_postgres`
1223 + 1 + 1317 passed; `python3 tools/agents/validate_governance.py` passed.

Status stays `waiting`. The record is not archived. No ready, queue or merge
action is taken; CP alone decides merge.

## CP D1440 checkpoint (2026-10-18)

MERGE_AUTHORITY: CP_ONLY. CP D1440 allocates
`crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs` to this
task (D1440 names it under `spell/`; the file is under `gameplay_transport/`)
for the composed-owner case, and keeps the release-path behaviour.

Release path (kept, D1440 item 2): a death flush that returns `FencedOut`
during the actor-end release (`settle_released_death`, `releasing = true`)
still settles the death, priced from the durable build as it stands. Training
the fenced-out save could not write is not retried; the session's own
training saves already accept a fenced-out session the same way. A live death
(`respawn_after_death`, `releasing = false`) keeps refusing a `FencedOut`
flush, like `Unknown` (`death_flush_permits_settle`).

New composed-owner PostgreSQL case
`gameplay_transport::monk_save::postgres_tests::death_ml_loss_composed::composed_owner_death_lowers_rebases_and_trains_on_postgres`
(64 MiB stack thread, the existing `Dependencies` owner, no new `SeamRooms`
construction). The production manifest generation supplies the training
formula and the progression; only its training content revision is the
fixture Character's `content-1` (production pairs both as `s3b-content-1`).
The production admission step binds progression, the root takes a druid
choice, 98,800 experience (level 20) and a trained build (magic 3, sword 14),
and the live actor holds one unsaved sword try. A real lethal creature hit
records the death; `respawn_after_death` then force-saves the try, settles
the death with the generation's formula, places and consumes the respawn and
rebases the actor. Checks: the receipt has binding version 2, experience
before 98,800 and the level 20 loss of 10,780, a before build that holds the
flushed try and an after build equal to an independent SKILLS-0 §3.6
restatement (magic and sword both lower); the durable build is the lowered
one; no pending respawn remains; the live magic level is the lowered one and
the next training save commits on the lowered build.

Flaky monk case (CP D1440 follow-up):
`monk_save::postgres_tests::boundary_unknown_monk_save_keeps_lease_until_store_recovers`
asserted the release had not ended within a fixed 600 ms window and had
reached one monk save. Under parallel load the loop could need longer than
that to reach the save, so the case failed. It now drives the real release
until a second monk save attempt starts. That attempt happens only after the
first one ended `Unknown` and the loop retried instead of ending the lease. A
release that ends first fails the case. The 60 s deadline only bounds a hang.
Every other assertion is unchanged (revision 1 and a `Reconnectable` session
while the store is unavailable, then `Released`, revision 2 and one receipt
once it recovers).

Validation (PC, shared PostgreSQL 17.6, `-j 4`): `cargo fmt --all --check`
and `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`
pass. `--lib -- death_ml_loss` 3/3 pass. The parallel full `--lib` run has
3502 passed, 3 failed and 29 ignored. The new
composed case and the reworked monk case pass there. Three cases unrelated to
this diff failed in that run and pass on their own:
`party::shared_xp_postgres_tests::party_block_limit_refuses_the_101st_and_allows_an_existing_block_at_100`
(1/1 alone) and two `node::secure_file` cases (4/4 outside the netns wrapper,
whose root uid mapping trips their owner check). `--lib -- monk_save::postgres_tests`
in parallel passes 21/21, three runs in a row. `--lib -- gameplay_transport
spell:: durability:: domain::` 1697 passed, 0 failed, 12 ignored.
`--test character_progression_postgres` 1223/1223.
`--test durability_postgres` 1316 passed, 1 failed:
`migration_lock_interruption_releases_before_any_ddl_and_allows_fresh_retry`
timed out (`Elapsed`) under load and passes 1/1 alone. The governance
validator passes.

Status stays `waiting`. The record is not archived. No ready, queue or merge
action is taken; CP alone decides merge.

## Codex P1 4306765047 follow-up (2026-10-18)

Thread PRRT_kwDOT8SzxM6tp0K9, checked against D1440. The release path is
unchanged and correct: a `FencedOut` flush during the actor-end release
settles the death from the durable build, with no retry. A committed
checkpoint is already in that build. The live path did retry without end.
A forced save whose retained checkpoint committed, but whose session
connection, lease or scope generation moved before the post-write check,
returns `FencedOut` without applying the receipt
(`training_save.rs`, not changed here). Each later replay fails the same
immutable retained fence, so the actor stayed dead until its actor-end
release.

Fix (in `gameplay_transport/mod.rs`, the runtime caller this branch already
changes): on a live `FencedOut` flush, `adopt_committed_training_checkpoint`
reads the retained checkpoint's receipt with `reconcile_character_build`. It
applies the receipt only when the receipt is durable and the live actor is
still this session's under the current session, scope and owner authority.
It takes the Character's revision slot first, as the training save does, and
writes nothing. The checkpoint then retires, and the training banked after
it stays pending. A second forced save then prepares under the current
fence. Only that save's `Saved` or `NotApplicable` settles the death, so no
committed outcome and no in-memory successor is lost. With no retained
checkpoint, no durable receipt, or a moved or terminal session, the outcome
stays `FencedOut` and the live actor stays dead, as before.

The pinning case,
`death_ml_loss_composed::composed_live_death_applies_a_committed_but_fenced_checkpoint_on_postgres`,
runs the composed scenario with the death flush's own checkpoint for the
unsaved try retained under a stale lease fence, after the lethal hit (a
retained checkpoint refuses creature hits, so only the death flush itself
can leave one on a dead actor). The checkpoint's write is committed. Two
forced saves are checked to be `FencedOut` with the checkpoint still
retained. `respawn_after_death` then settles and respawns. The death
receipt's `before` build holds the try, and the live actor continues on the
lowered build.

Pre-existing and out of scope: the regular cadence save has the same stale
retained fence after such a commit. Its owner is `training_save.rs`, which
this task does not own.

Validation (PC, shared PostgreSQL 17.6, `-j 4`, D1445 scope): `cargo fmt
--all --check` and `cargo clippy --locked -p oteryn-game-server --all-targets
-- -D warnings` pass. `--lib -- death_ml_loss fenced_out_training_flush` 5/5
pass, including the new case. `--lib -- gameplay_transport` 709 passed, 0
failed, 6 ignored. With only the `mod.rs` fix reverted, the new case fails
with "the death settles and the actor respawns", so it pins the endless
retry.

Status stays `waiting`. The record is not archived. No ready, queue or merge
action is taken; CP alone decides merge.

## Codex P1 4307011952 follow-up (2026-10-18)

Finding (PRRT_kwDOT8SzxM6tqaKI): the actor-end release settled every
`FencedOut` death flush from the durable build. `save_spell_training_once`
also returns `FencedOut` for a pre-commit `AuthorityRejected`: a flush
whose retained checkpoint predates a reconnect or a generation change is
refused before any write. Its checkpoint and pending mana or tries then stay
only in memory. `commit_death` resolves the newer fence and commits from the
older durable build, and the rebase or the actor's removal discards the
pending progress.

Fix (in `gameplay_transport/mod.rs`): a fenced-out flush first applies a
retained checkpoint proven committed (`adopt_committed_training_checkpoint`)
on the release path too, then saves again. If the flush is still
`FencedOut`, the actor's in-memory training is read back.
`death_flush_permits_settle` lets the release settle only when no retained
checkpoint and no pending training remain, so the durable build already
holds everything and no write remains to land. Otherwise the release keeps
the dead actor and retries the flush under the session's current authority,
as for `Unknown`. The live path is unchanged: a fenced-out flush never
settles a live death. `training_save.rs` (not owned) is not changed.

The pinning case,
`death_ml_loss_composed::composed_release_keeps_a_death_whose_flush_was_rejected_on_postgres`,
retains the death flush's checkpoint under a stale lease fence without
committing it. It checks that the forced save is `FencedOut` and that
`settle_released_death` returns `false`. No death receipt is written, the
durable build is still the pre-try build, and the actor stays dead with its
checkpoint and pending training. The unit case
`a_fenced_out_training_flush_refuses_a_live_death` covers the new
`training_left` input.

Validation (PC, shared PostgreSQL 17.6, `-j 4`): `cargo fmt --all --check`
and `cargo clippy --locked -p oteryn-game-server --all-targets -- -D
warnings` pass. `--lib -- death_ml_loss fenced_out_training_flush` 6/6 pass,
including the new case. `--lib -- gameplay_transport` 710 passed, 0 failed,
6 ignored. A first run lost `kill_reward_e2e` to an unavailable progression
store while the shared PostgreSQL restarted; it passes alone and in the
re-run. With only the `mod.rs` fix reverted, the new case fails at the
`settle_released_death` assertion, so it pins the release settle.
`validate_governance.py` and the `tools/agents/tests` suite (124) pass.

Status stays `waiting`. The record is not archived. No ready, queue or merge
action is taken; CP alone decides merge.

## CP D1452 checkpoint: Codex P1 4307264455 and liveness (2026-10-18)

Finding (PRRT_kwDOT8SzxM6trAeP): a forced checkpoint the store refuses before
any write (`AuthorityRejected`, because the session's connection, lease or
scope generation moved after it was prepared) kept its immutable stale fence.
Every later save replayed that fence and was refused again, and
`adopt_committed_training_checkpoint` found no receipt. Neither the live nor
the release path could clear it, so the dead actor never respawned or
released. The root cause is in `save_spell_training_once`, which also runs
the cadence and actor-end saves.

Fix (CP D1452 paths):
- `training_save.rs`: on `AuthorityRejected` the save reconciles the
  checkpoint's occurrence. A receipt is history and takes the existing
  post-write authority check (and the adopt step). No receipt proves nothing
  committed. The save then resolves the current fence independently: build
  unchanged, Character revision bracketed, session not terminal, and runtime
  scope and generation equal to the Channel binding. It re-fences the
  checkpoint and returns `Unknown`, so the next attempt retries under the
  current authority. `RespawnPending` is still `FencedOut`. The rollback path
  (`CharacterRevisionMismatch`) shares the same fence helper, with unchanged
  behaviour.
- `spell/mana_training.rs`: the new
  `PlayerSpellState::refence_uncommitted_training_checkpoint` keeps the
  occurrence, request and formula, so a replay can never commit a second
  build. It refuses an unmoved fence, another Character or session, a lower
  revision or another retained checkpoint.
- `mod.rs` is unchanged: `Unknown` already keeps the actor dead for the next
  cadence or release retry.

Pinning cases (`monk_save_postgres_tests.rs`):
- `composed_release_refences_a_rejected_flush_then_settles_on_postgres`
  (it replaces `composed_release_keeps_a_death_whose_flush_was_rejected_on_postgres`
  and keeps all its assertions). The first release returns `false` with no
  death receipt, the durable build still pre-try, the actor dead, and the
  checkpoint (occurrence 93) re-fenced to the current fence. The retried
  release settles: the receipt's `before` holds the flushed try, and the
  durable build is the SKILLS-0 §3.6 loss of it.
- `composed_live_death_refences_a_rejected_flush_then_respawns_on_postgres`:
  the first `respawn_after_death` keeps the actor dead and re-fences. The
  next one flushes, prices the death from the flushed build, respawns,
  rebases, and the next training save commits.

Liveness through the real dispatch path (verified on this head):
- `connection.rs` calls `tick_vitals` on the actor cadence.
- `tick_vitals` calls `respawn_after_death`, then `settle_death`, then
  `flush_training_before_death`, then `commit_death`, then
  `settle_runtime_player_death` with the active generation's
  `training_formula()`.
- `content/spells.manifest.json` on main pins `build_training`, so production
  training and the build-aware death are both active.
- The actor-end release runs `settle_released_death` on the same chain.
- The runtime capture and drain, the `commit_death` integration and the
  actor rebase therefore already land in this PR (CP D1425, D1433, D1439,
  D1440 and D1452). They are not left with other slot owners.
- LOOP-PROGRESS-1 merged as #2192. No unmerged PR blocks this task.
- What remains is CP review, CI and the merge decision.

Validation (PC, shared PostgreSQL 17.6, `-j 4`): `cargo fmt --all --check`
and `cargo clippy --locked -p oteryn-game-server --all-targets -- -D
warnings` pass. `--lib -- death_ml_loss fenced_out_training_flush` 7/7 pass.
`--lib -- gameplay_transport spell::` 1099 passed, 0 failed, 12 ignored.
With only `training_save.rs` reverted, both new cases fail at the re-fence
assertion. `validate_governance.py` and the `tools/agents/tests` suite (124)
pass.

Status stays `waiting` for CP review and CI. The record is not archived, and
no ready, queue or merge action is taken.
