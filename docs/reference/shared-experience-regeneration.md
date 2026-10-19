# Shared vitals: measured references and owner-approved Sorcerer equivalence

Current reference values for vocations without active promotion:

| Vocation | Hungry HP | Fed HP | Hungry mana | Fed mana | Basis |
|---|---|---|---|---|---|
| Druid | +2 / ~4s | +2 / ~3s | +4 / ~6s | +4 / ~5s | Controlled client observations |
| Sorcerer | +2 / ~4s | +2 / ~3s | +4 / ~6s | +4 / ~5s | DERIVED: owner-approved 1:1 Druid equivalence |
| Monk | +3 / ~4s | +3 / ~3s | +3 / ~6s | +3 / ~5s | Controlled client observations |
| Paladin | +3 / ~4s | +3 / ~3s | +3 / ~6s | +3 / ~5s | Controlled client observations |
| Knight | +4 / ~4s | +4 / ~3s | +2 / ~6s | +2 / ~5s | Controlled observations; fed HP short series |

The owner explicitly instructed Sorcerer to equal Druid 1:1. Its two normal-vocation fields are DERIVED from measured Druid amounts and intervals. Sorcerer was not directly measured; the controlled recording and observation counts are unchanged.

Twelve qualified lossless recordings cover sixteen hunger/resource observations for four vocations and resolve eight active measured fields. Original game media remain private; reviewed values, timestamps and hashes are in [live-client-regeneration-probe.json](content-prep/shared-vitals-content-1/live-client-regeneration-probe.json). The same document retains the two owner-derived Sorcerer references separately from measurements.

Knight fed HP is a short series: 174→178→182, two full +4 gains and one 3s interval. The following 182→185 gain is clipped and excluded. Timing agrees with the official 3s fed interval; replication is weaker than longer trials. All displayed periods are approximate, with capture/network jitter and no server telemetry.

All five normal-vocation reference branches now have values. Ten promoted regeneration fields, exact party activity timeout and initial stamina debit remain UNKNOWN. No promotion values are invented. Food observations and the owner's three food choices are preserved; withdrawn fractional accumulators remain withdrawn.

Active classifications: 151 DERIVED, 40 PROVEN, 12 UNKNOWN. Exact remaining fields and historical changes:[gameplay-source-reconciliation.json](content-prep/shared-vitals-content-1/gameplay-source-reconciliation.json); source registry:[sources.json](content-prep/shared-vitals-content-1/sources.json). Reference/data only; no code, generated pins, native runtime selection, activation or merge.
