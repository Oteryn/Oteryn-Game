# Current populated Oteryn reference parameters

All values below are working Oteryn data, classified DERIVED with explicit assumptions in the target records. They do not assert that an unproven Global value has become official evidence. Native activation remains blocked.

| Subject / field | Populated Oteryn working value |
| --- | --- |
| shared_experience.eligibility / activity_timeout | `120` |
| shared_experience.eligibility / bonus_source_disagreement | `{"distinct_professions_1":20,"distinct_professions_2":30,"distinct_professions_3":60,"distinct_professions_4_plus":100}` |
| stamina.pool / initial_combat_drain_minutes | `2` |
| soul.regeneration / cap_source_disagreement | `{"normal":100,"promoted":200,"premium_affects_cap":false}` |
| regeneration.knight / normal_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":6},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":6}}` |
| regeneration.knight / normal_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":6},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":6}}` |
| regeneration.knight / promoted_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":4},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":4}}` |
| regeneration.knight / promoted_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":6},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":6}}` |
| regeneration.paladin / normal_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":8},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":8}}` |
| regeneration.paladin / normal_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":4},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":4}}` |
| regeneration.paladin / promoted_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":6},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":6}}` |
| regeneration.paladin / promoted_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":3},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":3}}` |
| regeneration.druid / normal_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":12},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":12}}` |
| regeneration.druid / normal_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":3},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":3}}` |
| regeneration.druid / promoted_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":12},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":12}}` |
| regeneration.druid / promoted_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":2},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":2}}` |
| regeneration.sorcerer / normal_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":12},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":12}}` |
| regeneration.sorcerer / normal_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":3},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":3}}` |
| regeneration.sorcerer / promoted_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":12},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":12}}` |
| regeneration.sorcerer / promoted_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":2},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":2}}` |
| regeneration.monk / normal_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":7},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":7}}` |
| regeneration.monk / normal_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":4},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":4}}` |
| regeneration.monk / promoted_hp | `{"interval_seconds":4,"saturated_interval_seconds":3,"points_per_tick":{"numerator":4,"denominator":5},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":1,"denominator":5}}` |
| regeneration.monk / promoted_mana | `{"interval_seconds":6,"saturated_interval_seconds":5,"points_per_tick":{"numerator":12,"denominator":3},"fractional_accumulator":true,"historical_rate_points_per_second":{"numerator":2,"denominator":3}}` |
| regeneration.food / cap_seconds | `1200` |

Parameter register: docs/reference/content-prep/shared-vitals-content-1/oteryn-parameters.json. Original source limitations: docs/reference/content-prep/shared-vitals-content-1/tibia-source-archive.json.

## Earlier Global source investigation

# Shared experience and vital regeneration

The live official manual was retrieved on 2026-10-18. Shared experience requires `3 * minimum_level >= 2 * maximum_level`, distance at most 30 tiles from the leader, and at most one floor difference. Its bonus is 20/30/60/100% for 1/2/3/4+ distinct vocations. These primary-source values take precedence over the conflicting community list; both observations remain recorded. The exact activity timeout still lacks official numeric support.

Official stamina and soul rules are recorded in their ruleset packets. Promotion sets the soul cap to 200 instead of 100. Numeric HP/MP intervals for all five vocations and their promotions are retained as structured community observations, not discarded or admitted as official formulas. Food reference data now covers all 146 regenerative rows in the current Wiki table, including 140 existing native IDs and six explicitly unbound source records. Five nutrition values have direct official-client spot checks; the other numeric values are labelled as Wiki-derived, not individually measured. HP/MP interval formulas, the food cap, and exact active-combat stamina drain remain held. Three formerly missing nutrition values now have explicit owner-selected reference defaults.

See [sources](content-prep/shared-vitals-content-1/sources.json), [regeneration](content-prep/shared-vitals-content-1/regeneration.json), and [food durations](content-prep/shared-vitals-content-1/food-durations.json).

## Food source coverage and measurement limits

The current rendered Food page contains 146 regenerative rows and 42 separately listed non-regenerative items. Every regenerative row is reconciled to its own current item-page revision. The duration packet now contains 140 existing-native-ID records and six explicitly source-only records (four IDs absent from native definitions and two multi-state cake pages). 143 numeric nutrition durations are retained from own-item Wiki data. The owner selected three additional DERIVED reference defaults: Cake (Item)240s, Cake with a Cherry240s, and Cheese Cookie24s copied from ordinary Cookie3598. All146 reference rows now have numeric durations; these three defaults are not exact-item source measurements. Five ordinary foods were spot-checked from an explicit hungry baseline using one item and a fresh official Food tooltip: brown mushroom 264 s, blueberry 12 s, cherry 12 s, raspberry 12 s, banana 96 s. These five checks agree with Wiki; no other row is claimed individually measured. The 42 non-regenerative names are retained separately without inventing zero-duration values. HP/MP regeneration rates, the food cap and other behaviour-sensitive formulas retain their existing evidence holds. No original binary, screenshot, character identity or balance is published.
