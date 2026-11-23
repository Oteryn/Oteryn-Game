# Wave 7 — content-tools exact remaining scope

Baseline wave-2 `content_tools_remainingfiles.tsv`: **695** paths, of which original issue-only ten were full reads. After waves 3 (8), 4 (10), 5 (10), 6 (10), and current wave 7 (10), **58** distinct fully-read paths, **637** remain. This wave's ten paths are disjoint from those previous full-read lists and were originally `STATIC_AST_ONLY`.

| Category | Before wave 7 | After wave 7 |
| --- | ---: | ---: |
| STATIC_AST_ONLY | 403 | 393 |
| PARTIALLY_REVIEWED | 10 | 10 |
| INVENTORY_ONLY | 234 | 234 |
| **TOTAL** | **647** | **637** |

Every subtracted path and current source Git blob SHA is in `semantic_scope.tsv`. All **45** test methods were executed successfully, but the independent coin-ID-set oracle is still inadequate; PASS does not prove all intended converters. An additional **130 global review groups** remain separate and are not reduced by this wave. No historical fixture missing in earlier waves has become PASS.
