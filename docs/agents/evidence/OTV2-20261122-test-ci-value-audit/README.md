# Test and CI value audit — first candidate

The [second-wave supplement](wave-2/README.md) adds bounded durability, server admission/recovery and content-tooling findings with exact remaining scopes. It does not apply additional code changes.

## Evidence basis and status

**PROVEN:** baseline [8e9f0e16631aab1beb8416f8d5ca707aa3031083](https://github.com/Oteryn/Oteryn-Game/commit/8e9f0e16631aab1beb8416f8d5ca707aa3031083), tree `cef077886adb38c7b86bdb5d56b877a8833e7b1e`, is the squash of already merged [PR #3391](https://github.com/Oteryn/Oteryn-Game/pull/3391). The static scan read all 11,650 scoped Rust/Python source files and checked their Git blob hashes: zero missing, modified or Python-parse-error files. It identified 5,437 candidate/test-bearing files, 18,270 literal Rust test attributes and 6,749 Python `test_*` definitions across 68 workspace members. These are source counts, not executed cases or semantic coverage.

**PROVEN:** all 78 committed workflow definitions and their caller/job structures were inventoried (143 job definitions before matrix expansion). The paginated GitHub registration snapshot at 2026-11-22 22:12:43 UTC shows 44 active and 34 manually disabled definitions. Another 56 historical registrations are absent at this baseline. Disabled and historical definitions are not evidence of current CI spend.

**UNKNOWN:** semantic value of unvisited assertions; runtime-discovered Rust IDs by feature/target; whole-suite runtime, flakiness and cost; real game/Platform/PostgreSQL/Windows outcomes on the proposed head. This is a bounded first repair candidate in a continuing audit, not a completed semantic review of every test. The live draft PR and exact-head checks govern publication and validation status.

Evidence labels: **PROVEN** = inspected source/observed result; **DERIVED** = inference; **UNKNOWN** = missing evidence; **RECOMMENDATION** = proposal. Every manual-review record names its actual scope. Names such as legacy, source-text reads, missing `assert!`, identical bodies and environment guards are triage signals, not deletion criteria.

## Applied changes

| Change | Reason and retained evidence |
| --- | --- |
| Remove cascading repository-test calls and imported sibling TestCases; explicitly run owner modules in Agent governance | Merge-gate already invokes all owners. Five affected unittest modules: 54 executions become 42 with exactly the same 42 normalized unique IDs and zero failures/errors/skips. All 32 merge-gate entrypoints remain. See [CI findings](ci_lane_findings.md), [before](root_ci_before.json), [after](root_ci_after.json), [dispatch map](ci_dedup_entrypoints.json). |
| Remove three identical craft methods and the early unittest main guard | Baseline discovery10/direct5 becomes discovery7/direct7, preserving all seven unique bodies/assertions. Direct execution now reaches two previously omitted unique cases. See [discovery proof](craft_discovery_comparison.json) and TV-01 in [test-value findings](test_value_findings.md). |
| Match known node-boot failures by scenario, cause and root | Baseline matched only scenario ID. A synthetic changed E01.full root makes the actual original stage report PASS; the candidate reports FAIL(rows). All 46 recorded signatures remain recognized; their data is unchanged. See [original reproduction](blackbox_repros_baseline.json), [reproducer](blackbox_repros.py), BB-01 in [black-box findings](blackbox_findings.md). |
| Reject ambiguous/missing/nonterminal/prefixed failure metadata and invalid/duplicate baseline entries | Six malformed-signature subcases were red on the initial parser draft and green after repair. Actual-stage tests also cover conflicting tokens. Contract: `apps/game-server/src/qualification/client_phases/verdict.rs::Cause::suffix`. |
| Check child exit and final stage status in StageDumpTests | The original test passed even though its child exited1/127 before completing the claimed behavior. [Baseline oracle evidence](blackbox_oracle_repro.json). Current test completes the real stage function with its required fixture/helpers. |
| Replace imported-class collection in diagnostics with a direct-entry load_tests hook | The workflow entrypoint omitted WorldFixture/PgSampler; full discovery repeated 26 sibling cases. Direct execution now includes both classes and full discovery has unique IDs. WorldFixture now validates the existing11 fixture identities/intents including LightCaster; production fixture code is unchanged. |

The only production-script behavior change is stricter classification in `tools/qualification/node_boot/run.sh`. No game code, migration, authority fence, durable-value oracle, baseline row, required-check list or workflow activation state is removed. Retained proposal patches describe their original snapshots; the PR diff is the applied candidate.

## Remaining priorities

| Priority | Finding / disposition | Next step |
| --- | --- | --- |
| P1 | **BB-02, PROVEN:** matrix aggregation can accept a shard whose selected scenario is missing | Carry exact expected terminal IDs from plan into extract/summary. The reproducer selects row206/S09.client but supplies only I01.full PASS plus NODE_BOOT_PASS and is accepted. Assert membership, not a count floor; resolve capture-only and group contracts explicitly. |
| P1 | **PROVEN:** at least12 full rows never produce PASS | Map each missing public action/readback; implement the journey or explicitly report the capability gap. Preserve existing assertions. Never manufacture PASS or silently delete the row. |
| P2 | **PROVEN:** 31 run_packet functions return immediately without capture configuration | Distinguish live acceptance, capture and replay. Ordinary Rust PASS alone does not prove a live game journey. |
| P2 | **DERIVED:** input-platform test cfg is narrower than its public API and dedicated Windows command | Align supported target selection, then collect/run on a qualified Windows runner. C1 in [client findings](client_test_findings.md). |
| P2 | **PROVEN:** source-label and minimum-file-count oracles add maintenance cost | Rewrite around actual build operations/execution closure; retain archive checksum, provenance and shell-failure tests. [Label-only mutation](ci_snapshot/label_mutation_probe.json). |
| P2 | **PROVEN / runtime UNKNOWN:** AI bootstrap path-includes seven library unit cases | Qualify [public-library import proposal](ai_bootstrap_dedup.patch) with Cargo/nextest IDs. Keep seven distinct local integration cases. Not applied here. |
| P2 | **PROVEN:** main-only PG receipt-key script discovers zero unittest cases | Map its consumer and unique key/malformed-input assertions into current PG qualification before retiring/wiring it. Its SQL function is current. |
| P2 | **PROVEN:** ignored hireling reason describes an obsolete predicate | Requalify that precise test. Current command220 tests already cover Query/Summon/Dismiss/replay; this is not a missing-feature claim. |
| P2 | **PROVEN:** weak protocol/identity expected-value checks | Qualify the four test-only proposals in [protocol/foundation findings](protocol_foundation_findings.md): literal PKCE vector, fixed daily-boost wire bytes, exact GuildWar limit error, and removal of a subsumed Debug-string smoke. Rust execution remains unknown. |

The active real-clock nightly failed in the three inspected scheduled runs; investigate ownership and repair. CodeQL is not duplicated on main push: the merge-group copy is conditional on merge_group. Existing #3391 optimizations are already in the baseline, not attributed to this audit.

## Black-box direction

**RECOMMENDATION:** extend the existing live harness. First establish trustworthy selection and verdicts; then add a small deterministic set of public journeys with independent expected results:

1. OAuth/PKCE → ticket → Gateway/TLS admission → character selection. Preserve the existing Game–Platform journey and independent database readbacks.
2. Native input → move/target/cast dispatch → externally observed position, health/mana/cooldown and acknowledgement.
3. Bank/inventory/trade → lost reply/reconnect/retry → exactly-once durable value and ownership.
4. Invalid/stale authorization → explicit denial → independent confirmation of unchanged durable state.
5. Isolated fixture reset and controlled clocks; explicit separate qualification for real-clock and rare-event behavior.

Keep fast codecs/properties, independent formula/reference oracles, malformed protocol cases, focus/IME cancellation, render rollback, corruption rejection, supported compatibility and migrations. Keep PostgreSQL fencing/concurrency tests for exhaustive failure interleavings. Full E2E cannot replace their input-space coverage economically. Retire a test only after proving unsupported/dead behavior or an equivalent current oracle with the same fixture, configuration, boundary and failure mode.

## Validation and limits

Authoring commands/results: [validation.json](validation.json). Exact candidate SHA, CI and external review must be recorded after publication in PR/check evidence, without a self-referential metadata commit.

- Direct node-boot diagnostics: **48 tests PASS**. Includes strict-signature and completed-stage regressions plus the two formerly omitted classes.
- Full node-boot discovery: final collection **90 unique cases**. Final execution: **83 PASS, seven provision errors** because this environment denies AF_UNIX socket creation; zero assertion failures. Command output is retained in validation evidence. No provision assertions are weakened or skipped; the full suite is not claimed green.
- Affected CI slice: **42 unique cases PASS**, the same IDs as the54-execution baseline slice.
- Craft: **7 PASS** in discovery and direct modes.
- Final policy, workflow pins, governance, shell and whitespace checks: see command evidence. The complete32-entrypoint repository sweep was also attempted:30 PASS; dispatch-shard checks require unavailable rustc and native failure canaries require unavailable pwsh. See [full sweep](repository_suite.json) and [independent failure classification](repository_suite_failure_review.md). It is not a green full gate.
- No Cargo/Rust compiler, PowerShell, live PostgreSQL, live game/Platform, Windows or GPU qualification was performed. No whole-CI time/cost saving is claimed; focused sequential timing samples are illustrative only.

[Independent authoring peer review](code_review_findings.md) found one parser ambiguity and verified its repair: 17/17 signature probes, 12/12 KnownFailRows+StageDump cases; no unique IDs lost. It is not external exact-head review or full CI.

## Evidence map and continuation

| Artifact | Scope |
| --- | --- |
| [Source inventory](test_value_inventory.tsv), [summary](test_value_inventory_summary.json), [workspace members](test_value_workspace_members.tsv) | Complete baseline lexical scan; original rows deliberately retain UNREVIEWED_STATIC_SCAN. |
| [Manual scopes](test_value_semantic_reviews.tsv), [remaining groups](test_value_remaining_queue.tsv) | Initial exact review scopes and130 remaining groups. Later manifests supplement them; unlisted scopes remain unreviewed. |
| [CI inventory](ci_lane_inventory.json), [CI report](ci_lane_findings.md) | All78 workflows, triggers, jobs, commands, live state and caller findings. Recommendations are triage, not necessity proofs. |
| [Client manifest](client_test_review_manifest.json), [client report](client_test_findings.md) | 1,022 test files:35 fully read,1 partial,986 static-only;100 literal attributes in fully read files. Context counted separately. |
| [Protocol/foundation scopes](protocol_foundation_reviewed_scopes.tsv), [remaining files](protocol_foundation_remaining.tsv), [report](protocol_foundation_findings.md) | 119 bounded review rows across55 paths;408 test-bearing files indexed,370 unreviewed in that lane. No whole-file completion inferred from partial reads. |
| [Black-box inventory](blackbox_inventory.json), [report](blackbox_findings.md) | 302 active indexed selectors, plans, bounded source investigations and synthetic reproductions using the real harness functions. |

Continue separate read-only lanes for server/gameplay/PG boundaries, client/input/rendering, and tooling/content/CI consumers. Root alone edits/publishes coherent batches. For each actual case, record consumer, selection/feature/target, fixture, public boundary, independent oracle, unique failure mode, disposition and validation. Append evidence with its source revision; do not convert static hints into semantic completion.

Reproduce into a separate output directory, using the pinned baseline for baseline comparisons:

```sh
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/scan_test_value.py --repo /path/to/baseline-checkout --output /path/to/new-inventory
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/ci_dedup_probe.py /path/to/checkout /path/to/results.json
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/blackbox_repros.py --repo /path/to/checkout --mode all
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/blackbox_oracle_repro.py --repo /path/to/checkout
```

The [BB-02 bounded proposal](aggregate_completeness_proposal.md) and [unapplied patch](aggregate_completeness_proposal.patch) are ready for the next wave:33 matrix tests pass in a scratch overlay, and the original missing-row reproducer fails closed there. The current production matrix is unchanged; basic/group/spells/Game–Platform completeness still needs separate contracts.

The user authorized a draft PR and autonomous continuation, not merging. Follow the bound exact-head lifecycle and single-writer allocation for each successor. Read live branch/PR state, reopen AUTHORING for a substantive repair, then freeze and requalify. The separate E2E branch is outside this task's ownership. Missing local push credentials select only the bound META API-native new-candidate exception; no selected/frozen local commit is reconstructed.

Patch artifacts use zero context to keep retained diff text free of whitespace-only context lines; apply with `git apply --unidiff-zero --check <proposal.patch>` before considering adoption. Their resulting source is identical to the original lane proposal. TSV evidence is normalized to quoted fields and LF; parsed values and source hashes are unchanged.
