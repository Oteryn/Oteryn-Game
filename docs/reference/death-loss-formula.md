# Current populated Oteryn reference parameters

All values below are working Oteryn data, classified DERIVED with explicit assumptions in the target records. They do not assert that an unproven Global value has become official evidence. Native activation remains blocked.

| Subject / field | Populated Oteryn working value |
| --- | --- |
| death.experience / levels_24_plus | `{"level_at_least_24_base_percent":"(tmpLevel+50)*50*(tmpLevel*tmpLevel-5*tmpLevel+8)/experience","tmpLevel":"level+levelPercent/100","below_24_base_percent":10,"blessing_reduction_per_bless_percent":"6.31 retro PvP, 8 otherwise","promotion_reduction_percentage_points":30,"override":"DEATH_LOSE_PERCENT != -1 selects a different formula"}` |
| death.experience / rounding | `{"policy":"floor_nonnegative_final_loss_once","intermediate_precision":"rational","minimum_loss":0}` |
| death.skills / magic_and_skill_loss | `{"expression":"floor(total_accumulated_points*effective_death_loss_fraction)","included":"completed_level_costs+current_level_progress","apply_reduction_once":true}` |
| death.skills / retro_hardcore_blessing | `{"per_blessing_reduction_percent":6.31,"promotion_reduction_percent":30,"maximum_total_reduction_percent":100}` |
| death.pvp / unfair_fight | `{"pvp_threshold_damage_fraction":0.05,"lookback_seconds":300,"retained_loss_percent":"max(20,floor(100*victim_level/sum_distinct_attacker_levels+0.5))","apply_when":"qualifying_pvp_death and sum_attacker_levels>victim_level"}` |

Parameter register: docs/reference/content-prep/death-loss-evidence-1/oteryn-parameters.json. Original source limitations: docs/reference/content-prep/death-loss-evidence-1/tibia-source-archive.json.

## Earlier Global source investigation

# Death loss formula: evidence

Target: current data fetched 2026-10-18, as requested by the owner.

The community XP candidate for L ≥ 24 is `((L + 50) / 100) * 50 * (L² - 5L + 8)` XP. Classification: **UNKNOWN for gameplay admission**. The official manual only gives worked examples; the candidate cannot be promoted merely because it agrees with them. Low-level official loss is 10% of total XP through level 23. Promotion subtracts 30% of base loss and each ordinary blessing subtracts 8%, so seven plus promotion gives 86% reduction. These are additive reductions of the base loss.

Magic-level and skill losses apply to accumulated spent mana and skill tries, not directly to visible skill levels. The closed percentage and rounding algorithm remain UNKNOWN. The Wiki prose and worked table also disagree over a `/100` factor.

Retro Hardcore reduction is CONFLICT: Death Wiki says 6.4% per blessing, while the live official manual retrieved 2026-10-18 says 6.31%. The primary official value is retained; the source discrepancy remains explicit. The unfair-fight ratio and 80% cap are community candidates only. The official PvP-death test is at least 5% PvP damage in the last five minutes OR a character/summon final blow; red/black skull exceptions must be applied separately.

Every observation, classification and source selector is in [facts.json](content-prep/death-loss-evidence-1/facts.json); exact URLs, revisions and capture digests are in [sources.json](content-prep/death-loss-evidence-1/sources.json). No runtime implementation was changed.
