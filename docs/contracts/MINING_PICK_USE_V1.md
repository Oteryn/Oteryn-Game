# Reference pick USE (MINING-1)

Owner direction: 2026-10-23, server-first basic mechanics, with the delegated
decision to implement only pinned Reference behaviour. Generic ore/stone
mining, invented yields and invented respawn timings are **NOT-IN-REFERENCE**.

The basic item conversion uses the existing command 2 USE, capabilities 11 and
15, and their existing bounded request/result codecs. A carried pick (3456)
used on a carried shiny stone (10310) consumes one stone and produces exactly
one crystal coin (1%), gold coin (5%), platinum coin (45%) or small diamond
(49%). The pick is unchanged. Evidence:
[Canary onUsePick, revision 52550d5e](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/data-otservbr-global/scripts/lib/register_actions.lua).

The native item-use writer admits `ItemUseCause` 3 `PickRefine` only with a
required one-unit output, reusing its existing unit TRANSFORM, nested-source
custody proofs and Type 2 event. Migration 0167 retains an immutable proof of
the unchanged carried pick, original USE digest and percentile. The source,
output, receipt, proof and audit commit in one transaction under the current
Character session-generation, recovery, lease and scope fences. WorldId and
ChannelId stay separate. An unavailable commit ends the connection without a
terminal result; replay reads the retained result before resolving handles.

The selected output is bound to the source instance and pre-use quantity.
Refusals cannot reroll it by changing CommandId. The writer rechecks native
definitions, backpack slots and carried weight under the Character root lock.
Wrong tools, unavailable targets, stale authority and full inventory change
nothing. Ground/map targets remain closed until their map-owner transition is
composed; the carried conversion grants no map mutation authority.

Bundle Worlds offer capabilities 11 and 15 only when their active native
generation includes the pick, shiny stone and every possible output. Existing
capability requirements and all other capability gates remain enforced.
No new wire numbers or resource ceilings are introduced: the existing two-item,
one-transform, one-event item-use shape covers the conversion; the tool is read
and locked, never modified.

Map digging, positioned obstacle removal and quest-state actions in onUsePick
are distinct transitions. This contract does not manufacture missing quest
state or admit them as generic mining.
