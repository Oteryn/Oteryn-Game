# Wave 8 — full-source migration review and test evidence

Task OTV2-20261122-test-ci-value-audit. PR https://github.com/Oteryn/Oteryn-Game/pull/3430, issue https://github.com/Oteryn/Oteryn-Game/issues/3429.

**PROVEN source head:** 759e99935fab5b9beab7ed2136db3841abbd67f8; Git tree e0012f3f2aedf7deb4390eda7d774a8eea6b91f1. The ten distinct full-file reads, Git blob IDs, consumers, selection/target, fixtures, independent oracles, unique failure modes and keep/rewrite dispositions are in semantic_scope.tsv.

**Actual execution:** Python 3.14.4 on isolated oteryn-vps, existing genuine zstandard==0.25.0 venv: **29/29 methods PASS, zero FAIL/ERROR/SKIP, rc=0**. Actual per-method results are in observed_methods.tsv; raw runner log SHA256 7dd95c97f0601bc66b3e2dd93db9bc23e36057ff65ec969caf9ef7725196516d. System Python attempt first gave 26 PASS plus one module-import ERROR for the three uncollected Nomads tests because zstandard was absent; authentic dependency allowed all 29 to execute. No mock or stub was used. These tests do not execute live PG/game/Platform/Windows/GPU.

**PROVEN oracle defect:** test_creature_level_rating.py calibrates a tolerance by looping over report calibration values, without asserting that any required row exists. Actual report has Bulltaur Lair 300, Nimmersatt's Breeding Ground 454 and Rotten Wasteland 790. The unchanged test still **PASS** when calibration rows are replaced in memory with an empty array. A proposed independent exact set-of-three mapping assertion passes real values and fails the empty mutant; this is an evidence proposal, not an applied test-code change. Preserve the tolerance and other creature tests.

**Useful negative control:** test_elephant_corpse_profiles.py is not similarly vacuous. Mutating producer IDS in memory from four real IDs 4360, 4361, 4362, 6052 to a singleton makes its existing source-capture equality test FAIL (3 of 4 methods still PASS); do not generalize the coin-converter defect to all IDS-driven suites. See oracle_repro.json.

**KEEP:** The other suites protect source/provenance digests, conflict refusal, corpse ground-only ownership, physical capacities, atomic Item admission, pinned Fugue combat, Nomads map location/revision, zero-XP elemental policies and training-spear stack/weapon constraints. No source, test, workflow or gameplay file changed.

**Selection evidence:** the real world_project_v2_to_tree.check_inventory(workflow_checks=True) lists 40 checks and none of these ten modules. Other indirect CI consumers and actual scheduling remain UNKNOWN. A missing literal filename is not proof a test never runs. See selection_evidence.json.

The content-tools queue declines 637 to 627 unread files; global 130 review groups remain. BB-02 exact scenario completeness is still an unapplied proposal, pending representative live logs, no-op and capture/replay semantics; CAPTURED is not PASS. No full Rust/PG/Game/Platform/Windows/GPU qualification or second Codex request. No merge, force, auto-merge or main write.
