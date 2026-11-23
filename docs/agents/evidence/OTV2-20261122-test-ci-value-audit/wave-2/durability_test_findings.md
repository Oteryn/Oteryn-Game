# Durability and PostgreSQL test-value audit — bounded follow-up wave

## Evidence basis and outcome

Baseline: `Oteryn/Oteryn-Game@8e9f0e16631aab1beb8416f8d5ca707aa3031083`. The local checkout remains on this baseline for the inspected source scopes; this wave is separate from the first frozen audit candidate. No repository file was changed. No Cargo build, Rust test, PostgreSQL process, game process or whole-suite timing was run.

**PROVEN** means inspected source or the explicitly described local source/predicate probe. **DERIVED** means an inference from those observations. **UNKNOWN** means missing runtime or contract evidence. **RECOMMENDATION** describes follow-up work, not an applied repair.

The useful next changes are narrow oracle repairs and source-mount consolidation. No PostgreSQL fencing, rollback, concurrency, authority or conservation case is recommended for deletion. The strongest redundancy is a five-iteration bank failure loop that always injects its failure after the complete writer. Another source-duplication candidate re-mounts five valuable bootstrap parser tests in three canonical PG targets.

| Finding | Disposition |
| --- | --- |
| DUR-01: V1 visibility guard asserts public API text while claiming the API is internal | Resolve intended contract and make the guard test it; do not alter authority semantics as cleanup |
| DUR-02: “failure at every step” repeats the same complete-writer abort boundary | Keep rollback/replay; introduce actual distinct failure points or narrow and consolidate the loop |
| DUR-03: expected admission-lock set is parsed from production SQL | Keep real PG observation; use an independent expected relation set |
| DUR-04: linkage cases inflate runtime case counts | Preserve required compile contracts; separate them from behavioral test accounting |
| DUR-05: conditional error assertions can silently skip the requirement | Assert the outer error variant and fail on invalid positive fixtures |
| DUR-06: bank/fee tests pin a provisional “no starter island” adapter | Resolve current departure-policy integration; preserve pure party rules |
| DUR-07: persisted result-code roundtrip uses the same mapping in both directions | Add independent named variant↔code expectations |
| DUR-08: bootstrap parser source is repeated in registered PG targets | Qualify public-module imports and retain the five parser cases in their owner |

## Scope and counting limits

The lexical scan read **1,368 Rust source files**, with Git blob checks showing no modified scoped source. It extracted **2,854 ordinary function-body candidates** and separately recognized two macro templates. It found six whitespace-normalized body groups and 296 bodies without a direct assertion macro; neither metric proves uselessness.

| Lexical scope | Source files | Ordinary body candidates | Baseline inventory test-bearing files |
| --- | ---: | ---: | ---: |
| `crates/gameplay/durability` | 963 | 965 | 277 |
| `apps/game-server/tests` | 322 | 1,424 | 254 |
| `apps/game-server/src/lifted_tests` | 83 | 465 | 72 |
| Total | 1,368 | 2,854 | 603 |

The original repository inventory used line-anchored attributes and records 2,855 attributes in these scopes. This wave's masked scanner also recognizes an inline `#[test]` in `support/authority_cases/pubproj_character_views.rs:243`, so it sees 2,856 attributes: 2,854 ordinary bodies plus two macro templates. The other template is `tests/integration_rest/native_scope_assignment.rs:958`. Macro invocations, feature/target selection, nested mounts and generated IDs require Cargo/nextest discovery; these source figures are not runtime case counts.

Manual evidence is **148 exact scope rows across 45 paths**: **99 case-body reviews** and **49 bounded context traces**. The authoritative queue contains **603 test-bearing files**, of which **572 have no manual scope in this wave** and **31 have partial review only**. No whole-file semantic completion is inferred, including where all visible test bodies were read but fixtures, callers or feature combinations were only sampled. The dispatch PG tree is outside this lexical wave; only the explicitly indexed Dawnport production call was traced there.

Artifacts, beside this report:

- `durability_test_reviewed_scopes.tsv`: exact body/context ranges and dispositions.
- `durability_test_remaining_files.tsv`: complete file queue for the three selected baseline groups.
- `durability_test_review_manifest.json`: counts and limits.
- `durability_test_case_inventory.tsv`, `durability_test_case_inventory_summary.json`: lexical triage; every original row remains unreviewed by default.
- `scan_durability_oracles.py`: portable lexical scanner with explicit macro-template handling.
- `durability_oracle_probe.py`, `durability_oracle_evidence.json`: read-only source and predicate demonstrations, not Rust/PG mutation runs.

## Findings

### DUR-01 — the V1 visibility guard proves the opposite spelling to its message

**PROVEN:** `crates/gameplay/durability/src/durability/schema_contract_tests.rs:145`, `generic_v1_terminal_reconciliation_is_not_a_production_recovery_api`, positively checks that `admission_journal.rs` contains `pub async fn reconcile(` and `core_modules.rs` contains `pub const fn legacy(`. The corresponding failure messages require crate-internal reconciliation and say the adapter must not expose generic V1 reconciliation to production.

The actual code is public: `core_modules.rs:13` exposes the module, line 176 re-exports `AdmissionReconnectJournal`, line 454 exposes `legacy()`, and `admission_journal.rs:492` exposes V1 `reconcile()`. None of those entry points has a test-only gate. The probe verified all three original string predicates as true against the baseline blobs.

**DERIVED:** this test does not establish its named visibility policy. Changing the implementation to the visibility described by the failure messages would make this spelling check fail. The bounded symbol search did not establish a current game-server/dispatch production caller of the public V1/V2 journal types; tests and the V2 implementation do use them. That is not a proof that their compatibility API is removable.

**RECOMMENDATION:** resolve the intended current public contract first. If the policy really requires an internal/test-only API, qualify an external-crate compile visibility check and migrate callers under authority review. If public compatibility is intentional, correct the test's policy claim and specify what must actually be protected. Do not delete V1 replay/PG tests or change public access under a stylistic cleanup. A runtime authority bypass was not demonstrated.

### DUR-02 — five bank “failure steps” have one fault-injection boundary

**PROVEN:** `apps/game-server/tests/support/bank_postgres_cases.rs:797`, `a_crash_at_any_step_leaves_nothing_and_an_ambiguous_commit_resolves_by_replay`, iterates five table names. Every iteration first calls `runtime_transaction(&harness, &request)`, then checks one table has rows, then executes `SELECT 1/0`, drops the transaction and checks the same full snapshot.

`runtime_transaction` at line 779 calls the entire production `bank_operation_in_transaction` before returning the open transaction. The production body in `crates/gameplay/durability/src/durability/bank.rs:956` completes its deposit/withdraw/transfer work before returning. The selected table changes only the observation after that body, not where the writer fails. The loop's comment about cutting each write prefix is inaccurate.

The rest of this case remains useful: explicit rollback, termination of the active PG backend before commit, and successful commit followed by public-entry replay. Its independent SQL `snapshot` covers items, placements, balances, operation/ledger/coin-line/outbox counts and character revisions. The ordinary public entry also adds recovery/admission checks in `bank.rs:813–838`; this helper-based crash case alone does not qualify that wrapper's fences.

**RECOMMENDATION:** preserve those distinct cases. For an inexpensive honest consolidation, check all five tables in one complete transaction and induce the post-body error once. To claim per-effect crash coverage, introduce distinct test-owned SQL failure triggers or comparable fault points, require the targeted failure reason, and compare unchanged durable state after each prefix. A suitable existing pattern is `apps/game-server/tests/durability_postgres.rs:774`, which installs a `BEFORE INSERT OR UPDATE` trigger on each of eight different tables before invoking `FreshAdmissionStore::commit`, then checks absent receipts, unchanged claims and zero partial memberships. No runtime saving or new bank-prefix coverage was measured here.

### DUR-03 — real lock observation has a production-derived expected set

**PROVEN:** `crates/gameplay/durability/src/durability/db_tests/holder_round_trip_tests.rs:26`, `admission_relation_fence_locks_every_relation_exclusive_in_one_statement`, reads actual `pg_locks` after the production lock call. However, its expected names at line 49 are parsed directly from `super::ADMISSION_RELATIONS_LOCK`, the same SQL executed by `db.rs:2401`. Independent extra checks establish only a count of 17 and lexical order.

**DERIVED:** a sorted substitution of one required admission table with another existing `game_durability_*` table can satisfy the isolated membership/count predicate when PG locks the altered SQL set. The retained predicate demonstration replaces `game_durability_admission_account_guards` with the existing `game_durability_native_source_registration` and passes the modeled count/order/equality conditions. It does not execute changed SQL or demonstrate that the entire suite would pass.

**RECOMMENDATION:** retain the real PostgreSQL `ExclusiveLock` readback and pin an independently reviewed list of required relations. If “one statement” is itself a performance contract, the final lock snapshot cannot distinguish one statement from multiple equivalent lock statements; qualify that claim with an appropriate statement observation only if the contract requires it. Keep the adjacent observation-transaction case: it explicitly expects SQLSTATE `25006` for `FOR UPDATE`, including on an empty table.

### DUR-04 — linkage-only bodies should be accounted for as compile contracts

**PROVEN:** all 23 bodies in `crates/gameplay/durability/src/durability/core_modules_linkage_tests.rs` were inspected. Twenty contain no direct assertion; 17 of those only name types, constants, variants, methods or generic function instantiations. The other three invoke constructors/parsers but discard the results. For example, `item_mint_api_is_linked` at line 302 only binds method items, and `recovery_evidence_composition_api_is_linked` at line 452 discards `RecoveryEvidenceSubject::new(...)`.

Three of the 23 do contain runtime assertions: `fresh_admission_composition_api_is_linked`, `account_characters_projection_api_is_linked`, and `runtime_scope_assignment_api_is_linked`. `bank.rs:2387::bank_api_is_linked` is another separately inspected symbol-only case. The source's historical motivation mentions targets that path-loaded the original server modules; current major PG roots instead use the durability library.

**RECOMMENDATION:** preserve any desired compile surface contract, but represent pure symbol references as compile-check items/helpers rather than independent behavioral test executions. First decide which API existence checks are still needed beyond real callers. Do not blanket-remove all 23 or their actual assertions. This wave did not collect binary IDs or measure their runtime/compile cost; fewer nominal cases would not by itself be a meaningful speed claim.

### DUR-05 — conditional checks skip wrong error variants and failed valid fixtures

**PROVEN:** `core_modules_linkage_tests.rs:624–633`, inside `runtime_scope_assignment_api_is_linked`, asserts the inner durability error only inside `if let RegistrationError::Unavailable(inner)` and `if let AssignmentError::Unavailable(inner)`. A conversion returning a different outer variant would skip each assertion. Later successful UUID and `ControlActor::new("operator")` fixture constructions use `else { return; }`, allowing a fixture regression to skip the command-encoding checks.

The shipping conversions currently return `Unavailable(error)` at `runtime_scope_assignment.rs:85` and `:478`; no incorrect production mapping is claimed.

**RECOMMENDATION:** require the complete outer/inner pattern with `assert!(matches!(...))`; propagate or explicitly fail invalid positive fixtures. Preserve the operation-key and three command-encoding checks. These are precise test-only changes suitable for a later focused Rust qualification, not a reason to remove registration/assignment coverage.

### DUR-06 — two tests preserve a provisional absence assumption after a departure writer exists

**PROVEN:** `bank.rs:2494::junior_is_a_function_of_the_departure_fact` asserts the current adapter is not junior and says no starter island exists. `bank.rs:649::starter_island_fact` unconditionally returns `NoStarterIsland`. The fee test `item_fee_burn.rs:1402::the_bank_pays_the_rest_only_under_v2_for_a_payer_who_is_not_junior` ends with the same assertion and rationale.

The repository now has `durability/dawnport_departure.rs`, which reads and inserts `game_dawnport_departures`; the production dispatch calls `depart_dawnport` at `crates/gameplay/dispatch/src/gameplay_transport/dawnport_departure.rs:295`. Meanwhile, current bank, fee, guild, house, market and forge code still calls the constant junior adapter. This is not dead code proved safe to delete.

**UNKNOWN:** the accepted contract for bank eligibility across old/new characters, tutorial placement and existing departure receipts. Existence of a departure writer does not alone establish the correct replacement predicate or migration/default semantics.

**RECOMMENDATION:** resolve the current bank/departure contract and replace the provisional adapter assertion with an actual current-state fixture where appropriate. Keep the independent pure `OnIsland`/`Departed` party-refusal checks and the fee tuple/arithmetic cases. Their value is separate from the obsolete “no island” explanation. This finding is an integration/coverage follow-up, not an implemented gameplay policy change.

### DUR-07 — persisted result-code roundtrip cannot pin code meanings

**PROVEN:** `bank.rs:2583::stored_codes_round_trip` checks `BankResult::from_code(code).map(BankResult::code) == Some(code)` for 0–8. The decoder in `crates/gameplay/economy/src/bank.rs:50` itself finds a variant by calling its `code()` method. A bijective permutation of named code meanings therefore preserves this property.

The isolated predicate model swaps `JuniorAccount=5` and `RecipientCannotReceiveTransfers=7`: the original roundtrip and invalid-9 checks still accept it, while an independently named mapping rejects it. This is not a full-suite mutation result. Existing PG tests are stronger in places: `typed_refusals_write_only_their_operation_row_and_replay` asserts named outcomes and a literal stored-code vector. Migration `0071` also constrains operation/result combinations. These protections must remain.

**RECOMMENDATION:** make the unit case a fixed table of named variants and persisted numeric meanings, checking both directions and invalid boundaries. Evaluate `BankOperationKind` and `BankLedgerKind` in the same case against their actual compatibility contract. The current property has consistency value; it simply is not sufficient as an independent persisted-format oracle.

### DUR-08 — five parser cases are source-mounted into three canonical PG targets

**PROVEN:** `apps/game-server/tests/durability_postgres.rs:7`, `native_admission_source_postgres.rs:8` and `runtime_scope_assignment_postgres.rs:9` path-include `crates/gameplay/durability/src/character_bootstrap_intent.rs`. That module's `#[cfg(test)]` mount includes four cases in `character_bootstrap_intent/tests.rs` and the nested `character_sex_tests.rs` case. Its owning durability library already declares the same module.

All three targets are explicitly archived by `merge-group-gate.yml:948–951`, and the durability library is archived at line 964. The wider static sweep finds 27 top-level server test roots with this parser-source mount; many are standalone wrappers with `test = false`, so 27 is not a count of active CI executions. `apps/game-server/src/lib.rs:91` already publicly re-exports the owner module. `character_authority_postgres.rs:10–13` demonstrates the production-import pattern. No actual parser symbol reference was found in the three targeted root files beyond their mount declarations; shared helper closure still needs compiler confirmation.

**RECOMMENDATION:** start with the three registered roots, replacing the path mount with the public library/server re-export, then compare discovered IDs and run the focused targets under their actual features. Preserve all five parser/request/sex/golden cases in their owner; they include literal wire expectations, malformed input and independent golden fixtures. Do not claim 15 avoided executions until compiled target discovery and partition selection confirm the repeated IDs. No patch was applied and no Rust validation is claimed.

## Preserved coverage and cleared static hints

The six lexical duplicate groups are not six proven redundancies. Account/public projection wrappers call different locally resolved `projection_cases`; seal/Feroxa wrappers call different local `run_case`; tool-custody/cast wrappers call different local `run`. The two M04 bodies with the same test name are in different modules (`retained_lane_case` starts at line 621), with visibly different `Content` fixtures. Their broader oracle overlap remains unreviewed. These observations prevent automatic removal; they do not mark the surrounding test families semantically complete.

Specific protected boundaries reviewed in this wave:

- Bank literal balances and coin plans, independent SQL snapshots, changed-request replay conflict, stale-fence rejection, deferred SQL guard reasons, runtime role denial, and audit retention that leaves authoritative ledger state unchanged.
- Prepared replay in the same process, a newly launched test process, and concurrent preparations. These reach different lifetime/race boundaries.
- Detached journal work after caller cancellation while a real PG lock is held; the retained request must later reconcile as prepared.
- Ambiguous-commit holder retirement, with a connection-local PG setting proving a successor holder is used. The ambiguity is injected as a typed error for this holder-policy case; it is not a claim to simulate a network fault here.
- Eleven holder retry/admission/fairness cases, including foreground demand across return and background competition. Some use wall-clock waits; no flake rate or timing benefit was measured.
- Exact definitive/ambiguous commit classifications with independently observed payload destruction and no formatting; configuration resource limits and secret/TLS options.
- Separate prepare and reconcile rejection of omitted replacement authority; pure claim-missing/claim-mismatch negatives remain valuable alongside persistence tests.

Missing-PG early returns are not counted as live database evidence. The shared wrappers gate on `OTERYN_TEST_POSTGRES_ADMIN_URL`; dedicated merge-gate jobs set it and execute archived targets/library cases. The standalone `bank_postgres` manifest entry is `test = false`, while the active `character_authority_postgres` includes `support/bank_postgres_cases.rs:2247`. A plain test pass without this selection/environment chain does not prove PG behavior; a source-level early return alone does not prove canonical CI is fake green.

## Next queue and bounded validation

1. Qualify the three parser import changes with exact Cargo/nextest target IDs before editing additional wrappers.
2. Strengthen the bank failure loop, independent lock-set expectation, conditional error checks and persisted-code mapping. Preserve the existing authority/value assertions and compare before/after cases.
3. Resolve the V1 visibility contract and junior/departure contract as separate owner questions; do not guess a new public API or eligibility rule.
4. Continue unreviewed durability authority/value code: `player_trade`, marketplace/coin market, item mint/transfer/retirement, journal V2 terminal/recovery matrices and character receipt sequencing. Continue server PG helper bodies and actual runtime routing, not only their thin wrappers.
5. Inspect the remaining 572 untouched files and every unlisted scope of the 31 partial files. The original global queue still applies outside these three lexical groups.

Observed checks: scanner completed with no modified scoped files; manual manifest and queue were generated from named scopes; source/predicate probe verified eight baseline blobs and produced the described isolated results. No existing Rust assertion was executed or mutation score measured.

Reproduce source-only artifacts into a separate directory, with the scripts beside this report and a materialized pinned baseline:

```sh
python scan_durability_oracles.py --repo /path/to/baseline-checkout --output /path/to/new-output
python durability_oracle_probe.py --repo /path/to/baseline-checkout --output /path/to/new-output/durability_oracle_evidence.json
```

The manual TSV records deliberate reading and cannot be regenerated from a lexical scan alone. The retained manual scope TSV is the authoritative record of those deliberate readings. Its original authoring helper is not shipped as a command that could perform or certify semantic review.
