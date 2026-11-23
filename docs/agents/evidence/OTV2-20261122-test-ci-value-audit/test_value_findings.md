> Retained lane report at the pinned baseline. The root [audit status](README.md) records which proposals were applied and the latest validation. Historical scratch overlays and proposal patches are evidence, not additional production changes.

# Oteryn-Game: test value audit, first evidence pass

## Basis and scope

**Baseline:** `8e9f0e16631aab1beb8416f8d5ca707aa3031083`, tree `cef077886adb38c7b86bdb5d56b877a8833e7b1e`, checked out at `<baseline-checkout>`. The baseline already contains merged PR #3391 (`perf(ci): reduce discarded cache builds and refresh PG scheduling`), committed 2026-11-22 21:38:12 UTC.

This lane reviewed Rust and Python unit/integration/tooling test value. It did not own workflow redesign, Node boot E2E, repository edits, remote publication, or approval. It produced read-only evidence and scratch proposals. Other lanes may subsequently modify the working tree; findings and inventory below describe the baseline SHA unless explicitly marked as overlay validation.

**Evidence labels:** PROVEN = directly inspected source or observed command result; DERIVED = inference from the inspected evidence; UNKNOWN = requires a missing execution or mapping; RECOMMENDATION = proposed next action.

### Coverage statement

PROVEN: the reproducible lexical scan read all tracked `.rs` and `.py` files under `apps`, `crates`, `tools`, `tests`, and `deploy` present in the baseline scope. Each read source was checked against its Git blob hash. At the recorded scan there were **0 missing source files, 0 modified sources, and 0 Python parse errors**.

| Inventory quantity | Baseline result | Meaning |
| --- | ---: | --- |
| Tracked paths in Git tree | 34,299 | All extensions and directories |
| Scoped Rust source files read | 9,742 | Includes production and test support |
| Scoped Python source files read | 1,908 | Includes production and test support |
| Candidate/test-bearing files | 5,437 | Rust test attributes, Python definitions/classes or candidate filenames |
| Literal Rust test attributes | 18,270 | Lexical occurrences; not unique runtime-discovered cases |
| Python `test_*` definitions | 6,749 | AST definitions; not proof that unittest discovers them |
| Matched Rust ignore attributes | 41 | Explicit `#[ignore]` matches only |
| Workspace members | 68 | Parsed workspace member list |

**This is not a semantic review of all 5,437 files and not an executed full suite.** The inventory deliberately retains `UNREVIEWED_STATIC_SCAN`; the bounded manual reviews below are a separate evidence layer. Helper modules with no test marker do not count as test-bearing, even when used by a harness. Macro expansion, duplicate inclusion, `cfg`, selection filters, runtime registration, skips, and environment gates change execution coverage.

Artifacts:

- `scan_test_value.py`: reproducible generator; uses working files and flags divergence from current HEAD blobs.
- `test_value_inventory.tsv`: file-level inventory and lexical triage flags.
- `test_value_inventory_summary.json`: aggregate groups, ignored attributes, candidate duplicate AST bodies and discovery hints.
- `test_value_workspace_members.tsv`: all 68 workspace members and lexical counts.
- `test_value_semantic_reviews.tsv`: exact manually inspected scopes and validation status; partial rows do not complete the whole file.
- `test_value_remaining_queue.tsv`: per-group remaining queue; every scope outside the manual index remains unreviewed.
- `python_shadowed_tests.json`: empty result of scoped duplicate test-name check; no same-scope Python test definitions shadowing an earlier same-name definition were found by that check.

All artifact names above resolve under `./`. Running the scanner after a parent edit will flag the edited file; after a commit it will inventory the new HEAD. Preserve this recorded baseline when comparing changes.

## Actionable findings

### TV-01 — Three duplicated craft tests and an early main guard (PROVEN, minimal patch validated)

**File:** `tools/content-schema/craft-authoring/test_tainted_souls.py`.

Two unittest classes repeat three exact AST bodies, use the same module imports/globals, and have no differing class fixture setup:

| Method | First copy baseline line | Second copy baseline line |
| --- | ---: | ---: |
| `test_source_script_and_nonstackable_output_are_independently_qualified` | 41 | 90 |
| `test_invalid_whole_stack_or_output_count_never_generates_a_recipe` | 53 | 102 |
| `test_source_recipe_uses_qm22_capture_target_stack_policy` | 65 | 114 |

An early `if __name__ == '__main__': unittest.main()` before the second class exits before that class is defined when running the file directly. The second class also contains two genuinely distinct cases: canonical catalogue registration and canonical outputs' quantity capture schema. A final main guard already exists after the second class.

**Consumer:** `.github/workflows/craft-authoring.yml:53` runs unittest discovery for `test_*.py`. It sees all ten methods, including the three redundant copies. Direct invocation sees only the first five methods and misses the two additional unique cases.

**Patch:** `./craft_tests_dedup.patch` removes 47 lines: three repeated methods, repeated imports/docstring, and the early main guard. It preserves both classes, all seven unique bodies, and the final main guard. No production logic changes.

**Observed proof:**

| Mode | Baseline | Proposed overlay |
| --- | ---: | ---: |
| unittest import/discovery case count | 10 | 7 |
| Direct main-entry case count | 5 | 7 |
| Unique test bodies | 7 | 7 |
| Real execution using discovery | 10 pass | 7 pass |
| Real execution using direct overlay | — | 7 pass |

Exact case lists and AST comparison are in `craft_discovery_comparison.json`; the enumerator is `verify_craft_discovery.py`. The direct enumeration hook observes the suite and stops before execution; it is separate from the three real test runs above. `git apply --check craft_tests_dedup.patch` passed at the baseline.

The first overlay behavior attempt exposed a missing sparse-checkout fixture, `docs/reference/content-prep/qmech-m22/actions_star_herb.lua`. After the parent materialized the tracked fixture, all three focused real runs passed. This was a checkout dependency, not a test defect. No runtime saving is claimed from subsecond wall-clock measurements.

Historical validated commands (original baseline/scratch cwd, not a portable recipe; the temporary overlay is not retained):

```sh
python -m unittest discover -s tools/content-schema/craft-authoring -p test_tainted_souls.py -v
PYTHONPATH=<baseline-checkout>/tools/content-schema/craft-authoring python -m unittest discover -s ./test_value_overlay -p test_tainted_souls.py -v
PYTHONPATH=<baseline-checkout>/tools/content-schema/craft-authoring python ./test_value_overlay/test_tainted_souls.py -v
```

Portable enumeration against the pinned Git baseline and the current candidate: `python verify_craft_discovery.py --repo /path/to/checkout --output /path/to/new-proof.json`. It loads the baseline test source using Git and does not depend on the historical overlay.

**RECOMMENDATION:** suitable for the first small draft change. Its value is consistent discovery and removal of exact redundant work, not a claimed material CI speedup.

### TV-02 — AI bootstrap integration harness recompiles library unit tests (source duplication PROVEN; Rust validation UNKNOWN)

**Files:**

- `apps/game-server/tests/ai_bootstrap.rs:1-2` path-includes `crates/gameplay/monster-ai/src/ai/mod.rs`.
- `crates/gameplay/monster-ai/src/ai/mod.rs:22-23` includes the module's own `#[cfg(test)]` tests.
- `crates/gameplay/monster-ai/src/lib.rs:3` already exposes `pub mod ai`.
- `tools/repository/check_duplicate_test_sources.py` explicitly allows this duplicated source target.

The nested unit module contains seven source test cases: six in `ai/tests.rs`, one in `ai/tests/path_request_tests.rs`. These source tests are consequently mounted in both the owning library test harness and the bootstrap integration target. Actual selection/execution frequency depends on the CI Cargo target list; no runtime cost was measured.

The bootstrap file also owns seven distinct integration tests. Those are meaningful and should remain. Its uses of `ai` resolve to public API in the existing library dependency; no `pub(crate)` or test-only API is needed by the inspected bootstrap cases. The public types, constructors, decision functions and methods are available outside `cfg(test)`.

**Proposal:** import `oteryn_gameplay_monster_ai::ai`, remove the obsolete duplicate-source allowlist entry, and update the module comment. Scratch patch: `ai_bootstrap_dedup.patch`, which passed `git apply --check` at the baseline. Additional stale comments are in `ai/perception.rs:13-18` and `ai_think/behaviour_profile.rs:11-12`; they describe the standalone path-included compilation model and should be revisited if the migration is landed.

**UNKNOWN:** no Rust toolchain/build execution in this lane. Before landing, run the owning library tests, the `ai_bootstrap` target, focused lints if required by policy, and duplicate-source inventory. Compare lists so the seven local integration cases and seven library unit cases remain once in their intended owners. This is a reviewable next-wave proposal, not a verified implementation.

The allowlist has eight target entries in total (seven server targets and dispatch). They include real PG harness fixtures and are not a blanket deletion list. For example, `apps/game-server/tests/support/character_revision_sequencer_pg_tests.rs` contains six important revision/fencing cases; the include in `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs:1009` is guarded by `not(oteryn_pg_shard_secondary)`. Shared mounting with dispatch must be measured with the real shard/config inventory before reducing execution.

### TV-03 — A Python file named as a test runs zero unittest cases (PROVEN); CI ownership unresolved

**File:** `tools/content-migration/test_m42_cooking_receipt_keys_postgres.py`.

The file is an explicit `main()` script, not a unittest suite. It extracts `game_quest_npc_reward_claim_key` from `apps/game-server/migrations/2080_quest_utc_cooking_receipts.sql`, requires PostgreSQL 18.6, exercises 10,000 cooking identities, once-only key preservation and malformed-counter inputs, and rolls back the temporary schema. It also rejects implausibly short elapsed time. These are substantive checks if directly invoked.

An isolated copy of the exact source was passed to unittest discovery: **0 tests; exit 5, `NO TESTS RAN`**. Its basename is therefore not evidence that a usual `test_*.py` discovery job executes its PG body. Targeted `rg` over available `.github`, tools and docs found no invocation of this filename or its output marker outside the script itself; this bounded search does not prove that no external/manual invoker exists.

The function is not dead: current `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/support/m42_hot_cuisine_fixtures.rs:459` calls it and tests that a non-advancing occurrence cannot authorize a second reward. All occurrences of this SQL function in the source/migration/tool scope were confined to that fixture, migration2080 and this script.

**RECOMMENDATION:** either register the remaining distinct assertions with the owning current PG harness or label this as explicit historical/manual qualification. First map the 10,000-identity and malformed/ambiguous cases to existing dispatch coverage. Do not silently remove it as “legacy” and do not count it as CI coverage without a proven caller. No real PG run was attempted here.

### TV-04 — Hireling E2E ignore reason cites removed implementation (stale explanation PROVEN; re-enable UNKNOWN)

**File:** `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/client_e2e/hireling.rs:8`, test `client_e2e_hireling`.

Its ignore reason says Query `(None,false)` cannot reach durable reads because `position.is_some() == dismiss` in `durability/house_hireling.rs:14` rejects it. The current file is a module/reexport root; the cited condition is absent from the hireling implementation subtree.

Current production flow:

1. `gameplay_transport/connection/routing/hireling.rs:10-13` decodes the request and calls `FreshAdmissionAuthority::hireling`.
2. `gameplay_transport/hireling.rs:41-45` handles Query via `query_legacy_hireling`, then refreshes projection and returns whether the stored state is nonempty.
3. `durability/house_hireling/appearance.rs:26-78` performs the actual semantic transaction, recovery/admission/inventory fences, current house query, immutable reply receipt and commit.

There is also a current non-ignored test, `hireling_postgres_command220_durable_replay_and_lost_ingress_refusal`, in `postgres_tests/hireling_lifecycle.rs:43`. `support/hireling_lifecycle/legacy_wire.rs` covers Query(false), Summon, Query(true), Dismiss, Query(false), further state changes and exact original replies after ingress loss/owner recreation; it asserts custody and projection remain current during old-command replay.

**RECOMMENDATION:** run the precise ignored CLIENT-E2E fixture on the qualified PG18.6 topology. If it passes, remove the stale ignore; if it fails for a different current reason, replace the reason and record that failure precisely. Do not remove the ignore on source inspection alone. The historical reason is outdated, but whether this fixture's current setup succeeds is UNKNOWN. Do not describe all command220 coverage as disabled.

### TV-05 — Source-shape checks should not be reported as runtime behavior (PROVEN examples; replacement is a design recommendation)

`apps/game-server/src/node/serve/qualification_fixture.rs:995-1014`, `qualification_spawn_watchdog_caller_contract`, reads implementation source and checks substrings including `finish_spawn_holder(holder, budget, failure)`, `retry_spawn_collisions(`, `published`, and absence of `.detach()`. It enforces a source-shape convention, not actual cancellation/cleanup behavior. Nearby `qualification_spawn_reports_publication_if_cleanup_stalls` does execute a stalled async cleanup/published-result scenario and should be retained.

`crates/gameplay/durability/src/durability/item_mint_audit.rs` includes:

- `every_type2_insert_takes_its_tuple_from_a_type2_transaction`;
- `every_type2_transaction_takes_the_fence_before_any_other_lock`;
- `the_fence_key_and_the_grandfather_list_match_the_migrations`.

These scan source for Type2 transaction use and migration/fence conventions. The intended fence constraints matter, but text matching is coupled to spelling, source placement and filename allowlists. This lane did not prove that the guards are worthless or redundant with runtime coverage.

**RECOMMENDATION:** label such cases architecture/source-contract checks. Prefer typed boundaries where feasible, and retain targeted runtime/PG negative cases that prove the dangerous outcome cannot occur. Move or consolidate repeated source scans only after ownership and failure modes are mapped. Do not replace security/fencing guarantees with a generic happy-path black-box scenario.

### TV-06 — Historical fake composition harness needs a boundary decision (PROVEN structure; retirement not proved)

`apps/game-server/tests/ability_engine.rs` builds a standalone source-inclusion harness with a large local foundation shim (including stub runtime behaviors) around real ability code. Its plan/order/capacity/atomicity checks can still be useful for the pure logic they execute. This harness is not proof that the current full native combat owner composes those parts correctly.

**RECOMMENDATION:** inventory its individual cases against the owning library and current composed gameplay fixtures, then migrate the genuinely unique pure cases to the owning crate API. Preserve ability refusal, capacity and atomicity coverage. Neither age nor test-only shims alone prove that all its cases can be removed.

### TV-07 — An additional exact positive governance-case duplicate (PROVEN source; low priority)

`tools/agents/tests/test_enqueue_readiness.py:182` (`EnqueueReadinessTest.test_unresolved_threads_block`) and `:517` (`FailClosedTimestampAndCountTest.test_unresolved_positive_integer_blocks`) have identical bodies: build the same `ready()` fixture, set total/unresolved to 2, expect `REVIEW_THREADS_UNRESOLVED`. Their classes have no differing setup/teardown. This is one duplicated positive example, while surrounding invalid-type, missing-fact and timestamp checks are distinct.

**RECOMMENDATION:** optional consolidation in a focused governance cleanup, retaining one positive unresolved-thread case and all fail-closed boundary cases. No material speedup is established. No patch was made here because the first craft patch has clearer value and governance is active in the parent's lane.

## Tests that the audit explicitly protects

| Source / named example | Inspected value | Decision |
| --- | --- | --- |
| `tests/pre-native-client/src/lib.rs`, `product_state_is_explicit_and_fail_closed` | Current client defaults to no native-login configuration and rejects before endpoint/credential access; production `apps/client/src/lib.rs` still has that guard | Keep; package name alone is not evidence of obsolescence |
| `tools/content-schema/quest-authoring/test_legacy_procedural_writers.py` (23 methods) | Parser scope, receiver identity, alias shadowing, argument arity and source-occurrence boundaries for current legacy-source migration | Keep; these are current input semantics |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_forge/legacy_budget.rs`, `forge_postgres_legacy_retry_exhaustion_cannot_reset_through_command181` | Legacy/current commands must share the bounded retry state | Keep while compatibility path is supported |
| `crates/gameplay/foundation/src/foundation/admission_tests/tests_reconnect_fences.rs`, `prepare_has_no_authority_and_commit_fences_predecessor` and `control_loss_lost_response_reconciles_on_exact_retry` | Exact generation fences and committed-but-response-lost transitions are exercised directly and deterministically | Keep beneath process black-box coverage |
| `crates/gameplay/foundation/src/foundation/damage_schedule_crystal_parity_tests.rs`, both C08 tests | Pinned Crystal arithmetic oracle compares decreasing damage schedules, interval, clamped/default starts and totals over bounded input ranges | Keep as compatibility arithmetic evidence; source restatement with external provenance is not automatically a tautology |
| `crates/protocol-oteryn/src/world_spatial_entities_fuzz_tests.rs` with `decoder_fuzz_support.rs` | Generated roundtrips, valid seeds, truncation execution/no-panic surface, explicit garbage/length errors and selected-capability variants | Keep; roundtrip tests also need independent malformed/boundary cases, which this helper supplies |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/support/hireling_lifecycle/legacy.rs` | Real pre-0719 migration, ownership/provenance preservation, explicit quarantine for missing historical origin, recall after original session retirement | Keep; historical migrations protect existing persisted state |

These are source-semantic reviews, not newly executed Rust/PG results.

## False-positive handling and audit limitations

- The AST scan found 13 groups of identical Python test bodies. Three were the craft duplicates; one was the governance positive-case duplicate. Most other groups use different imported generators or different `setUp` configuration. Equal method text across files does not mean equal executed behavior.
- Examples retained after context check include parallel content stage tools, SRX3 witness generators, different spawn/quest candidate producers, and NPC progress/retention tests with different setup sources.
- Eight `assertEqual`-family calls had textually identical argument ASTs, but the inspected arguments were repeated function calls testing determinism, not `x == x`. The heuristic did not establish eight tautologies. It is also not a proof that no tautology exists anywhere.
- Twenty-seven Python candidate filenames have no standard unittest-style definitions/classes. Some expose custom `load_tests` or explicit self-test `main`; only a runtime discovery/caller trace decides whether each contributes to CI. TV-03 is one verified example.
- `build_npc_programs.rs` and `build_mechanics.rs` initially looked like unreachable build-script test modules. Caller inspection disproved that: quests `src/quest/npc_rules/tests.rs` includes the former and dispatch `connection/serve/registry_tests.rs` includes the latter. They are not reported as undiscovered.
- `tools/content-format-spike/README.md` explicitly describes an isolated Issue95 evidence-only prototype. Bounded runner searches found no current CI caller. Treat it as research/manual archive until ownership is verified; deleting its tests cannot be credited as CI savings without an execution edge.
- The 41 ignore matches span deliberate host benchmarks, disposable topology integrations, retained native/source captures and the stale Hireling reason. An ignored test is not automatically dead, and a fixture-heavy test is not necessarily part of ordinary PR CI.
- No full Cargo build, Rust test binary list, PG18.6 integration run, mutation score, coverage profile, p95/p99 runtime measurement or complete CI call graph was produced by this lane.
- Fixture/source catalogs outside the scoped text snapshot were read only when needed. Claims about no caller are bounded searches, not proof about external automation or private manual routines.

## Black-box direction grounded in these findings

RECOMMENDATION: prefer black-box checks for externally observable gameplay and composition contracts: accepted/refused command, durable resulting state, replay result, reconnect behavior, and no duplicate effects. Use the real public process/protocol boundary where the topology is already qualified. Make each scenario name its invariant and observe a real outcome; a mock receipt alone is not process behavior.

Retain a focused lower layer for cheap exhaustive boundaries: arithmetic, parsers/codecs, exact identity/authority transitions, deterministic state machines and transactional failure modes. A process test is an addition when it covers a composition failure that a pure test cannot see; it is a replacement only when the old test's unique failure-detection value is demonstrably preserved.

Apply a per-case disposition, not a global ratio target:

1. **Keep:** meaningful current invariant, independent outcome, useful boundary or failure case.
2. **Consolidate:** same execution owner, same inputs/fixture and same asserted outcome; prove lists and unique assertions survive.
3. **Relocate:** useful unit/architecture/qualification check currently in a misleading harness or wrong gate.
4. **Requalify:** ignored/test-only setup cites old implementation or no current execution caller is proven.
5. **Retire:** production contract removed, caller absent and replacement/remaining obligation explicitly accounted for; age or filenames are insufficient evidence.

For expensive suite prioritization, first collect actual per-test duration, selected target/config and fixture topology. Literal source case counts are not a runtime-cost model.

## Remaining semantic audit queue

The inventory covers every workspace member lexically. The following grouping prioritizes the remaining manual work; it does not mark these groups completed.

| Priority | Area | Next evidence needed |
| --- | --- | --- |
| 1 | Duplicate-source allowlist and shared PG fixtures | Runtime Cargo case lists per real shard/cfg; owning-package comparison; retain unique acceptance assertions |
| 1 | Discovery anomalies and ignored tests | Exact invoked command, `--ignored`/fixture producer, last qualified result, current reason and owner |
| 1 | `dispatch`, `durability`, server composition | Map behavior vs fixture setup, replay/fence/race contracts, source-text-only checks and real boundary observations |
| 2 | Client, client-ui, client-runtime | Separate pure state reducers, command serialization, mock owner UI tests and actual client/process flows; identify duplicated presentation assertions |
| 2 | Protocol, session, foundation | Retain malformed/limits/stale authority cases; compare codec roundtrips to independent bytes/spec oracles; find genuine duplicates across harnesses |
| 2 | Content schema and migration tools | Identify distinct imported generator/fixture before merging equal bodies; trace manual `main`, load_tests and subprocess ownership |
| 3 | Remaining gameplay crates and compiler tools | Per-package invariant register and public caller; math/property/parity coverage before acceptance replacement |
| 3 | Research/spike/manual benchmarks | Explicit archive or qualification ownership; exclude from ordinary gate only where currently invoked and evidence allows |

A completed semantic row should record: path and exact case; runtime target; production caller/contract; fixture origin; independent asserted outcome; failure modes unique to it; actual duration if measured; proposed disposition; replacement mapping; validation result. Leave every unvisited case unreviewed.

## Deliverable state

- TV-01 has a minimal scratch patch and focused before/after behavior/discovery proof; parent owns application/publication.
- TV-02 has an apply-clean scratch proposal, public API trace, and explicit missing Rust validation.
- TV-03/04/05/06/07 are evidence-backed review items with bounded claims; no unvalidated disabling or deletion was performed.
- No repository files were edited by this lane. No branches were pushed, PRs opened, messages to external recipients sent, protection changed, or tests broadly removed by this lane.
