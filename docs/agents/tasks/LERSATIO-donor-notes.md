# LERSATIO donor findings and implementation plan

Branch: `claude/happy-davinci-ijz3wy-lersatio`.
Base: `e2152d050038cecdb3728c222af2ac184e1cac5a`.
Status: donor inspection only; implementation and verification pending.

Primary reference: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/scripts/quests/blood_brothers_quest/actions_lersatio_mirrors.lua`
(read in full from `/workspace/crystal`).

- Lines 11–13 reject use after Mission08 reaches 2.
- Lines 15–19 specify mirrors at (32965,31457,2), (32966,31456,2),
  and (32968,31456,2); lines 33–39 transform 8667 to 8721.
- Lines 41–47 normalize a negative counter to zero, increment once per use,
  and schedule mirror reversion after 30 seconds: `addEvent(revertMirror, 30 * 1000, toPosition)`.
- Lines 49–53 transform the native statue 8325 to 8326.
- Lines 55–61 distinguish the first use from the second; the second resets
  MirrorCounter to zero: `player:setStorageValue(Storage.Quest.U8_4.BloodBrothers.MirrorCounter, 0)`.
- Lines 62–76 census current spectators on floor 2 and transfer only players
  to (32967,31459,1). The spectator call uses symmetric x/y extents of 9
  around (32963,31456,2), which must be considered when authoring the census.
- Lines 79–81 spawn Lersatio at (32967,31459,1) and Vampires at
  (32966,31459,1) and (32968,31459,1).

Next implementation steps:

1. Trace the production mirror-use callback and existing scene, encounter,
   timer validation and execution. Extend their shared composition minimally;
   preserve rejection of unrelated invalid combinations.
2. Author the mirror counter transitions, current player census, cross-floor
   relocation and cohort with existing shared encounter primitives. Use
   revision-fenced timers so a stale mirror reset cannot overwrite newer state.
3. Add mounted PostgreSQL cases through the real use caller covering both
   counter transitions, current-player filtering, relocation, one cohort,
   30-second reset, stale reset and retained invalid-combination rejection.
4. Run focused PostgreSQL cases, format and clippy, then required content
   authoring checks, unit tests, tree validation and generated-pin checks.

No runtime or content behavior is changed by this note. Reserved identifiers
remain migration 2888, capability 541 and ordinal 6276 if required.
