# ACCEPT-CONDITIONS-0 Acceptance of the actor conditions decision

- Decision: `ACCEPT-CONDITIONS0-V1`
- Status: **ACCEPTED BY THE OWNER 2026-10-14 VIA THE CONTROL PLANE**. Owner `blakinio` accepted CONDITIONS-0 with the §3.5 scope amendment (owner accepted via control plane). The acceptance takes effect only when this PR passes exact-head validation, independent review (combat, determinism and protocol) and CI on its frozen head and is integrated through the protected merge path. It changes no code, wire, migration or content.
- Accepts: `reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md` (`CONDITIONS0-ACTOR-CONDITIONS-V1`).
- Answers: MAGIC-COND-1 (mechanics 27, 28, 29) cites the decision while it still reads CANDIDATE. The acceptance unblocks only the decided part of that batch (§1); the families CONDITIONS-0 leaves deliberately not decided (§2 "Later families", §10: drunk, invisible, outfit, skill boosts, fear, root) stay gated on their own decisions.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What becomes effective

- Once the condition in the status line is met, the CONDITIONS-0 status line reads ACCEPTED, pointing here.
- §3.5 is new rule text accepted with it: keys, policies and gates for `muted`, `pacified`, `powerless` and `agony`, `exhaust` and `infight` as projections only, and `COND0-RL-01` raised from 16 to 32.
- The condition icons for the §3.5 families are not part of this acceptance; they go to COND-WIRE-2 (§7), which also carries the new typed refusal reasons for `pacified` and `powerless` only (a `muted` cast keeps the SPELL-PRESENT-0 `MUTED` detail; `agony` gates nothing). No wire change is made here.
- MAGIC-COND-1 scope after this acceptance: the six decided §3 families, exactly `SPEED` (haste, strong haste, paralysis), `DAMAGE_OVER_TIME`, `FOOD_REGENERATION`, `RECOVERY`, `MANA_SHIELD` and `LIGHT` (of which mechanics 27 and 28 are damage over time, haste and paralysis), the §3.5 keys `muted`, `pacified`, `powerless` and `agony`, and `exhaust` and `infight` as projections. Invisibility, outfit, drunk, skill boosts, rooted and feared policy are not decided here: §3.5 only keeps the existing rooted and feared keys and movement gate. No MAGIC-COND-1 child that needs their duration, refresh, dispel or immunity policy is allocated until its own decision is accepted.
- Amendments that other decisions wrote as "pending on acceptance" keep their wording; each takes effect only when its own source decision is accepted.

## 2. Acceptance record

- Owner: `blakinio`. Date: 2026-10-14. Basis: owner accepted via control plane.
- Review history in the decision stays as written. Implementation children keep their own exact-head validation, independent review and protected integration.
