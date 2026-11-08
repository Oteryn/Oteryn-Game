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
