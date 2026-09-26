# Reference evidence/parity manifest revision 4 — public source packet

- Date: 2026-09-17
- Worker: `REFERENCE_MANIFEST_V4_514`
- Allocation: #162 comment `5710521667`
- Evidence gate: #514
- Admission main: `b44fefe08f6aaf1b2c1c23dedd92bab0de87146e`
- Reference target: `global-tibia-observable-2026-07-28-post-server-save`
- Manifest change: revision `3 -> 4`; schema version `1` unchanged
- Classification ceiling: the exact bounded fields below; no `PARITY_CONFIRMED`

## Disposition and publication safety

This packet normalizes only the sources already bounded by #514 comments `5609250509`, `5609275116`, `5613858159`, `5613885727`, `5703451569`, the exact #162 allocation, protected #576/#637 evidence, and the schema-v1 serialization amendment in #162 comment `5711026915`. It does not perform or rely on a broader source sweep.

All external locators are public web pages. This packet paraphrases facts and stores no page copy, proprietary asset, restricted capture, credential, personal data, or third-party implementation. For the admitted, bounded claims, provenance and legal review are `CLEARED`; that disposition does not clear excluded fields or promote evidence beyond the classification shown. `COMMUNITY_CORROBORATION` is the explicitly selected schema-v1 serialization for the investigator role `STRUCTURED_REFERENCE_DATA`; it remains `DERIVED`, never `PROVEN`. Canary, Crystal, TFS, other OTS code, and legacy Oteryn repositories are not used to promote a target claim.

The official Radiant Skyhold `2000` threshold candidate is omitted. Its expected target-boundary locator was not directly retrievable in the recorded refresh, and a community mirror cannot satisfy the exact allocation's provenance/legal prerequisite for `PROVEN`. The starter-item template and exact low-level XP thresholds are also omitted rather than weakening their `UNKNOWN`/`CONFLICT` ceilings.

## Source records

<a id="s1-newhaven-to-targuna-route"></a>
### S1 — Newhaven to Targuna route

- Locator: `https://www.tibia.com/news/?id=8733&subtopic=newsarchive`
- Effective date: `2026-03-17`
- Retrieved/index-verified: `2026-09-10T05:53:03Z`
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: level-8-or-higher Newhaven continuation goes to Targuna rather than directly to Thais Peninsula.
- Uncertainty: no coordinates, map geometry, transition protocol, NPC identity, vocation transaction, or starter template.

<a id="s2-targuna-production-fix"></a>
### S2 — Targuna production fix, 2026-03-31

- Locator: `https://www.tibia.com/news/?id=8747&subtopic=newsarchive`
- Effective date: `2026-03-31`
- Retrieved/index-verified: `2026-09-09T21:55:09Z`
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: pre-target follow-up corroboration that the Targuna flow was live.
- Uncertainty: does not prove exact travel, geometry, or transition semantics.

<a id="s3-targuna-production-fix"></a>
### S3 — Targuna production fix, 2026-04-08

- Locator: `https://www.tibia.com/news/?id=8757&subtopic=newsarchive`
- Effective date: `2026-04-08`
- Retrieved/index-verified: `2026-09-09T21:55:09Z`
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: further pre-target production continuity for the Targuna flow.
- Uncertainty: does not prove exact travel, geometry, or transition semantics.

<a id="s4-discovery-speed-envelope"></a>
### S4 — Discovery speed envelope

- Locator: `https://www.tibia.com/news/?id=8834&subtopic=newsarchive`
- Effective date: `2026-06-11`
- Retrieved/index-verified: `2026-09-09T21:55:09Z`
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: fully discovered areas grant a movement-speed reward within the `+100..+195` envelope depending on overall Discovery progress.
- Uncertainty: exact progress/count mapping, interpolation, rounding, stacking, activation timing, and scheduler interaction remain `UNKNOWN`.

<a id="s5-summer-update-production-presence"></a>
### S5 — Summer Update production presence

- Locator: `https://www.tibia.com/news/?id=8845&subtopic=newsarchive`
- Effective date: `2026-07-13`
- Retrieved/index-verified: `2026-09-10T05:53:03Z`
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: the improved Discovery system was released before the target cut.
- Uncertainty: the release notice alone does not independently establish the numeric envelope.

<a id="s6-global-death-family"></a>
### S6 — Global death progression-loss family

- Locator: `https://www.tibia.com/gameguides/?subtopic=manual&section=characters`
- Effective date: not supplied by the current manual
- Reviewed in the 2026-09-09 evidence generation; exact per-source retrieval instant was not recorded.
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: the bounded qualitative family of experience loss and skill loss on death.
- Uncertainty: target-cut XP basis/rate, exact skill/magic arithmetic, thresholds, modifier ordering, rounding, item loss, respawn destination, and other death behavior are not promoted from this record.

<a id="s7-accepted-oteryn-death-difference"></a>
### S7 — Accepted Oteryn Reference death difference

- Locator: `docs/architecture/OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md`
- Effective date: `2026-09-09`
- Retrieved: `2026-09-17T07:14:16Z` from protected admission main
- Source type: `OTERYN_ACCEPTED_CONTRACT`
- Provenance/legal: `CLEARED / CLEARED`
- Supports only the Oteryn parity-side decision: `DeathXPBasis = LevelXPSpan(current_level)`, `DeathSkillLoss = 0`, and `DeathMagicLevelLoss = 0`.
- Uncertainty: this declared difference is not Global target evidence and does not declare unrelated death behavior different.

<a id="s8-corpse-authority-introduction"></a>
### S8 — Corpse authority introduction

- Locator: `https://www.tibia.com/news/?id=399&subtopic=newsarchive`
- Effective date: `2006-08-01`
- Retrieved/index-verified: `2026-09-09T21:55:09Z`
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: introduction of the first-ten-seconds highest-damage-character/party loot authorization and corpse immovability family.
- Uncertainty: exact target-cut timer scheduling and edge attribution are not established by this source alone.

<a id="s9-corpse-authority-continuity"></a>
### S9 — Corpse authority continuity

- Locator: `https://www.tibia.com/gameguides/?section=controls&subtopic=manual`
- Effective date: not supplied by the current manual
- Reviewed in the 2026-09-09 source-preflight generation; exact per-source retrieval instant was not recorded.
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: continuity of the authorized-looter window family.
- Uncertainty: exact `10.000s` boundary tick, ties, summons, party-change timing, dead-character edges, nested containers, quick loot, and natural loot RNG remain `UNKNOWN`.

<a id="s10-newhaven-npc-trade-widget"></a>
### S10 — Newhaven NPC trade-widget flow

- Locator: `https://www.tibia.com/news/?id=8543&subtopic=newsarchive`
- Effective date: `2025-10-15`
- Retrieved/index-verified: `2026-09-10T05:53:03Z`
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: clickable NPC dialogue keywords/shortcuts and a trade shortcut opening the trade widget.
- Uncertainty: exact NPC identity, complete keyword graph, catalogue, prices, stock, currency, and transaction semantics remain `UNKNOWN`.

<a id="s11-goblin-intruder-retaliation"></a>
### S11 — Goblin Intruder retaliation

- Locator: `https://www.tibia.com/gameguides/?subtopic=quickstart`
- Effective date: not supplied by the current guide
- Reviewed in the 2026-09-09 evidence generation; exact per-source retrieval instant was not recorded.
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public factual paraphrase
- Supports: the tutorial Goblin Intruder fights back after being attacked; dated Newhaven production evidence makes this a bounded `DERIVED` continuity claim.
- Uncertainty: HP, XP, attacks, damage, target/threat selection, reaction timer, pathfinding, spawn behavior, corpse, and loot remain `UNKNOWN`.

<a id="s12-rat-target-near-structured-record"></a>
### S12 — Target-near Rat structured record

- Locator: `https://www.tibiawiki.com.br/index.php?oldid=441001&title=Rat`
- Effective date: `2026-07-25`
- Reviewed in protected Rat/#576 evidence; exact retrieval instant was not recorded.
- Source type in schema v1: `COMMUNITY_CORROBORATION`
- Investigator role: `STRUCTURED_REFERENCE_DATA`
- Schema-v1 serialization authority: #162 comment `5711026915`
- Provenance/legal: `CLEARED / CLEARED` for locator and bounded factual paraphrase
- Supports at `DERIVED HIGH` only: Rat HP `20`, base XP `5`, and ordinary candidate item types Gold Coin and Cheese; the stable revision/diff is three days before the target.
- Uncertainty: not primary CipSoft proof; no probability, guaranteed drop, final Character XP delta, numeric ID, or unlisted field is asserted.

<a id="s13-dead-rat-structured-record"></a>
### S13 — Dead Rat structured record

- Locator: `https://www.tibiawiki.com.br/index.php?stableid=45839&title=Dead_Rat`
- Effective date: not established
- Reviewed in protected Rat/#576 evidence; exact retrieval instant was not recorded.
- Source type in schema v1: `COMMUNITY_CORROBORATION`
- Investigator role: `STRUCTURED_REFERENCE_DATA`
- Schema-v1 serialization authority: #162 comment `5711026915`
- Provenance/legal: `CLEARED / CLEARED` for locator and bounded factual paraphrase
- Supports at `DERIVED` only: semantic corpse family `Dead Rat`.
- Uncertainty: numeric item identity, weight, decay, and protocol mapping are excluded.

<a id="s14-official-creature-catalogue-anchor"></a>
### S14 — Official creature catalogue anchor

- Locator: `https://www.tibia.com/library/?subtopic=creatures`
- Effective date: not supplied
- Reviewed in protected #576 evidence dated 2026-09-12; exact retrieval instant was not recorded.
- Source type: `OFFICIAL_PUBLIC`
- Provenance/legal: `CLEARED / CLEARED` for public locator/terminology use
- Supports: official creature-catalogue terminology anchor only.
- Uncertainty: direct numeric Rat fields were not retrieved from the official catalogue and are not upgraded to `PROVEN`.

## Per-case admission result

| Case | Domain | Target evidence | Parity | Preserved ceiling |
|---|---|---|---|---|
| `content_world.newhaven_targuna.level8_route.v1` | `CONTENT_WORLD` | `DERIVED` | `PARITY_PENDING_EVIDENCE` | chronology only |
| `movement.discovery.completed_area_speed_bonus_range.v1` | `WORLD_INTERACTION` | `DERIVED` | `PARITY_PENDING_EVIDENCE` | range only; mapping unknown |
| `character.death.low_level_base_xp_skill_loss.v1` | `CHARACTER` | `DERIVED` | `DECLARED_DIFFERENCE` | progression loss only; Global target evidence separated from accepted Oteryn difference |
| `ability_combat.corpse_loot.authority_window_10s.v1` | `ABILITY_COMBAT` | `DERIVED` | `PARITY_PENDING_EVIDENCE` | qualitative ten-second family; exact tick/edges unknown |
| `world_interaction.newhaven_npc.trade_widget_flow.v1` | `WORLD_INTERACTION` | `DERIVED` | `PARITY_PENDING_EVIDENCE` | product shape only |
| `ai_spawn.newhaven.goblin_intruder_retaliates_when_attacked.v1` | `AI_SPAWN` | `DERIVED` | `PARITY_PENDING_EVIDENCE` | retaliation only |
| `content_world.rat.first_creature_static.v1` | `CONTENT_WORLD` | `DERIVED` | `PARITY_PENDING_EVIDENCE` | exact bounded static tuple; RNG and final XP unknown |

No case is `PARITY_CONFIRMED`. Every Oteryn implementation state remains `NOT_STARTED`, with no exact implementation revision or fixture/test link claimed.
