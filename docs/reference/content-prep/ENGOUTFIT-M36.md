# M36 — durable NPC outfit/addon sources

Source checked on 2026-11-08:
<https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/rock_steady.lua>

The fetched 4,255-byte file matches the retained donor corpus SHA-256
`2111b23a930e50619fde1541883e443e434d347b0b2d2116d52cab9bd0bc47d1`.
Lines 56–98 define raw storage 72326: help/addon starts stage 1;
collect then yes advances to 2 without taking an item; subsequent confirmations
consume one item 14022 and then one 14023, reaching stages 3 and 4;
collect at stage 4 consumes one 14021 and grants addon 1 for looks 464/463,
then records stage 5. The initial confirmation must not invent an item hand-in.
`engoutfit_m36.py` lowers these five durable operations and three offers to the
existing bounded NPC reward dispatcher. The collection uses the source-private
`oteryn:quest.rock_steady_collection` state owner; adding these new stages must
not change the published aggregate Outfit and Addon Quests definition hash. The shared appearance writer owns the
final addon and base entitlements in the same transaction as the last hand-in.

Yalahar uses the already admitted source document
`content/dialogues/quest-rules/quest-batch3-a-in_service_of_yalahar.json`:
Palimuth `conversation-244-27-1-1` grants looks 324/325 and moves questline 52→53;
`conversation-323-38-1-*` grants the source-selected addon bits. Those immutable
source rules now select the shared durable outfit planner and account snapshot.
The donor citations in that document retain their exact pinned source lines.
This change does not certify the separate money, teleport or encounter stages.
