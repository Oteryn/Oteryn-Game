# HUNT-1 — prey and hunting tasks (skeleton task record)

```yaml
task_id: HUNT-1
title: "HUNT-1: prey and hunting tasks"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row HUNT-1 (rows 45, 46); §7 'HUNT-1, RANK-1' PREY0 rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/hunt-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Prey slots, resource and wildcard balances, ledger, draws, hunting-time checkpoints (PREY-1); gold reroll fee; prey effects; Task Board, delivery, shop; `PREY_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

PREY-CONTENT-1, PREY-1 (hard), PREY-FEE-1, PREY-EFFECT-1 (combat), TASKBOARD-1 (hard), TASKBOARD-DELIVERY-1, TASKSHOP-1, PREY-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-MONSTER-1 (family entry gate) | BLOCKING (Wave 2) |
| PREMIUM-1 | MERGED (#2110) |
| CHAR-REV-SEQ-1 | MERGED |
| GOLD-FEE-1b | MERGED |
| ATTACK-0 / SPELL-TARGET-1 | ATTACK-0 ACCEPTED; SPELL-TARGET-1: verify |
| ITEM-USE-1c (TASKBOARD-DELIVERY-1) | BLOCKING (Wave 0 residue) |
| acceptance of PREY0 | Effective acceptance #2209 on main e2da9805 |
| PREY-CONTENT-1 (direct PREY-1 dependency) | BLOCKING: ruleset directory is READY_UNPOPULATED; admitted grade/list/preyable tables absent |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/prey.rs`
- `apps/game-server/src/domain/task_board.rs`
- `apps/game-server/src/durability/prey.rs`
- `apps/game-server/src/durability/prey_audit.rs` (D1341c; native event codec)
- `apps/game-server/src/durability/character_authority.rs` (D1336a/D1341c; only the PREY receipt UNION, inseparable from migration 0115)
- `apps/game-server/src/durability/task_board.rs`
- `apps/game-server/src/durability/prey_tests.rs`
- `apps/game-server/src/gameplay_transport/prey.rs`
- `apps/game-server/src/gameplay_transport/prey_tests.rs`
- `crates/protocol-oteryn/src/prey.rs`
- `crates/protocol-oteryn/src/prey_tests.rs`
- `docs/contracts/game-events/v1/prey_wildcard.proto` (D1341c; new closed event payload)
- `apps/game-server/tests/prey_postgres.rs`
- `rulesets/progression/prey/ and rulesets/progression/task-board/ authoring inputs`
- `docs/agents/tasks/HUNT-1.md` (this record: status and notes only)

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

- death chain kill receipt for task kills (gameplay_transport/kill_reward.rs)
- XP lane (D118) for the multiplier
- connection.rs dispatch; capabilities.rs `GATED` (`PREY_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + prey/task writers + durability
- reroll fee burn + ledger; task shop debit + item MINT + ledger
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Weekly reset settles the finished week under one barrier (TASKBOARD obligation a).
- [ ] Replay-stable overflow policy at `PREY0-RL-14`.
- [ ] Hunting-time checkpoints survive restart.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server hunt
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md §3-§11
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded PREY-1 preparation (2026-10-15, draft #2149)

Branch `skeleton/hunt-1`, source `509d99e32cf4be4ff6cc2ea4f8f4d85221f4c05d`.
The lead allocated five paths, but this batch needs only `domain/prey.rs`,
`durability/prey_tests.rs` and this record. No durability writer or PostgreSQL
test is added: the full state writer needs shared sequencer, receipt/event,
consistency-guard and migration integration at the CP slot. This preparation
does not create a knowingly uncommittable parallel writer or schema.

### Accepted basis and direct content gate

The lead established effective ACCEPT-BATCH #2209 at main
`e2da9805d296a27b06b32335bf67394509d4910b`. PREY0 §§4–6 and §12 govern this
slice. Its historical CANDIDATE marker on the skeleton does not override the
integrated acceptance. Native Premium policy requires current authority at use;
PREM-1 is delivered, so PREY0's historical pre-delivery slot-2 opening is not used.
The helper takes the actual native `PremiumGate`, with no boolean override or
`NotActivated` branch. Future production calls must remain inside the existing
`with_premium_gate` seam, using the admitted controller's Account, trusted time
and current producer/fence classification; synthetic gate inputs are not evidence
that a live account holds Premium.

PREY-CONTENT-1 is a direct PREY-1 dependency, independent of the skeleton's
LOOP-MONSTER-1 family-entry gate. Both allocated HEAD and accepted main contain
only `rulesets/progression/prey/index.json`, marked `READY_UNPOPULATED`. Creature
`prey`/`exclusive_prey` source flags do not constitute the admitted pool, level
bands or grade table. Those tables, the 27-Creature generation gate, deterministic
RNG purpose/binding and activation remain unimplemented. No inferred pool, grade,
fee, random draw or random-unit conversion is introduced. Fixed 1/5 wildcard costs
and cap 52 below are explicit accepted §5.2/§12 values.

### Prepared nonrandom rules

- Validate the explicit slot 1–3 before every other command check. A valid current
  build must have a vocation other than `none`. Slot 1 needs no Premium; slot 2
  needs the current native Premium gate or its own permanent Account+World fact;
  slot 3 always needs its separate permanent fact. No Store fact is minted.
- An Account+World snapshot cannot lend balances or permanent facts to a different
  Account or World. An initialized slot projection must bind the exact Character
  and requested slot. An absent slot remains `SLOT_LOCKED`; commands never perform
  first-list initialization. Existing state is not cleared when access is locked.
- `set_option` checks at least 1 held wildcard for Auto Reroll, 5 for Lock, and none
  for clearing the option. It returns a before/after option plan with unchanged
  balances, never a debit or reservation. No extra ACTIVE-only restriction is
  invented; the owner must supply an initialized, already validated slot.
- A separate arithmetic-only debit helper has closed accepted spend kinds, costs
  1 or 5, exact checked subtraction and a validated 0–52 balance. It includes native
  acting Character, Account and World bindings. It does not authorize a command,
  draw or expiry, implement insufficient-funds expiry fallback, grant resources,
  create ledger entries or claim durable idempotency. No arbitrary amount, credit,
  Store delivery, paid fee or test-grant API exists.

Native `AccountId`, `CharacterId`, `WorldId`, `DurableBuildState` and `PremiumGate`
are reused. No parallel identity, Premium classification, content framework or
durable schema is created. The narrow slot/Account projections are caller data,
not authority tokens. Matching IDs do not establish current ownership, liveness,
generation, entitlement provenance, compatible active content or transaction time.

### Required integration (REPORT_ONLY)

The future owner must load and validate current live Character→Account+World
binding, current build and initialized content-compatible slot under the owning
transaction locks. It must read the native current Premium gate at use, preserve
closed behavior for lapsed/unavailable/conflicting status, and read permanent
facts only from admitted Account+World storage. Known missing balance rows mean
zero; an unknown/failed read must not be replaced with a zero snapshot. Account
facts are shared across that Account's Characters on that World and never copied
as Character balances. World and Channel remain distinct.

All real writes still need the Character sequencer and exact receipt kind/guard,
the command's explicit slot/cause/occurrence binding, current session/recovery
fences and exactly one CharacterRevision advance. PREY0 §10 lock order remains:
occurrence; Character root/revision; prey/task rows; Account wildcard/unlock rows;
then item/bank rows where applicable. Recompute debit and option requirements on
the locked current snapshot. A prior option check reserves no balance against a
second Character. Actual debit must atomically compose one SPEND ledger entry,
wildcard event/outbox row and Character receipt; replay/conflict behavior comes
from that writer, not these pure plans.

Before draws, initialization, selection, active bonuses, hunting checkpoints or
expiry can run, PREY-CONTENT-1 must provide admitted tables and the owner must
integrate existing RNG, clock and server-originated fences. PostgreSQL atomicity,
two-Character shared-balance serialization, replay, restart/checkpoint recovery,
Premium expiry/renewal, active-generation changes and outbox/retention tests remain
future slot work. Wire/registry/migration/shared wiring remain unmodified.
Independent exact-head review and CP/architect acceptance are still required;
neither PREY-1 nor HUNT-1 is complete.

### Focused validation and self-review

- Source-bound direct `rustc --test`, importing the actual native server library:
  **11 passed, 0 failed, 0 ignored**. The external three-line harness path-loads
  only the owned module; it copies no native types, Premium logic or source excerpt.
  Its cfg-test inclusion loads the actual owned `prey_tests.rs` once.
- Cases cover slot/error priority, vocation eligibility, native current/closed
  Premium gates, distinct slot entitlements, Account/World/Character/slot binding,
  absent initialization, exact option holds with no debit, locked-row preservation,
  all four closed spend costs, zero floor, 52/53/u32-max boundaries and a second
  Character's changed shared balance. Fixtures are synthetic pure inputs, not
  production authority, Store claims or executed concurrency/expiry tests.
- Strict `clippy-driver --test --emit=metadata` with repository deny lints,
  owned-file `rustfmt --check`, `cargo fmt --all --check`, governance validation
  (22 documents, 9 lanes), **124 governance tests** and `git diff --check` passed.
  Rust commands used the shared flock/target, two jobs and `CARGO_INCREMENTAL=0`.
  No Cargo build, implicit server-bin link or missing-file RED ceremony was run.
- Retained real-library hash before/after is
  `75fc517e5ad7d15462174567fc868ab14d22ea34e24605843e40040b05c0f6f5`.
  Used native type/behavior sources are byte-identical between this allocation,
  accepted main and that library build's baseline
  `aaeffba3e6982ba721afa147b82fd8242731ddff`. This is focused native-type
  qualification, not a full package, live Premium, PostgreSQL, ledger/outbox,
  RNG/content or restart qualification.
- Actual commands/logs, harness, test ELF, source hashes and local patch are
  retained in `/workspace/oteryn-prep/hunt-1-evidence/`; `provenance.json` binds
  the exact sources, library and results. No native binary/cache was deleted.
- Whole-diff review confirms only the three necessary allocated paths, no staged
  content/commit, no old Premium bypass, no arbitrary resource credit or amount,
  no silent balance clamp, and no shared registration, migration, content/pin,
  registry or runtime-write change. Matching facts remain separate from current
  authority; all actual writer/fence/admitted-content gates above remain open.

Final Rust SHA-256:

- `domain/prey.rs`: `7723fc7778db7e8eb31c5133674deb0b3989b4dac826dcb5808b93a5977c7b34`.
- `durability/prey_tests.rs`: `342d6c45214b216887bca9bdff68610257a50cdd8ea578ac78789f529c2ea9a4`.


### CP review repair: domain test placement (2026-10-16)

The minor finding in CP comment 6216371834 is addressed by placing all 11
existing pure domain tests inside the owned `domain/prey.rs` test module.
The cross-layer `#[path = "../durability/prey_tests.rs"]` include and its
durability file are removed. This uses only already allocated paths; it
adds no domain-file allocation, registration or runtime behavior.

The production prefix is preserved byte-for-byte; all 11 test bodies are
unchanged apart from module indentation and rustfmt line wrapping. Historical 11/11 native
results above remain evidence of the predecessor only. Fresh package Clippy
and tests remain NOT_RUN; no PG/content/runtime coverage is claimed.
The writer, accepted PREY-CONTENT-1, reset, content-path allocation and
slot-only gates described above remain open. This repair does not complete HUNT-1.


### CP D1151/D1122 checkpoint

AUTHORING resumed for the checkpoint record after the reviewed test-placement
repair d6d17ca06c6cc03b569944f3082a33470acbb99f. Its Rust sources are unchanged.
CP6224947337 corrects the two rulesets authoring paths and allocates
PREY-CONTENT-1 plus its schema/compiler leaves in `tools/content-schema/` to
this PR; content remains subject to CP review and Reference provenance.
The LOOP-MONSTER caller belongs to #2200, consuming this PR's API.

NOT_FULLY_QUALIFIED under D1122: source review, fmt, diff and governance124
PASS; fresh Cargo all-target Clippy and package/PG tests NOT_RUN. This
checkpoint preserves the 11 existing test bodies and makes no fresh runtime
PASS claim. D1160 module registration, writer/content/compiler/API and the
rest of HUNT-1 remain unfinished; shared hunks follow the J order.


## PREY-CONTENT-1 loot-budget compiler preparation (2026-10-17)

This coherent source batch starts from draft #2149 at
`4ce908e266db03dc767b016bc7bc6a93dd1991ec`. CP D1151 in
[CP6224947337](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224947337)
allocates the PREY-CONTENT-1 authoring and schema/compiler leaf to this PR.
PREY0 §3, §6.1 and `PREY0-RL-16` are the accepted basis. The historical
predecessor evidence above is preserved; this section records the new leaf only.

The existing native loot producer now accepts a closed authored PREY provider
containing the complete exact Creature pool and SHA-256 pins of the emitted
Creature-profile and Loot-section bytes. Supplying the provider requires the
progression/loot path and mandatory qualification before native payloads or the
manifest are written. Unknown eligibility, absent canonical definitions, stale
pins, incomplete pools and unsupported table shapes refuse. Eligibility in the
native generation is checked against actual canonical Creature authoring; the
source flags alone do not admit production PREY content.

The compiler refuses a pool below 27 Creatures and any preyable final emitted
table above seven entries. Repeated Item references and zero-chance entries still
count; oversized tables cannot be silently omitted from the declared pool.
Accepted PREY0 then bounds two rolls plus the improved-loot trigger to 29 RNG
draws and 14 output entries, subject to the existing native Item, quantity and
byte qualification. Ordinary native compilation without a PREY provider retains
its existing payloads. The source-closure proof explicitly records
`PREY0-RL-16_BUDGET_ONLY` and `runtime_activation: false`.

### Source-bound validation of this leaf

From `tools/content-schema/native-gameplay`:

```bash
python -m unittest test_prey_loot_budget test_full_spell_artifact.LootTablesTests.test_null_binding_is_explicit_and_a_missing_item_refuses
```

**21 passed, 0 failed**: 20 new synthetic-content regressions through the actual
native producer and one existing producer regression. These cover the seven/eight
entry boundary, complete-pool enforcement, exact source hashes and revisions,
canonical/native eligibility disagreement, missing canonical authoring,
duplicate identities, unknown entry/algorithm facts, the 27-Creature minimum and
mandatory provider wiring. The canonical table-level-alternative regression first
failed because the producer silently discarded that field; it now passes with
the mandatory pre-projection shape check. Its ordinary-production comparison
also preserves output without a PREY provider. An independent source reviewer
reran the predecessor's 19 cases successfully. Python AST, provider JSON syntax
and whitespace checks pass.
These results do not replace the historical Rust evidence or claim fresh native
Cargo, PostgreSQL or production-content qualification.

The positive fixture now includes the actual canonical Loot discriminator,
`kind: "Loot"`, alongside `identity`, `algorithm` and `entries`. Independent
source review caught the initial three-field canonical check rejecting this
valid field. Its regression first failed on that valid canonical shape, then
passed after accepting the exact discriminator while rejecting missing/wrong
kinds and unknown/alternative fields. Native emitted tables retain their
existing three-field shape.

Root's first full-clone related-suite run of the predecessor returned 22 passes
and one existing checked-in Loot-section mismatch. Root compared the exact base
producer blob `49c460ae64ea883d1ce42daf3e81ddaeb9ebf6e5` with that candidate: both
emitted SHA-256
`01905f8e6b6cfb28118eb54fecb1274f3a0052c5631c2ecdbf33b021b31e5d3a`.
The checked-in artifact differs in 86 Item `materializable` flags. This is
inherited content drift; no shared artifact regeneration is included here.

### Remaining prerequisites

`PREY-CONTENT-1` is only partially implemented. The authoring directory remains
`READY_UNPOPULATED`; no production pool, level bands, grade/draw tables or random
purpose/binding policy has been authored or admitted. This compiler qualification
neither admits content nor activates Prey. CP/Reference review, the remaining
content bundle, migration/persistence writers, LOOP-MONSTER caller integration,
runtime bonuses, draw/checkpoint/expiry behavior and wire/registry/activation
gates remain open. Shared integration follows the J order. HUNT-1 remains
`NOT_FULLY_QUALIFIED` and draft; merge authority remains CP-only.


## Pending DAILY wildcard sink authoring (2026-10-17)

This source batch starts from HUNT draft #2149 at
`c86ad8ab7943f4a0d1bc6697dec1de1cf6f7fd0b`. Its accepted basis is PREY0
§4.1, §4.2 and §10, DAILY_REWARD0 §4.5, and CP
[D1319](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232842478),
[D1336](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233232470),
[D1338](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233263821) and
[D1341](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233469694),
as [narrowed by CP](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233471923).
The historical source and qualification evidence above is retained. The three
owned-path additions record the explicit CP grants; they do not waive their
inseparable integration conditions.

### Implemented source scope

`DurabilityRoot::deliver_pending_daily_wildcard` consumes an existing retained
DAILY claim and `PREY_WILDCARD` line. The request carries the claim/line and
planned delivery/event identities, never a caller-supplied quantity, Account,
World or credit. A new delivery validates native current recovery, node,
session, lease, scope and CharacterRevision evidence in its writing transaction,
then locks the actual source and Account+World balance. Unknown, mismatched or
terminal-without-receipt source state refuses instead of inventing eligibility
or credit.

The writer credits `min(quantity, 52 - balance)` and records the exact excess.
Its receipt, positive ledger entry, balance/head, positive audit event, terminal
DAILY line and one CharacterRevision advance commit together. A zero credit at
the backed cap still terminalizes the line and writes one receipt/revision;
it creates no balance-change entry or event. The accepted once key is the actual
`(claim transaction, line)`. Replay reads the retained receipt, returns the
winner's original delivery identities and result, and never reacquires gameplay
authority or repeats a grant. The binding excludes the retry cursor and newly
planned delivery/event identities.

The migration has typed immutable receipts and ledger history, same-transaction
entry/event/source proofs, exact balance head transitions, and closed privileges.
Its progression guard extends the complete effective
`0093_character_appearance.sql` body, retaining all prior arms, including Wheel
and appearance. The native recovery-integrity hunk adds only the corresponding
PREY receipt UNION. Both belong to the same migration integration.

D1341 assigns `PREY_WILDCARD_CHANGED` **event type 6**, schema 1, and
`0115_prey_daily_wildcard.sql`. The closed payload admits positive `DAILY_REWARD`
only. D1338 binds the new `PREY_WILDCARD_LEDGER_RETENTION_V1`, policy revision 1,
restricted player-linked class and **2,592,000 elapsed seconds** from the
immutable trusted event timestamp. Only event/outbox envelope and payload
expire; balances, ledger, unlock facts and once receipts do not. The outbox
preserves original expiry, immutable content and one-way publication; bounded
expiry and operator-only hold placement/release cannot rewrite the retention
clock. Case authorization, audited access/export and runtime operator integration
remain separate qualification work. `ECONOMY_LEDGER` is unchanged.

### Qualification status and exact integration prerequisites

New native source qualification is **NOT_RUN** at this authoring checkpoint.
The five native PostgreSQL cases cover cap/excess/zero delivery and receipt
replay/reopen; current fence and source refusals; rollback at the terminal write
and deferred missing-event proof; two independent connections on the once key;
and immutable history/outbox plus bounded expiry and hold-storage behavior.
Four real prost codec tests cover the closed payload, supported envelope,
maximum source values, byte overflow and stale/unknown metadata. The calculated
ceilings are **191 payload bytes**, a conservative **500 envelope bytes**
reserving the full uint32 event-identity width, and **64 events per expiry call**.
D1341 keeps these resource rows CANDIDATE until those tests run natively; no
calculation is relabeled as an executed Rust, PostgreSQL or runtime result.

The review base is main `f010472bcb6794a2cbc56dfaf755bd122c10225e`.
The current module file, both registries, authority source and existing Bestiary
entry were freshly read there; their source blobs match the earlier pinned
`cecbfda3` versions. Registry changes are additive only. Publication must still
re-read then-current main and keep the numeric allocations and existing entries
intact.

Migration 0115 references the actual DAILY claim/line relations. Installing it,
the authority UNION and shared mounts remains **HELD until the DAILY schema
prerequisite is satisfied**. The explicit native qualification assembly consumes
`daily_reward::DAILY_REWARD_SCHEMA_CANDIDATE` directly from the byte-exact real
DAILY module, with its real domain/store/test files and two validation mounts.
It reproduces no DAILY DDL or guards. Its pending claim/line rows are synthetic
source data; it does not replace the claim writer, committed reset-month port,
Inbox delivery or an installed DAILY migration. The HUNT schema itself must be
installed by the actual embedded SQLx migrator with the exact 0115 checksum.

The existing `--test bestiary_progress_postgres` root loads the real durability
modules and their tests; its existing cases remain unchanged. No new wrapper,
Cargo target or Cargo manifest delta is required. The normal two-line HUNT
module registration makes the five cases available in that entry through the
owned test module. Qualification uses the authorized shared PostgreSQL 17.6
wrapper and is bound to the complete explicit source assembly.

The native `RevisionSlot` adapter and sixth retry/replay-cursor case stay held
under D1336b until #2151 merges, then reconcile with current main and #2193.
They are separate integration material, not a disabled test or speculative
alternate sequencer. This checkpoint adds no runtime caller or claimed current
session lookup. LOOP integration remains with #2200; the DAILY owner controls
claim production and source staging.

HUNT-1 remains draft and `NOT_FULLY_QUALIFIED`. Production content/grade/draw
admission, other earning/spend/unlock sources, slots, active bonuses,
checkpoints/expiry, Task Board, wire/transport and runtime activation remain
open. The source above is the bounded pending-DAILY sink; it does not complete
PREY-1 or the HUNT family. Merge and ready authority remain with CP.

## Independent source review and post-merge checkpoint (2026-10-17)

CP [merged current main into HUNT](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233702121)
without installing the held packet. The publication predecessor for this
checkpoint is now `69c1a38144944a63db81332786cc4f3987e19244`, with parents
`c86ad8ab7943f4a0d1bc6697dec1de1cf6f7fd0b` and
`f010472bcb6794a2cbc56dfaf755bd122c10225e`, and tree
`5239d127c2a59577d3cb6ebf949c558d5c371e5f`. The earlier authoring predecessor
above is retained as history. Fresh GitHub reads confirm that the published
task card still has blob `911cf3bb13e9f1c6a053eb96c1d15bc4fb8507d3`; the three
new owned Rust files and the new event proto remain absent. The five owned
preimages therefore still match the reviewed source packet.

An independent nonauthor review by `/root/ach_rank` returned **PASS: no concrete
blocking source defect found in the bounded pending-DAILY sink**. It binds the
original ten-file handoff, SHA-256
`0e8986c9646b6806c28b9700917276d4f136ccaf8532886c911f650afff42840`,
and complete patch, SHA-256
`8d71e0d2e9a3cb30fbe4664c1b7df5bcd8c0ac96cd68de4bfbfcf8a5dd21c6ff`.
All ten source hashes and their existing preimages were checked. Review covered
the actual live-fence and retained-receipt paths, same-transaction
receipt/entry/balance/event/terminal-line proofs, the complete effective 0093
guard extension, closed native payload/envelope encoding and the original
fixture linkage. The four owned Rust/proto source blobs and every held
integration hunk remain identical to that reviewed packet. This new appendix
preserves the entire previous candidate card as an exact byte prefix.

This is source review, not executed native qualification. The five PostgreSQL
cases and four prost cases remain **NOT_RUN** at this checkpoint; fmt, strict
Clippy and full-package/runtime qualification are not claimed. The authored
retention case checks early expiry refusal and hold storage/placement/release.
It does not qualify actual expired deletion, an active hold on an expired event,
or case-authorized operator access/export. The 191/500-byte and 64-event resource
bounds remain CANDIDATE pending their required native qualification.

This five-path authoring checkpoint leaves migration 0115, the inseparable
authority UNION, shared module registration and both registry changes held.
Installing them still requires the actual DAILY schema prerequisite and the
current CP integration slot. The sequencer adapter and sixth case remain
deferred until #2151 merges, then require current main/#2193 reconciliation.
Fresh main and exact numeric allocation checks remain publication gates.
The merge and this source-review PASS do not satisfy those gates or authorize
runtime activation. HUNT-1 remains draft and `NOT_FULLY_QUALIFIED`; ready/merge
authority remains CP-only.

## Formatted owned source checkpoint (2026-10-17)

The three owned Rust files were formatted on Molehill-PC using the repository's
Rust 2024 formatter, scoped with `skip_children=true`. This includes removal of
the previously recorded extra EOF blank line in `prey_tests.rs`. Root reviewed
the complete formatting delta; its SHA256 is
`393e786bd8f3909a9e3c9ce89c691146bca248332160d4d3c1ac7102b5a0c490`.
The formatter changed layout only. The earlier frozen source hashes remain
historical; the exact formatted inputs in this checkpoint are:

| Path under `apps/game-server/src/durability/` | SHA256 |
| --- | --- |
| `prey.rs` | `d540a34b1a47142be1e5dedcf8c87c78dc211e9874d9914724ca77e72cb18030` |
| `prey_audit.rs` | `86045b426c698e3a1544fa15df28b9426925f5204d82d25b0249b0cf9dce67ca` |
| `prey_tests.rs` | `eef85282670033d2d39bba37ff6e2406b7e2d1df4aebaaf2fae03e4f3adecc9f` |

The event proto remains byte-identical to the reviewed D1341 packet. This
five-path source publication uses CP's `69c1a381` predecessor. Its complete
authority/module/registry/Bestiary entry and current 0093 preimages were checked
and preserved; migration 0115 is absent. All earlier text above is retained.

Scoped source formatting is PASS. The five PostgreSQL and four prost cases,
strict Clippy, workspace Cargo fmt and runtime qualification remain NOT_RUN for
this combined pending-DAILY source packet. Publication does not install the
held migration/authority/module/registry bundle or the sequencer/sixth case.
The actual DAILY schema and CP integration conditions above remain required.

## D1398 audit source checkpoint; D1392 native blocked (2026-10-17)

**Current status: HUNT remains `NOT_FULLY_QUALIFIED` and draft. The revised
D1392 combined assembly has native qualification `NOT_RUN`.**

[CP D1398](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236792716)
permits one two-file draft checkpoint: the exact reviewed audit patch
`5f5b810c…` and this append-only card. It keeps the physical disk guard at
50,000,000,000 bytes, assigns space recovery to CP and the owner, and suspends
new full PC Cargo/PG builds until CP announces another heavy window. This
checkpoint makes no readiness or native-pass claim.


This appendix preserves every byte of the prior task card at HUNT
`0d4a1483fdb19327aae3803e6ae893575a7878dd`. The current source checkpoint changes
only `apps/game-server/src/durability/prey_audit.rs`: the payload type and
envelope decoder become public under
[D1392](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235803221),
and the oversized-payload codec assertion uses a fixed array slice instead of
an unnecessary vector. Payload fields, encoded bytes, decoder behavior and
the pending-DAILY writer are unchanged. The complete published `prey_tests.rs`
and `prey.rs` remain intact in this checkpoint.

The audit candidate has SHA256
`ec3356e76f8d52feea7276c0730e737e9163d241fffb391d73ca921c6ae3460f`
and Git blob `4ff62d6dbf78cb22eb243784e239c0ef9efc0441`. The source handoff is
`77b69314d85624f9dd7afafc3c84c7116f4e316430a29313d748a7f25e5c01d1`;
the audit-only publication projection is
`5f5b810c77adca69fa0ba3cef2f4d4aaf00a3a168981820a74098fe48c043bee`.
The whole extraction patch is separate local integration material.

### Granted PG placement and source preservation

D1392 moves the five existing PostgreSQL cases into
`tests/support/prey_postgres_cases.rs`, consumed by a thin focused
`tests/prey_postgres.rs` wrapper and the existing Character authority aggregate.
Each consumer uses its canonical harness once. The support file declares no
module or harness. Its complete 19,750-byte fixture/helper/case suffix is
identical to the published original and the second native attempt; all five
complete test bodies, SQL, assertions, cleanup and names remain unchanged.
The thin wrapper preserves the exact 818-byte canonical Bestiary namespace
and harness block. The four codec cases remain in `prey_audit.rs`.

The separate registration packet adds the `test = false` focused Cargo target
and one cases mount after Bestiary in the Character authority aggregate.
The held durability module declaration becomes `pub mod prey_audit` under the
same decision. No protected workflow or PG allowlist is changed. The previously
proposed 17 root mounts were not executed. Root and an independent nonauthor
reviewed the exact source and preservation maps with no open source blocker;
those reviews do not supply native execution evidence.

This full PG placement remains local until DAILY #2151 merges and current main
is merged into HUNT. It is not part of the current audit-only source checkpoint.
The native assembly consumes the actual DAILY owner sources at
`6b75183305b53277d0ca0a3ab2d8bb8f12ff09e9`, the real migration 0115 and its
SQLx checksum, and the two explicit DAILY validation registrations. It does
not reconstruct the DAILY schema or replace the native writer/receipt/fence.
Published HUNT alone still lacks the held integration and DAILY prerequisites.

### Retained failed attempts

Both earlier runs remain FAILED. Their selected test successes do not form a
completed final qualification, and the later repair does not relabel them.

| Native attempt | Selected PG and codec filters | Strict focused Clippy | Strict all-target Clippy | Workspace fmt and final artifact checkpoint |
| --- | --- | --- | --- | --- |
| D1365, finished 13:54:52 UTC | Original five PG and four codec cases passed | Failed: duplicate canonical harness load and unnecessary vector | Not reached | Not reached |
| D1365-v2, finished 14:30:13 UTC | Original five PG and four codec cases passed | Passed | Failed: `crate::bestiary_postgres_harness` absent from other aggregate roots | Not reached |

The first terminal receipt has SHA256
`39ee94294c6d007a2e7cf256a977ee65ec7d3ff28900b2f2c46b7db4d57450c9`
and its complete focused Clippy log
`4516718543fd07876062edb3e74bc504adb7fb778806469323f6986efa2b04a5`.
The second terminal receipt has SHA256
`073dcfb469b67b704cd807c771deb423048a9c12618eecbc903bb83911df0b84`
and its complete all-target Clippy log
`9964d76bafe3e33017e6a28caf38d450a5a4af3fc4e10e8d01c2c94de448e856`.
Both record unchanged source after execution. Their Cargo artifact/protocol
checks completed before the failing Clippy stage; neither reached the final
artifact rehash/checkpoint or produced a final qualified receipt.

The initial D1392 invocation at 16:11:17 UTC stopped during environment
preflight because `/root/oteryn-cache` did not yet exist. Its retained raw
receipt, SHA256
`6a7575d172cb5eb99e579700fcc94fe700eb25976b3152c9fdccddbdb0758cca`,
records `commands: []` and `Unexpected D1396 heavy-lock path`. It acquired no
slot and ran no Cargo or PostgreSQL command. The real prepared assembly was
already bound by receipt
`ee87465b4b9c8480a7ceda97b6cf7856e655df956d06da12ed941e36124c9dc6`.
This was an environment refusal before any test or source check; it supplies
no native qualification. D1397 subsequently identified the actual canonical
lock in the other WSL distribution; D1397a grants an ordinary outer flock in
Ubuntu/mole around the complete Ubuntu-24.04/root runner. The runner records
that execution policy as static metadata, without claiming a measured outer
FD or lease state.

The first D1397a outer invocation at 16:33:00 UTC also stopped before the
inner runner: the canonical lock was occupied, the outer process exited 1
in 0.57 seconds, and the inner evidence directory did not exist. Root retained
this actual observation as `77f231dbf67b3f134a2761c26185812fd1384ae7d9b451b704e62bf5e8e90ffb`.
It is an outer busy refusal with native execution `NOT_RUN`; it is neither a
fourth source attempt nor a failed native test run. The reviewed source
attempt remains 3. D1397a requires no additional lease receipt.

### Actual D1392 preparation and capacity refusal

The real isolated assembly was prepared at
`/root/oteryn-six-pr-20261017/hunt-daily-current-native-d1392`, with HUNT
`0d4a1483fdb19327aae3803e6ae893575a7878dd` and actual DAILY
`6b75183305b53277d0ca0a3ab2d8bb8f12ff09e9` inputs. Preparation completed at
16:00:30.641798 UTC and verified all **31 source SHA256/Git-blob/size bindings,
15 dirty paths and 16 preserved inputs**. The original five held integration
files remain one bundle; D1392 contributes the audit/type visibility, PG test
placement and registration described above. The two DAILY module lines are
explicit local validation registrations. No substitute DAILY schema is used.

The actual preparation receipt is
`ee87465b4b9c8480a7ceda97b6cf7856e655df956d06da12ed941e36124c9dc6`,
and its saved changed-source JSON is
`e46d79e68637b2387e6309a3b2732066b5a21240d2caae22af38ecdef97583a3`.
The preparation's direct rustfmt check exited zero, its recorded source was
unchanged by that check, and the retained parent checkout was clean and
unchanged. These are preparation facts, not results of Cargo tests or of the
later full-workspace Cargo fmt stage.

After the busy refusal, the same source attempt was invoked through the
[D1397a outer command](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6236550340)
at 16:35:31 UTC. The reviewed driver was
`f25e5460cfc15b130fab356236069f53a9c7e736f979a212cd0c4ba0345c42d1`.
It obtained its own D1342 slot A and stopped at the existing physical disk
capacity guard **before Cargo and before any native source snapshot**. The
raw terminal file records `status: FAILED`, `commands: []`, and
`PC disk capacity guard`; it finished at 16:35:32.651251 UTC and records slot
A released at 16:35:32.651603 UTC.

That raw capacity receipt has SHA256
`028274cf9e30ee0a4467dc25ac0aa5205de7112c280eff893986f8b8e998008d`
(5,637 bytes). It has no `driver_sha256`, `source_after`, source snapshot or
qualified-checkpoint field, because the refusal occurred before those
assignments. The driver identity belongs to the separately retained actual
outer launch observation; no missing raw field is synthesized. In particular,
this refusal does **not** establish a native `source_after: UNCHANGED` result.

The separate outer launch/capacity observation is
`6fc1508200bbf4bd025888ea9d98ec489b00fa6fd0c4a6292f386b06534052f9`
(4,117 bytes). It records the actual outer exit 1 after 0.94 seconds and the
physical Windows C: readout of **9,700,433,920 available bytes**. The WSL
readout showed 747,142,721,536 available bytes, but Windows physical capacity
was the binding condition. D1398 confirms that the 50 GB guard remains in
force. These numerical readouts come from the outer observation and CP's
decision, not from additional fields in the capacity terminal JSON. The
observation is retained execution evidence, not a new lease protocol.

| Revised D1392 stage | Actual result |
| --- | --- |
| Source assembly and direct preparation rustfmt check | Prepared with all 31 inputs bound; format check passed |
| Canonical outer invocation | First refused a busy lock; later entered the runner and reached its capacity guard |
| Fresh locked Cargo fetch | `NOT_RUN`; zero command records |
| Five original PostgreSQL cases in the D1392 placement | `NOT_RUN` |
| Four original codec cases under the D1392 target | `NOT_RUN` |
| Explicit six-target and all-target strict Clippy | `NOT_RUN` |
| Original duplicate-source checker on all-target JSON | `NOT_RUN` |
| Full-workspace Cargo fmt and final artifact rehash | `NOT_RUN` |
| Native before/after source snapshots and qualified receipt | Not produced |

The raw capacity status `FAILED` denotes a pre-Cargo environment refusal. It
does not count as a fourth source attempt or a failed PostgreSQL/codec case.
The two earlier Clippy-failed source assemblies remain separately identified
above; their selected five-plus-four successes do not qualify the revised
D1392 assembly. No additional test run or relaxed guard is used for this
source checkpoint.

### Remaining publication and runtime gates

Only the owned audit file and this card belong to the D1398 checkpoint. The
published writer and all five cases in its original test leaf remain intact.
The complete D1392 PG extraction, thin wrapper, support cases, Cargo target,
Character authority mount and public durability module declaration stay local.
Migration `0115_prey_daily_wildcard.sql`, the authority receipt UNION, shared
durability registration and both event/resource registry hunks remain the
original held integration bundle. They wait for DAILY #2151 to merge, then
for current main to be merged into HUNT and shared preimages to be reread.
The sequencer and sixth case retain the D1336b DAILY/current-main/#2193 order.

The unexecuted five-case D1392 placement preserves coverage for capped pending
DAILY delivery/replay, source/fence refusals, atomic PostgreSQL rollback,
concurrent once-key delivery and immutable event/ledger storage. The earlier
retention case covers early expiry returning zero and hold placement/release;
it does not qualify real expired deletion, an active hold on an expired event,
or operator access/export. The **191/500/64 resource rows remain CANDIDATE**;
this checkpoint supplies no new native resource measurement or batch-deletion
qualification.

Production content/grade/draw admission, other earning/spend/unlock sources,
slots and bonuses, Task Board, transport, runtime activation and full HUNT
family qualification remain open. Space recovery and any future heavy build
window belong to CP and the owner under D1398. Ready/merge authority remains
with CP; the PR remains draft and `NOT_FULLY_QUALIFIED`.
