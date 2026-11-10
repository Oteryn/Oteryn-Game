# Validation scope

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

The checkpoint builder independently verifies every ledger file hash and range against the exact clean input checkout, deduplicates range unions, partitions all tracked files into the actual Cargo packages and supplementary domains, and refuses completed-package claims with missing file coverage. Historical analysis transfers only across identical Git blobs. `snapshot-checks.json` records these results.

Existing source tests executed by workers are scoped in their reports: error-codes direct rustc unit tests 25/25 and panic-hook integration tests 3/3; characters stamina 13/13 and hunting-experience 6/6; input-actions 11/11 and input-platform 17/17. These direct unchanged-source runs do not replace native Cargo, Windows, PostgreSQL, TLS/GPU or full-system validation. Other test suites were inspected, not executed. No heavy integration suite was launched.

Publication qualification consists of `python3 tools/agents/validate_governance.py`, `python3 -m unittest discover -s tools/agents/tests`, `git diff --check`, full owned-path comparison, exact remote-head readback and payload hashes. The live issue records the actual published candidate and exact-head check outcome after execution; this file does not preclaim a future run.

Client UI semantic data validation executed separately: all 574 JSON descriptors/registrations/goldens/locales were checked for schema/required fields, safe source references and symbol presence, ownership/command coverage, normalized chords and parent cycles, expected golden/baseline equality, locale key/placeholder equality and actual spell-bundle manifest hashes. This does not claim Rust generator/AST execution or linguistic translation correctness. Rust UI source coverage remains separately incomplete.
