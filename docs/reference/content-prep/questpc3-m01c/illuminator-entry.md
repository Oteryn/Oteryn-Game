# Illuminator collection entry

The chosen recipe in `content/quests/definitions/quests-00200-00299.json:1`
requires level 200, premium and Between the Lines Quest. Its first stage collects
one Scallop Shell, explicitly resolved there as Item 51656. The retained Item
record in `content/items/definitions/items-28000-28499.json:1` names it
"scallop shell"; the parent exists as `oteryn:quest.between_the_lines_quest`.

The native entry observes carried custody without consuming the shell. It writes
only the first route checkpoint and cannot complete or grant the outfit. A
committed native inventory/reward occurrence is required; an arbitrary command
is not a custody proof. The
prerequisite reads the existing quest-completion owner; it does not create or
change the parent quest.

The PostgreSQL admission case uses a bounded fixture XP policy through the real
revision slot to produce an authoritative level-199/200 boundary. The existing
parent owner is represented by a registered completion-receiver fixture; this
case does not qualify the parent's gameplay route or production XP balance.
