# QUEST-RT-1

```yaml
task_id: QUEST-RT-1
title: "QUEST-RT-1: quest runtime on the bundle, chest rewards, cap 16 (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/quest-rt-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 QUEST-RT-1 (rows 86, 87); App. 7 QUEST-RT-1; wave 2"
class: "hard (rewards are mints)"
migration: maybe
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/quest/
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/tests/quest_rt_*.rs (new)
slot_only_paths:
  - capabilities.rs (cap 16)
  - connection.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - content quest pins
public_contracts: []
depends_on:
  - "QUEST-LOWER-1: MERGED #1727"
  - "ITEM-LOOP-1: BLOCKING: not merged"
  - "MAP-PICKUP-1: BLOCKING: not merged"
  - "QUEST-XP-ADMISSION-1: BLOCKING: not merged"
  - "QUEST_GATE0, QUEST_STATE0 ACCEPTED: effective ACCEPT-BATCH #2209 (control-plane verified)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Quest runtime on the bundle World, chest rewards, capability 16 offer.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| QUEST-LOWER-1 | MERGED #1727 |
| ITEM-LOOP-1 | BLOCKING: not merged |
| MAP-PICKUP-1 | BLOCKING: not merged |
| QUEST-XP-ADMISSION-1 | BLOCKING: not merged |
| QUEST_GATE0, QUEST_STATE0 ACCEPTED | ACCEPTED: effective ACCEPT-BATCH #2209; control-plane allocation cites main `3ef900c160c9713092a86bfcd1a8377c7587c898` |

**Full-family dependency snapshot above is historical.** Acceptance is effective.
QUEST-XP-ADMISSION-1's direct prerequisites, PROGRESSION-OWNER-1 and QUEST-XP-1,
are MERGED (control-plane verified). Its missing caller remains the residue below;
the ITEM-LOOP/MAP-PICKUP rows have not been independently reclassified here.
Nothing in slot-only paths is authorized by this local preparation.

## Owned paths (builder edits freely)

- `apps/game-server/src/quest/`
- `apps/game-server/src/interaction/chest_use.rs`
- apps/game-server/tests/quest_rt_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- capabilities.rs (cap 16)
- connection.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- content quest pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/quest_log.rs` (domain, `observe_quest_log` l.633)
- `interaction/chest_use.rs` MINT
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`

## Inseparable bundles

mint+ledger; wire+registry+capability+handler.

## Acceptance criteria

- [ ] Chest reward minted once per character; replay safe.
- [x] Builder confirms the residue against the exclusive allocated base before code; the required caller is outside this allocation (findings below).

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

QUEST_GATE0; QUEST_STATE0. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 QUEST-RT-1 (rows 86, 87); App. 7 QUEST-RT-1.

## Builder instructions

- Work on branch `skeleton/quest-rt-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: inspected exact residue; prepared integration packet without duplicate code
status: waiting
branch: skeleton/quest-rt-1
blocker: QUEST-XP-ADMISSION-1 caller and lifecycle ownership are outside this allocation
next_action: CP allocates the accepted transport caller/test bundle at integration
```

## Bounded preparation — REPORT_ONLY (2026-10-15, #2198)

- Exclusive branch `skeleton/quest-rt-1`, base
  `6160009b3d2d57da37a8baacb45dd794b638b863`. Allocation permits only new
  `quest/admission.rs`, `admission_tests.rs`, `tests/quest_rt_admission.rs` and this
  record. **Only this record changes.** No new module, wrapper, test-only facade,
  content, capability, registry, writer or migration was invented.
- Accepted `ARCH_PROGRESSION_SOURCE_PACKETS` §2.3 explicitly says "This packet adds
  the call" and owns `gameplay_transport/mod.rs` (`refresh_quest_session` and retry
  scheduling), its PostgreSQL case and registration. Those paths are outside this
  allocation. A new `quest/admission.rs` would merely compose existing functions
  while leaving all session binding/current-fence/lifecycle ownership unconnected.
- Existing `durability/quest_state.rs::read_character_quest_state` loads one
  coherent copy under the root share lock, bounds tracks/states/claim and XP
  obligations, and preserves `WAITING_MIGRATION`. `admit_character_quest_state`
  already loads that copy and requests pending claim obligations through the
  existing sequencer. No second loader/admission owner is needed.
- `request_pending_quest_experience` already iterates the bounded pending XP copy,
  calls `RevisionSlot::commit_quest_experience`, reports definitive refusals, settles
  consumed obligations on success/replay, and requests backoff only for unknown
  `Unavailable` outcomes. The award uses `obligation.award(policy)`; quest content
  pins remain provenance, never the active reward revision. This logic must not be
  duplicated. Source references confirm no production caller currently exists.

### Exact CP integration bundle (unowned; no edits here)

1. In `ComposedFreshAdmission::refresh_quest_session`, after the existing durable
   admission returns a usable copy, obtain this session's existing
   `progression_sessions` binding via `progression_binding`; clone its `Arc` and
   release the map lock before awaiting. Pass the independently resolved
   `current_quest_fence`, existing authority/node/sequencer and that binding's
   `policy` to the existing XP helper. Do not construct a policy from quest data,
   reconstruct current authority from the copy, or add an alternate XP writer.
2. OR the helper's retry result with the existing load/claim retry decision; retain
   `QUEST_OBLIGATION_RETRY` (60 seconds). Preserve current terminal/read-failure
   handling: terminal installs no usable copy and no retry; an unknown fence read
   retains the prior copy and schedules existing backoff. `insert: false` must not
   recreate an entry removed while the await was outstanding.
3. With no progression binding, make no XP request: keep obligations pending, log
   `quest_experience_deferred reason=no_progression_binding` once for that session,
   and schedule no retry solely for that reason. Preserve any independently needed
   quest/load retry. Once-only logging and session lifecycle belong to the existing
   transport owner, not a new quest-side session map.
4. Existing callers then cover fresh admission (progression binding precedes quest
   admission), same-session resume, cadence retries, and chest refresh after a
   committed quest transition. Preserve their actor/session/current-generation
   controls and terminal cleanup. Revalidate against then-current protected main
   at the CP slot; do not merge/rebase this isolated preparation.
5. Add the accepted PostgreSQL 17.6 tests in the owning packet's actual transport
   bundle: earlier-session pending XP paid once at bound admission; chest-created
   XP paid through refresh; unknown first attempt schedules backoff and later pays
   once; unbound deferral followed by bound admission; current-fence failure and
   terminal/removal races do not authorize/recreate a session. Existing
   `tests/support/quest_xp_postgres_cases.rs` already tests durable pending replay
   and exactly-once consumption; it does not prove the missing runtime caller.

### Validation scope

This is a task-record-only integration packet. Governance validation/tests and
`git diff --check` apply. No Rust code or storage flow changed, so no Rust build,
snapshot registration or PostgreSQL run is claimed. Durable source hashes and
original reference/callsite outputs are in
`/workspace/oteryn-prep/quest-rt-1-evidence/`. Full QUEST-RT-1 chest/capability
acceptance remains open; this packet makes no playable/runtime readiness claim.
