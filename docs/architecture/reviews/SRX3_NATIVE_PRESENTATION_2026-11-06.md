# SRX-3 native missile presentation

## Source rule and caller

Pinned Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/scripts/spells/monster/heal_brain_head.lua:2–15`, and pinned
Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
`data-otservbr-global/scripts/spells/monster/heal_brain_head.lua:2–15`, emit
`CONST_ANI_HOLY` from the actual caster to the fixed Brain Head cell, followed
by `CONST_ME_MAGIC_BLUE`. The production source callback adapter emits the
existing presentation protocol's missile cue. Its production server play test
and capability/idle-delivery qualification are tracked with W01c; these client
tests alone do not qualify that caller.

## Native client consumer

Feature 0023 now retains decoded `Missile` events in its bounded transient
visual queue and projects them through the native `PlayView` into the scene's
bounded emissive sprite batch. It preserves both server positions and the
missile ID, requires both endpoints on the displayed floor, animates the
tracer, and expires it using the existing transient lifetime. While official
missile sprites remain unresolved, this uses the same palette fallback as the
existing magic-effect consumer. This is a rendering limitation, not evidence
of official sprite parity.

## Executed regression loop

`cargo test --locked -p oteryn-client srx3_native_missile -- --nocapture`

The corrected baseline run failed all three tests (3.00 s), including a real
Session admission/snapshot/domain-19 delta decoded into the native play loop.
The implementation run passed all three (0.03 s):

- `srx3_native_missile_crosses_real_session_into_play_scene`
- `srx3_native_missile_mailbox_reaches_scene_and_expires`
- `srx3_native_missile_animates_and_culls_other_floors`

Development evidence: `/tmp/srx3-native-client-missile-red3.log` and
`/tmp/srx3-native-client-missile-green.log`. These paths are local execution
logs, not committed fixtures. No capability, protocol message, command ID,
or casting rule was introduced by this consumer.

The Plagirath declaration retains the literal `canary.appearance:effect/poison`
source reference, but it must not invent a numeric magic effect. Both pinned
`plagirath_bog.lua:3` files use `CONST_ME_POISON`; neither donor's C++ enum or Lua
registration defines that constant. Canary
`src/lua/functions/creatures/combat/combat_functions.cpp:59-65` passes the missing
value through `Lua::getNumber<uint32_t>`, and
`src/lua/functions/lua_functions_loader.hpp:215-233` converts Lua nil to zero.
Consequently its magic effect is NONE. Its separately defined
`CONST_ANI_POISON = 15` remains a distance effect. This preserves the executable
donor behavior rather than substituting POISONAREA21.
