# Wave 5 — protected governance CI compatibility and content test semantics

Current task: [draft PR #3430](https://github.com/Oteryn/Oteryn-Game/pull/3430), issue #3429. Root alone authored this batch in **AUTHORING** on a new isolated worktree from exact source head `c8737fa0bc9d6801f2f4de6afd0a81ed149ed8e8`, leaving all prior checkouts untouched. No merge/force/ref replacement or production/live change. The first native Codex review covers only `978d40a0`; no duplicate requested.

## P1 — actual protected-base gate rejects changed workflow (PROVEN)

At [CI job 137453644301](https://github.com/Oteryn/Oteryn-Game/actions/runs/45101654105/job/137453644301), `Merge authority audit / protected-base validate` failed before any merge because `agent-governance.yml` blob `67636ef86a52ee15dcccf3b18719761dcde47d37` differs from the **protected-base approved blob** `42d84c9089cdef928c6cb17268269156c87c0abb`. The earlier CI optimization had changed the protected workflow's test dispatch, which is prohibited by this independent gate. Other passing PR workflows cannot override this failure.

Repair (only the minimum source impact): restore the original approved `.github/workflows/agent-governance.yml` byte-for-byte; reintroduce imports of both standalone owner TestCase classes into `tools/repository/test_agent_governance_pr_targets.py`. The protected workflow again runs its exact original command and **the direct command executes 16 distinct governance cases, not 5** (5 own + 10 main-applicability + 1 merge-authority). On the local candidate `git hash-object` matches the expected SHA, direct 16/16 PASS, governance validator PASS and repository policy PASS. The **new-head protected-base CI itself remains pending** until GitHub executes it; the local hash match is not a CI PASS.

**Tradeoff disclosed:** the focused affected CI probe now records **53 invocations of the same 42 unique case IDs**, versus the earlier initial candidate's 42/42 and baseline 54/42. This restoration deliberately returns the 11 sibling calls to the protected workflow; do not claim the original 54→42 reduction on the repaired candidate. The test and workflow owner selection remains explicit and all unique cases retained. See `validation.json` and the separately captured CI probe output. This is a source/protection constraint, not evidence that 11 cases can be dropped.

## Exact manual test scope and observed execution

Ten new `tools/content-migration/test_*.py` files were read **in full**; their 62 literal `test_*` methods, blob identities, fixture, actual runtime consumer, independent oracle, unique failure mode and disposition are listed separately in [semantic_scope.tsv](semantic_scope.tsv). All ten have `unittest.main()`. None of their literal filenames appears in the 78 committed workflow YAML bodies; dynamic/inherited invocation remains UNKNOWN. Nine families retain meaningful refusal, immutability, source-digest and reference checks; historical names alone do not imply retirement. The tenth (mechanics) has meaningful checks that cannot run without its declared research inputs.

The single actual 10-module `python3 -m unittest -v` invocation yielded **49 reported method outcomes: 42 PASS, 2 method ERROR and 5 SKIP**, plus **two `setUpClass` errors** stopping **13 other methods** before execution; exit 1, zero assertion FAIL. **62 literal definitions are not 62 executed tests.** The exact blockers are:

- `test_complete_monster_audio_fields.py:89`: `lupa.luajit21` missing, so the genuine Lua parser/RNG oracle is unqualified (5 other methods PASS)
- `test_complete_monster_behavior_fields.py:19`: missing `/workspace/monster-final-output/population/population-index.json` (7 methods never started)
- `test_complete_optional_creature_behavior_fields.py:20`: missing `/workspace/monster-field-fill-20261002/population/population-index.json` (6 methods never started)
- `test_complete_remaining_loot_fields.py:43`: missing `/workspace/monster-field-next-20261002/loot-final-v5/field-patches.json` (7 other methods PASS)
- `test_complete_remaining_monster_mechanics.py`: 5 `skipUnless` cases skipped because both pinned `/workspace/monster-mitigation-estimate-output/population-v2` and `/workspace/monster-reference-sources/canary` are absent

**Disposition:** KEEP all current meaningful independent behavior/refusal oracles; REWRITE portability/execution selection of the historical dependent cases only when original fixtures can be recovered with provenance and digest verification. Missing fixture/dependency and `SKIPPED` are NOT PASS. Do not fake Lua/Python, fabricate the 718 census or historical actor snapshot, or loosen assertions. No content production/test code is changed in this batch.

`remaining_queue_overlay.md`: content-tool unread paths **667→657** by the ten exact full-file reads, not a claim of whole-repository completion. The separate global 130-group queue, BB-02 exact selected-ID matrix production adoption (unapplied), actual PG/Game/Platform, Windows/GPU, and full CI qualification remain OPEN. `validation.json` distinguishes source-head evidence from new-head CI.
