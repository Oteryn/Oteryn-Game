# Exact local audit of242 Source-waiting definitions

All counts refer to the current local completion worktree bytes pinned in audit.json.input_sha256. This is not a claim about fetched origin/main: Root reported newer main changes four Source cores. Re-run against their materialized definitions before any publication. No product files modified.

-352definitions:242Source waiting_data,68authored waiting_native_bindings,42definition_ready.
-All242Source-waiting definitions carry a qualified CHOSEN recipe with chosen_data_complete=true and waiting_native_bindings. No missing selected recipe data was found.
-21quests still depend on22live Item-semantics hold references. Zero stale Item hold flags found. Actual Item/variant work remains before reward execution can be admitted.
-187quests carry retained Source unknowns/disagreements; these are not cleared by a chosen journey. This count overlaps21Item-hold quests and228execution-binding quests.
-42quests have exclusively Native-execution flags; calling these missing quest data is misleading. Source metadata remains preserved.
-96quests already have QuestState track artifacts. Their179-source-quest Native flags include this96subset; derive TRACK_ARTIFACT_PRESENT_EXECUTION_UNPROVEN instead of saying no lowering file exists.42of96have COMPUTED effects and44have inexact Source predicates. Catalogue presence alone cannot admit execution.
-105quests have imported exact donor supplements.125previously partial guard records now have complete operational Source specifications across40quest owners, with execution still unproven; original guard statuses/holds are intentionally unchanged.
-Source codes:292report gap fields;33unknown requirement fields across26quests;3kind/log conflicts;3claim Source hold references across2quests. Unresolved_items includes conditions/control flow/blocked operations/reference discrepancies, not a count of missing inventory Items.

Minimal useful change: consume profile_readiness(definition) as a derived reporting field with original_source_readiness and chosen_data_readiness. Do not overwrite canonical readiness, remove missing_data, or call donor supplements Source completeness. Source completion selection and pinned baseline validation currently depend on the242 original waiting definitions. The same function uses explicit profile/complete/runtime/readiness fences, so invalid or future profiles fail closed.

PerQuest audit.json records retain every original missing_data issue, its concrete disposition/current claim status, donor supplement counts, Source guard profiles, track/effect counts and next work. The distinct status SOURCE_HOLD_NOT_CHOSEN_DATA_HOLE describes only the selected path: Source data gaps remain unresolved and must remain visible.

Run:
python audit.py --root /path/to/repo --out audit.json
PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s . -p 'test_*.py'

Six focused tests PASS: profile fencing/no Source mutation, existing lowering without admission, preserved Source holds, actual vs stale Item holds, unknown-code fail-closed, exact242 packet coverage.
