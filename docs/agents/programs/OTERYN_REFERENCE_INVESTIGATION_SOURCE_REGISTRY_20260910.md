# Oteryn Reference Investigation Source Registry

- Programme: #486
- Control plane: #162
- Date: 2026-09-10
- Purpose: shared evidence/source discipline for Reference investigation agents
- Runtime/write authority granted: **NONE**
- External target: Global Tibia behavior/data at the accepted post-2026-07-28 cut

## 1. Why this registry exists

Reference investigation uses several fundamentally different source types. They must not be collapsed into one linear notion of "truth".

There are two independent questions:

1. **What is currently true/accepted/owned inside Oteryn?**
2. **What external Tibia behavior/data should Oteryn Reference reproduce?**

The first is governed by live Oteryn repository authority. The second is established by evidence strength and target-date relevance.

## 2. Project truth

### `PROJECT_GITHUB_LIVE`

Authority:

```text
Oteryn/Oteryn-Game protected main
+ live Issues/PRs/branches/checks
+ current exact allocations/leases
+ protected contracts/evidence manifests
```

Use for:

- current implementation state;
- ownership;
- whether a worker exists;
- exact branch/head/CI/review state;
- current accepted contract/evidence state;
- whether a mutation is authorized.

GitHub LIVE outranks cached chat/prompt/task prose for lifecycle state.

A Tibia website, wiki or OTS repository can never grant Oteryn write/merge/production authority.

### `REFERENCE_ACCEPTED_EVIDENCE`

Primary anchors:

- #483;
- protected Reference evidence/parity manifest(s);
- accepted target/baseline contracts on protected main.

This is the canonical Oteryn interpretation used by implementation until a higher-quality/newer evidence change is reviewed and protected. New public evidence may justify an amendment proposal; it does not silently rewrite an accepted manifest.

## 3. External evidence source roles

### `CIPSOFT_OFFICIAL` — source role `PRIMARY_OFFICIAL`

Examples:

- https://www.tibia.com/
- official news/archive;
- manuals/game guides;
- library/maps;
- update announcements;
- official tables/documentation.

Best for:

- chronology and target-date changes;
- system rules;
- official terminology;
- explicit requirements/limits;
- published values/formulas when CipSoft exposes them.

Limits:

- official prose often does not expose every static content field or implementation detail;
- marketing/update wording must not be expanded beyond what it actually proves.

Possible evidence outcomes: `PROVEN`, or input to `DERIVED`; current post-target material also requires target-continuity analysis.

---

### `GLOBAL_BLACKBOX` — source role `CONTROLLED_GLOBAL_OBSERVATION`

Use when real runtime behavior is not sufficiently documented publicly.

Suitable for:

- timing/order;
- range/LoS/floor edge cases;
- movement/collision behavior;
- NPC interaction behavior;
- combat/corpse/loot interactions;
- client/server-visible outcomes;
- other controlled observable semantics.

Required provenance where possible:

```text
observation date
client/server revision context
preconditions
action
result
repetition/consistency
target-cut continuity assessment
```

Typical classification: `OBSERVED`.

---

### `TIBIAWIKI_STRUCTURED` — source role `STRUCTURED_REFERENCE_DATA`

Primary structured encyclopedia:

- https://www.tibiawiki.com.br/

This source is deliberately promoted to a **first-class bulk Reference data source** for programme #486.

Use aggressively for high-volume static/semi-static content extraction:

#### Items

- names/classes/categories;
- weights;
- attack/defense/armor;
- slot/equipment properties;
- charges/requirements;
- resistances/attributes;
- buy/sell relationships and prices where documented;
- quest/reward/source relationships.

#### Creatures

- names;
- HP/XP;
- resistances/weaknesses;
- attacks/spells;
- loot tables;
- locations/habitats;
- behavioral descriptions useful for hypothesis generation.

#### NPCs

- identity;
- location;
- profession/service type;
- buy/sell catalogues;
- travel/services;
- quest relationships;
- dialogue/keywords when documented.

#### Spells / abilities

- spell identity;
- vocation/level requirements;
- mana/soul/carrier requirements;
- cooldown/category data when documented;
- basic range/targeting descriptions;
- training/NPC relationships.

#### Quests / world

- quest prerequisites/rewards;
- location names/routes;
- NPC/service relations;
- map/location references;
- static progression requirements.

Reproducibility:

- store exact URL/page title;
- prefer `oldid`, `stableid` or other stable revision locator when available;
- record extraction date;
- record each atomic field rather than citing one page for unrelated claims.

Promotion discipline:

- Wiki data may be the **default extraction source** for bulk static content.
- Wiki-only does not automatically mean `PROVEN`.
- A consistent field may become an explicitly reasoned `DERIVED` candidate when target continuity is supported and no stronger conflict exists.
- Behavior-sensitive/runtime algorithm claims require stronger corroboration when they matter to parity.
- Official or controlled direct evidence wins when the scopes genuinely conflict.

---

### `OTHER_STRUCTURED_TIBIA_DATA` — source role `STRUCTURED_REFERENCE_DATA`

Examples include other maintained Tibia encyclopedias/databases such as Tibiopedia when accessible.

Use primarily as an **independent structured cross-check** against Tibia Wiki and for fields absent from one source.

Do not treat mirrored/copied data as independent merely because it appears on a second hostname. Where provenance suggests one site copied another, record `SOURCE_DEPENDENCY_POSSIBLE` and avoid counting it as independent corroboration.

---

### `COMMUNITY_CORROBORATION` — source role `COMMUNITY_CORROBORATION`

Examples:

- reputable guides;
- calculators;
- archived forum discussions;
- community test reports;
- historical videos/screenshots when provenance is adequate.

Use for:

- falsification;
- historical dating;
- edge-case discovery;
- corroborating otherwise sparse fields.

Community popularity/consensus is never an automatic evidence upgrade.

---

### `OTERYN_LEGACY` — source role `MIGRATION_EVIDENCE`

Historical Oteryn/Otheryn data and code are valuable migration/discovery sources.

For any material claim record exact repository, revision, file/path and, for binary source packages, digest when governed by an existing import contract.

The current #511 map corpus is an example of correctly pinned migration evidence. Its coordinates/content do not become Global target truth merely because the data are internally consistent.

Use for:

- map/source identifiers;
- old item/monster/NPC data;
- migration transforms;
- implementation history;
- regression and conversion test cases.

---

### `CANARY_OTS`, `CRYSTAL_OTS`, `OTHER_OTS` — source role `OTS_HYPOTHESIS_ONLY`

Use OTS engines **actively** for investigation speed.

They are especially useful for finding:

- formulas and arithmetic candidates;
- item/monster/NPC/spell/quest datasets;
- edge cases;
- protocol interpretations;
- code paths to reproduce/falsify;
- historical data that can then be checked against stronger sources.

Every material OTS candidate records:

```text
repository
exact revision
path/symbol/entity
value/formula/behavior candidate
```

Cross-OTS rule:

```text
Canary == Crystal == legacy OTS
```

is a strong **search/prioritization signal**, but remains `OTS_HYPOTHESIS_ONLY`. Several OTS implementations may share the same copied source/data lineage.

OTS never silently fills `UNKNOWN` or resolves `CONFLICT` against stronger evidence.

## 4. Evidence classifications

The investigation source roles above are not replacements for canonical evidence status.

Use:

- `PROVEN` — directly and adequately established by accepted primary evidence for the exact claim/scope;
- `OBSERVED` — controlled observation established the exact behavior in the observed context;
- `DERIVED` — reasoned conclusion from identified evidence; derivation and assumptions must be explicit;
- `UNKNOWN` — insufficient evidence;
- `CONFLICT` — material sources/evidence disagree and the conflict is unresolved;
- `DECLARED_DIFFERENCE` — separately accepted Oteryn Reference divergence from the external target.

Keep source role separately, for example:

```yaml
classification: DERIVED
source_roles:
  - STRUCTURED_REFERENCE_DATA
  - STRUCTURED_REFERENCE_DATA
  - OTS_HYPOTHESIS_ONLY
```

Do not create `PROVEN_BY_WIKI` or `PROVEN_BY_OTS` aliases that blur the model.

## 5. Confidence is separate from evidence status

Optional confidence:

```text
HIGH
MEDIUM
LOW
```

Confidence helps prioritize work but never upgrades authority.

Examples:

```text
HIGH confidence OTS hypothesis != PROVEN
LOW confidence official inference != automatic CONFLICT
```

## 6. Field-level extraction rule

Entity pages must be decomposed into atomic fields.

Example:

```yaml
entity: Demon
field: hitpoints
sources:
  structured_reference:
    - source: TIBIAWIKI_STRUCTURED
      value: <value>
  ots_hypotheses:
    - source: CANARY_OTS
      value: <value>
classification: <status>
```

A conflict in `loot_probability` must not contaminate an independently well-supported `hitpoints` field.

## 7. Bulk-data fast path

For large static data families, use this efficient process:

```text
Tibia Wiki structured extraction
        |
        +--> second structured source where available
        |
        +--> Canary / Crystal / legacy OTS comparison
        |
        v
field-level agreement/conflict table
        |
        +--> no conflict + plausible target continuity
        |       -> DERIVED candidate / parity-pending as appropriate
        |
        +--> conflict / target-sensitive / behavior-sensitive
                -> official CipSoft search and/or controlled Global observation
```

This avoids spending expensive black-box/primary-source research on thousands of mundane static fields while preserving honest evidence semantics.

## 8. Behavior-sensitive slow path

Do not accept wiki/OTS consensus alone for subtle semantics such as:

- combat/damage/armor formulas and rounding;
- loot RNG algorithm/order;
- attack/cooldown timing interactions;
- line-of-sight/range geometry edge cases;
- AI scheduling/decision ordering;
- spawn timing/repopulation semantics;
- party/shared XP distribution;
- corpse ownership/authority timing;
- durable/retry/reconnect behavior;
- target actor generation/authority;
- any security/ownership/fencing semantic.

These should preferentially resolve through official evidence, controlled Global observation or accepted Oteryn architecture/evidence decisions.

## 9. Target-date rule

For every external field that may have changed over time, record:

```yaml
target_cut: 2026-07-28
source_date_or_revision: <date/revision>
continuity_to_target: PROVEN|DERIVED|UNKNOWN|CONFLICT
```

A September 2026 value is not automatically the July 28, 2026 value. Likewise an old wiki/OTS value is not automatically continuous to the cut.

When the exact target value remains uncertain, preserve `UNKNOWN`/`CONFLICT` or a weaker `DERIVED` claim rather than inventing certainty.

## 10. Conflict resolution

Use deterministic precedence by **claim scope**, not hostname alone:

1. exact target-boundary primary official evidence;
2. accepted target-bound primary/controlled observation;
3. accepted protected #483/manifest interpretation until formally amended;
4. structured-reference consensus with explicit continuity reasoning;
5. community corroboration;
6. migration evidence;
7. OTS hypotheses.

If a stronger source does not actually address the same atomic field/context/date, it does not resolve the conflict.

Every unresolved material conflict returns:

```yaml
classification: CONFLICT
conflicting_sources: []
missing_discriminator: <exact fact/observation needed>
next_evidence_action: <one bounded action>
```

## 11. Source acquisition boundaries

- Public internet research is read-only.
- External repositories are read-only unless an unrelated exact owner allocation explicitly grants otherwise.
- Do not redistribute proprietary client/map/pixel/source payloads into Git.
- Record digests/aggregate evidence instead of copyrighted/proprietary raw payload where existing policy requires it.
- Remote Desktop/private-host use requires separate exact authority; investigation aliases do not grant it.
- If a site blocks automated access, do not weaken source classification or invent its data. Use another lawful source or record the access limitation.

## 12. Output minimum

Every investigator should be able to hand #162 a table where each row contains at least:

```text
domain
entity/mechanic
atomic field
target cut
exact source locators
source roles
value/behavior candidate
evidence classification
confidence
target continuity
current Oteryn state
owner/dependency
next action
```

The goal is not a giant undifferentiated scrape. The goal is a reproducible, conflict-aware Reference dataset that can feed exact implementation allocations.
