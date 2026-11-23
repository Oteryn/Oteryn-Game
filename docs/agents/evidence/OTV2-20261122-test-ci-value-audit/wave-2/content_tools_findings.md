# Content tooling test-value audit — second wave

Baseline: `Oteryn/Oteryn-Game@8e9f0e16631aab1beb8416f8d5ca707aa3031083`.
Scope: `tools/content-migration/**` and `tools/content-schema/**`, excluding craft-authoring and repository tooling. Read-only; no shared repository source edits or publishing. Source checkpoint: `2026-11-22T23:11:23Z`.

## Result and limits

**FACT / PROVEN:** The current item engine test entrypoint selects 107 of its 108 top-level test functions. It omits the focused test for the still-current non-pickupable Frost Cannon route, item ID 9132. This is an actionable coverage-registration defect, not a reason to remove that test. Two additional opportunities repeat the same core reproduction check through different entrypoints; their runtime cost and intended post-test integrity role are unmeasured. No wholesale legacy retirement is supported by this wave.

The indexed population is **706 test-bearing files / 5,510 literal Python test definitions**. These are static inventory numbers, not executable case counts. The largest groups are migration (213 files / 1,394 definitions), quest (213 / 1,522), and item (46 / 394). This wave selected those three groups for a static AST pass (472 files, no parse errors), then read targeted current consumers and representative tests semantically.

| Scope | Verified amount | Meaning |
| --- | ---: | --- |
| Entire test source files read | 11 files / 81 literal test definitions | Includes imports, fixtures, assertions and entrypoints; no implication that all dependencies or runtime cases were executed |
| Test files read in part | 10 | Exact line ranges in `content_tools_scopes.tsv`; other tests in those files remain unreviewed |
| Entire context files read | 5 | Runner, wrapper, scheduler, canonicalizer, and Quest workflow |
| Context files read in part | 7 | Exact caller/producer ranges only |
| Files not fully read in this wave | 695 | `content_tools_remainingfiles.tsv`; earlier audit scopes are preserved in a separate column |
| Successful test executions in this wave | 0 | One focused attempt stopped during import because `rfc3339-validator` is absent locally |

Every source file in the semantic scope manifest has a SHA-256 and a Git blob comparison proving that its reviewed bytes match the stated baseline. The static AST scan has its own manifest. It must not be reported as full semantic review. The remaining file list removes only this wave's 11 fully read files; it does not erase or expand earlier reviewers' exact scopes.

## Findings and decisions

### CT-01 — Register the omitted current Frost Cannon case

**FACT / PROVEN, change recommended.** `tools/content-schema/item-authoring/test_engine_items.py:3732–3750` defines `test_frost_cannon_routes_non_pickupable_blocking_prop`. The full `main()` list at lines 3811–3920 does not select it. Static AST comparison finds 108 top-level functions named `test_*`, 107 unique manually selected names, zero duplicate selections, and exactly that one omission. The complete list is recorded in `content_tools_oracle_evidence.json`.

The test exercises `engine_items.non_item_route` through the real conversion helper. It checks that ID 9132 routes to `WorldObject` with reason `non_pickupable_blocking_prop`, produces no item and no blocker, and that adjacent ID 9133 does not inherit this special route. The production rule remains in `tools/content-schema/item-authoring/engine_items.py:1662–1686`. This is a deliberately narrow exception whose adjacent negative control is useful.

The active consumer is `.github/workflows/item-authoring-schema.yml:130`, which runs the historical-context engine cohort. `check_tibiawiki165_historical_context.py:339–358` invokes the test script as a process. Its final main guard either enters the historical wrapper or runs the same manual `main()` list. Its additional unittest discoveries select three other files; they do not collect this omitted free function. A bounded source search across `tools` and `.github` found no other reference to the omitted function name. This proves the named case is missing from this selected entrypoint, not that no other test covers any part of the rule.

**RECOMMENDATION:** Add this function to the existing list, or move this file to reliable discovery while demonstrating preservation of every current selected case. The one-entry repair is the smallest change. A static inventory guard can detect future omissions if manual registration remains the chosen interface. Keep the focused case.

**Runtime limitation:** The attempted direct invocation was blocked before any assertion ran: `date-time format checking needs rfc3339-validator; pip install -r requirements.txt`. The dependency is pinned in item-authoring `requirements.txt:3`; the import guard is in `validate_item.py:30`. This wave does not claim the case passes, nor that its absence causes a currently observed gameplay failure. No dependencies were installed and no stub bypass was used.

### CT-02 — One CI step calls the same Weapon 103 reproduction routine twice

**FACT / PROVEN overlap; necessity and cost UNKNOWN.** `.github/workflows/item-authoring-schema.yml:210–211` consecutively runs:

```text
python check_tibiawiki165_historical_context.py --cohort weapon-tests
python check_tibiawiki165_historical_context.py --cohort weapon
```

Both branches call `current_weapon103(root)` (`check_tibiawiki165_historical_context.py:322–325`). The first additionally runs `test_lower_item_weapon_metadata_packet.py` in a temporary historical context. `current_weapon103` at lines 192 onward validates the current weapon owner scope and successor provenance, builds the historical weapon packet, and compares its canonical output against the pinned packet, allowing only the declared compiler-hash update.

This is the same core guard repeated in a single step, not two different 103-item cohorts. It is not evidence that either the producer unit tests or the pinned packet check is useless. The second invocation also occurs after the child tests; an intended post-test integrity role has not been established or ruled out. Temporary historical contexts can share input bytes using hardlinks, so blindly deleting a post-test check would assume immutability that has not been proven here.

**RECOMMENDATION:** Decide which entrypoint owns the reproduction check and explicitly retain any required post-test integrity check. Measure this step before claiming useful time savings. Do not remove `weapon-tests` or the historical provenance checks merely because the names mention an older cohort.

### CT-03 — Quest candidate byte reproduction has two current owners

**FACT / PROVEN overlap, low-priority consolidation candidate.** The `checks` part of `quest-authoring/run_checks.py:161` invokes `quest_reward_only_binding_candidates.py --check`. That CLI computes `compact(expected(root))` and compares it to the committed target (`quest_reward_only_binding_candidates.py:171–184`). The `unit-tests` part discovers `test_quest_reward_only_binding_candidates.py`, whose method at lines 100–103 recomputes and compares the same target bytes using the same generator and serializer.

The workflow deliberately has separate `checks`, `donor`, and `unit-tests` matrix parts; those parts are not themselves three copies of the same suite. The overlap is the specific byte-reproduction condition. The CLI additionally covers its invocation path, whereas the unit test separately checks target existence; it is therefore more accurate to describe duplicate core checking than identical execution.

The other five tests in that file assert independent, literal constraints: exactly 42 quests / 59 claims, explicit runtime non-admission, correct policy holds for single versus multiple claims, consistent storage versus key/value shape, and retention of a selected child quest outside the title inventory. They have distinct semantic value. `setUpClass` uses the real generator, so these tests can fail when its output violates those fixed constraints.

**RECOMMENDATION:** Give byte reproduction one explicit owner if runtime or maintenance noise justifies it. Preserve the policy assertions and any separate CLI argument/failure contract that is required. Runtime savings are UNKNOWN; this is not a major-cost finding.

### CT-04 — Exact method-body matches are false positives for identical execution

**FACT / PROVEN:** The static class-method scan over 472 Python files found three cross-file body clusters. All have distinct fixture or producer context:

| Cluster | Context inspected | Disposition |
| --- | --- | --- |
| Four Quest PC2 byte-replay methods | Entire Albinius, Gerimor, Gnomus and Henricus test files; each imports its own rule generator and targets its own outputs | KEEP. The remaining assertions also cover different gameplay transitions and guards |
| Two missing-declaration tests | Entire `test_npc_progress_retention.py` and `test_progress_enrichment.py`; one fixture exercises inferred mission progress, the other declared gate progress | KEEP. Identical assertion bodies do not make the fixtures equivalent |
| Three candidate packet replay methods | Entire reward-only test file; import/setup prefix and replay method only in explore-area and talk-NPC files | KEEP the separate producers. No complete semantic claim for the other methods in those two files |

Four apparent unreferenced free-function groups in item-authoring also use dynamic `globals()` discovery at the end of the file (`test_appearance_membership.py`, `test_lower_bed_packet.py`, `test_lower_ranged_packet.py`, `test_lower_timed_item_packet.py`). Their tails were inspected; a simple name-load scan does not prove they are skipped. They differ from CT-01's explicit incomplete list.

`content_tools_static_hints.json` records the exact methods, source hashes, and comparison definition. It compares class test method bodies; it is not a proof of semantic equivalence or an exhaustive duplicate-execution analysis of all top-level functions.

### CT-05 — Preserve real parser constraints and failure boundaries

**FACT / PROVEN in the reviewed scopes; KEEP.** Several tempting cleanup targets protect external-input semantics rather than reasserting local implementation details:

- `quest-authoring/test_converter_enrichment.py:1–116,260–398` passes handwritten Lua snippets through the real parser/converter. Literal expectations cover quotes and long strings, receiver identity, declaration/scope boundaries, operator precedence, source-position arguments, and rejection of Python-only numeral forms. The selected override test mocks unrelated source/provenance inputs while preserving the conversion boundary under examination. Only the listed ranges were reviewed.
- `item-authoring/test_engine_items.py:2959–3090` distinguishes harmless text line endings from binary-byte corruption and checks weekly-pool source identity against a decoy source. Other reviewed portions protect Boolean/type distinctions, precedence and guarded capacity. These are good negative boundaries; no mass snapshot replacement is recommended.
- `content-migration/test_world_project_v2_to_tree.py` invokes the real migration entrypoint and checks routing ownership, admitted weapon metadata, source round-trips, bank admission and the package/tree contract. Mutations remove or alter one declared source/binding to require refusal. Fixed inventory counts and explicit IDs here protect the currently admitted migration boundary; their presence alone is not an invalid oracle.
- `content-migration/test_parallel_checks.py` uses real child processes to test overlap, failure propagation, unique check names and invalid-plan rejection. The workflow aggregate test drives the actual extracted aggregate shell through failed/cancelled/skipped dependency combinations. It protects whether red work can become a green workflow, not merely the formatting of a Python list.
- `content-migration/test_world_compiler_artifact.py` uses fake API responses and real archive bytes to exercise the real restore/checksum/unpack path. Its 18 methods include wrong head/run/attempt/repository, digest or archive corruption, pagination, copied-run aliases, and refusal to reuse an older artifact after an unproved/newer producer. These are current CI boundaries, consumed by `.github/workflows/merge-group-gate.yml:444` and `integration-shards-dispatch.yml:91`. The external GitHub API itself is not contacted by these unit tests.

**RECOMMENDATION:** Retain small independent source snippets and one-fact-at-a-time negative mutations. Removing these in favor of a single successful full-world build would lose precise failure coverage. A new black-box layer should complement them.

### CT-06 — Canonicalizer component tests and real native CI have different jobs

**FACT / PROVEN:** `test_native_world_canonicalization.py` constructs much of its fixture inventory from `canonical.DOCUMENTS`, and most cases replace the native subprocess with a controlled writer. It validates orchestration: staging the explicit input, refusing malformed/missing/extra output, preserving unowned documents, refusing a concurrent input change, rejecting symlinks, and checking an explicitly supplied prebuilt binary. `test_regenerate_content.py:489–605` similarly replaces generation stages to verify order and stop behavior. These are component tests and do not independently prove the Rust compiler's serialization or gameplay output.

There is a real native complement in `.github/workflows/world-project-package.yml:114–134`: materialize two packages via the native prebuilt path or Cargo fallback; canonicalize both; compare them; canonicalize an already canonical package again; compare again; require 11 files and no symlinks. This is a concrete determinism/idempotence boundary. The optional binary and Cargo branches are mutually exclusive, not automatic duplicate work.

**INFERENCE / DERIVED:** A fixture built from production inventory constants is less independent for detecting a mistaken change to those constants. Other reviewed fixed-inventory assertions and the workflow's literal file count provide complementary constraints. The mocked component suite must not be credited as native end-to-end evidence. The real workflow's deterministic output, in turn, does not establish that every output field is semantically correct for gameplay.

**RECOMMENDATION:** Preserve both layers. The next black-box review should locate or define a small, manually specified source package with explicit expected serialized fields and failure/no-partial-write behavior, executed through the actual CLI/native boundary. Check for existing equivalent Rust/integration coverage before adding another lane. That next review has not been completed in this wave.

### CT-07 — Historical names and compatibility branches are not retirement evidence

**FACT / PROVEN within the selected consumers:** The item historical wrapper still validates the old pinned bindings, reproduces frozen packets, and separately validates current successor provenance. The selected workflow invokes it today. The migration workflow uses `parallel_checks.py` when present and an older command sequence only in the mutually exclusive fallback. The native materialization workflow explicitly supports an exact checked-out head predating its prebuilt helper. These branches are not proved dead by their labels.

The package test's `legacy_until_separately_qualified` expectation records that a runtime source switch is not yet authorized by that package; it is not evidence that the test has no consumer. A successful new pipeline and an explicit retired compatibility promise would be needed before deleting such coverage. No such retirement evidence was established here.

## Current consumer evidence and runtime qualifications

The CI API snapshot at `2026-11-22T22:12:43.922581Z` records the selected content-tree migration, item schema, quest schema, world package, merge-group and integration-dispatch workflows as **active** (`../ci_snapshot/workflow_live_state.json`). This is a point-in-time registration check. It does not prove every path filter matched the latest PR or that a recent run passed.

Current source traces include the Quest three-part runner, content-tree workflow's `--jobs 2 --workflow-checks` entrypoint, item historical-wrapper cohorts, and the real native package workflow. No historical list of all PRs was read. No Rust build, full content suite, workflow dispatch or paid review was launched.

Missing evidence is explicit: successful execution of the omitted case; current measured cost of the overlapping guards; the intended post-test immutability contract of the weapon wrapper; semantic review of every case outside the manifest; and independent whole-native/gameplay coverage beyond the selected workflow source. These limits prevent claims of full-suite qualification, large savings, or safe wholesale deletion.

## Reproducible artifacts

- `content_tools_scopes.tsv`: 33 exact semantic/context source scopes, line ranges, source hashes and baseline-blob equality.
- `content_tools_remainingfiles.tsv`: 695 files not fully read in this wave, including partial ranges and any earlier audit scopes.
- `content_tools_review_summary.json`: numeric scope, group inventory and qualifications.
- `content_tools_static_hints.json`: 472-file AST inventory and the three context-dependent method-body clusters.
- `content_tools_oracle_evidence.json`: complete 108-versus-107 selection proof and the blocked focused attempt's original stderr/exit code.
- `content_tools_checkpoint.py`: rebuilds the checkpoint using source reads and static AST only; imports no repository modules and runs no tests.

**Recommended next action:** Fix CT-01 in a focused follow-up with the declared dependency environment, preserving every current selection. Treat CT-02/03 as measured consolidation proposals. Keep the parser, source/provenance and native-boundary negative tests covered by this wave; continue the remaining queue from its exact scopes.
