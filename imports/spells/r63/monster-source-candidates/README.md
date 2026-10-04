# R63 — complete typed inline MonsterSlot data

This private candidate packet closes the exact ten inline slots remaining after the nine accepted D25 normalizations in r54. It covers three donor/path variants of Grimeleech, six Angry Sugar Fairy slots, and one Cake Golem slot. All slot identities, original slot SHA values, and source parameter objects remain unchanged. Existing D25 candidates and sealed r51/r54 artifacts remain unchanged.

The proposed controller contract is `urn:oteryn:monster-inline-controller:1#/$defs/controller`, with kind `undefined_signed_health`. It contains concrete constructor defaults, signed bounds and branch semantics, resource destination, targeting/area geometry, visual identifiers, and actual source-helper audio values. This is full MonsterSlot authoring DATA with a pending private contract; consumer implementation, runtime/native admission, provider equivalence and activation are all false.

The undefined type stays `COMBAT_UNDEFINEDDAMAGE` (donor ID 4), including the misspelled life-drain symbol that resolves to nil in the pinned donor enum registry. No intended element, real life drain or mana drain is invented. Grimeleech retains the complete mixed interval [-565, 100], so a positive donor draw enters health healing despite the still-aggressive undefined type. Negative values enter undefined health damage; zero follows the nonpositive health path with zero base value. The health-provider mana shield guard excludes the undefined type after hooks; the initial undefined cast dispatches health rather than mana. Donor event/modifier providers remain explicit dependencies.

The sampler is the exact donor truncated normal: reject samples outside [0, 1], then return `minimum + lround(sample * (maximum - minimum))`. Equal bounds still consume the source normal draw and return that bound. A radius constructor value of 3 uses the donor's 13×13 stencil and activates nine cells, rather than a fabricated three-tile disk. The area draws once before dispatch and gives each target a damage copy, with donor per-target hooks retained. Constructor armor/shield blocking stays false, origin spell, aggression true, charges false, and no conditions or chain/target callbacks.

Visual symbols and numeric IDs come from immutable donor enums. The actual `readSpell` fallback helper writes its default impact sound into `castSound`, overwriting the earlier projectile sound; impact sound stays silence. The packet preserves this source behavior without repairing it. Sound and visual playback are unqualified.

All helpers and monster declarations use immutable `git show` at the exact Canary/Crystal pins. The proof records full file hashes and nine bounded function scopes per donor, covering constructor, signed sampling, health routing, radius construction, cast selection and scheduler range/timing guards. There are no new external reads or wiki normalizations.

```sh
/workspace/spell-tools/bin/python tools/content-schema/monster-authoring/project_monster_inline_complete.py
/workspace/spell-tools/bin/python -m unittest discover -s tools/content-schema/monster-authoring -p test_project_monster_inline_complete.py -v
```

Eight tests pass: exact population/raw invariants, schema metaschema and every controller, mixed signs and sampler offset, absence of drain/type/aggression repairs, exact area stencil/draw scope, actual audio overwrite, immutable helper scopes, and refusal of unsigned conversion or generic source programs.
