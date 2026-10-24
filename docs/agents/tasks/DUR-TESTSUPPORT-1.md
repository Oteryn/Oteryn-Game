# DUR-TESTSUPPORT-1 — durability integration-test source reuse

```yaml
task_id: DUR-TESTSUPPORT-1
mode: IMPLEMENT
status: published-for-review
repository: Oteryn/Oteryn-Game
branch: claude/happy-davinci-ijz3wy-durtest
base_sha: fcd048da6f7d49116c16455fa2ad6a82881d38ce
candidate_head: 8299042e7de994db457dd917eb710a680ee63259
pr: null
publication: pushed to the task branch by the coordinator for PR review
```

The original implementation evidence below predates the rebase. The follow-up
section at the end supersedes its current-state and unchanged-source claims.

## Result and scope

Coordinator selected **option B** on continuation: durability-internal tests
may run only in `oteryn-gameplay-durability --lib`; each game-server target
retains every integration case. The earlier strict same-target blocker is
resolved by that explicit decision. Work remains local in AUTHORING/validation;
no remote candidate has been frozen or published.

All **27** wrappers in the parity table now import
`oteryn_gameplay_durability::durability` instead of compiling
`durability/core_modules.rs` via `include!`. Existing game-server dev-dependency
already enables durability's `test-support` feature. No fixture export,
visibility change, feature change, new function or dependency was necessary.
All **317** durability Rust source files are byte-identical to the main base.

The WP5 wrapper also imports the canonical library's existing
`read_authenticated_intent`, so `bootstrap_character` receives its canonical
`CharacterBootstrapIntentV1` type. Its source-loaded bootstrap module remains,
including all four original tests; only the caller's import changes. All other
source-loaded modules, integration case mounts and test bodies remain intact.
Targeted clippy found a shared promotion fixture unused by the standalone charm
wrapper: its only workspace consumer is `character_authority_postgres`. One
existing helper has a narrowly scoped `dead_code` allowance with that reason;
no test body or assertion changes. The standalone progression wrapper likewise
marks its shared PvP-case mount and PG harness as intentionally partial uses;
the existing helpers remain available to the authority target.

Owned changed files are `docs/agents/tasks/DUR-TESTSUPPORT-1.md`,
`apps/game-server/tests/<target>.rs` for exactly the 27 rows below, and
`apps/game-server/tests/support/charm_state_promotion_fixture.rs`. Ignored local
evidence lives under `target/DUR-TESTSUPPORT-1/`; the earlier lowercase
`target/dur-testsupport-1/` retains the initial checkpoint's evidence.

## Baseline provenance and combat harness

**PROVEN:** HEAD is the main-base commit above and is an ancestor of read-only
`origin/main` (`0388c427359c9c8ed0014c6aebfe85d3f164e6f3`). The original combat
wrapper is byte-identical on both revisions and declares the identical
`bestiary_postgres_harness` module twice. The initial run proved baseline E0428
before any Rust edit; this is not caused by the import refactor or PC load.

Remove only the second identical harness declaration, retaining one mount of
exactly the same source. Its executable **before** list is from this minimally
normalized base, still including all 418 durability core tests; raw unmodified
base cannot produce a list for this target. The card does not claim otherwise.
`combat-baseline-original.rs` and `combat-baseline-normalized.rs` preserve the
precise distinction. The normalized baseline compiles and lists 446 tests;
after lists 28 integration tests plus the same 418 tests in the library.

Every other before list uses untouched main-base target sources. The successful
achievement and bank inventories from the initial run are reused unchanged.
All other before/after lists were collected by `inventory.py`, using:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test <target> -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib -- --list
```

Listing registers tests but executes no PostgreSQL case. The continuation's
`before-summary.json` and `after-summary.json` retain exit codes, exact commands,
counts, wall times and reuse flags. All 28 lists in each phase completed with
exit 0. The existing target binaries are used even for targets marked
`test = false`; explicit `--test <target>` builds and lists them.

## Hard condition 1 — per-target and overall parity

**PROVEN:** each removed name has prefix `durability::`, appears under its exact
same name in both durability library lists, and comes from the same unchanged
source body. `durability-source-before.json` records SHA-256 hashes of all 317
source files, checked against main base before authoring and checked unchanged
afterward. The library's complete inventory remains **435 -> 435**.

For every row, full before/after names are in
`target/DUR-TESTSUPPORT-1/{before,after}-<target>.names`, removed names in
`removed-<target>.names`, and every target/name -> owning-library/name mapping
in `removed-to-library.tsv`. Those files contain all **11,286** duplicate
registrations removed across the 27 wrappers, not an inferred representative
sample. `parity.py` enforces all invariants, writes `parity.json` and the two
union inventories, and fails on any unmapped disappearance or added name.

| Target (changed file basename without `.rs`) | Before | After | Removed, in lib | Lost |
| --- | ---: | ---: | ---: | ---: |
| `account_achievement_postgres` | 451 | 33 | 418 | 0 |
| `bank_postgres` | 435 | 17 | 418 | 0 |
| `bestiary_progress_postgres` | 434 | 16 | 418 | 0 |
| `character_forge_dust_postgres` | 437 | 19 | 418 | 0 |
| `character_proficiency_modification_postgres` | 439 | 21 | 418 | 0 |
| `character_progression_postgres` | 473 | 55 | 418 | 0 |
| `charm_state_postgres` | 455 | 37 | 418 | 0 |
| `chest_use_postgres` | 485 | 67 | 418 | 0 |
| `combat_bestiary_postgres` | 437 | 19 | 418 | 0 |
| `combat_death_reward_postgres` | 446 | 28 | 418 | 0 |
| `combat_pickup_postgres` | 471 | 53 | 418 | 0 |
| `corpse_decay_postgres` | 460 | 42 | 418 | 0 |
| `corpse_transfer_postgres` | 455 | 37 | 418 | 0 |
| `durability_postgres` | 571 | 153 | 418 | 0 |
| `house_interior_postgres` | 478 | 60 | 418 | 0 |
| `item_fee_burn_postgres` | 437 | 19 | 418 | 0 |
| `item_mint_postgres` | 455 | 37 | 418 | 0 |
| `item_ref_production` | 465 | 47 | 418 | 0 |
| `item_timed_state_postgres` | 442 | 24 | 418 | 0 |
| `item_transfer_postgres` | 450 | 32 | 418 | 0 |
| `item_use_postgres` | 442 | 24 | 418 | 0 |
| `map_item_mint_postgres` | 440 | 22 | 418 | 0 |
| `native_admission_source_postgres` | 444 | 26 | 418 | 0 |
| `npc_econ_bank_postgres` | 458 | 40 | 418 | 0 |
| `reward_claim_mint_postgres` | 441 | 23 | 418 | 0 |
| `runtime_scope_assignment_postgres` | 452 | 34 | 418 | 0 |
| `wp5_s3b_composition` | 431 | 13 | 418 | 0 |

**PROVEN union equality:** union(all 27 targets + durability lib), before ==
after == **901 distinct names**; **0 lost, 0 added**. Total registrations fall
from 12,719 to 1,433 solely by removing repeated executions of the same tests.
Each target's after set equals its before set excluding exactly its 418
library-owned core tests. No target-local test disappeared.

## Hard condition 2 — CI ownership

**PROVEN:** all 418 embedded core tests already exist in durability `--lib`;
there is no core test whose only registration was a game-server include.
Therefore no exception include is needed. Read-only workflow verification:

- `pr-postgres-harness.yml:234-235` archives durability/dispatch/game-server
  `--lib` into `pg-lib.tar.zst`; lines 360-367 execute that archive under PG.
- `merge-group-gate.yml:715` builds the same archive; line 845 executes it.

Every target's own cases remain registered in that same target. Existing case
mounts into registered PostgreSQL CI targets remain unchanged. CI routing,
protection, registered targets and PostgreSQL requirements remain unchanged.

## Hard conditions 3 and 4 — bounded edits

**PROVEN:** no edits to `.github/**`, `tools/repository/**`, any Cargo manifest,
`Cargo.lock`, migrations, production implementation, or dependency source. No
new `tests/*_postgres.rs` target, third-party dependency or public API. Existing
`test-support` gates suffice. Combat harness baseline/repair is documented
above; one unchanged harness mount remains. No assertions, tests or guards were
weakened, skipped or deleted. No Git authoring/publication command, credential
access, Docker operation, dependency download or external write ran.

The exact bound META v3.1.0 policy was read from the earlier immutable evidence;
root/path policies and playable-first doctrine were applied. This is source
reuse in test harnesses, with no protocol, identity, persistence, authority,
production security or public-contract semantics changed; those high-risk
mutation/recovery qualification categories are NOT_APPLICABLE.

## Hard condition 5 — validation and measurement

**PROVEN measurement:** exact same filtered target-listing/build command on
`character_forge_dust_postgres`, warmed dependencies, before/after wrapper
compilation: **20.471 s -> 1.816 s** wall time, including queued wrapper and test
listing. Cargo reports 20.36 s -> 1.71 s (see per-phase logs for exact output).
This is one local compilation observation on a shared PC, not a controlled cold
benchmark or production-runtime claim; no cold-vs-cold run was attempted.

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test character_forge_dust_postgres -- --list
```

The earlier initial baseline also measured 93.411 s for touching durability and
building all enabled game-server tests, rebuilding 37 test artifacts of which
four include the core. The other 23 include targets are marked `test = false`;
that broad historical command is not repeated as an after benchmark. Source
inclusions across the 27 target roots are now **27 -> 0**.

**PROVEN local validation:**

- `cargo fmt --all --check`: PASS.
- `cargo clippy --locked --offline -p oteryn-gameplay-durability --all-targets --quiet -- -D warnings`: PASS.
- `cargo clippy --locked --offline -p oteryn-game-server --test <each of the 27 table targets> --quiet -- -D warnings`: PASS (one invocation listing all 27 `--test` flags).
- `cargo clippy --locked --offline -p oteryn-game-server --test character_authority_postgres --quiet -- -D warnings`: PASS for the shared fixture consumer; `authority-clippy.json`/`.log` retain the result.
- Three filtered game-server PostgreSQL cases, one per achievement/bank/combat target: PASS, one test each.
- Filtered durability library PostgreSQL case moved out of every wrapper: PASS, one test; positive `GROUND-CAP-NATIVE-PG server_version_num=180006 fixture=ground_xid` evidence.
- Focused durability bank unit tests: PASS, four tests, including the private-internals example from the earlier blocked checkpoint.
- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- Final source/body hash and union-equality checks: PASS, `parity.py`.
- Final `--list` refresh after helper-lint/import formatting changes: charm 37, progression 55, WP5 13; identical to the recorded after inventories. Parity was regenerated successfully afterward.
- `git diff --check` and final whole-diff checklist review: PASS.

All cargo commands used `/home/mole/oteryn-cache/codex-bin/cargo` through one
build lane. Exact arguments, exit codes and elapsed times are retained in
`qualification.json`; complete logs remain local. Existing vendored tokio/sqlx
dependency warnings are emitted but no selected target has a clippy error or
warning under `-D warnings`; no vendor source was changed. No PostgreSQL sample
returned early due to missing configuration, and no unfiltered PG suite ran.
There was no load-contention test failure needing a CI-only deferral.

Exact cargo test commands and filters covering the change:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test account_achievement_postgres -- account_achievement_postgres_cases::a_duplicate_grant_keeps_one_fact_and_its_first_committed_provenance --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test bank_postgres -- bank_postgres_cases::deposit_withdraw_and_transfer_commit_entries_lines_and_one_event_each --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test combat_death_reward_postgres -- attack_kill_reward_postgres_cases::swing_committed_kill_composes_with_explicit_reward_settlement --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib -- durability::ground_cell_limit::tests::postgres::native_scope_token_cannot_cross_physical_transactions --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib -- durability::bank::tests:: --nocapture
```

## Final checklist review (D1462)

Full uncommitted diff reviewed against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; no material findings remain.
Production SQL, transaction ordering, authority/fencing, wire codes, enum
variants, struct fields and trait methods are unchanged. No new `pub fn` or
other public API was added, so workspace-wide match/constructor/impl migrations
and new-function production/dispatch caller requirements are NOT_APPLICABLE.
All new Rust lines are imports, comments or scoped dead-helper allowances.
Existing fixture consumers were traced across `apps/`, `crates/`, `tools/`.
No test assertion or body changed; the complete test-name and source-body parity
checks above protect against deleting or weakening tests while removing repeats.

## Unfinished

No implementation or required local-validation blocker remains. A controlled
cold-vs-cold benchmark was not attempted on the shared PC; measured per-target
build/list timings and their limitations are recorded above. The raw combat
baseline cannot list tests until its pre-existing duplicate mount is normalized;
this limitation and the exact normalization are explicit in the baseline proof.

Publication, full-suite GitHub CI and merge are for the coordinator. This worker
left all changes uncommitted and did not stage, commit or push. No PR or frozen
remote candidate is claimed; archive the task record in the coordinator's final
authoring commit when the task's completing PR is prepared.


## Follow-up — three PostgreSQL regressions (2026-10-24)

State: explicitly returned to **AUTHORING** for this local repair; no published
or frozen remote candidate exists. HEAD is `8299042e7`, based on `fcd048da6`.
The owner requested an uncommitted repair and forbids Git publication/mutation.
The follow-up adds ownership of only
`crates/gameplay/durability/src/durability/fresh_admission.rs` to the original
owned paths. There is no manifest, feature-definition or test-body change.

### Baseline and reproduction

**PROVEN:** each of the three reported cases passes twice on immutable
`origin/main` (`fcd048da6f7d49116c16455fa2ad6a82881d38ce`) and fails twice on
unmodified HEAD, in isolation with `--exact`. These are regressions introduced
by linking the shared library, not pre-existing failures or shared-PC load.

The hard current-directory-only rule and session sandbox prohibit the requested
sibling worktree. Instead, `git archive --format=tar fcd048da6f7d49116c16455fa2ad6a82881d38ce`
was extracted into
`target/DUR-TESTSUPPORT-1/follow-up/durtest-base/` inside the current worktree.
No worktree was added or Git state changed. The archive contains the complete
tracked baseline tree, with no source normalization. Baseline runs used the
separate requested target directory
`/home/mole/oteryn-cache/target-codex/lanes/durtest-base`; HEAD and repair runs use
`/home/mole/oteryn-cache/target-codex/lanes/durtest`.

The queued wrapper overrides `CARGO_TARGET_DIR` from the detected Git root.
Initial baseline repetitions therefore used the `durtest` build lane; those
additional passing runs are retained in `base-results-initial-shared-lane.json`.
Baseline was then rerun twice per case with
`GIT_CEILING_DIRECTORIES=$PWD/..` while inside the archive, making the wrapper's
normal fallback select the `durtest-base` lane. These final separately routed
runs are the baseline evidence in the table and `base-results.json`. No wrapper
or Git configuration was modified.

For **each** directory, each command below ran **twice**, using the relevant
`CARGO_TARGET_DIR` above; baseline also set `GIT_CEILING_DIRECTORIES` as described
(the cargo arguments are identical across phases):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test durability_postgres -- complete_reconnect_resumes_an_owning_loss_session_exactly_once --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test durability_postgres -- resumed_session_loss_opens_the_next_epoch_and_resumes_again --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test durability_postgres -- world_required_capabilities_postgres_cases::admit0_v2_prepare_and_commit_refusal_preserve_current_authority --exact --nocapture
```

| Exact test | Main, run 1 / 2 | HEAD, run 1 / 2 | Repair, run 1 / 2 |
| --- | --- | --- | --- |
| `complete_reconnect_resumes_an_owning_loss_session_exactly_once` | PASS / PASS | FAIL / FAIL, line 6105 | PASS / PASS |
| `resumed_session_loss_opens_the_next_epoch_and_resumes_again` | PASS / PASS | FAIL / FAIL, line 6471 | PASS / PASS |
| `world_required_capabilities_postgres_cases::admit0_v2_prepare_and_commit_refusal_preserve_current_authority` | PASS / PASS | FAIL / FAIL, line 563 | PASS / PASS |

Each baseline/HEAD run executed exactly one test; passing runs exited 0 and
failing runs exited 101. All used the absolute queued cargo wrapper with actual
PostgreSQL configuration; no missing-configuration early return occurred.
Exact commands, working directories, durations, exit codes and full local logs
are in `target/DUR-TESTSUPPORT-1/follow-up/{base,head,fixed}-results.json` and the
corresponding `.log` files. The separate baseline snapshot metadata is in
`baseline-snapshot.json`.

### Root cause and bounded repair

**PROVEN:** `apply_complete_reconnect` and
`apply_complete_reconnect_with_capabilities` already use
`#[cfg(any(test, feature = "test-support"))]`, but their common private
implementation's ADMIT0 match still checked only `cfg!(test)` twice. Compiling
the former `include!` inside the test target made both checks true; linking the
library makes them false despite the enabled `test-support` feature.
Consequently the first two cases rejected legitimate PREPAREs, while the third
rejected before calling its independently observed capability check.

Both private match guards now use `cfg!(any(test, feature = "test-support"))`,
matching the existing test seams. This retains source reuse and avoids an
exception that recompiles all core tests in `durability_postgres`. No new
manifest or feature is necessary. No hook registry or duplicated static state
is involved. A workspace search found these are the only `cfg!(test)` runtime
checks in durability source.

Production behavior is unchanged: without `test-support`, the two predicates
retain their previous values. Even with that feature, the production
`apply_registered_complete_reconnect` always supplies `Some(custody)` and
`Some(capabilities)`, so it continues through registered recovery validation
and the mandatory capability callback. The real caller is
`crates/gameplay/dispatch/src/gameplay_transport/resume.rs::decide`.
Only the already gated test entry points can supply `None` custody.

### Rebased test-name parity

**PROVEN preservation:** every name in the original **901-name** union remains,
with **0 lost**. Rebase onto `fcd048da6` brought eight upstream guard/wire tests:
seven shared core cases plus the library-only sorted-module-registration guard.
Thus the current total union is **909 before -> 909 after**, including all 901
original names. Claiming a total of 901 on the rebased tree would omit upstream
tests. This repair adds or removes **zero** registrations.

The shared include core is now **425** tests (formerly 418), and the durability
library is **443** tests (formerly 435). `durability_postgres` remains **153**
integration tests, versus **578** total on current main. The new executable
lists for main/repair `durability_postgres` and repair durability `--lib` are in
`follow-up/*-list.names`. The other 26 integration inventories are reused from
unchanged source: the only upstream edit under `apps/game-server/tests/` since
the original base is `integration_rest/spell_lane_key33_pin.rs`, outside these
targets. `follow-up/parity.json` records this bounded reuse, every row below and
all eight added names. The historical inventories remain intact.

The `durability_postgres` row is directly measured. Rebased before counts for
the other 26 rows are **DERIVED** from their unchanged integration inventories
and the seven directly measured upstream additions to the shared core; the
pre-existing combat duplicate-mount normalization still applies to its row.

| Target | Rebased before | After | Removed, in lib | Lost |
| --- | ---: | ---: | ---: | ---: |
| `account_achievement_postgres` | 458 | 33 | 425 | 0 |
| `bank_postgres` | 442 | 17 | 425 | 0 |
| `bestiary_progress_postgres` | 441 | 16 | 425 | 0 |
| `character_forge_dust_postgres` | 444 | 19 | 425 | 0 |
| `character_proficiency_modification_postgres` | 446 | 21 | 425 | 0 |
| `character_progression_postgres` | 480 | 55 | 425 | 0 |
| `charm_state_postgres` | 462 | 37 | 425 | 0 |
| `chest_use_postgres` | 492 | 67 | 425 | 0 |
| `combat_bestiary_postgres` | 444 | 19 | 425 | 0 |
| `combat_death_reward_postgres` | 453 | 28 | 425 | 0 |
| `combat_pickup_postgres` | 478 | 53 | 425 | 0 |
| `corpse_decay_postgres` | 467 | 42 | 425 | 0 |
| `corpse_transfer_postgres` | 462 | 37 | 425 | 0 |
| `durability_postgres` | 578 | 153 | 425 | 0 |
| `house_interior_postgres` | 485 | 60 | 425 | 0 |
| `item_fee_burn_postgres` | 444 | 19 | 425 | 0 |
| `item_mint_postgres` | 462 | 37 | 425 | 0 |
| `item_ref_production` | 472 | 47 | 425 | 0 |
| `item_timed_state_postgres` | 449 | 24 | 425 | 0 |
| `item_transfer_postgres` | 457 | 32 | 425 | 0 |
| `item_use_postgres` | 449 | 24 | 425 | 0 |
| `map_item_mint_postgres` | 447 | 22 | 425 | 0 |
| `native_admission_source_postgres` | 451 | 26 | 425 | 0 |
| `npc_econ_bank_postgres` | 465 | 40 | 425 | 0 |
| `reward_claim_mint_postgres` | 448 | 23 | 425 | 0 |
| `runtime_scope_assignment_postgres` | 459 | 34 | 425 | 0 |
| `wp5_s3b_composition` | 438 | 13 | 425 | 0 |

No test bodies, assertions, enum variants, struct fields, trait methods or
public functions were added or changed. Workspace-wide constructor/match/impl
migrations and new-public-function production/dispatch E2E requirements are
NOT_APPLICABLE. All 318 current durability source files match current main
except the three-line repair in `fresh_admission.rs`.

### Follow-up validation and unfinished work

**PROVEN:** all six repaired exact PostgreSQL runs pass (two per case), and
all nine cargo validation/listing commands exit 0. Under
`CARGO_TARGET_DIR=/home/mole/oteryn-cache/target-codex/lanes/durtest`:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib -- durability::fresh_admission::resource_preflight_tests:: --nocapture
/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check
/home/mole/oteryn-cache/codex-bin/cargo check --locked --offline -p oteryn-gameplay-durability --lib --quiet
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-durability --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-durability --all-targets --features test-support --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-game-server --test durability_postgres --quiet -- -D warnings
```

The focused unit filter executes **2 tests, both PASS**. The remaining three
cargo commands are `-- --list` for baseline/repair `durability_postgres` and
repair `oteryn-gameplay-durability --lib`; their full arguments, working
directories, output paths, durations and exit codes are in
`follow-up/validation-results.json`. Governance validation and all **129**
`tools/agents/tests` pass. `git diff --check` passes. The complete candidate
diff against main and the local repair diff were re-read against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; no remaining finding. No test,
assertion, authorization or production capability requirement was weakened.

The in-directory baseline archive has been removed; cleanup and the exact
318-file source comparison are retained in `follow-up/baseline-cleanup.json`
and `follow-up/source-comparison.json`. No sibling worktree was created, and
no Git state mutation, credential access, Docker use, dependency download or
external write occurred. The repair and this card remain **uncommitted**.

**Unfinished:** the requested unfiltered `durability_postgres` target has not
run: the owner's PG SCOPE hard rule explicitly overrides task-card validation
lists and forbids it on this shared PC. An explicit exception was requested
but has not been granted; full-target validation remains with the coordinator
or CI. No unrelated load-contention failure needs a CI-only deferral. The
original 901-name union is fully retained, while the rebased union is 909
because of eight upstream additions, as itemized above.
