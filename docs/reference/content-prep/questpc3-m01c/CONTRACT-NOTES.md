# M01c decisions

Numbered-key collection checks current carried type, number and quantity. It
does not require the character to prove which historical box or tree supplied
a fungible key. A committed native acquisition/move is required; a fabricated
CommandRef is insufficient. Collection consumes no key.

For Black Knight, the accepted donor claims both bind key number 5010 to Item
2969: `tools/content-schema/quest-authoring/samples/chests/claims.json:9032`
and `:9057`, commit `9e5c8994d2784bb29d1013d7379f4f26c3f2e98b`.
The chosen recipe's minimum level 50 is enforced when fresh progress commits.
Entry progress does not authorize the final chest, reward or completion.

Grimvale's harvested blossoms are Item 22082, not planted nightshade 22063 or
22064. Source: `imports/canary/items-xml/items.xml:43886`, commit
`1b2e040732a7b0da269579380a4d3fa8a93d17aa`; the existing source importer also
checks the pinned appearance flags. The chosen first step requires five;
collection preserves their custody and requires registered premium admission.

Grimvale stage s2 lowers the chosen recipe's ten kills of Werewolf, Werebear
or Wereboar into one shared counter, active only after s1 and advancing the
route to s3 at ten. This slice follows the structured species/count contract;
it does not add a geographic predicate from the objective's prose. The PG
fixture exercises trusted native-death notification admission at Root, not
transport capture or an end-to-end completed Grimvale quest.

Native prerequisite identities use their registered completion owner. In
particular `oteryn:quest/rotten-blood` owns `game_character_rotten_blood.completed`:
`crates/gameplay/durability/src/durability/quest_state/rotten_blood.rs:569`,
commit `c68395ccc0029cb3c8511ec804dfe3536afd4cc6`. Admission reads this existing
flag under the Character root lock; it never creates a generic completion row
or falls back to one when the native owner says false. Other prerequisite keys
retain the generic completed-CharacterRevision rule. The PG query component
fixture rolls back its native rows with ownership guards enabled; it proves
the read path, not a Native boss victory or playable Decaying Defender route.

Decaying Defender's entry is one Bakragore victory gated by level 250, premium
and the registered Rotten Blood completion owner. It advances only its first
route phase and grants no outfit; Rotten Blood's existing victory owner keeps
its base-outfit grant. The admission PG component reads the production binding
and probes its level boundary, premium declaration and prerequisite matrix.
It does not claim a live level-250 boss victory or full quest completion.
