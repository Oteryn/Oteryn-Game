# QUESTPC boot-test hypotheses
1. Boot tests confuse raw SOURCE inventory with runtime-serving scope. Prediction: the two document sweeps error on the first Dawnport key; raw boot event counters still equal all SOURCE records. Correct only the sweeps to assert excluded keys absent and apply existing assertions to every served transition.
2. A normal quest transition is missing at boot. Prediction: the first missing key belongs to a non-Dawnport quest, and the scoped sweeps remain red. This requires escalation outside the owned test file.
3. Boot event counters changed to runtime counts. Prediction: after asserting the exclusion boundary, raw SOURCE counter equalities fail. Preserve those equalities and escalate production changes rather than modifying counts.
