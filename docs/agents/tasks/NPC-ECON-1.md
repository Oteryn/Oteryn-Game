# NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1

```yaml
task_id: NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1
title: "NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1: Bank NPC service and NPC trade wire"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/npc-econ-1
issue: 1622
pr: 2197
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 NPC-ECON-1 (rows 39, 81); App. 7 NPC-ECON-1; wave 2"
class: "hard (value)"
migration: yes
registry: maybe
owner: "Codex family B lead (direct owner allocation 2026-10-16)"
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-16
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
  - "BANK-FEE-0 / BANK_FEE0: ACCEPTED via #2209 ACCEPT-BATCH §1"
  - "NPC0: ACCEPTED via #2209 ACCEPT-BATCH §1"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Recorded implementation disposition at source head
`222c7dce99ee3fb854b25bcb2140e4fde8e7fc9e`: **owned native/content/pin core
qualified; shared protocol/resource/protected-PG tail unapplied**. The title, PR
link, acceptance rows and narrow value-path criterion are reconciled with
[CP gap review 6216355456](https://github.com/Oteryn/Oteryn-Game/pull/2197#issuecomment-6216355456)
(2026-10-16). Other allocation/base fields are retained historical metadata, not
a live lease. Historical skeleton checkpoint and preparation sections below
remain retained. See the integrated implementation-slot and published-candidate
CI successor sections for the actual #2197 scope, recorded evidence and open
gates; this administrative refresh does not qualify a new candidate. The owner
directly confirmed exclusive family-B authorship on 2026-10-16 and authorized
return to AUTHORING in the task's owned paths without an additional CP lease
identifier. This card-only successor leaves all shared slot paths untouched.

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Bank NPC service and the NPC trade intent (command 8) wired to the merged writer (#2040), including coins-first BUY with the bank remainder. Promotion purchase belongs to PREM-2b/PREM-5 under [ACCEPT-BATCH §3](../../architecture/reviews/OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md#3-architect-rulings-a).

## Dependencies (acceptance reconciled 2026-10-16; other states retained from the implementation record)

| Dependency | State |
|---|---|
| #2040 NPC trade writer | MERGED |
| NPC-CMD-1b read window | MERGED #2035 (read part); residue confirmed by builder |
| BANK-1 | MERGED #1766 |
| GOLD-FEE-2 | MERGED #1781 |
| BANK-FEE-0 / BANK_FEE0 | ACCEPTED via #2209, ACCEPT-BATCH §1 |
| NPC0 | ACCEPTED via #2209, ACCEPT-BATCH §1 |

The former BANK_FEE0/NPC0 candidate holds are superseded by merged #2209
[ACCEPT-BATCH §1](../../architecture/reviews/OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md#1-accepted-briefs-owner-items-1-to-6);
their primary brief Status lines read ACCEPTED. This acceptance grants no live
write lease. The shared protocol/resource/Cargo/protected-PG tail remains held
by HOUSE-RUNTIME-1a (#2163), pending CP release, main composition, a new
authorized candidate generation and exact-head qualification as recorded below.

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

- [x] Trade command commits one DUR-03 transaction; bank fees burned through the ledger; idempotent retry. Evidence for the owned value path at source head `222c7dce99ee3fb854b25bcb2140e4fde8e7fc9e`:
  - `apps/game-server/src/durability/npc_trade.rs:697-711` begins one native transaction, locks the occurrence and returns its stored outcome on replay; `:793-808` inserts the matching `FEE_DEBIT`/BURN value line; `:827-840` applies the trade and commits that same transaction.
  - `apps/game-server/tests/support/npc_econ_bank_postgres_cases.rs:395-429`, `actual_npc_buy_uses_coins_then_bank_in_one_native_trade_and_single_event`, asserts bank-only and mixed coins-first conservation, matching ledger/audit data, one audit event, unchanged retry footprint and insufficient-funds rollback.
  - Recorded execution evidence: **Actual local validation and review evidence** below (33 native bodies and eight real PostgreSQL 17.6 cases, including the qualification limits), plus the **Published-candidate CI successor (slot 2)** record and `/workspace/oteryn-prep/npc-econ-1-slot-2-evidence/`. These are retained qualification reports, not a fresh run by this administrative refresh. The shared-tail, exact-head CI, independent architect review and CP gates remain open.
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

## Historical skeleton checkpoint (superseded by the integrated implementation record below)

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/npc-econ-1
blocker: BANK-FEE-0 / BANK_FEE0 ACCEPTED; NPC0 ACCEPTED
next_action: CP rechecks BLOCKING rows
```

## Historical detached BANK-NPC-1 preparation (2026-10-15; superseded by the integration slot below)

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


## Integrated implementation slot, 2026-10-15

**Publication scope: NATIVE/CONTENT/PIN CORE QUALIFIED. Shared protocol field 14,
resource value-line 1 and protected PG aggregate/Cargo registration remain
UNAPPLIED. This owned draft packet is not full mechanic or merge readiness.**

The skeleton metadata/dependency table and detached preparation above are historical.
Accepted NPC0/BANK_FEE0 (#2209) and CP implementation slot 6196051193 supersede their
old candidate/slot holds for the exact root allocation. This authoring base is
`73e023070e90036045e8842840f414ae1105c5ef` (main b62 integrated by CP into the existing
draft #2197). Promotion purchase is excluded by ACCEPT_BATCH §3 and remains
PREM-2b/PREM-5. Existing registered command 8 already reaches `commit_npc_trade`;
this change preserves that route and writer. The full-task acceptance checkbox,
exact-head CI, independent architect review and CP activation/queue decision remain open.

### Native gameplay and value path

- Command 7 now reaches the registered Bank owner through the actual current NPC,
  typed service, admitted Dialogue, actor range, conversation and native session
  fence. Preview reads actual Account/World balance, eligible native coin custody
  and same-World live recipient identity. Transfer checks sender funds before
  prompting and keeps recipient-limit feedback reason-free. Native Bank owns
  Junior, balance, inventory and recipient policy; no request boolean supplies it.
- One volatile confirmation binds actual engagement, NPC generation, catalogue,
  session/fence and node incarnation; reconnect, reengagement, cancellation,
  reversal and the exclusive 30-second deadline invalidate it. `deposit all`
  freezes the resolved exact amount. At acceptance the NPC owner rechecks its
  actual current facts while the native Bank owner fences and retains the exact
  original CommandRef, semantic request, transaction/event/output IDs and source
  provenance. No raw talk text is persisted.
- The retained row is Bank-owned pending work, not evidence that physical SQL was
  sent, a client SENT marker, a new generic queue or current authority. Crash
  between enqueue and money execution leaves the original pending work. Resume
  drains only that original command under an independently reconciled current
  same-GameSession Root/fence; original-command and native Bank occurrence locks
  serialize it. Exact committed Bank receipt lookup precedes current fresh-write
  checks. A held/unknown attempt stays unknown; only locked receipt/absence after
  the old transaction resolves permits classification. Frozen outer identities
  remain unchanged; native internal ledger IDs may rematerialize only after
  authoritative noncommit. There is one bounded semantic pass, no retry loop and
  no invented BANK max-three refusal (RL08 presently belongs to MINT/TRANSFER).
- Original pending work is classified before replacement talk/NPC/command type.
  A closed OPS input gate disconnects/holds without sequencing a false refusal
  over it. Historical replies preserve the actual current conversation state;
  deposit/withdraw custody changes refresh inventory through the existing domain 9.
- NPC BUY reuses native coins-first fee planning and the existing trade transaction:
  all eligible coins plus the bank remainder, V2 only, one real FEE_DEBIT and one
  trade event. Junior/V1 behavior, native current fences and writer replay remain
  intact. The Bank debit cause is a closed typed GoldFee/NpcTrade source, not an
  arbitrary SQL cause; the two shared fee helpers changed visibility only.
  Audit validates positive item custody units divisible by purchase quantity
  (NPC0 §3.4/§5.3/§5.4); the current catalogue/native writer owns exact offer
  count. Quantity and unit price remain per purchase, never normalized per item.
  The measured Bank shape fits the existing BUY 23 participants/66 work
  and 27,094 payload/27,408 envelope bounds (actual 22/64 and 25,959/26,273).
- Migration 0114 (numbered 0090 before the 2026-10-17 main merge) stores the immutable resolved continuation plus same-transaction
  completion proof, and extends the native Bank entry cause with an exact NPC
  trade foreign key. It preserves the complete latest base 0072 Bank and 0082
  trade guard bodies except the explicitly closed new cause/value arms. Native
  0071 operation/entry immutability and no-TRUNCATE guards retain terminal Bank
  receipts; ordinary audit-outbox expiry does not delete those records. No
  operation/ledger retention writer is introduced. All native
  value, receipt, outbox, location and deferred guards stay active. Qualification
  uses real MINT → TRANSFER → DEPOSIT funding, never initial bank balances or
  disabled guards. Final CP composition must preserve any later main guard arms.

### One real admitted Bank NPC and matching World artifact

Only Paulie is added. Banker profession alone, Alia's payment helper and unexecuted
Lua module names are not Bank provenance. The native admission packet pins the
actual registered financial callback in Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
`data-otservbr-global/npc/paulie.lua`, SHA-256
`ca77d5451037ecbd704ed80a99684dbd42756ab5fd5d4ae9e5273367f7986aa1`.
Its family identity is reference evidence; accepted BANK0 owns behavior. No quest
bonus, legacy capacity/Junior rule or Lua execution is imported. Original Oteryn
Bank greeting/topic text is explicitly separate from that source proof and uses
the existing typed Dialogue consumer. Unsupported topics remain unanswered; an
admitted Dialogue never falls back to a generated reply. Existing NPC facts,
farewell/voices, all other 1,281 NPCs and Trade324/Travel56 remain preserved.

The real native materializer and owning Python producer generated the exact
leased outputs, with one Bank service (381 total), 1,282 NPCs and 836 Dialogues.
All 16 actual catalogue files, including three preserved HEAD service indexes,
produce native digest `42ab7d0884bb801f53d2e824c6ddc1d6504bfcc81957638e6c616719377153e8`;
project digest is `18162da6522d436998ae773b21c6f47ab731c65d7df7790fb8ebae2ab782de40`.
The invalid sparse 13-file/23acc proposal is retained as superseded evidence.
The native loader/model admits Paulie's actual Bank Dialogue and no generated
reply. Actual full World compile/equivalence and native visit/model qualification
produce a 25,089,875-byte artifact, digest
`cbae6fc22e3509feb5c93de8ee0cc77a795851ae871725a035409febcf298204`,
with 21 realized NPCs including Paulie; the four existing held placements remain
held. The exact root-leased NPC/World/identity/topology pin batch preserves
`production=false`, entry `(32369,32238,-7)` and every unrelated topology field.
Standard native `pin-check` then passed its double-build determinism, equivalence,
identity/input digest and actual entry-cell checks. Its artifact bytes equal the
independently visited/model-qualified artifact. An isolated evidence Git index
matches all 1,448 actual input files (1,447 fingerprinted); the branch index never
changed. No bundle was deployed or World activated.

### Actual local validation and review evidence

Evidence: `/workspace/oteryn-prep/npc-econ-1-slot-1-evidence/`.
The retained raw logs distinguish the initial source/setup failures from final
results: type errors, SQL constraint preflight, missing exact source inputs,
incorrect test expectation (31/1), native rejection of hardlinked validation
inputs, a tiny driver dependency-search failure and the attempted bundle-test
constructor compile error (attempt12 E0599, no runtime tests ran). None is a passing result or
a behavioral RED claim. Old artifacts have verified lossless gzip/SHA restore
manifests; no shared target/dependency/anchor was removed.

- Actual registered production check-lib/fmt passed in the initial integrated
  generation. Final untransformed native unit Clippy metadata and genuine normal
  `cfg(test)` rlib compiled; all 33 selected original native bodies passed in
  the final bundle-count repair (attempt 13), including original native BUY/SELL
  validate→plan→message→codec cases at quantity2/count5→10 units and
  quantity20/count5→100 units under both tuples, explicit moved-unit assertions,
  nondivisible custody-only negatives, duplicate raw field 14, max+1 value-line
  shape and exact measured max sizes. Earlier attempt6 32/0 omitted that producer
  composition and did not prove bundled-trade correctness.
- All five original repository tests and all four original NPC catalogue tests
  passed in the actual complete single-link candidate tree, with original target
  Clippy metadata (attempt 7). Native public catalogue/model checks passed on that
  tree and on the matching actual full World artifact (attempt 9).
- All eight unchanged genuine native PostgreSQL cases passed on PostgreSQL 17.6
  (`server_version_num=170006`) against the final bundle-compatible audit core
  (attempt 13; the same eight source bodies also passed in attempt7 followup).
  They cover actual current-fence substitutions, immutable original binding,
  restart/reload/replay, deferred rollback, two independently maintained Roots,
  native BUY coin+bank conservation/V1 refusal/V2 admission, current preview,
  held occurrence timeout/unchanged footprint/awaited rollback/fresh Root recovery,
  and actual inflight commit followed by receipt replay. They do not inject a
  physical COMMIT-acknowledgement loss. Tests retain the genuine private Root and
  fixture bodies, native guards and actual source types; only exact own test
  attributes/visibility and validation-only dispatch paths are bridged. Original
  standalone wrapper metadata also passed. Fixture/admin-role semantics are not
  production runtime-role or live-account deployment qualification.
- Native policy9/0, source predicate5/0 and existing Python schema27/0 passed.
  The full staged tree validator and original regression script passed, including
  one positive and nine closed Bank negatives; the actual copied candidate tree
  validator also passed. Native standard full pin-check passed (77.86 seconds),
  deploy NPC pin tests17/0 with no skips, and topology tests passed; their real
  runner-access probe explicitly remains unexercised (requires root/sudo).
  Successful runs retain matching source, native extern, compiler binary and
  private/branch index hashes at their captured qualification boundaries. All
  1,448 compiler inputs and the qualified private index still match at closure.
  The branch index later changed nonsemantic metadata; every one of its 18,839
  stage-0 path/mode/blob entries remains exactly HEAD, with no staged changes.
  Its binary SHA changed from `1df2f40a…` (attempt11) to `44a47175…` at
  closure; `final-branch-index-semantic-proof.json` retains both complete SHAs
  and the semantic proof, without restoring the index to fabricate byte equality. No full server/global unit ELF or live client/server
  E2E is claimed.

Independent source lanes found and verified repairs for original-command bypass,
missing inventory refresh, replay reopening chat, missing early sender-funds
refusal, closed OPS-gate sequencing, generated Dialogue fallback, sparse pin
hashing and overly broad old Service-field expectations. Final review then
identified a regression in the initial audit repair: purchase quantity was wrongly
equated with item units although admitted stackable offers carry quantity×count.
That equality was replaced with the integral positive bundle-shape check and
qualified through the real producer/codec on BUY and SELL. The earlier claim
that MINT100/purchasequantity1 was inherently invalid is superseded: count100
can explain it. The present MINT1 control is also valid and keeps the same encoded
width; exact catalogue count is not reconstructed from a payload lacking count.
The bounded runtime, content and protocol/Bundle peer reports retain exact hashes
and their qualification limits; none substitutes for architect/CP acceptance.

### Remaining inseparable integration gates

The native NPC trade nested bank field, measured registry value-line update and
canonical protected PostgreSQL aggregate registration are exact evidence proposals
only while the House owner retains those shared paths. The standalone target is
Cargo-discoverable; it must not silently stand in for the canonical PG CI lane.
The same eight source bodies are prepared for that aggregate, with no skipped-PG
claim. CP must release and compose the actual shared protocol/resource/Cargo/
aggregate changes and latest main, preserve every newer native guard arm, qualify
that final exact candidate and keep the PR draft as instructed. No new wire/event/
capability number, hidden authority source, promotion writer, deployment or ready/
merge action was added here. BANK-specific retry admission beyond the existing
bounded native owner remains an explicit architect review point, not a new policy.

Prior owned-packet checks (published C344e, before external CI): governance and all 124 agent tests passed after exact
HEAD materialization of five missing lifecycle-registry handoff inputs; the first
E8001 failure is retained, without changing that registry. All 60 dirty paths are
allocated, whitespace checks pass and the four common-held files are byte-identical
to the base. No code/pin mutation follows the owned-packet freeze; a shared-tail
successor requires a new CP-authorized generation and exact qualification.


### Published-candidate CI successor (slot 2)

The prior owned packet was published as draft candidate
`344e43b7452f447a56ebfb04a0e752739b27cdbd`. Its real Rust all-target CI
failed with four E0432/E0433 errors: `interaction_workflow` path-includes the
pure interaction module, and the added Bank child required domain/durability
roots absent from that harness. Earlier native/selected-target green results
did not qualify this consumer. The data-freshness lane also found three stale
Quest binding evidence outputs after the admitted Paulie data update. Neither
failure is hidden by the earlier local qualification or this retained history.

The successor changes only the existing Bank registration and two imports: the
same physical `interaction/bank_npc.rs` is now a private child of its actual
`gameplay_transport` consumer. The pure interaction module no longer imports
Bank custody dependencies. There is one production Bank policy module. All Bank
function bodies, native writers/fences/receipts/SQL, serialized requests, library
visibility and original workflow test bodies remain byte-identical to the prior
candidate. No fake native module, callback authority, public API, test gate or
lint suppression was added. The nine-case standalone Bank policy fixture still
includes its same source independently.

The unchanged real Quest assembler produced ten outputs (nine packets plus the
index); only three differ, and only seven digest leaves (4+1+2). The exact
NPC/Dialogue-link packet, supplementary recipe index and root packet index were
copied from that checked proposal. Predicate/hold/admission/summary/semantic
fields and all seven other outputs are unchanged. The subsequent full 953-test
diagnostic found one additional talk-NPC witness stale only in two NPC input
SHA leaves; its original unchanged producer supplied that fourth measured
packet. Root copied only those two digest updates, with all other parsed
semantic/summary/matching/hold/admission fields identical. The diagnostic
1 failure/31 errors/8 skips is retained, never relabelled green; missing baseline
fixtures were restored from exact C before the final unchanged full run.
There is no source/content
regeneration or Quest rule change hidden in these evidence updates.

Current evidence: `/workspace/oteryn-prep/npc-econ-1-slot-2-evidence/`. Fresh
original native unit Clippy and normal cfg(test) native rlib passed; all 33
unchanged selected native bodies and eight unchanged real PG17.6 cases passed.
The complete original `interaction_workflow` target passed strict original
Clippy and all 15 tests (14 target bodies plus one inlined module test), with
its original test file unchanged. The original PG wrapper metadata also passed.
Exact source/extern hashes and test-body/dispatch maps are retained. The real binding `--check` and all 11 binding unit tests passed without changing
their producers, assertions or registered targets. The final original Quest
runner completed its first 30 commands, then exited 1 after discovery ran 987
tests with one error and eight skips: an exact-C island shard was absent.
That failed orchestrator result is retained. Only the missing 56,788-byte
tracked shard was restored; its affected original class passed 8/0. Independent
consumer/source closure proved this restored Area input affects none of the
30 earlier commands. The exact complete original discovery vector was then
resumed, with no producer/assertion/source/test-selection change. Thus all 31
original vectors are qualified by explicit source-bound resumption; this does
not claim that the earlier whole orchestrator returned zero.
Missing tracked baseline check inputs were restored from exact candidate C,
with per-file consumer/SHA proof; earlier missing-input failures remain logged.

The first native setup attempt stopped before Cargo/tests because a retained
PowerShell script used checkout CRLF bytes while the literal Gitblob used LF.
Only the outside validation snapshot was rematerialized from exact C after
breaking the historical hardlink; prior evidence and candidate bytes were not
rewritten. This is a setup failure, not a behavioral RED or passing result.
The actual published CI errors supply the compile RED for this repair. A later
native archive attempt exhausted disk space before any core/PG case ran; its
ENOSPC failure remains recorded. After verified completed-artifact archival, the
same source and captured extern hashes were checked and that exact native stage
resumed successfully, retaining the earlier strict Clippy result.

All eight successor paths are outside the World compiler input set. All 1,448
qualified compiler-input Gitblobs and all four pins remain exactly the prior
qualified C; available physical inputs were checked too. No unchanged Bundle
rebuild is claimed or required. The shared protocol/resource/Cargo/protected-PG
tail remains unapplied under its CP/House release gate. This remains an owned
draft packet; fresh exact-C CI, independent architect review, protected-main
composition and CP acceptance remain mandatory before ready/queue. Final
governance, 124 agent tests, whitespace and exact-eight-path checks pass.

The source-bound composite qualifies all 31 original vectors;
the resumed complete original unit discovery ran 995 tests with 0 failures, 0 errors
and 8 recorded skips. These skips remain explicit. Completed slot-2 native
libraries and drivers were losslessly archived only after all native/workflow/PG
qualification, with raw/gzip/decompressed hashes, no-active-consumer checks and
exact restore commands in `completed-custom-artifact-archive.json`. Sources,
logs, bridges, captured externs and shared dependencies remain retained.

### CP D1160/D1122: canonical NPC Bank case registration (2026-10-16)

Status NOT_FULLY_QUALIFIED. CP comment6224912485 grants minimal own test-module
registration without a shared slot. Register support/npc_econ_bank_postgres_cases.rs
in the existing character_authority_postgres root; the standalone wrapper and canonical
root now name the same actual eight native case bodies. This supersedes the historical
held aggregate-registration proposal only. No case body, harness, production runtime,
protocol field, value-line registry, schema, capability or other shared hunk changes.
New-head canonical PG/CI: NOT_RUN, not a new PASS. Existing native component evidence
is retained for its tested source/context. D1122 comment6224930201 allows one coherent
existing-draft checkpoint after fmt/diff; the protocol/resource slot still follows #2163.

### Main merge and Bank stage order (2026-10-17)

Main (cecbfda3) was merged at 5ef65a81, and the migration is renumbered 0114.
CP chose option A for composing with NPC-CONTENT-2: the Bank admission now runs
after `npc_presence::apply`, on the presence package
(`npc-content-2-presence-20261015-r1`), instead of inside the ranged stage. The Bank
packet's `from_project_revision` and `before_npc` now name that package. Paulie's
five presence fields come from the tracked post-presence declaration and equal it
exactly; the packet digest is updated in all three pins. Still open: the derived
content and every pin measured from it need a cargo-capable `regenerate_content.py`
run. No regeneration has run, and no new PASS is claimed.
