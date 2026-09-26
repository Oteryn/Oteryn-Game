# CONTENT-QUEST-01 r9 — Annihilator Reference field promotion matrix

Status: **REFERENCE_EVIDENCE_MATRIX / PROPOSED_NONCANONICAL**
Date: 2026-09-21
Tracking: #707 / PR #709
Reference target: `global-tibia-observable-2026-07-28-post-server-save`

## Purpose

Consolidate r1-r8 into one field-level promotion matrix so source completeness, current/historical official evidence, secondary corroboration and OTS physical/runtime hypotheses cannot be conflated into a whole-quest parity claim.

This matrix does not replace `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md`. It applies its field-level evidence discipline to the Annihilator investigation.

## Official source locators retained

- CipSoft Content Fixes, 2009-12-22: `https://www.tibia.com/news/?id=1188&subtopic=newsarchive`
- CipSoft current Achievement Library: `https://www.tibia.com/library/?subtopic=achievements`
- CipSoft historical Demon Outfit announcement, 2012-11-21: `https://www.tibia.com/news/?id=2240&subtopic=newsarchive`
- Official Character Trade examples around the target period, retained only as character-state observations and never as causal quest/reward proof.

## Promotion matrix

| Field | Best evidence currently retained | Evidence classification | Target disposition |
|---|---|---|---|
| Quest exists / Annihilator product identity | current official Achievement Library + pre-target official Character Trade achievement observations | `OBSERVED` official | strong target-era identity evidence; whole quest still not parity-qualified |
| Group size = exactly 4 | official 2009-12-22 Content Fixes explicitly says group of 4, no more/no less | `PROVEN` for historical cut | `DERIVED` continuity candidate for 2026 target; continuity still required |
| Achievement text/product shape = enter/survive/take reward | current official Achievement Library | `OBSERVED` current official | supports survival+reward shape; not exact target completion algorithm |
| Annihilator participates in Demon Outfit prerequisite chain | official 2012-11-21 announcement | `PROVEN` historical official | target continuity `EVIDENCE_REQUIRED` |
| Minimum level = 100 | Crystal source + current community corroboration | `OTS_HYPOTHESIS_ONLY` + secondary | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Exact lever coordinate | r7 strict parse of exact Crystal OTBM + startup/script composition | `OTS_HYPOTHESIS_ONLY`, physical source binding proven | `UNKNOWN` for Global target geometry |
| Exact room/reward/exit coordinates | r7 strict Crystal OTBM evidence | `OTS_HYPOTHESIS_ONLY`, physical source binding proven | `UNKNOWN` for Global target geometry |
| Six authored demon spawn coordinates | Crystal lever source + r7 exact tile existence | `OTS_HYPOTHESIS_ONLY` | `UNKNOWN` for Global target |
| Monster count = six | Crystal source + secondary corroboration | OTS + secondary | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Exact monster/stat block | Crystal Angry Demon source | `OTS_HYPOTHESIS_ONLY` | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Kill all six is required | no admissible official proof; Crystal source itself does not encode a typed KillAll completion outcome; secondary descriptions do not justify promotion | insufficient | `UNKNOWN`; must not infer from spawn count |
| Survival/reach reward is sufficient | official achievement wording is compatible with survival+reward, secondary material is supportive | official wording + secondary, but not exact algorithm | `UNKNOWN` as exact completion predicate |
| Reward opportunity exists | current official Achievement wording refers to taking home a reward | `OBSERVED` official | generic reward existence strongly supported; exact reward definition unresolved |
| Exactly four reward alternatives | Crystal four physical selectors/shared storage + secondary corroboration | OTS + secondary | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Demon Armor is an Annihilator reward | Crystal source + secondary corroboration | OTS + secondary | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Magic Sword is an Annihilator reward | Crystal source + secondary corroboration | OTS + secondary | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Stonecutter Axe is an Annihilator reward | Crystal source + secondary corroboration | OTS + secondary | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Annihilation Bear / Present is an Annihilator reward | Crystal source + secondary corroboration | OTS + secondary | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| ONE_OF_4 claim semantics | Crystal shared reward storage strongly composes as one source entitlement; r2 defines safe native lowering | source-derived OTS | `UNKNOWN` for Global target; safe fixture only |
| Reward source items 3213/3288/3319/3388 | existing CW2-B1 catalogue exact source-node evidence | `OTS_HYPOTHESIS_ONLY` | 4/4 source identified, 0/4 native resolved; not executable |
| Exact reward item stats / imbuement slots | existing CW2-B1 typed source observations | `OTS_HYPOTHESIS_ONLY` | not Reference/native values without owning GAME-ITEM/Content acceptance |
| Reward binding/transfer restrictions | no qualifying target evidence retained | none | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Achievement and item reward are one atomic commit | no qualifying target evidence retained | none | `UNKNOWN`; architecture keeps channels separate |
| Quest completion and reward commit are same operation | no qualifying target evidence retained | none | `UNKNOWN`; r2 supports explicit acyclic alternatives |
| Door/access behavior after reward | Crystal source has value-sensitive access behavior and later NPC mutation of the same legacy scalar | OTS source conflict/suspicious composition | `CONFLICT`/`EVIDENCE_REQUIRED` for target behavior |
| Base Demon Outfit transition after Annihilator | Crystal NPC is compatible with historical official relationship | historical official + OTS | exact target transition/state remains `EVIDENCE_REQUIRED` |
| Daily/server-save reset | Crystal default deployment shutdown/reload hypothesis + historical secondary wording | OTS operational hypothesis + secondary historical | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Exact reset clock/timezone | Crystal default 06:00 is source-profile config only | `OTS_HYPOTHESIS_ONLY` | `UNKNOWN`; never promoted |
| Process restart resets quest/reward cycle | no authoritative target evidence; conflicts with Oteryn recovery invariants as an implementation rule | none | prohibited as inferred Oteryn semantics |
| Death behavior | no qualifying target evidence retained | none | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Disconnect/reconnect behavior | no qualifying target evidence retained | none | `UNKNOWN`; Oteryn FND continuity still governs transport recovery |
| Retry/re-entry behavior | no qualifying target evidence retained | none | `UNKNOWN` / `EVIDENCE_REQUIRED` |
| Exact QuestLog text/state | no qualifying target evidence retained | none | `UNKNOWN` / `EVIDENCE_REQUIRED` |

## Explicit non-inferences

The following shortcuts are invalid and must remain rejected by importer/review tooling:

- official Character Trade possession/proficiency of Magic Sword or Stonecutter Axe does not prove the item was obtained from Annihilator;
- an official Annihilator achievement does not prove level, reward list, spawn count, reset cadence or completion predicate;
- six Crystal spawn records do not imply a Global or native `KillAll(6)` objective;
- four Crystal reward chests do not by themselves prove four independent claims; source composition instead shows one shared reward-consumption scalar;
- exact Crystal map bytes do not promote Crystal coordinates to Global target geometry;
- identical item-catalog blobs across Crystal revisions do not make unrelated behavior scripts interchangeable;
- community consensus cannot fill an official/accepted Reference `UNKNOWN` field;
- current official continuity cannot silently establish the selected historical target cut without the manifest continuity rule.

## Activation rule

A future Reference Annihilator definition may activate only the fields whose evidence state satisfies the accepted manifest policy. Material unresolved fields remain fail-closed or explicitly declared differences.

`PARITY_CONFIRMED` is not available while material fields such as minimum level, exact reward alternatives, completion qualification, reset policy, target geometry/access behavior, death/re-entry and QuestLog state remain `UNKNOWN`/`CONFLICT`.

## Current test horizon

No additional test IDs are minted by r9. Existing T42, T65-T71 and T72-T79 already cover evidence honesty, continuity, OTS non-promotion, physical binding and native reward resolution. Required corpus remains **79 cases**.

## Current disposition

```yaml
annihilator_reference_field_matrix:
  official_identity_and_achievement_shape: PARTIALLY_OBSERVED
  historical_exact_group_size_4: PROVEN_HISTORICAL
  target_group_size_4: DERIVED_CONTINUITY_CANDIDATE
  physical_crystal_binding: PROVEN_FOR_PINNED_OTS_BYTES
  source_reward_identities: IDENTIFIED_4_OF_4
  native_reward_bindings: RESOLVED_0_OF_4
  target_level: UNKNOWN
  target_reward_list: UNKNOWN
  target_completion_predicate: UNKNOWN
  target_reset_policy: UNKNOWN
  target_geometry: UNKNOWN
  target_access_after_reward: CONFLICT_OR_UNKNOWN
  target_death_reentry: UNKNOWN
  target_questlog: UNKNOWN
  whole_quest_parity: PARITY_PENDING_EVIDENCE
```
