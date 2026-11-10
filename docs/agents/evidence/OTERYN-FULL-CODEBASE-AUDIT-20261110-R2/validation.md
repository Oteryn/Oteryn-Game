# Validation scope

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

The checkpoint builder independently verifies every ledger file hash and range against the exact clean input checkout, deduplicates range unions, partitions all tracked files into the actual Cargo packages and supplementary domains, and refuses completed-package claims with missing file coverage. Historical analysis transfers only across identical Git blobs. `snapshot-checks.json` records these results.

Existing source tests executed by workers are scoped in their reports: error-codes direct rustc unit tests 25/25 and panic-hook integration tests 3/3; characters stamina 13/13 and hunting-experience 6/6; input-actions 11/11 and input-platform 17/17. These direct unchanged-source runs do not replace native Cargo, Windows, PostgreSQL, TLS/GPU or full-system validation. Other execution is explicitly scoped below; remaining test suites were inspected, not executed. No heavy integration suite was launched.

Publication qualification consists of `python3 tools/agents/validate_governance.py`, `python3 -m unittest discover -s tools/agents/tests`, `git diff --check`, full owned-path comparison, exact remote-head readback and payload hashes. The live issue records the actual published candidate and exact-head check outcome after execution; this file does not preclaim a future run.

Client UI semantic data validation executed separately: all 574 JSON descriptors/registrations/goldens/locales were checked for schema/required fields, safe source references and symbol presence, ownership/command coverage, normalized chords and parent cycles, expected golden/baseline equality, locale key/placeholder equality and actual spell-bundle manifest hashes. This does not claim Rust generator/AST execution or linguistic translation correctness. Rust UI source coverage remains separately incomplete.

Protocol registry data was semantically checked for actual capability ownership, prerequisites, offer closure and 137 matching registry/golden pairs. Two binary appearance fixtures were manually decoded field by field against the reviewed codec; byte coverage is separate from text lines. This does not claim native Rust protocol execution.

Architecture baseline semantic validation PASS: 15 size ratchets, exact 26 unsupported mechanics, the bounded purity exception and 61 logging owner/path sites. See `architecture-data-validation.json`; the Rust architecture binary and integration tests were inspected, not executed.

World compiler bounded validation: exact-source Cargo-input logic 5/5 PASS in a direct rustc harness; existing Python spawn-converter tests 9/9 PASS; two compiler binary fixtures decoded with the actual Python codec and compared with expected content. These 173 bytes are separate from text coverage. No full Rust compiler integration, allocation-heavy case or full-world pin check was run. See `world-compiler-review.md` and `world-compiler-binary-validation.json`.

A separate exact-source Highscores test extraction compiled successfully but its existing test failed: selection index 13 actually maps to ID 16 while the existing assertion expects ID 17; the next existing assertion also assumes the previous list length. The lead independently compared both extracted ranges with the exact source. This is a test-suite inconsistency observation, not proof of a production category-mapping defect and not a full Cargo suite result. Result: compile exit 0; test exit 101; 0 passed / 1 failed. The private packet retains the exact harness and log.
