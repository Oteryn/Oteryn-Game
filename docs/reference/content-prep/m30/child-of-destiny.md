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
Existing qualified native ranged/rune profiles supply combat behavior; the new
binding observes committed positive health loss and does not award item USE.
The unit regression checks the canonical recipe digest and all six ordered
stages; the PG regression exercises the named spear/rune bindings from a
fixture whose initial phase is explicitly set to two. It does not execute the
NPC lessons, melee kill, or final reward delivery in PostgreSQL; the ordered
reducer test covers those bindings. The PG fixture uses genuine native damage
witnesses and the production sequenced writer, without fabricating a character
revision or receipt chain.
