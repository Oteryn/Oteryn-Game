# HUNT-1 — prey and hunting tasks (skeleton task record)

```yaml
task_id: HUNT-1
title: "HUNT-1: prey and hunting tasks"
mode: IMPLEMENT
status: local_preparation_pending_qualification
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
- `apps/game-server/src/durability/task_board.rs`
- `apps/game-server/src/durability/prey_tests.rs`
- `apps/game-server/src/gameplay_transport/prey.rs`
- `apps/game-server/src/gameplay_transport/prey_tests.rs`
- `crates/protocol-oteryn/src/prey.rs`
- `crates/protocol-oteryn/src/prey_tests.rs`
- `apps/game-server/tests/prey_postgres.rs`
- `content/rulesets/progression/prey/ and content/rulesets/progression/task-board/ authoring inputs`
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
