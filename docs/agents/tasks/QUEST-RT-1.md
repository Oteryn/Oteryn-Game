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


## Native quest XP caller preparation (2026-10-17, draft #2198)

This preparation starts from draft head `5e6513963c21796e768f9aa8dbe46810b6c99432`
and was assembled for native validation on protected main
`b1e90b421692289db3aac78a915629251ba80481`. The historical record above is preserved.
The current allocation is [CP D1150](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224947337):
this PR owns the `ComposedFreshAdmission::refresh_quest_session` call to the
existing `request_pending_quest_experience` helper, in the ordered shared slot
after #2197 and after merging then-current main. Accepted
`ARCH_PROGRESSION_SOURCE_PACKETS` §2.3 remains the behavior contract.

### Published owned leaf and held integration bundle

The owned source leaf is `apps/game-server/src/quest/admission_tests.rs`, with
this task amendment. It uses the existing actual `ComposedFreshAdmission` and
native monk-save PostgreSQL fixture; it introduces no alternate quest loader,
progression authority, XP writer, migration or content catalogue for production.
The synthetic quest and progression inputs are explicitly test fixtures.

The complete reviewed caller and test-mount proposal is retained in the PR
description, bound to the main revision above. The shared
`apps/game-server/src/gameplay_transport/mod.rs` hunk is held for D1150's slot.
The four-line child registration in
`apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs` is necessary
to reuse that native fixture and is validation-only until its integration
allocation is confirmed. Neither shared file is part of this owned publication.
Consequently, the owned test leaf alone is not a registered executable suite on
the PR branch; native validation below applies to the explicitly assembled
caller + mount + leaf candidate, not an assertion that the held integration has
already landed. Existing aggregate quest-state/XP support is reused unchanged.

The earlier prerequisite inspection used #2197 draft head
`427fe6224d30557af4b70afb651768b90a07446d`, which was then conflicted; its relevant
session binding and quest refresh APIs matched the pinned main baseline. That
is a historical source observation, not a claim about a newer prerequisite head.
The coordinator's fresh read at 2026-10-17T09:08Z found #2197 advanced to
`6be32b062b741df8070d49d4e2c79dc197271ffe`, updated at 09:07:21Z: open, draft,
unmerged and mergeable. The new prerequisite source has not been compared with
this validated assembly. The gate is the required prior PR remaining unmerged
and the ordered shared slot, not a current merge conflict. Re-read the exact
prerequisite and then-current protected main at the slot.

### Prepared caller behavior

After a usable quest copy is loaded, the existing transport owner clones this
session's immutable progression binding and releases the session-map lock. The
existing XP helper receives that policy, the independently read current
session/lease/scope fence, and the existing root, authority, node and sequencer.
The sequencer supplies the current expected CharacterRevision for each award;
quest content provenance is never substituted for the progression reward
revision. No new XP path or retry loop is introduced.

The helper's unknown-outcome retry result is ORed with the existing load/claim
retry decision and uses the existing 60-second cadence. An unbound session keeps
its XP obligations and logs deferral once per session, without adding retry
solely for absence of a binding. Logging occurs after releasing the quest-session
mutex. Existing unknown-fence read preservation, proven-terminal handling and
`insert: false` removal behavior are retained.

The full native policy contains its threshold table by value. In the assembled
candidate, heap-pinning the existing XP-helper future reduced the measured
refresh future from 202,504 to 8,376 bytes, following the existing
`admit_progression_session` allocation precedent. This comparison concerns the
prepared candidate before and after the allocation change; it is not a
measurement of the unmodified main baseline. The native award calls and values
are unchanged.

The corresponding instrumented scenario shrank from 426,160 to 164,544 bytes.
Execution then entered the scenario, completed the real admission fixture,
initialized progression and committed the quest transition, but still overflowed
inside the first quest admission. Each fixture initializer call is heap-pinned to
remove its measured 71,736-byte future from the retained scenario state; the
scenario itself is also heap-pinned before the shared executor receives it.
The five-case run still overflowed at the default test stack after that placement
change. Those default-stack failures are retained as evidence, not reported as
successful qualification.

The local test runner now uses a named thread with an explicit 64 MiB stack,
following the existing native `kill_reward_postgres_e2e_tests.rs` scenario recipe.
It creates the same fixture/runtime on that thread, forwards scenario errors and
resumes the original panic payload on the calling test thread. This is an
explicit debug-test execution policy: neither the production runtime stack nor
the process-wide `RUST_MIN_STACK` setting is changed. The five functional scenarios
passed on this runner; no default-stack or full runtime pass is claimed.
The fixture's quest catalogue pin is the admitted `content-1` context, and its
separate reward-revision inequality remains asserted.

### Five native PostgreSQL scenarios

1. Pending XP from a previous native owner pays once at bound quest admission;
   repeated admission and subsequent transition refreshes do not pay twice.
2. Repeated unbound admission preserves pending XP and the one-session deferral
   marker without scheduling retry; attaching the valid binding later pays once.
3. An injected failure in the XP-receipt transaction leaves the obligation
   pending, schedules existing backoff, and pays once when the real cadence is
   due after the store recovers.
4. Replacing only the durable current connection generation after the native
   current-fence read refuses the XP award, preserves the obligation, and adds
   no retry for that definitive refusal.
5. Removing only the local session and progression binding while a refresh is
   paused prevents `insert: false` from recreating either entry.

These cases execute the real quest admission/refresh methods and inspect durable
XP, receipt and obligation rows. The transition-refresh scenario does not dispatch
a literal chest-use command. Actual chest-use-to-refresh E2E, complete fresh
admission/transport E2E, unknown-fence-read and terminal lifecycle regressions,
and full QUEST-RT-1 runtime qualification remain open; existing lower-level
chest/quest cases and source wiring are not relabeled as those new E2E passes.

### Native validation checkpoint — 2026-10-17

The assembled candidate used canonical PostgreSQL 17.6 and the real shared
fixture with `OTERYN_TEST_POSTGRES_ADMIN_URL` set. The run used the explicit
64 MiB scenario thread above; `RUST_MIN_STACK` was unset. The coordinator's native
PC readback at 2026-10-17T09:02:47Z reports **5 passed, 0 failed, 0 ignored**, with
3,267 other cases filtered out, in 8.34 seconds. Strict all-target Clippy and
format checking completed successfully at 2026-10-17T09:05:59Z. The same three
source hashes were unchanged after the tests and Clippy. Initial stack failures
and diagnostic-only cases are not counted as passing regression cases.

```sh
cargo test --locked -p oteryn-game-server --lib quest_admission:: --quiet -- --test-threads=1
cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo fmt --all -- --check
```

The exact validated assembly is main
`b1e90b421692289db3aac78a915629251ba80481` plus these source bytes:

| Source | SHA-256 |
|---|---|
| `apps/game-server/src/gameplay_transport/mod.rs` | `45631d837035110e5e77c870f8cf2d4074e9d74e871fa5cd82a7943769abdb4c` |
| `apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs` | `582e50fc7889e599d506d9c2560f8b63e4f7df856e516e207e9160a22862d399` |
| `apps/game-server/src/quest/admission_tests.rs` (PC-formatted) | `c326744eb5dce32c218c342462d8fe747e68ec5fc6c6393cbf2df63e1dd30368` |

| Check | Result |
|---|---|
| Five native quest PostgreSQL scenarios on the assembled candidate | PASS: 5/5, with the explicit 64 MiB debug-test thread |
| Final future placement measurement | Refresh 8,376 bytes; cadence 8,448; admission 8,416; default-stack failures retained |
| Strict game-server all-target Clippy on the same final source | PASS, exit 0; existing vendor Tokio `missing_docs` warning only |
| `cargo fmt --all -- --check` | PASS |
| Independent caller and fixture-delta source review | PASS; PC formatting changes no test values, assertions, hooks or runner behavior |
| Owned-publication governance checks, tests and whitespace check | Required on the frozen owned commit; exact result is recorded in the PR description |

The independently reviewed test leaf before PC formatting has SHA-256
`00fb3a486f3a1f28cac9db42e974a4b90d5dcc7ffc9b7974f13b4b577d4fdadc`;
only formatting changed to the validated leaf above. The frozen publication
commit and its governance readback are recorded in the PR description so that
this source checkpoint does not need a status-only follow-up commit.
Full-package tests, production content qualification and runtime completion are
not claimed by this focused packet.

Native run evidence remains under
`/root/oteryn-six-pr-20261017/evidence-quest/` on Molehill-PC:

| Evidence log | SHA-256 |
|---|---|
| `native-scenario.log` | `a7e3f8bcf9e7c29595eaef8693b1935efb0f469be00a60990900572449ab1ed7` |
| `clippy-scenario.log` | `984a0050d7b0091cd2c5701c8ec6e6b74b8587417704637c594d8849234b24a1` |
| `fmt-scenario.log` (empty, exit 0) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

### Remaining integration and product gates

D1150's shared caller slot follows #2197; the native fixture mount also needs its
integration allocation. Merge main and revalidate the exact assembled source at
that slot. Capability 16 remains **unoffered**. Journal text/wire ownership is the
separate `QUEST-LOG-WIRE-1` task. Content/registry/capability activation, literal
chest-use E2E and full QUEST-RT-1 acceptance remain unfinished. This draft is
`NOT_FULLY_QUALIFIED`; only CP has integration/merge authority.
