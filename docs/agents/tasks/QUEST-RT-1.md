# QUEST-RT-1 — quest runtime on the bundle World: XP admission caller, chest rewards, capability 16 (consolidated task card)

```yaml
task_id: QUEST-RT-1
title: "QUEST-RT-1: quest runtime on the bundle World, quest XP admission caller, chest rewards, capability 16 offer"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 QUEST-RT-1 (rows 86, 87); App. 7 QUEST-RT-1"
brief: "docs/architecture/reviews/OTERYN_GAME_ARCH_PROGRESSION_SOURCE_PACKETS_2026-10-05.md §2.3 (QUEST-XP-ADMISSION-1 caller); QUEST_GATE0 / QUEST_STATE0 decisions"
class: "hard (rewards are mints)"
base_branch: main
branch: null
pr: null
needs_migration: maybe   # none needed for the XP caller; if chest rewards need one: next free number at allocation time (only one migration PR in the queue)
needs_registry: true     # capability 16 QUEST_LOG_V1 offer flip
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2198, branch skeleton/quest-rt-1

## Goal

Quest runtime on the bundle World: pending quest XP obligations paid exactly once at
quest admission/refresh (the missing production caller), chest rewards minted once per
Character and replay-safe, and capability 16 (QUEST_LOG_V1) offered once its gate
holds. The draft delivered the XP admission caller patch (held, never landed) and six
native PostgreSQL scenarios; chest-use E2E and the capability offer remain open.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| QUEST-LOWER-1 | MERGED (#1727) |
| QUEST-XP-1 / QUEST-XP-1b (obligation + XP writer, migration 0069) | MERGED (#1724, #1731) |
| PROGRESSION-OWNER-1 / PROG-BIND-1 (session progression binding) | MERGED (#1964) |
| QUEST-LOG-WIRE-1 (capability 16 wire, domain 16; `offered: false`) | MERGED (#1741); offer gate still needs QUEST-CONTENT-2 journal content |
| QUEST_GATE0 / QUEST_STATE0 acceptance | ACCEPTED (#2209) |
| NPC-ECON-1 (#2197, ordered before the D1150 slot) | MERGED (#2197) |
| ITEM-LOOP-1 | MERGED (#2190) |
| MAP-PICKUP-1 | MERGED — #2290 (migration 0130) |
| QUEST-XP-ADMISSION-1 caller | not on main: `request_pending_quest_experience` still has no production caller — this card owns it (CP D1150) |

## Accepted decisions

- **ARCH_PROGRESSION_SOURCE_PACKETS §2.3:** "This packet adds the call" — the XP caller
  lives in the transport owner's `refresh_quest_session`; no second loader, admission
  owner, XP writer or retry loop.
- **CP D1150:** this task owns the `refresh_quest_session` → `request_pending_quest_experience`
  call, landed in the ordered shared slot after #2197 on then-current main.
- **CP D1316:** grants the native monk-save fixture mount for the admission test leaf,
  atomically with the D1150 caller.
- **CP D1329 / D1342:** native PG execution policy (evidence only for the draft).
- Capability 16 stays **unoffered** until its QUEST-LOG-WIRE-1 offer gate holds; journal
  text/wire ownership is QUEST-LOG-WIRE-1's, not this task's.
- Quest content pins are provenance only, never the active progression reward revision;
  the award uses `obligation.award(policy)`.

## Reference behaviour from the draft (re-implement, do not copy)

Existing pieces to reuse unchanged: `durability/quest_state.rs`
`read_character_quest_state` (bounded coherent copy under root share lock, preserves
`WAITING_MIGRATION`), `admit_character_quest_state` (requests pending claim obligations
through the sequencer), `request_pending_quest_experience` (iterates the bounded pending
XP copy, calls `RevisionSlot::commit_quest_experience`, settles on success/replay,
requests backoff only for unknown `Unavailable`).

Caller behaviour in `refresh_quest_session`:

1. After admission returns a usable copy, get this session's progression binding via
   `progression_binding` (clone the `Arc`, release the map lock before awaiting); call
   the XP helper with the independently resolved `current_quest_fence`, existing
   root/authority/node/sequencer and `&binding.policy`. Never build a policy from quest
   data or reconstruct authority from the copy. Heap-pin the helper future
   (`Box::pin`, following `admit_progression_session`) — the full policy carries its
   threshold table by value; the draft measured the refresh future 202,504 → 8,376 bytes.
2. OR the helper's retry with the existing load/claim retry; keep
   `QUEST_OBLIGATION_RETRY` (60 s). Terminal installs no copy and no retry; an unknown
   fence read retains the prior copy and schedules backoff; `insert: false` never
   recreates an entry removed during the await.
3. No progression binding → no XP request, obligations stay pending, log
   `quest_experience_deferred reason=no_progression_binding` once per session (flag on
   the existing `QuestSession` entry, logged after releasing the mutex), no retry solely
   for that reason.
4. Existing callers (fresh admission, resume, cadence retries, chest refresh after a
   committed transition) then cover every path.

Six native PostgreSQL scenarios (draft: 6/6 pass, run on a 64 MiB test thread; default
stack overflowed — keep the explicit test-thread recipe from
`kill_reward_postgres_e2e_tests.rs`, never change production stack or `RUST_MIN_STACK`):

1. Pending XP from an earlier native owner pays once at bound admission; repeated
   admission and transition refreshes do not pay twice.
2. Repeated unbound admission keeps pending XP and the one-time deferral marker with no
   retry; a later binding pays once.
3. Injected XP-receipt transaction failure leaves the obligation pending, schedules
   backoff, pays once when the cadence is due after recovery.
4. Replacing only the durable connection generation after the fence read refuses the
   award, keeps the obligation, adds no retry.
5. Removing the local session and binding while a refresh is paused: `insert: false`
   recreates neither.
6. Refresh after a real expired-loss release in FreshAdmissionStore: fence `Ok(None)`;
   two refreshes discard the copy and retry state; durable XP/receipts/obligations stay
   `(0, 0, 1)`.

The fixture's quest catalogue pin is the admitted `content-1` context and its
reward-revision inequality is asserted.

## Remaining (not done in the draft)

- Land the caller and the fixture mount (they were never merged).
- Literal chest-use command → refresh E2E; complete fresh-admission/transport E2E;
  unknown-fence-read and terminal-lifecycle regressions.
- Chest reward minted once per Character, replay-safe, through the existing chest MINT
  (`crates/gameplay/social/src/interaction/chest_use.rs`).
- Capability 16 offer flip in `capabilities.rs` and the registry once its gate holds.

## Target layout (ADR-0022)

- Caller: `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
  (`refresh_quest_session`, ~line 2050; call-site + `QuestSession` flag only).
- Durable helpers (exist, unchanged): `crates/gameplay/durability/src/durability/quest_state.rs`.
- Admission scenarios: `crates/gameplay/dispatch/src/gameplay_transport/quest_admission_postgres_tests.rs`
  (new), mounted from `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`
  (registration line only).
- Chest: `crates/gameplay/social/src/interaction/chest_use.rs` (exists); PG cases
  `apps/game-server/tests/support/chest_use_postgres_cases.rs` (exists) or a new child
  case file; quest XP cases `apps/game-server/tests/support/quest_xp_postgres_cases.rs` (exists).
- Quest log projection: `crates/gameplay/dispatch/src/gameplay_transport/quest_log.rs`,
  `crates/protocol-oteryn/src/quest_log.rs` (exist).
- Capability: `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` +
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (kept equal by `capabilities_tests.rs`).

## Acceptance criteria

- [ ] Pending quest XP paid exactly once at bound quest admission/refresh; six scenarios above pass.
- [ ] Chest reward minted once per Character; replay safe; literal chest-use E2E.
- [ ] Capability 16 offered only when its QUEST-LOG-WIRE-1 gate holds; registry and offer sets equal.
- [ ] Every concurrent transition serialized and the losing writer typed.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`gameplay_transport/mod.rs` takes the call site only).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-gameplay-dispatch quest_admission -- --test-threads=1
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test chest_use_postgres
cargo test --locked -p oteryn-game-server capabilities
python3 tools/agents/validate_governance.py
```
