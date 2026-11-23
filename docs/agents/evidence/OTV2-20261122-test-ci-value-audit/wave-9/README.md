# Wave 9 — repair test selection and qualification evidence

Source predecessor: `245ba7838e58318a2d851857b6c2b2d7c757aa09`. Root explicitly reopened AUTHORING for the user's continuation, in separate local and VPS checkouts. Root is the only publication writer. [Draft PR #3430](https://github.com/Oteryn/Oteryn-Game/pull/3430) stays unmerged; the live PR and checks identify the subsequent frozen candidate. No main merge is needed for this bounded patch.

## Applied repairs and observed evidence

| Repair | Evidence and retained behavior |
| --- | --- |
| BB-02: require every selected indexed row's terminal identity | The planner derives 302 index-to-scenario contracts from the actual Rust dispatchers, including four explicit legacy emitters. Summary rejects an absent, duplicated, foreign-only, bounded-slice-only or capture-only substitute, even with `NODE_BOOT_PASS` and `--missing-informational`. Unknown dispatch grammar fails the plan. Existing FAIL/MISSING/SKIPPED policy and shard verdict ownership remain unchanged. |
| Fix the real aggregate test's positive fixture | `test_heavy_gate.py` now emits the plan's required IDs. A new mutation removes only `C03.full`, leaving 3/4 rows and successful producer/stage statuses. Both real workflow aggregate scripts reject it; their previous summarizer accepted it. [Independent replay](heavy_gate_fixture_review.json), [reproducer](prove_heavy_gate_fixture.py). |
| S3B cannot qualify zero or ignored tests | The actual shell tail requires exactly one successful libtest summary and retains child failure via `pipefail`. A real Rust 1.100.0 binary gives success 0→0, no match 0→1, ignored 0→1 and actual test failure 101→101. These are compiled fixture tests, not Game/PG/Platform qualification. [Before](s3b_real_baseline.json), [after](s3b_real_fixed.json), [reproducer](real_libtest_smoke.py). |
| Consolidate governance test ownership | Two unchanged TestCase modules become helper modules; the protected workflow still runs its original wrapper. Direct repository entrypoints and unittest discovery both drop 27→16 executions, with the same 16 unique cases. The focused original slice therefore drops 53→42, before counting the four newly added S3B tests. [Exact selection proof](governance_selection.json). |
| Restore the omitted Frost Cannon case | The manual engine runner now selects all 108 defined functions once. The actual engine cohort increases 599→603 checks; existing subsidiary unittest groups remain 3+3+4. [Root execution](content_root_validation.json), [oracle/selection probe](content_oracles.json). |
| Make content oracles independent and non-vacuous | Coin converters assert all four literal IDs, independent of the producer's loop. Calibration asserts exactly three named observations before applying the existing 10% bound. Empty/missing/duplicate/changed-reference mutants fail. Terminal-reference migration compares record count before/after and against the index, retaining the canonical identity, ordinal, uniqueness and unchanged body; an unrelated allocation still fails. |
| Connect three previously omitted content checks | The existing migration inventory grows 40→43; every previous command and working directory is retained. The actual parallel runner executes the three added modules successfully. Terminal-reference also gains its missing direct `unittest.main()` entrypoint. [Caller preservation](content_caller_preservation.json), [actual selected run](content_selection.json), [generator/pin closure](content_runner_pin_review.json). |

The source delta is 14 files including two byte-identical renames. No gameplay Rust, SQL, content data or workflow YAML changes. [Source manifest](source_manifest.json) records every new/modified/deleted source path and its hash. The governance workflow retains approved Git blob `42d84c9089cdef928c6cb17268269156c87c0abb`; the other protected workflows are unchanged as well.

## Real artifact replay and its limits

Root independently verified four downloaded ZIP digests against GitHub's artifact metadata for [run 45144642861](https://github.com/Oteryn/Oteryn-Game/actions/runs/45144642861), head `aa16e94d338a71ef9962b54ede37d1f3e8f4750b`. All 302 dispatcher blocks match the source predecessor; whole `world.rs` files differ outside those blocks. The old matrix and row-batch source are byte-identical. See [artifact provenance](ci_artifact_provenance.json), [source comparison](ci_source_comparison.json) and retained [result.json values](real_artifact_results.json).

| Real shard | Observed evidence | New summary behavior |
| --- | --- | --- |
| `rows-000-010` | 9/9 terminal FAIL rows, old stage/job success | Retains existing known-failure accounting; complete evidence is accepted. |
| `rows-080-090` | 9/10 terminal rows, old job already failed | Names the missing `C01.client`. |
| `rows-220-230` | 9/10 terminal rows, old job already failed | Names the missing `C20a.client`. |
| `rows-200-210` | 0/10 rows despite old stage/job success | Rejects it and names absent IDs; the previous aggregate already rejected zero-row evidence. |

These four samples contain FAIL rows, not successful live journeys or real CAPTURED samples. They do not establish a previously green whole workflow. The independent review also exercises complete synthetic 302-row PASS evidence and capture handling. [Root replay](real_matrix_replay.json), [CLI replay](real_cli_replay.json), [matrix review](matrix_peer_review.json). Basic/group/spells/Game–Platform completeness needs separate contracts; this patch claims only indexed-row evidence completeness.

The three workflow consumers carry the complete plan object and select their planner/consumer from the same event revision. Prebuilt helpers import the adjacent validator. The standalone workflow's summary has no additional explicit HEAD assertion; its ordinary event checkout is unchanged. [Caller closure](bb02_caller_closure.json) records the exact evidence and this limit.

## Validation

Root's integrated VPS sweep passed all **31 repository test entrypoints**, **109 node-boot tests**, **135 governance tests**, and **11 focused migration/runner tests**. Policy, governance validation and workflow pins passed. [Authoring sweep](authoring_validation.json) retains every command, exit code and log digest; [final documentation checks](authoring_final_checks.json) also passed.

The initial item-engine fixture setup failed with `EXDEV` followed by `/tmp` quota exhaustion, before collecting its assertions. The unchanged test was rerun with `TMPDIR` on the source filesystem: **603 checks and 3+3+4 subsidiary tests passed**. No fixture or assertion was weakened. [Environment recheck](authoring_environment_recheck.json) retains both the reason and successful output, plus the separate S3B shell syntax check. The sweep's original two-argument `bash -n` command only checks its first script; the recheck explicitly covers the second, and the retained proof driver now runs them separately.

These are AUTHORING receipts; the published candidate must subsequently be frozen and qualified from its exact commit. The proof driver checks all source hashes before running tests. PowerShell is a real isolated 7.7.0 release with a matching official archive digest; Rust is the pinned 1.100.0 compiler; Python dependencies are genuinely installed rather than substituted. [PowerShell receipt](powershell_verification.json).

Independent lanes reviewed both source changes and real caller fixtures. The content review retained the canonical-record mutation controls; item-authoring Ruff checks passed. [Content review](content_peer_review.json), [lint receipt](content_lint_validation.json). These peer checks are distinct from external Codex Review. The sole prior native review covers `978d40a0`; this wave does not invent or copy an exact-head review receipt.

## Reproduction

Run with real Python dependencies from the repository requirements, Rust 1.100.0 and PowerShell on PATH. Set `TMPDIR` to an isolated writable directory on the source filesystem for historical hardlink fixtures. Keep output outside the checkout. `--before` means an unchanged checkout of the source predecessor; `--after` means this candidate.

```sh
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/wave-9/prove_governance_selection.py --before /path/to/predecessor --after /path/to/candidate --output /tmp/governance-selection.json
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/wave-9/prove_heavy_gate_fixture.py --before /path/to/predecessor --after /path/to/candidate --output /tmp/heavy-gate-fixture.json
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/wave-9/replay_real_results.py --repo /path/to/candidate
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/wave-9/content_oracle_probe.py --repo /path/to/candidate --output /tmp/content-oracles.json
python docs/agents/evidence/OTV2-20261122-test-ci-value-audit/wave-9/real_libtest_smoke.py --repo /path/to/candidate --expected fixed --output /tmp/s3b-real.json
```

## Remaining work

This repairs already reviewed cases; it does not complete the audit. The content-tool queue remains 627 paths (383 static-only, 10 partially reviewed, 234 inventory-only), with 130 separate global review groups still open. No current-head full Game/Platform/PostgreSQL, Windows/GPU, historical research-fixture recovery or whole-CI cost/flakiness measurement is claimed. Physical journeys must have actual public action and independent readback before they count as passed. Eight scheduled overnight runs completed; this batch follows the user's explicit continuation.
