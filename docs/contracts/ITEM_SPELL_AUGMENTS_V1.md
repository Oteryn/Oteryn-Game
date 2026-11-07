# Item spell augments v1 (SPELL-AUGMENT-1)

Server Content owns `NativeItemAttributes.spell_augments`. Each entry has an exact
ASCII case-folded spell name, a closed `ItemSpellAugmentKind`, and a nonnegative
integer value. Missing/empty preserves existing artifact serialization. The field
is part of the pinned server Item profile; it is never supplied by a client.
This field is independent of Wheel allocation and weapon proficiency.
Compatibility is `ATOMIC_REQUIRED` for the server binary and newly generated
server Content; the updated server also reads old profiles with the field absent.
Client and protocol bytes have no new fields.

Limits: 16 entries per Item, 128 bytes per spell name, 100000 permyriad per
percentage entry, 86400000 milliseconds per cooldown entry. The active artifact
loader rejects malformed names, values and excess entries before activation.
Only the fenced equipment snapshot contributes, once per occupied Item, never
per stack quantity. Inventory and unequipped Items contribute nothing.

Base damage/healing scales the corresponding base magnitude, truncating each
source contribution before critical and target mitigation and before Wheel flat
bonuses. Critical chance and extra damage add permyriad to the existing critical
stage. Life/mana leech uses actual staged HP loss, capped by target damage and
caster maxima, with the source area factor `(0.1*n + 0.9)/n` and nearest-integer
rounding. Leech and payment install together; command replay restores neither.
Cooldown values subtract milliseconds from the individual spell deadline, with
a floor of half its original cooldown; ordinary shared group cooldowns remain
owned by their existing rules.

The existing cast commands carry no new data and need no new capability,
command, domain or protocol registration. There is no new durable state and no
SQL migration. Reserved cap 131, cmd 152, dom 96, mech 0366 and migration 0261
remain unused; resource entries 1610–1613 document the new Content ceilings.

Reference evidence is data, not authority:

- [Canary 47dfd51f item parser](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/functions/item/item_parse.cpp): nested `augments` XML and integer values.
- [Canary spell application](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/combat/spells.cpp): percentage points, per-item base application, additive leech/critical damage and cooldown floor. Its critical-chance item entry is parsed but not applied.
- [Crystal 201189f8 spell application](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/combat/spells.cpp): permyriad, including critical chance. Crystal `base` lowers to both typed base stages; only the matching resolved effect stage applies.
- [Global announcement, 17 May 2024](https://www.tibia.com/news/?id=7880&subtopic=newsarchive): damage/healing augments act on spell base; Sanguine Blade adds 4% to Fierce Berserk.
- [TibiaWiki Augments](https://tibia.fandom.com/wiki/Augments): named spell item bonuses and stacking with Wheel.

Numerical details not published by Global use the pinned source behavior and
are **not verified against Global** (owner decision 22). In particular the
critical-chance implementation follows Crystal where Canary omits its application.

The retained numerical source packet is
`tools/content-schema/native-gameplay/item-spell-augment-sources.json`. Reproduce it
with `item_spell_augments.py --canary-xml <pinned items.xml> --crystal-xml <pinned items.xml> --out <packet>`;
both XML hashes are checked. `register_spell_families.py` attaches these values to
already admitted Item profiles and regenerates their content descriptor hashes.
This introduces no new Item identities or equipment admission rules.
