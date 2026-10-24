# PHASE-SPLIT-C — extract ordered private phases

```yaml
task_id: PHASE-SPLIT-C
title: Split native combat, project validation, spell staging and reconnect preparation
type: refactor
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
pr: null
base_sha: 670633ad93893a7d7e82bec64481ed878d8cc1a7
head_sha: null
owner: Codex standalone PHASE-SPLIT-C worker
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane.rs
  - crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane/turn.rs
  - crates/gameplay/encounters/src/bosses/native_combat/monster_combat_lane/turn/proposals.rs
  - crates/gameplay/content/src/content/project/v2.rs
  - crates/gameplay/content/src/content/project/v2/state_validation.rs
  - crates/gameplay/entities/src/carrier/runtime_actor_spell.rs
  - crates/gameplay/entities/src/carrier/runtime_actor_spell/staging.rs
  - crates/gameplay/durability/src/durability/admission_journal.rs
  - crates/gameplay/durability/src/durability/admission_journal/prepare.rs
  - docs/agents/tasks/PHASE-SPLIT-C.md
public_contracts: []
```

## Task and scope

Owner-authorized structural extraction only: preserve public paths, SQL, errors,
effect order, transaction boundaries, test names and assertions. Work stays in
this worktree; no Git mutations or remote publication are authorized. The checkout
is detached, so the task remains in `implementing` without a canonical PR.

PROVEN: resolved the bound META policy at commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, version 3.1.0.
Read the root/path instructions, context router and playable-first policy.
The suggested `conntests` reference directory is unavailable in this environment;
the implementation follows the existing sibling private-module conventions.

## Mapping old to new location

| Original function | Private phase location | Entry-point lines before → after |
| --- | --- | --- |
| `monster_combat_lane.rs::run_encounter_turn` | `monster_combat_lane/turn.rs`: due thinks, per-actor selection/scheduling, time returns, player condition ticks; `turn/proposals.rs`: summons, proposal routing, self defenses, attacks | 801 → 170 |
| `v2.rs::validate_v2_state` | `v2/state_validation.rs`: identity collection, declarations, authoring, worlds, placements, assets, sources, editor | 484 → 8 |
| `runtime_actor_spell.rs::stage_spell_batch_inner` | `runtime_actor_spell/staging.rs`: cloned successor, ordered effects, creature effect application | 431 → 91 |
| `admission_journal.rs::prepare_internal` | `admission_journal/prepare.rs`: encoded binding, session preparation, existing-attempt disposition, epoch opening, prepared-attempt reservation | 408 → 14 |

All extracted functions are at most 225 lines. New Rust files contain 412–563
lines. Existing test modules stay in place, byte-for-byte. The new state types
are private; their constructors, destructuring sites and enum arms were searched
across `apps/`, `crates/` and `tools/` (`state-sites.txt` in the evidence directory).
There are no new public APIs or trait methods.

## Authority/recovery qualification and reference parity

NOT_APPLICABLE to new behavior: this task adds no authority rule, mechanic,
wire format, SQL statement or mutation operator. Existing independent continuity,
session and generation checks stay in their original order. Spell effects still
change cloned slots. PREPARE still starts one transaction and retains every
commit/rollback path, including rollback on capacity exhaustion and commit after
recording a stale attempt. SQL string literals are byte-identical to the original.

## Validation

Evidence directory: `target/PHASE-SPLIT-C/` (local, ignored build evidence).
The following commands use the absolute queued wrapper; offline mode is supplied
by that wrapper. PostgreSQL is configured only by the wrapper; no URL is recorded.

### Inventory

Before and after commands (each output saved as `<crate>-before.txt` or
`<crate>-after.txt`, stderr separately):

```bash
for crate in encounters content; do
  /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-$crate --lib -- --list
done
for crate in entities durability; do
  /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-$crate --features test-support --lib -- --list
done
for crate in encounters content entities durability; do
  diff -u target/PHASE-SPLIT-C/$crate-before.txt target/PHASE-SPLIT-C/$crate-after.txt
done
```

PROVEN: all four diffs are empty, including full test paths and benchmark counts.
No integration test target is changed. The initial entities invocation without
`test-support` could not compile its existing content test helpers; the existing
feature enables those helpers without changing code or tests.

| Crate | Before | After |
| --- | ---: | ---: |
| encounters | 27 | 27 |
| content | 346 | 346 |
| entities | 168 | 168 |
| durability | 435 | 435 |
| Total | 976 | 976 |

### Exact test commands

```bash
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-encounters --lib -- bosses::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --lib -- content::project::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --features test-support --lib -- carrier::runtime_actor_spell::tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --features test-support --lib -- durability::schema::contract_tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --features test-support --lib -- durability::schema::terminal_replacement_postgres_red_tests:: --nocapture --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --features test-support --lib -- durability::db::runtime_scope_identity_red_tests:: --nocapture --test-threads=1
```

Results: encounters 20 passed, 7 pre-existing fixture-dependent ignores; content
25 passed; entities 26 passed; durability contract tests 8 passed. PostgreSQL
terminal-replacement tests 20 passed; runtime-scope identity tests 13 passed.
Both PostgreSQL filters ran against the wrapper-configured database, with no
missing-configuration early returns or ignored cases. Total: 112 passed.

### Formatting, lint and governance commands

```bash
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo fmt -p oteryn-gameplay-durability -- --check crates/gameplay/durability/src/durability/admission_journal/prepare.rs
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-encounters -p oteryn-gameplay-content -p oteryn-gameplay-entities -p oteryn-gameplay-durability --features test-support --all-targets -- -D warnings
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
python3 tools/repository/validate_repository_policy.py
git diff --check
```

PROVEN: fmt and clippy pass. Vendored Tokio/SQLx emit existing dependency warnings;
none originate in the four selected crates. Explicitly check the new PREPARE
module as well because the durability module registry uses `include!`.
`python3 tools/agents/validate_governance.py`: pass (22 documents, 9 lanes).
`python3 -m unittest discover -s tools/agents/tests`: pass (129 tests).
`python3 tools/repository/validate_repository_policy.py`: pass (23 files, 68 workflows).
`git diff --check`: pass. Function/file line counts are in `function-sizes.txt`.
There is no separate source file-size checker in this checkout; all new Rust
files are below 600 lines. `structural-review.txt` records unchanged existing
code outside the extracted functions and byte-identical SQL/error literals.

## Self-review

Implementing agent re-read the complete structural change against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Checked all early exits, proposal
priority, allocation/effect ordering, original SQL/binds and transaction exits.
No tests/assertions changed and no out-of-scope files changed. Private state-site
search covers all added fields and all `PrepareEpochOutcome` arms. No new public
function requires a new dispatch caller or E2E surface.

## Unfinished

- Seven encounter tests already carry `#[ignore]` for retained actual source
  captures. `OTERYN_MONSTER_NATIVE_CAPTURE_ROOT` and
  `OTERYN_FULL_SPELL_TEST_MANIFEST` are absent; no fixtures or assertions were
  substituted. These explicit fixture runs remain for the fixture-equipped lane.
- No commit, push, PR, freeze or CI claim: publication is outside this worker's
  authorization.

## Next step

Hand off the qualified local diff to the owner/control plane for the separately
authorized publication lifecycle and retained-fixture validation.
