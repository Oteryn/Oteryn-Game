# WHEEL-GEM-0A Wheel and gem reference values

- Decision: `WHEEL-GEM0A-REFERENCE-VALUES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (content and
  economy) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the architect rulings on #162 5933264015 (cap 225, Supreme Grade III 12.5M) and
  5936454476 (Basic slot 2, Guiding Presence), recorded by the control plane as the WHEEL-GEM-0
  amendment follow-up; the owner-directed Wheel/Gem packets 5932656862 and 5936424268.
- Builds on: WHEEL-GEM-0 §2, §4, §5.1 and its rows; WHEEL-0's W-R ruleset; owner decision
  5905825574 (TibiaPal planners are owner-verified evidence for Wheel stage values and costs); the
  FORMULA rule of the base-mechanics plan (#162 5929069698, item 6).
- Runtime, migration and production authority: NONE. GEM-R and W-R apply the values.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

Which source wins for Wheel and gem values, and what are the values the reviews flagged?

## 2. Source order (amends WHEEL-GEM-0 §4 and WHEEL-0's W-R)

For every Wheel and gem value (W-R and GEM-R), in this order:

1. an official CipSoft statement or the official manual;
2. owner-verified evidence: the TibiaPal planners and the tibiatools.io engine (5905825574);
3. the English TibiaWiki;
4. the pinned Canary revision (`OTS_HYPOTHESIS_ONLY`); Crystal corroborates only.

The highest source that states a value wins. A lower source that disagrees is recorded as conflict
evidence and never used. A value no listed source states stays `PARITY_PENDING` and is not
admitted at runtime. The newest upstream Canary is never chosen automatically.

## 3. Values

| Value | Was | Now | Source |
|---|---|---|---|
| `WHEELGEM0-RL-02` revealed gems per character | 250 (Canary, `PARITY_PENDING`) | **225** | English TibiaWiki; Canary 250 is conflict evidence |
| Supreme mod Grade III cost | 12 M gold, 15 fragments | **12.5 M** gold, 15 fragments | English TibiaWiki; Canary 12 M is conflict evidence |
| Basic slot-2 list, the five vocations | Canary/Crystal: includes ID30 Mitigation Multiplier, excludes ID2 Death Resistance | **includes ID2 Death Resistance, excludes ID30 Mitigation Multiplier** | TibiaPal (owner-verified) over Canary and Crystal, which agree with each other in all four checked versions |

These three are `PARITY_CONFIRMED` by their source and no longer `PARITY_PENDING`.

## 4. Guiding Presence (W-R)

The official value is +33%. Its arithmetic (a relative ×1.33 or added points, the rounding, and
whether the monk's own Serene copy is raised) follows the §2 order. If no listed source states it,
it stays `PARITY_PENDING` and the perk is not admitted at runtime until one does. A result shown by
the owner-verified tibiatools.io engine counts as a statement.

## 5. Migration

None. No character holds revealed gems or grades yet (GEM-1 is not built). GEM-R and W-R carry
these values in their first revision. If GEM-1 ships before this decision is accepted, the change
to 225 follows WHEEL-GEM-0 §5.3: characters above 225 keep their gems and cannot reveal more until
they are below the cap.

## 6. Owner questions

None. The FORMULA rule and owner decision 5905825574 settle every value here.

## 7. Before-freeze checklist

1. **Contract amendments:** WHEEL-GEM-0 §2, §4, §5.1, the rows and the declared differences. Applied
   in this PR.
2. **Serialization, restart, wire:** unchanged.
3. **Split work:** none.
