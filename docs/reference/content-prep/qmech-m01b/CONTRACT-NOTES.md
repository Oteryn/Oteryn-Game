# M01b imported EXPLORE bindings

The assignment is the46 EXPLORE routes in ASSIGN.md, comprising56 selected
EXPLORE stages. This mechanic binds chosen Oteryn stage progress; it does not
claim original donor quest equivalence or completion of unbound later stages.

First slice: Longsword Quest, s1. The canonical Area declaration is
content/world/areas/hunting-places/hunting-places-00000-00444.json, record99,
identity oteryn:area.hunting_place.dusalk_s_troll_clan_cave. Its recorded coordinate is
32663,31970,7. Source: https://tibia.fandom.com/index.php?curid=12046&oldid=1097493 .

Conservative Oteryn rule: crossing that declared entry point credits the visit.
This is an explicit point, not an invented room rectangle or a source-equivalent
entrance boundary. Native floors negate the imported floor. The shipped region
content/world/placements/region-z07-x127-y124.b3 contains a plain4525 ground
entry at that point and the adjacent route tiles; it is not modified.

The ordinary fenced STEP_IN owner checks the exact native position, ground
ordinal and appearance. A separate once-only visited flag and the chosen stage
counter share one transaction. Admission grants no credit. Nearby equal ground,
revisits and literal command replay must not add credit or complete the quest.

The legacy callback absent=-1 sentinel is unsuitable for imported stage counters,
whose declared initial value is0. M01b requires explicit zero initialization for
its owned authored/chosen-source counter namespace, preserving legacy callbacks.
A chosen exploration profile must have a native selector, exact owned counter
tracks, the explicit chosen marker and no Item, XP, teleport or world effects.
No donor citation is fabricated to bypass the existing source admission policy.

The first cycle is bound here. Repeatable-event reset ownership remains with the
assigned world-change mechanics; exploration alone must not award a reward or
claim full event completion. No migration is allocated.

The chosen Longsword route is level0, free account, no prerequisites. The
generator fails closed on additional required guards. The earlier Asura
prototype is not activated/published: it requires an authoritative Premium
guard before inclusion. M01b remains in progress; only2000-2009 may be used.

## Geomancer s1

The second chosen entry is the canonical Mount Sternum/Undead Cave Area point,
32418,32017,z7, pinned by https://tibia.fandom.com/index.php?curid=20335&oldid=1114489
and Area record280. The unchanged retained region-z07-x126-y125.b3 has plain4529
at that point; the adjacent eastern tile4515 supplies the real STEP_IN approach.
This binds the chosen route's first exploration stage, not the later Geomancer
kill or reward. The recipe is free-account, level0 and has no prerequisites.
The generator now checks that a binding names an existing EXPLORE first stage,
its count is1, and its counter namespace belongs to that exact canonical quest.
Other stages fail closed until their individual visit/predecessor guards land.

## Current Premium entry admission

Reuse qmech's existing Callback.admission.premium field and the transaction-scoped
GrizzlyPremiumGate implemented by the native SpellPremiumCoordinator. The live
World writer obtains the actual Character account under the gameplay fence and
joins that provider before any exploration storage change. The ordinary legacy
World APIs retain NoGrizzlyPremium, so absence of an authority fails closed.
Normal execution and pending-command recovery both supply the native provider.

Asura's existing chosen recipe requires Premium. Its exact Area point remains
32953,32685,z7, plain923, pinned by
https://tibia.fandom.com/index.php?curid=75337&oldid=1175412 . No map data changes.
The PG case keeps the admitted session's Premium projection while removing the
transaction provider: cached session flags cannot authorize this write. After
restoring Current Premium, replay must preserve zero credit; a fresh entry is
required. Re-entry and replay after success stay at one.

The bounded M01b profile rejects promotion/regular_blessings declarations;
neither is needed by these bound exploration stages. The Premium latch-before-
Character-lock implementation is reused from inspected qmech M03 code. The
coordinator-requested0326 merge was aborted under the subsequent owner decision;
only the older already committed base is retained. Quest prerequisites still
fail closed in the producer. Existing minimum_level is
emitted for eligible recipes and remains owned by the locked progression read;
Fire Axe's level60 mapping is tested, but its physical anchor is not activated.
