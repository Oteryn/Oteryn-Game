# New Wrath terminal reward slice: excluded from this batch

Only the unpublished terminal outfit/jade slice was dropped. Earlier green Wrath
opening, imperial hunt, source dream and crystal components remain on the branch.
Neither whole Wrath nor Colours is qualified.

Historical executed failing command (six-case run):

```sh
cargo nextest run -p oteryn-gameplay-dispatch --lib --config-file /tmp/questpc8-focused-nextest.toml --profile questpc8_focus --success-output immediate -E 'test(questpc8_wrath_late_journey_final_reward_atomic_outfit_and_jade) or test(questpc8_wrath_four_native_crystals_require_tool_order_and_replay) or test(nx6_wayfarer_real_npc_atomic_finale_replay_and_fences) or test(fathers_burden_eight_handins_cape_experience_and_replay) or test(questpc8_winterlight_native_disguise_four_fires_queen_and_base_outfit) or test(questpc8_venore_native_daily_circuit_and_exact_repeat_boundary)'
```

Failing case: `questpc8_wrath_late_journey_final_reward_atomic_outfit_and_jade`.
PostgreSQL marker reported107.911672992s, then:

```text
event=join_prepare_failed phase=appearances
event=join_prepare_failed phase=items
POSTGRES_CASE_FINISHED questpc8_wrath_finale duration=107.911672992s result=Err("early eof")
Error: "early eof"
```

The other five cases passed. The dropped new case and code are retained locally:
`/tmp/questpc8-wrath-finale-dropped.rs` and
`/tmp/questpc8b-wrath-terminal-dropped.patch`. Restore that local work before
attempting its historical command. The existing published helpers and callbacks
were restored exactly; no retained test was deleted or weakened.
