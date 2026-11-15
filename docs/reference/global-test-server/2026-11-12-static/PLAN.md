# Global Winter Update comparison programme

Task: `OTV2-20261112-global-test-server-static`; carrier: PR #3143.

## Owner target decision — 2026-11-12

The owner explicitly selected the new Winter Update as the Oteryn target:
"uzywamy nowej wersji i ja rpzyjmujemy ... to jest nowy winter update".
The directly observed client is **Tibia Test Server Client 15.33.ad0955**.
This is the selected investigation baseline, not a claim that production already
uses this build or that all Oteryn data/runtime have migrated.

This decision supersedes the older 15.30 / 2026-09-27 target for this programme.
Repository-wide target references and family migrations must be reconciled in
bounded follow-up changes; this research PR does not edit runtime, content pins,
protocol contracts or historical evidence. Relevant predecessor decisions:

- `docs/agents/programs/OTERYN_GAME_VERSION_1530_AND_OTS_BRANCHES_DECISION_20260928.md`
- `docs/agents/programs/OTERYN_TARGET_DATE_20260927_DECISION.md`

Every observation retains its actual build and timestamp. A later Test Server
patch requires a new session context and an affected-field delta check; neither
the name Winter Update nor the observation date identifies a build. Production
release behaviour requires subsequent observation; release timing is not verified.

## Operator and comparison responsibilities

- One lead owns desktop input, screen inspection, evidence retention and publication.
- One comparison worker reads captured evidence and an exact Oteryn revision.
  It has no desktop-input or repository-write authority.
- The lead reviews the worker's source locators and results. No two operators
  share keyboard/pointer focus. Short UI batches are permitted only on inspected
  controls; each captured panel is reviewed before its values are admitted.
- Publish coherent batches to the existing PR. Keep large images/videos out of Git.
  Canonical data are the registers, not comments or recollections from chat.

## Phases and acceptance

| Phase | Work | Exit evidence |
|---|---|---|
| S0 | Build, server label, character and display context | Readable Info/context evidence; unavailable metadata explicitly null |
| S1 | Pilot 10–15 item entries with different requirements/bonuses | Field transcriptions visually checked, evidence retained, identities and exact Oteryn locators checked |
| S2 | Items by category: weapons/ammunition, equipment, containers, tools, consumables, valuables and special forms | Enumerated available catalogue with observed/locked/not-displayed/unreadable counts; variants and scrollable details preserved |
| S3 | Spell definitions; visible creature/boss entries; charms, imbuements, proficiencies and character panels | Same field-level evidence discipline; locked values remain unknown |
| S4 | NPC price/residence lists and other visible static catalogues | Direction buy/sell and currency explicit; partial lists never labelled complete |
| D1 | Movement, item operations, NPC interactions and failure messages | Controlled Global/Oteryn scenarios under a separately bounded dynamic task |
| D2 | Combat, professions, statuses, AI, cooldowns and formulas | Repeated measurements with controlled modifiers and uncertainty |
| D3 | Death/reconnect, persistence, party, trade, PvP and access rights | Transition and interaction cases, including failure/recovery |
| D4 | Quests, encounters, world objects and presentation details | Per-entity coverage with dependency and access gaps recorded |

PR #3143 executes S0–S1 initially and can continue static collection S2–S4.
Dynamic phases, purchases, rewards, destructive actions and runtime fixes do not
belong to the present static batch. No full-catalogue completion is inferred from
the pilot. No programme completion date is asserted before access/inventory sizing.

## Coverage matrix

Track each entity, variant and observable field, including fields absent on either
side. Inventory both directions: Oteryn-only entries and Global-only entries.
The operational matrix covers:

- Character/professions: growth, resources, skills, regeneration, speed, requirements,
  equipment and all displayed modifiers.
- Items: names/descriptions, weight, armor/attack/defence, skill/element bonuses,
  requirements, slots, charges, duration, tiers, classification, mantra, augments,
  market eligibility, container properties and vendor lists.
- Spells/runes: names/incantations, profession/level, premium requirement, resource
  cost, target, range, area, cooldown groups, effects, augments and failure conditions.
- Creatures/bosses: catalogue stats, resistances, attacks, summons, loot visibility,
  targeting, pursuit, fleeing, reset and encounter transitions.
- Movement/world: cardinal/diagonal steps, turns, terrain, blockers, autowalk,
  floors, stairs, teleports, doors/keys/levers, movable objects and spawn changes.
- Progression/economy/social: charms, imbuements, proficiencies, wheel, prey,
  blessings, stamina, party, shared experience, banking, market, storage and housing.
- Durability/quests: item ownership, nested containers, death, reconnect, quest
  conditions/transitions/logs, rewards, repeatability and group participation.
- Presentation: layout, filters, ordering, tooltips, shortcuts, units, rounding,
  exact messages, sprites, animation, effects, projectiles and sound.

For each behavioural rule include the ordinary case, thresholds immediately below/
at/above the boundary, missing requirements/resources, retry/cancel and relevant
death/logout/floor changes. Use dependency-based pairwise combinations plus all
known high-risk interactions; do not claim exhaustive Cartesian coverage.

## Comparison rules

Retain raw text and normalized values/units. Global internal IDs are null unless
actually displayed. Match using name, variant, sprite/category and distinctive
properties together; record ambiguity. Use exact Oteryn SHA, stable entity key and
JSON/code locator. Unknown Oteryn fields are not zero, and UI omissions are not
negative assertions. Do not compare a donor source packet as though it were an
admitted runtime profile. Record materializability separately from field matches.

`TARGET_BUILD_OBSERVATION` in baseline_relevance denotes an observation from the
explicitly selected build. It does not establish production parity or runtime
execution. Status remains MATCH/DIFFERENCE/UNKNOWN/etc. independently of relevance.
MATCH certifies only the compared field under the recorded conditions.

Preserve the exact signed meaning of resistance/weakness, units and percentages.
Record both vendor direction and price; screenshots of part of a scrolling vendor
list prove only those visible entries. Market-average prices are time-dependent and
are not static NPC prices. Spell descriptions do not prove actual effects.

## Dynamic measurement protocol for follow-up

Each case records ID, preconditions, exact action sequence, Global evidence,
Oteryn revision/runtime context, result, uncertainty and cleanup. Change one
parameter at a time before testing combinations. Measure event timing locally
from timestamped capture/log evidence, never from remote-tool round trips.
Random outcomes require retained complete trial sequences, sample sizes and
interval estimates chosen for the effect size; a single outcome is not a formula
or loot-rate estimate. Verify inferred formulas on held-out parameter settings.

## Findings, repairs and reporting

Observation -> identity/context check -> exact-revision comparison -> reproduction
where applicable -> triage -> bounded repair -> regression -> re-observation.
Keep source-data differences, missing evidence/admission, dynamic runtime defects
and declared Oteryn differences distinct. Batch fixes by owning mechanic/module;
this PR changes research records only. Existing source audit #3088 does not prove
Global behavioural parity, and this programme does not replace that audit.

Report counts separately: inventoried, observed, compared, matching, different,
unknown and inaccessible. Include build/SHA and retained evidence. Green internal
tests or source-review completion are not external parity proof.
