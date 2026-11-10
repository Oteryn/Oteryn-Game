# Validation scope

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

The checkpoint builder independently verifies every ledger file hash and range against the exact clean input checkout, deduplicates range unions, partitions all tracked files into the actual Cargo packages and supplementary domains, and refuses completed-package claims with missing file coverage. Historical analysis transfers only across identical Git blobs. `snapshot-checks.json` records these results.

Existing source tests executed by workers are scoped in their reports: error-codes direct rustc unit tests 25/25 and panic-hook integration tests 3/3; characters stamina 13/13 and hunting-experience 6/6; input-actions 11/11 and input-platform 17/17. These direct unchanged-source runs do not replace native Cargo, Windows, PostgreSQL, TLS/GPU or full-system validation. Other execution is explicitly scoped below; remaining test suites were inspected, not executed. No heavy integration suite was launched.

Publication qualification consists of `python3 tools/agents/validate_governance.py`, `python3 -m unittest discover -s tools/agents/tests`, `git diff --check`, full owned-path comparison, exact remote-head readback and payload hashes. The live issue records the actual published candidate and exact-head check outcome after execution; this file does not preclaim a future run.

Client UI semantic data validation executed separately: all 574 JSON descriptors/registrations/goldens/locales were checked for schema/required fields, safe source references and symbol presence, ownership/command coverage, normalized chords and parent cycles, expected golden/baseline equality, locale key/placeholder equality and actual spell-bundle manifest hashes. This does not claim Rust generator/AST execution or linguistic translation correctness. All 758 Rust UI files, the manifest and README are now fully reviewed in addition to the semantic JSON coverage; native UI execution is not claimed.

Protocol registry data was semantically checked for actual capability ownership, prerequisites, offer closure and 137 matching registry/golden pairs. Two binary appearance fixtures were manually decoded field by field against the reviewed codec; byte coverage is separate from text lines. This does not claim native Rust protocol execution.

Architecture baseline semantic validation PASS: 15 size ratchets, exact 26 unsupported mechanics, the bounded purity exception and 61 logging owner/path sites. See `architecture-data-validation.json`; the Rust architecture binary and integration tests were inspected, not executed.

World compiler bounded validation: exact-source Cargo-input logic 5/5 PASS in a direct rustc harness; existing Python spawn-converter tests 9/9 PASS; two compiler binary fixtures decoded with the actual Python codec and compared with expected content. These 173 bytes are separate from text coverage. No full Rust compiler integration, allocation-heavy case or full-world pin check was run. See `world-compiler-review.md` and `world-compiler-binary-validation.json`.

A separate exact-source Highscores test extraction compiled successfully but its existing test failed: selection index 13 actually maps to ID 16 while the existing assertion expects ID 17; the next existing assertion also assumes the previous list length. The lead independently compared both extracted ranges with the exact source. This is a test-suite inconsistency observation, not proof of a production category-mapping defect and not a full Cargo suite result. Result: compile exit 0; test exit 101; 0 passed / 1 failed. The private packet retains the exact harness and log.

Gameplay-domain semantic validation covers every field of 1,875 source monster rows, 819 Bestiary indices, 835 core items, 1,675 trade rows and 23 packages. Consumed catalogue values match; the historical whole core-profile-file hash differs and is explicitly retained as a provenance observation. Existing unchanged-source Echo Warden tests 11/11 PASS and timed-item lane tests 21/21 PASS; full Cargo/property/SQL runs were not performed. See `domain-review.md` and `domain-catalogue-validation.json`.

Gameplay-combat validation: complete unchanged `charm_desc_fence_gate_tests.rs` compiled directly with rustc and passed all 7 existing tests, including the repository unbound-attacker-write gate. This lexical scan provides no additional substantive coverage of the files it scans. Exact hash, scope and output: `combat-fence-gate-result.json`.

Latest-main delta: the client lane read all four changed files in full at af33fd3baeac15d9991bd24c91bcacc8733492e2; the lead independently validated every latest/base hash and full range. Its source review stays separate from the immutable base coverage. No PowerShell, Windows, installer or relaunch tests were executed.

Checkpoint 7 adds parent-validated integration of the completed 422-file client worker allocation and further entity/durability source review. New creature periodic-processing candidates have independent source counterguard checks and selected catalogue-value inspection; neither is a native runtime reproduction or a confirmed finding. No source tests were added or changed.
