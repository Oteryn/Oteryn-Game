# SRX-3 controlled companion offense: source qualification

Status: excluded from SRX-3 production scope after read-only duplicate/scope audit. Soulpit explicitly excludes familiars; normal controlled companion offense is not a W07 caller requirement. No unresolved W01 callback requirement was established for this additional feature. Preparation is preserved under /tmp/srx3-excluded-*; no source data or offense consumer was activated.
This supplements W01 and does not expand S17 spectator casting or change familiar acquisition/lifecycle.

All references below use Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
The complete attack vectors are `data-otservbr-global/monster/familiars/druid_familiar.lua:69-72` (4),
`knight_familiar.lua:69-75` (7), `monk_familiar.lua:69-73` (5),
`paladin_familiar.lua:69-73` (5), `sorcerer_familiar.lua:69-72` (4), and
`data-otservbr-global/monster/undeads/skeleton.lua:92-93` (2).
The existing formal 83 dependency records and profiles are preserved exactly.

The two named challenge slots are loaded, not inactive-pack references.
`data-otservbr-global/scripts/spells/monster/summonchallenge.lua:1-21`
(Git blob `298091db221994d5e5d8ee7a1c59238951f9acc7`) registers the spell and applies
BLUE, `AREA_CIRCLE2X2`, wall blocking and `doChallengeCreature(creature, target, 8000)`.
The exact rounded 5x5 area contains 21 cells; it is not a 3x3 area.
`src/lua/functions/core/game/global_functions.cpp:581-603` passes the actual caster to
`target->challengeCreature`; the default 6000 applies only when the argument is omitted.
`src/creatures/monsters/monster.cpp:3470-3485` rejects summon victims, selects the actual
caster, and installs the cooldown only when target selection succeeds. Lua returns true
even when this target-side operation refuses, so a presentation or return value alone
cannot prove taunt. The new source descriptor explicitly requires callback-only dispatch.

Identity is part of qualification. Legacy `canary:creature/monk_familiar` has five attacks
with chances 100/18/18/18/18, whereas canonical `oteryn:creature.monk_familiar` has a different
four-attack source vector. The producer never aliases these by display name. It compares
all actual legacy Native records with their complete formal bodies, allowing only the
already imported null reference-level field and the separately retained native melee
binding. A changed health value or canonical key refuses. Production must bind the real
physical policy key/revision and owner role before it consumes this declaration.

The preparation-only producer and test were removed from the repository and retained as `/tmp/srx3-excluded-build_controlled_companion_offense.py` and `/tmp/srx3-excluded-test_controlled_companion_offense_source.py`.
Evidence: complete-closure RED (25 versus 27, 4 tests, 0.486 s) then GREEN (0.484 s);
identity-guard API RED (5 tests, 0.472 s) then GREEN (0.457 s);
callback authoring codec RED (uppercase center, 6 tests, 0.514 s) then GREEN (0.492 s).
These are source-producer proofs, not production combat or native-client play proofs.

Remaining production requirements: actual master target and lease/lifecycle guards;
real companion physical caster and genuine Creature targets; source schedule/rolls;
normal HP callbacks with NonPlayer caster; separate controlling-master PvP authority and
reward credit; no master mana/rune/cooldown/weapon attribution; NPC-to-pet targeting and
actual pet defenses; meaningful dispatch/PG and native-client play coverage.
Wiki checks are `wiki-pending`.
