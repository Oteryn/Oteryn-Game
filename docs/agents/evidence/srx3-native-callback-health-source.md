# SRX-3 physical health source and component ordering

Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
This is an execution record, not a claim that the seven mechanics rows are complete.

## Source rules

- `data-global/scripts/quests/soul_war/soul_war_mechanics.lua:1091–1098`:
  Goshnar's Cruelty scales both signed components for a Player caster. The
  source callback has no Instance-only predicate. Missing defense drain uses
  the source default two; zero drain preserves the original components.
- `src/game/game.cpp:7934–7941`: ordinary negative health changes invoke the
  health callback before Wheel additions and only when origin is not NONE.
- `src/game/game.cpp:7827–7838`: the positive health path also invokes the
  health callback before its Wheel additions.
- `src/creatures/creatures_definitions.hpp:1732–1738`: a constructed
  `CombatDamage` defaults to `ORIGIN_NONE`.
- `src/io/iobestiary.cpp:32–49,131–147,200–235`: offensive/Parry charm
  leaves retain that origin and pass the real player to `doCombatHealth`.
- `src/game/game.cpp:8278–8288`: lethal damage still invokes prepareDeath
  with the actual attacker. Skipping healthChange does not erase the player.
- `src/game/game.cpp:7400–7401`: absorption healing passes a NULL attacker.
  A residual negative component retains its own real source.

The production implementation therefore distinguishes physical caster,
reward attribution and invocation of healthChange. Autonomous stream IDs are
not fabricated client commands. The existing current Quest reader prepares
player facts for the native monster pass; reflection revalidates those facts
against current session, lease, position, scope, generation and content pins.
Missing lethal-player facts remain refused rather than inventing zero taints.

Cruelty's periodic component is scaled after native mitigation. Auto-attack
components are scaled before Wheel additions, separately for physical and
elemental parts. Generic spell components use their existing magnitude owner.
The final HP ledger does not apply a second scalar.

## Observed regression results

All timings below are actual execution timings, separate from compilation.

| Exact filter | Snapshot | Observed result |
| --- | --- | --- |
| `srx3_cruelty_literal_health_event_reads_current_drain_on_world_creature` | 36b | RED, 56.71 s: `WrongScope` instead of absent drain; Instance mutation guard remains required |
| `srx3_cruelty_actual_player_condition_scales_health_component_once` | 36b | RED, 57.42 s: 946 instead of 927; NULL-source control retained native mitigation |
| `srx3_soul_war_reflection_stream_keeps_actual_player_source_without_health_event` | 36b | RED, 57.25 s: `PlanConflict` on a real nonlethal native FourthTaint target |
| `srx3_stationary_native_target_selection_publishes_source_facing_without_translation_postgres` | 36b | GREEN, 48.41 s: real scheduler publishes facing without translation |
| `srx3_charging_outburst_actual_native_compound_default_global_zero_postgres` | 36b | RED, 47.66 s: actual combat succeeded, postlude rejected the missing NUL-separated binding before NULL-health receipt |
| `srx3_charging_outburst_actual_native_compound_default_global_zero_postgres` | 38b | GREEN, 58.47 s: native combat and postlude succeed; real replacement reaches zero HP without Player credit; original caster is removed after one second |

The physical NULL-death corpse is checked separately; the successful attack
does not establish durable corpse materialization. The additional native boundary filter
`srx3_autonomous_player_source_keeps_prepare_death_and_bypasses_mirror_health_event`
checks lethal Player provenance and the independent Mirror health-event gate.

No attachment decision is inferred from these health-engine tests. The three
unattached donor declarations remain a separate production-attachment audit.
