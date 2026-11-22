# Second wave: durability, server boundaries and content tooling

## Evidence and scope

This is a substantive audit-evidence supplement to [draft PR #3430](https://github.com/Oteryn/Oteryn-Game/pull/3430), tracked by [issue #3429](https://github.com/Oteryn/Oteryn-Game/issues/3429). It changes no production code, test selection or workflow behavior. All reviewed source bytes still match baseline `8e9f0e16631aab1beb8416f8d5ca707aa3031083`; manifests retain the exact ranges, hashes and remaining queue. Partial readings and lexical scans are not whole-file semantic reviews.

| Lane | Actual semantic scope | Retained report and queue |
| --- | --- | --- |
| Durability and server PG | 148 exact scopes across 45 paths: 99 case-body readings and 49 context readings. No whole source file marked complete. | [Findings](durability_test_findings.md), [scopes](durability_test_reviewed_scopes.tsv), [remaining files](durability_test_remaining_files.tsv) |
| Server admission/recovery/public journeys | 34 source files with explicit bounded ranges; 13 remaining queue entries. | [Findings](server_boundary_findings.md), [scopes and queue](server_boundary_reviewed_scopes.json) |
| Content tooling | 11 test source files fully read (81 literal definitions), 10 partially read; 12 context files. The 472-file AST pass is static triage only. | [Findings](content_tools_findings.md), [scopes](content_tools_scopes.tsv), [remaining files](content_tools_remainingfiles.tsv) |

These scopes can overlap earlier waves and each other. Do not add them into a claimed total of unique completed tests. The original [global inventory and queue](../README.md) remain applicable.

## Concrete findings and disposition

- **PROVEN, CT-01:** the current item engine manual runner selects 107 of 108 defined test functions and omits the current Frost Cannon/9132 case. Keep and register it after execution in the declared Python dependency environment. The focused attempt stopped on missing `rfc3339-validator`; no case passed in that attempt.
- **PROVEN, SB-03:** the actual S3B shell verdict tail emits `SEAM_PASS` for synthetic zero-test/exit-zero output. Controls with one pass and a failing command are retained. The prebuilt path already checks the test name; the issue is proof of execution under the actual flags. No claim that current main actually ran zero tests.
- **PROVEN source / DERIVED consequence, SB-01:** a tolerant wire helper can label timeout or incomplete frames as `Closed`. Require real closure where the contract needs it, with a deadline longer than the server boundary. No runtime mutation was executed.
- **PROVEN source / DERIVED consequence, SB-02:** a persistence test overwrites the account floor after reopening before checking the retained floor. Add independent readback before the first new write; retain existing shared-floor and integer-boundary checks.
- **PROVEN, DUR-02:** five bank crash-loop iterations fail after the entire writer has returned. Different observed tables do not create different internal fault points. Preserve rollback, backend-termination and replay cases; introduce genuine intermediate faults or consolidate only equivalent post-write aborts.
- **PROVEN source, DUR-01/03/05/07:** contradictory visibility source assertions, expected lock relations derived from production SQL, conditional assertions that skip wrong variants, and a self-confirming persisted-code roundtrip need stronger independent oracles. The V1 visibility and starter-island eligibility contracts remain unresolved; do not silently redefine either.
- **PROVEN overlap / runtime UNKNOWN, CT-02/03 and DUR-08:** repeated weapon/quest reproduction checks and source-mounted bootstrap parser tests are consolidation candidates. Their runtime cost, execution counts and any post-test integrity role require evidence before deletion.

Keep independent PostgreSQL fencing and value conservation, real EOF/TLS negatives, source/provenance and archive-integrity checks, parser boundaries, and distinct generators with superficially identical test bodies. Old names alone do not establish dead contracts.

## Black-box pilot

**RECOMMENDATION:** extend the existing real OAuth → ticket → Gateway/TLS journey with one move, logout and a fresh login. Assert expected position/revision, a new GameSessionId and independent durable state. Add a still-valid consumed-grant replay after logout, requiring exact `AdmissionGrantReplayed` and no additional state mutation. Existing lower-level replay, generation and concurrency cases remain valuable. A real public recovery issuer flow is still unverified. Detailed source anchors and the separation of live, harness-signed, capture and replay evidence are in the [server report](server_boundary_findings.md).

The earlier BB-02 expected-row completeness prototype remains unapplied. Resolve exact terminal membership and capture/group contracts before extending a matrix whose aggregate can currently miss a selected scenario.

## Verification and continuation

The first code-bearing candidate is `978d40a02e9e9d4b1d8c16fe7dea25aad586e108`. Its clean exact-head local results are preserved in [first-candidate validation](first_candidate_exact_head_validation.json), including environmental failures. The external provider [completed review without findings](https://github.com/Oteryn/Oteryn-Game/pull/3430#issuecomment-7087293938) for that exact commit, with a separate [provider receipt](https://github.com/Oteryn/Oteryn-Game/pull/3430#issuecomment-7087344354). Neither result is presented as external review of this later evidence supplement. No second paid review was requested for unchanged runtime/test/CI code.

Root explicitly reopened AUTHORING for this complete documentation batch, using a separate checkout with the published predecessor as its base. Root remains the only publication writer. The selected route remains the bound API-native new-candidate exception; no frozen local commit is reconstructed. After publication, freeze the new SHA and verify its exact tree, complete bounded delta, source-code identity and relevant governance checks. Live PR/check evidence records the resulting SHA without a metadata-only successor.

The enabled overnight continuation should read live PR/head/check state before every iteration, consume findings before further authoring, and work through the exact remaining scopes. No merge is authorized. Missing Rust, PowerShell, PostgreSQL or live topology cannot be replaced with a synthetic PASS.

Reproduce into a separate output directory; the durability probe verifies baseline source blobs. The content checkpoint requires the earlier inventory already present in this audit directory and preserves the historical blocked-import result without rerunning it.

```sh
python /path/to/wave-2/server_boundary_repros.py /path/to/checkout
python /path/to/wave-2/durability_oracle_probe.py --repo /path/to/checkout --output /path/to/new-output/durability_oracle_evidence.json
python /path/to/wave-2/scan_durability_oracles.py --repo /path/to/checkout --output /path/to/new-output
python /path/to/wave-2/content_tools_checkpoint.py --repo /path/to/checkout --output /path/to/new-output
```

The scripts perform source analysis or the explicitly synthetic shell-tail probe, not Rust/PG/game execution. For baseline reproduction, use a clean, materialized checkout of `8e9f0e16631aab1beb8416f8d5ca707aa3031083` with its Git objects available. Do not use Python `-O`, which disables the probes' assertions. The durability scanner reads HEAD/worktree and reports `modified_sources` without failing automatically; its output directory is not a result file. The durability predicate probe checks eight pinned blobs, but its mount count uses a filesystem glob and therefore requires that clean checkout, including no untracked Rust files in the scanned test directory. Its `--output` is a file. TSV files use quoted fields and LF while preserving parsed values. Manual scopes record earlier deliberate readings; rerunning a checkpoint does not itself perform or expand semantic review.
