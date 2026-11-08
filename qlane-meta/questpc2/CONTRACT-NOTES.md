# questpc2 n2: Maeryn helmet preference

Source crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d, data-global/npc/maeryn.lua:60-70,134-173, SHA43b066a640d3b3d01a73cec683c0c0ddf7fce54e8e8491b0ddd80dc9dec954f8.

Six values: sorcerer0,druid1,paladin2,knight club3/axe4/sword5. Literal Lua zero
is truthy. The source's Grimvale prerequisite check is commented out; do not
invent a quest/item gate. The selection does not mint or enchant a helmet.

Choice words use whole-message lowercase equality; compound helmetyes only
offers because the source is an elseif chain. Earlier informational MsgContains
branches exclude overlapping inputs. The source's no has no cancellation branch,
so keep its topic. Knight temporary choice is represented by topic3 and cleared
by the ordinary conversation lifecycle, matching the greet reset.

Existing generic npc_rules::extend supplies the otherwise absent native track
with initial/min=-1 and max5. The existing fenced NPC writer owns the write.
Source custody is one exact program/NPC/file/revision/hash; new helper41lines.
No generic engine, boss encounter, lunar world change, helmet materialization or
whole-quest qualification is introduced. Own source packet reference is retained
in QUEST-STATUS; the frozen collector branch is not changed.
