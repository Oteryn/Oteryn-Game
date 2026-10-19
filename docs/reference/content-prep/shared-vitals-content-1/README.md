# Shared vitals: source reference and controlled regeneration observations

Current Global client measurements on 2026-10-19 establish the following for characters whose self-Look reports Druid, Monk, Paladin or Knight without active promotion:

| Vocation | Hungry HP | Fed HP | Hungry mana | Fed mana |
|---|---|---|---|---|
| Druid | +2 / approximately 4s | +2 / approximately 3s | +4 / approximately 6s | +4 / approximately 5s |
| Monk | +3 / approximately 4s | +3 / approximately 3s | +3 / approximately 6s | +3 / approximately 5s |
| Paladin | +3 / approximately 4s | +3 / approximately 3s | +3 / approximately 6s | +3 / approximately 5s |
| Knight | +4 / approximately 4s | +4 / approximately 3s | +2 / approximately 6s | +2 / approximately 5s |

Twelve qualified lossless recordings cover sixteen hunger/resource observations and resolve eight active normal-vocation fields. Positions outside PZ, equipment, food status and current vocation were checked. HP measurements used deficits and exclude capped readings. Original screenshots/videos stay private; reviewed series, timestamps and hashes are retained in [live-client-regeneration-probe.json](live-client-regeneration-probe.json).

These are scoped client observations, with capture/network timing jitter and no server telemetry. Do not extrapolate to promotions or other vocations, and do not select a native runtime formula from these measurements alone. 12 regeneration fields plus party activity timeout and initial stamina debit remain UNKNOWN. Invented fractional accumulators stay withdrawn. Food observations and the owner's three explicit food choices are unchanged.

Active classifications: 149 DERIVED, 40 PROVEN, 14 UNKNOWN. Historical withdrawal evidence and exact remaining paths are in [gameplay-source-reconciliation.json](gameplay-source-reconciliation.json); source registry:[sources.json](sources.json). Reference/data only; no code, pins, runtime activation or merge.

Knight fed HP is a short series:174→178→182, two full +4 gains and one3s inter-event interval. The following182→185gain is clipped by maximum HP and excluded. Timing agrees with the official3s fed interval; replication is weaker than the longer trials.
