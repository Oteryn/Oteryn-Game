# SRVWLD M33 — shared physical STEP selectors

The current index lists M33 OPEN with claim none. Inspected the actual selector
validator and STEP dispatcher; both still rejected multiple candidates before
considering the Character's phase. No migration or numeric identifier allocated.

Source: retained tested questpc7 head
`2caf66704f735bde02cb68370a5b5e00dd2d87a6`,
`content/quests/questpc7-world-callbacks.json`: Feaster entry and purification
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

PARTIAL: Feaster's retained second route and its current-schema authoring
producer still need to be wired and qualified. No whole-quest claim is made.
