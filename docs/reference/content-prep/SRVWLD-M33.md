# SRVWLD M33 — shared physical STEP selectors

The current index lists M33 OPEN with claim none. Inspected the actual selector
validator and STEP dispatcher; both still rejected multiple candidates before
considering the Character's phase. No migration or numeric identifier allocated.

Source: retained tested questpc7 head
`2caf66704f735bde02cb68370a5b5e00dd2d87a6`,
`content/quests/questpc7-world-callbacks.json:6127,6708`: Feaster entry and purification
entry share `(32627,32076,-7)`, appearance231. Their read-only programme guards
are exactly0 and5. Recipe source: canonical Feaster s2/s6 and
<https://www.tibiawiki.com.br/index.php?title=Feaster_of_Souls_Quest&oldid=421868>.
This is chosen-recipe evidence, not original-source-equivalence certification.

Engine slice: duplicate selectors are accepted only for the same Quest/player
STEP and provably disjoint read-only ranges on a shared track. Overlaps remain
invalid. Selection uses the current Character quest copy, not registry order;
the existing fenced native commit rechecks all guards after that preview.
No Channel/runtime guard is retained over the independent Character read.
Selection remains bounded and ambiguous/unavailable cases perform no callback.

Red: `m33_disjoint_readonly_phases_share_one_step_selector_without_ambiguity`
failed at the duplicate-selector assertion. Green checks acceptance, overlap
refusal and selection for programme0,5 and an ineligible intermediate value.
SQL is unchanged; this slice does not add a fake phase flag or mutate the
read-only programme guard.

The second slice registers both retained entry routes in
`content/quests/world-callbacks/feaster-phase-routes.json`, using current
`minimum_level`, premium admission and world-admission fields. Their exact
Wiki/chosen-recipe profile is closed in `feaster_routes.rs`; arbitrary URL-only
callbacks remain rejected. The source's entry receipt is retained and prevents
replay. No fabricated donor attribution is used.

Red: the production registry initially contained neither route. Green: the
registered routes select the two source destinations for programme0/5, refuse
unstarted and replayed entries, and reject changed destination, level, phase or
source evidence. The general phase-selector overlap refusal remains intact.
This implements the M33 shared-entry mechanic; the full expedition's NPC,
collection and encounter stages are separate content wiring, not claimed here.
No SQL or migration changed in M33.

Validation of the route slice: both focused M33 tests pass; all 396 quest-crate
unit tests pass (109.17 s); quest-crate clippy with `-D warnings` passes (33.59 s).
Quest-authoring `run_checks.py --part checks` and `--part unit-tests --jobs 3`
both pass; the latter executes all 114 discovered modules. M33 changes no SQL,
so no new PostgreSQL migration or SQL-case claim is attached to this slice.
