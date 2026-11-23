# Wave 5 — exact content-tools remaining delta

The 10 distinct `STATIC_AST_ONLY` source paths listed in `semantic_scope.tsv` were read in full at source head `c8737fa0bc9d6801f2f4de6afd0a81ed149ed8e8`. They were not among wave 3/4 removals, and their original rows appear in `../wave-2/content_tools_remainingfiles.tsv`. The subtraction is only for fully read paths, **not** for passed tests or CI coverage.

Before wave 5: **667** paths = 423 `STATIC_AST_ONLY` + 10 `PARTIALLY_REVIEWED` + 234 `INVENTORY_ONLY`.

After wave 5: **657** paths = 413 `STATIC_AST_ONLY` + 10 `PARTIALLY_REVIEWED` + 234 `INVENTORY_ONLY`.

Historical-fixture methods, five explicit skips and one genuine Lua runtime dependency remain separately unqualified. The original 130 global groups and all other audit lanes remain separate.
