# NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1

```yaml
task_id: NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1
title: "NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1: bank, promotion purchase, NPC trade wire (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/npc-econ-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 NPC-ECON-1 (rows 39, 81); App. 7 NPC-ECON-1; wave 2"
class: "hard (value)"
migration: yes
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/npc_trade_command.rs
  - apps/game-server/src/gameplay_transport/npc_command.rs
  - apps/game-server/src/interaction/bank_npc.rs (new)
  - apps/game-server/tests/npc_econ_*.rs (new)
slot_only_paths:
  - connection.rs (`npc_trade` l.696 arm)
  - mod.rs
  - capabilities.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - migration number
  - content NPC pins
public_contracts: []
depends_on:
  - "#2040 NPC trade writer: MERGED"
  - "NPC-CMD-1b read window: MERGED #2035 (read part); residue confirmed by builder"
  - "BANK-1: MERGED #1766"
  - "GOLD-FEE-2: MERGED #1781"
  - "BANK-FEE-0 / BANK_FEE0 ACCEPTED: BLOCKING: CANDIDATE"
  - "NPC0 ACCEPTED: BLOCKING: CANDIDATE"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Bank NPC service, promotion purchase, and the NPC trade intent (command 8) wired to the merged writer (#2040).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #2040 NPC trade writer | MERGED |
| NPC-CMD-1b read window | MERGED #2035 (read part); residue confirmed by builder |
| BANK-1 | MERGED #1766 |
| GOLD-FEE-2 | MERGED #1781 |
| BANK-FEE-0 / BANK_FEE0 ACCEPTED | BLOCKING: CANDIDATE |
| NPC0 ACCEPTED | BLOCKING: CANDIDATE |

**BLOCKING:** BANK-FEE-0 / BANK_FEE0 ACCEPTED; NPC0 ACCEPTED. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/npc_trade_command.rs`
- `apps/game-server/src/gameplay_transport/npc_command.rs`
- apps/game-server/src/interaction/bank_npc.rs (new)
- apps/game-server/tests/npc_econ_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs (`npc_trade` l.696 arm)
- mod.rs
- capabilities.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- migration number
- content NPC pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `npc_trade` (l.696)
- `durability/bank.rs`, `bank_audit.rs`
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

'+' bundle as one draft; mint/burn+ledger (gold); wire+registry+capability+handler.

## Acceptance criteria

- [ ] Trade command commits one DUR-03 transaction; bank fees burned through the ledger; idempotent retry.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

NPC0; BANK0; BANK_FEE0; GOLD_FEE_BOUNDARY. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 NPC-ECON-1 (rows 39, 81); App. 7 NPC-ECON-1.

## Builder instructions

- Work on branch `skeleton/npc-econ-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/npc-econ-1
blocker: BANK-FEE-0 / BANK_FEE0 ACCEPTED; NPC0 ACCEPTED
next_action: CP rechecks BLOCKING rows
```

## Bounded BANK-NPC-1 preparation (2026-10-15)

Existing draft #2197, exclusive `skeleton/npc-econ-1`, unchanged authoring base
`59f6d8e4b4fcc0ba6f6d92c5f1113fa8e0c996f4`. The allocation is restricted to the
two NPC command files, new `interaction/bank_npc.rs`, `tests/npc_econ_policy.rs`
and this record. Root alone may publish the completed draft packet; this worker
does not commit, push, mark ready, merge, trigger review or modify slot-only files.
The lead supplied META 3.1.0, one author and AUTHORING-only scope.

### Acceptance and actual residue

Read primary `main` at `e2da9805d296a27b06b32335bf67394509d4910b` without editing
it. Its merged #2209 ACCEPT-BATCH decision explicitly accepts NPC-0 and BANK-FEE-0;
their primary Status lines now read ACCEPTED. This supersedes the historical
candidate holds above without changing those historical rows. The applied rules
are BANK-0 §4.2/§4.3/§6 and its limits, plus NPC-0 §4 text/confirmation rules.

Residue inspection confirmed that `npc_trade_command.rs` already resolves command
8, performs replay and calls `commit_npc_trade` under the native fence (line 235).
`npc_command.rs` already owns talk/trade-window/session state. Both remain unchanged;
no second trade path was written. Promotion is allocated separately to PREM and
is not implemented by this bank-only parser.

### Prepared detached adapter

Only `interaction/bank_npc.rs`, the new tests and this record are changed:

- Parse bounded ASCII-folded whole bank commands: `balance`, exact deposit or
  withdrawal, `deposit all`, and transfer to a canonical `CharacterName`. Reuse
  native bank amount caps, NPC text byte limit and the native 30-second timeout.
  Reject signed/overflow/zero/over-cap amounts, unsupported commands, controls,
  oversized text and invalid recipient names. No parallel name normalizer exists.
- Store resolved operation data for one consuming pending confirmation. In
  particular, `deposit all` must already be resolved to an exact bounded amount;
  a later `yes` cannot recompute a larger amount. Missing or individually changed
  session, Character, World, NPC identity/generation or conversation-revision data
  cancels, as does every non-`yes` response or expired/reversed supplied time.
  Timeout arithmetic uses `[opened, opened + 30s)` in the caller's millisecond
  domain; final runtime clock/precision and prompt-revision binding need review.
- Compare the operation to the actual native `BankDepositRequest`,
  `BankWithdrawRequest` and `BankTransferRequest`; the transfer check binds exact
  amount, recipient CharacterId and canonical name key. These are consistency
  predicates, not native request validation or permission to execute a request.
- Map native terminal `BankResult` values to replies. Transfer feedback follows
  BANK-0 §4.3 verbatim and collapses recipient balance-limit reasons to the same
  reason-free refusal. No recipient balance or private reason is exposed. An
  unknown/storage outcome is never converted into a terminal refusal or success.

The context and operation are explicitly unqualified data. The adapter never
constructs an occurrence, CommandRef, session fence, current-authority witness,
money write, Account balance or World/Channel conversion. The future caller must
independently establish the current open/in-range Bank conversation, session and
service, prompt eligibility/junior and sender balance checks, resolve a live same-
World recipient, and clear pending data on reconnect/transfer. It must allocate
the occurrence bound to the confirming CommandRef, verify request/record binding,
and call the existing bank writer with independently current fences. Request data
or stored prompt data must never be substituted for current authority.

### Source-accurate evidence and validation limits

Durable evidence is `/workspace/oteryn-prep/npc-econ-1-evidence/`.
`prepare_harness.py` preserves 39 byte-identical native excerpts, their source and
excerpt hashes and line bounds, and imports the actual new test/adapter paths.
The small standalone harness includes native request/result data, canonical typed
IDs, the native bank occurrence constructor/UUID validator and its native error
type dependencies. It does not replace a money writer, authority implementation,
identity validator or persistence layer. Native coin-definition fixture data in
consistency tests is not qualified Content or current authority evidence.

The initial harness extraction incorrectly treated an array-type semicolon as an
item terminator; its real compiler log is retained as
`compile-initial-harness-error.log`. A separate module-reexport import warning is
retained in `compile-harness-module-import-error.log`; test imports were made local
to their explicit native module reexports. These were setup errors, not behavioral
RED claims. The extractor was repaired without changing native snippets. Root's
disk HOLD stopped an intervening queued command before it began; after root's GO,
the corrected isolated run used the shared lock and `CARGO_INCREMENTAL=0`. This
worker did not remove shared/cache artifacts or compile a full server.

Final local result: **11 passed, 0 failed** (eight new bank-policy tests and three
existing character-name tests). This covers each operation cap/max+1, text 255/256
bytes, Unicode/invalid names and unsupported commands, resolved deposit-all amount,
every independently changed context field, missing context, timeout equality and
clock reversal, cancellation, native request kind/amount/recipient/name-key
substitution, exact transfer replies and collapsed recipient-limit feedback.
Native request fixtures use the actual bank occurrence constructor and UUID
validator; no replacement validation function is used.

The literal-source harness passed `rustc --test --edition 2024 -D warnings` and
standalone clippy-driver metadata validation with the workspace deny list. Direct
rustfmt and `cargo fmt --all --check` passed. Source/harness hashes matched before
and after all checks (17 source files, 39 native excerpts); raw logs, manifests and
reproduction scripts remain in the evidence directory. Governance and **124/124**
agent tests passed. These are isolated arithmetic/dialogue-data tests, not current-
authority or native money-writer qualification.

Prepared source SHA-256:

- `interaction/bank_npc.rs`:
  `40626968f5e9471b701650e29d427f59739b9c3242eae604809f2475918a3017`.
- `tests/npc_econ_policy.rs`:
  `1fca37d1cb2fa9772a0c7b0cfe3520bff99a01366d09441b07c1e0b849159eda`.
- Harness:
  `fee0a6ba294c981345c6618c454db32734abb2b4e64537bcf7a95202f367ef23`.

Whole-diff adversarial self-review and final owned-path/whitespace checks confirm
only this record and the two allocated new files differ. Both existing NPC command
files and the native bank writer are unchanged. No source was frozen or accepted
as a complete runtime implementation.

Full package/all-target compilation, PostgreSQL/native writer tests and live NPC
bank E2E are not claimed by the detached harness. Shared `mod.rs`/connection
wiring, Bank content/schema registration and pins, actual conversation-owner
storage/retirement, prompt-time current lookups, confirming-command occurrence
assignment, current fencing, ledger execution/replay and whole-outcome reply
binding remain integration work. Independent exact-head security/persistence
review and CP/architect/slot acceptance remain required; no bank service is enabled
by this unregistered module. The full-task acceptance checkbox remains open.
