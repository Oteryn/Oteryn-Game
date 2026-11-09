# M30: Child of Destiny native hit bindings

The binding follows the six-stage chosen Oteryn recipe in
`content/quests/definitions/quests-00200-00299.json`, including its explicit
adaptation: no vocation, spell, damage or character-template mutation. The only
final grants are three Meat (3577) and one Mana Potion (268). Dialogue text is
original Oteryn text. Completion is an explicit conversation with the Destiny
Seer at the exit; no inferred map position, action ID or legacy storage is used.

Checked sources:

- https://www.tibiawiki.com.br/wiki/Child_of_Destiny_Quest — training sequence,
  Party Skeleton practice targets, Neill, Salbra and Destiny Seer. Local captured
  MediaWiki response: `/tmp/engenc-source/wiki-Child_of_Destiny_Quest.json`.
- https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/npc/neill.lua
  — SHA256 `d616670eb790a99417d60983de782f8e383d327e7c703abcae4a08bca6ecaa36`.
- https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/npc/salbra.lua
  — SHA256 `e184df6402027b41cd78ffc7dda0506267542b6eacea82c9a081ee214f79fbf6`.

The pinned Canary NPC scripts are generic stubs, not implementations of this
quest. They do not justify additional grants or inferred legacy selectors.
The rune already has native Item admission. The mean paladin spear (17110)
needed its own bounded source qualification: its canonical record retains its
historical stack/admission hold, while the active native provider now uses the
exact pinned movable, pickupable, stackable and hand-slot facts. Capture them
with `engenc_training_profiles.py --capture <pinned appearances.dat>` under
`tools/content-migration/`, then run `register_spell_families.py` and
`refresh_equipment_profile_pin.py`. The source is Canary `47dfd51f`,
`data/items/appearances.dat`, SHA256
`aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50`,
with the repository's digest-pinned `items.xml`. Only 17110 is added; existing
native records are unchanged. Its explicit `None;true` source requirement is
represented by the existing `NONE` equipment vocation, and unknown source
vocation constraints still fail. No quest operation changes vocation.

This moves the strict equipment-profile union from 5795 to 5796 with one
independently declared cohort; no assertions or source holds are removed.
The binding observes committed positive health loss and does not award item USE.
The unit regression checks the canonical recipe digest and all six ordered
stages; the PG regression exercises the named spear/rune bindings from a
fixture whose initial phase is explicitly set to two. It does not execute the
NPC lessons, melee kill, or final reward delivery in PostgreSQL; the ordered
reducer test covers those bindings. The PG fixture uses genuine native damage
witnesses and the production sequenced writer, without fabricating a character
revision or receipt chain.
