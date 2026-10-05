# Bounded completion of partial donor guards

This batch refines the existing 636 concrete Source guard records; it does not create a general Lua interpreter, execute donor code or fetch sources. Before: 491 FULL_SOURCE_GUARD / 145 PARTIAL_SOURCE_GUARD. After: **511 FULL_SOURCE_GUARD / 125 PARTIAL_SOURCE_GUARD**. All 491 previously full guards remain full. Exactly 20 partial guards become full; two additional partial guards gain a comparison while retaining unknown siblings.

The new closed vocabulary preserves comparisons (`==`, `~=`, `<`, `<=`, `>`, `>=`) between proven lexical current-value reads, bounded exact literal values, and existing proven Source operands. It preserves actual left/right operands and operator; it does not infer the current value type, coerce nil into false, silently drop possible Lua comparison errors or substitute live storage reads for snapshots. Unknown getters, field accesses, helper calls and membership operations remain opaque. Bound locals retain the existing lexical declaration/scope/order proof.

Only already-proven acting-player progress reads with declared track identity and pristine Source `os.time()` are reused, via the existing guarded `donor_semantic_conditions.Normalizer`. Receiver mutation, Storage shadowing, reflection and undeclared progress reads remain opaque. Each Source file gets an isolated Script context; cache identity includes source, revision and path, not just a shared blob hash.

22 comparisons are added, including 12 declared-progress operands and two wall-clock operands. Examples include Dreamer's Challenge brotherhood/nightmare lever guards, Soul War pulsating-energy thresholds, Cults stored-health guard, Rotten Blood essence count, Warzone crystals and Shadows of Yalahar cooldown comparison. These are guard completions, not completed Quests or Native/runtime admissions.

The pinned upstream parser's long-bracket String value retains an initial newline that Lua removes. All long-bracket forms are conservatively excluded from literal normalization and remain opaque with lossless AST preserved; no parser fork. Verified quoted values retain decoded escapes. Integer literals are limited to the exact common integer range ±(2^53−1); unsupported forms remain opaque.

Remaining 125 partial guards contain 162 opaque leaves. Overlapping guard families: 103 unproven method identities/receiver types; 17 unproven helper/global functions; eight table-membership operations; 12 field/dynamic-index cases. Their exact Source witnesses and affected Quest keys are in `before-after.json`; none is automatically an architect blocker.

Copy-ready replacements: `builder.py`, `schema.json`, `test_builder.py`, `qualify.py`, `README.md`. Data/evidence: `conditions.json`, `qualification.json`, `before-after.json`, `handoff.json`. The schema extension helper is scratch-only. All current CLI arguments remain unchanged. The Source scope string now explicitly includes bounded comparisons.

23 focused tests PASS, including nil versus false, reversed operands, shadowed/undeclared/reflected storage reads, arbitrary calls/fields retained opaque, integer bound and long-bracket/quoted String controls. Qualification reads back all 636 concrete canonical gap references, pinned raw Source slices, AST scopes and normalized operands. Canonical opaque/readiness counters and Native/runtime admissions remain unchanged.
