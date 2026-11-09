# Shared boss phase owner

`NativePhaseOwner` extends the existing native boss encounter mechanic. It binds
one scope generation, the complete content pin, exact actor incarnations and the
admitted player sessions. Source adapters supply a bounded dependency plan and
use the existing map, spawn and settlement owners.

`cults::CultsEncounter` supplies the five independent pillar/guardian branches,
stop-form replacement and the final Essence join. Native deaths are proved from
sealed corpse projections. Name matches, zero health, duplicate deaths and
unrelated actors cannot advance the join. A failed physical replacement restores
the carrier and remains retryable.

The production World callback `engenc2-cults.json` selects this owner during
atomic party admission. `ChannelSpellStates` owns the bounded phase registry;
the native death queue observes sealed deaths before loot admission and retries
failed physical replacements on the existing monster cadence.

Terminal settlement uses the existing fenced native quest writer. Only admitted,
current-lease players with actual damage contribution receive Humans.Mission=2.
The queue preserves their frozen damage evidence even when the corpse cannot be
materialized. Native and PostgreSQL tests cover the phase join, source callback,
participant filtering and once-only durable settlement. This proves the shared
phase closure, not the remaining whole-quest route or reward qualification.

Source: zimbadev/crystalserver at
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/scripts/quests/cults_of_tibia/creaturescripts_destroyed_pillar.lua`
and `creaturescripts_essence_of_malice.lua`.
