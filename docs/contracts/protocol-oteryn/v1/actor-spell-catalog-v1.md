# Own-actor spell catalog (S-SPELLBOOK)

Capability 287 selects domain 100. Join and reconnect send snapshot type 1 at revision 1.
Delta type 1 is a complete replacement with the next connection-local revision, sent on the
existing one-second vitals cadence after level, learning or vocation changes. Unchanged
catalogs produce no delta. The catalog is derived from the exact admitted content book;
indices never come from a level-sorted display list. Inactive aliases are excluded.

Each active spell of the current vocation is included, including rune-use spells. Learned
spells retained across a vocation change are also included; eligibility follows the real cast learning/vocation gate.
Rune use has no spoken words: its words are empty and its item identity is present.
Instant rune-conjuring spells retain their own spoken words and cast index.
Cooldown group ordinals match presentation domain 20 and the existing S9 group order.

`learned` reports the existing durable known-spell set. `available` reports vocation, required
level, learning (when required), premium and rune magic-level eligibility. It does not promise
a legal target, equipment, sufficient mana/soul or an expired cooldown. Cast admission remains
authoritative. Mana preserves fixed versus percentage-of-maximum semantics.

The typed codec enforces the schema bounds, UTF-8, ascending unique indices/groups, booleans,
and carrier identity consistency. Unknown fields and repeated singular fields fail closed.
No migration and no new durable writes are introduced.
