# M19 source and binding decisions

The implementation exposes the existing native `StateOperation` elapsed-time /
`SetNow` pair to World callbacks; the fenced Quest writer evaluates both against
one SQL transaction clock. No new clock, client timestamp or migration is needed.

## Bewitched

Canonical reference: `content/quests/definitions/quests-00200-00299.json`,
`oteryn:quest.authored.bewitched`, `recipe.stages[s4]` and `recipe.repeat`.
The accepted recipe deliberately replaces rotating source ingredients and the
eight-hour global event with a fixed daily personal contribution. Its selected
reward is 1000 XP. Source pages retained by that recipe:

- https://tibia.fandom.com/index.php?curid=50849&oldid=1190949
- https://tibia.fandom.com/index.php?curid=50859&oldid=1190948

The clock proof uses a full 86400-second elapsed day, not an assumption that
calendar-midnight resets or the original eight-hour global event are implemented.
The source-capture receipt retains historical hashes, but its `/tmp` raw bodies
are not present in this workspace. Fresh normal Fandom/BR/Polish Wiki reads on
2026-11-08 returned HTTP 402/403. No protection bypass was attempted.

A normal GitHub read of the pinned Canary Item source succeeded:
https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml
It names cauldron item IDs (including huge cauldron 12023), but an Item identity
alone does not establish the chosen cauldron's native map placement.
Green Claw Swamp's retained Area anchor is `(32769,32037,7)` in
`content/world/areas/regions/areas-00000-00442.json`; this is an area anchor,
not evidence for a cauldron at that exact tile. Do not conflate them.

## Threatened Dreams

Pinned source:
https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/threatened_dreams_quest/event_raven_herb_bush.lua
The committed donor corpus contains the literal branch that refuses collection
while `RavenHerbTimer > os.time()`, then stores `os.time() + 60 * 30 * 1000` and
grants one item 5953 from bush 25783 at `(33497,32196,7)`. Because `os.time()`
is in seconds, the proof preserves the literal 1800000-second interval; it does
not silently reinterpret it as 1800 seconds.

Important scope distinction: the canonical selected Oteryn recipe's s5 is the
moon mirror/fairy rescue. The Raven Herb timer belongs to source Mission03,
not that selected stage. Its Moon Mirror script checks the world day/night
period; Fairy Release schedules a 30-second world-object reset. Neither is a
personal elapsed-time predicate. Do not add the optional herb cooldown as an
invented mandatory prerequisite of selected s5, and do not call an isolated
clock proof a full Threatened Dreams walkthrough qualification.

## Validation boundary

`qready1_world_clock.rs` exercises lowered World callback clock effects through
the PostgreSQL Quest writer, including exact interval, atomic refusal, stale
revision and replay. It is a durable-owner proof, not a production region/input
qualification. Native bindings and end-to-end content checks remain separate.

## Registered bindings (M19 second slice)

`qready1_clock_callbacks.py` generates both registered callbacks. Bewitched
consumes three spider fangs and three wood units on the authored underground
cauldron site, fenced by a daily SQL clock. It does not grant final quest XP.
`qready1_clock_landmarks.py` installs that explicitly Oteryn-authored landmark
without changing donor terrain, houses or zones. Item flags and weights are
normalized from pinned donor XML and official appearance metadata, not guessed.

The Raven Herb grant uses the new committed clock value as its reward epoch.
The positive elapsed guard prevents command IDs from manufacturing reward epochs.
`qready1_raven_herb_clock_reward` proved atomic inventory/refusal/replay in PG
(52.713960823 seconds). `qready1_world_clocks` additionally exercises both
registered callbacks through wire USE in a composed PostgreSQL dispatch fixture;
that fixture is not a full quest walkthrough or night-spawn qualification.

Regeneration scope: M19 changes Native Item providers and a Native placement,
not legacy WorldProjectV2 inputs. Use `regenerate_content.py --native-only`
for provider/cohort/raid registries, the authored landmark overlay for the B3
placement, and the World bundle compiler's `pin-refresh` for Native World pins.
The equipment cohort is independently fixed at the four source-qualified IDs;
the pin tool rejects duplicate IDs or any changed zero-speed qualification.
