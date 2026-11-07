BossLever Source conversion
===========================

Run `python builder.py --assignment ASSIGNMENT.json --corpus-manifest MANIFEST.json --ast-root AST_ROOT --out components.json`.

Uses verified existing raw Source and full structural AST only. Supports acquisition manifests with absolute caches or portable manifests with relative CAS paths. AST_ROOT accepts physical `captures/` plus index.json, or portable index.json.gz plus tar.gz shards; only assigned Source and shared-controller digests are retained in memory. No network, Lua execution, parser installation, wiki or runtime promotion.

83 component files become typed ordered Lua tables, literals, syntactic Position calls, ordered registrations, callback references, constructor references and all top-level statements. Duplicate table keys and unknown fields remain ordered. Computed expressions and callback bodies retain exact raw text and references into the existing verified full AST. Null name spans are honest upstream parser null spans; enclosing field/source text remains present.

Two shared controller Source candidates are retained with full hash witnesses, every controller table (including constructor default expressions), top-level statements and callback references. Controller activation/global bindings are not proven by filename or syntax. Source positions do not prove built-in Position identity. Directory owner candidates are not asserted canonical owners.

Status is TYPED_SOURCE_WITH_SEMANTIC_HOLDS for all 83; semantically complete/runtime-ready count is zero. These are Source component files, not 83 independent quests. Run `python -m unittest test_builder`; schema is standalone Draft 2020-12.
