# Current populated Oteryn reference parameters

All values below are working Oteryn data, classified DERIVED with explicit assumptions in the target records. They do not assert that an unproven Global value has become official evidence. Native activation remains blocked.

| Subject / field | Populated Oteryn working value |
| --- | --- |
| training.offline / exact_skill_rates | `{"melee_tries":"training_seconds / (promoted_base_attack_ms/1000) / 2","distance_tries":"training_seconds / (promoted_base_attack_ms/1000) / 4","magic_mana":"training_seconds * current_vocation_mana_amount / max(promoted_mana_ticks_ms/1000,1)","rate":"multiply by RATE_OFFLINE_TRAINING_SPEED before addOfflineTrainingTries"}` |
| exercise.efficiency / melee_hits_per_charge | `7` |
| exercise.efficiency / mana_per_charge | `600` |
| exercise.efficiency / bow_rate | `{"tries_per_charge":7,"charge_interval_ms":2000,"base_rate_multiplier":1}` |
| exercise.efficiency / stamina_use | `{"stamina_consumed":false,"stamina_regenerated_by_training":false}` |

Parameter register: docs/reference/content-prep/training-content-1/oteryn-parameters.json. Original source limitations: docs/reference/content-prep/training-content-1/tibia-source-archive.json.

## Earlier Global source investigation

# Offline training and exercise weapons

Current source revisions were fetched on 2026-10-18. Statue combinations, offline pool/delay, Free-account caps and charge counts are recorded in [facts.json](content-prep/training-content-1/facts.json). The official manual confirms statue/bed activation, while exact skill rates require official or controlled evidence.

Training/exercise/durable/lasting charge counts are 50/500/1800/14400 according to the current Wiki; at two seconds per charge this derives 100/1000/3600/28800 seconds. The offline pool is distinct from hunting stamina.

The Exercise Weapons article contains a bow-efficiency conflict: its introduction gives 3.6 hits/charge, while the detailed section distinguishes 2.16 blood and 4.32 miss hits and its 500-charge table gives 2160 hits. Keep the rates CONFLICT or UNKNOWN; never use them as a proven vocation skill formula. The Training School article explicitly calls its mathematical models estimates.

No missing Item identity or charge field is invented. Item-definition wiring remains a separate exact-identity admission step. [Sources and exact revisions](content-prep/training-content-1/sources.json).
