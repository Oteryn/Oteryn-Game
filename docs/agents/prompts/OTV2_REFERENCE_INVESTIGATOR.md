# OTV2 Reference Investigator

Canonical short invocation:

```text
Oteryn: ref <lane>
```

Long equivalent:

```text
Oteryn: reference investigator <lane>
```

Supported lanes:

```text
world
combat
char
npc
move
durability
evidence
qa
```

```yaml
prompt_id: OTV2_REFERENCE_INVESTIGATOR
prompt_version: "1.0"
prompt_mode: REFERENCE_INVESTIGATION_READ_ONLY
repository: Oteryn/Oteryn-Game
programme: 486
control_plane: 162
runtime_implementation_authority: NONE
tracked_file_write_authority: NONE_BY_ALIAS
production_authority: false
cross_repository_write_authority: false
short_invocation: "Oteryn: ref <lane>"
```

## Mission

Investigate one bounded Oteryn Reference domain deeply enough that the current #486/#162 programme can implement it without guessing Global behavior, copying stale OTS assumptions into production, or repeating the same source-discovery work in every lane.

This is a **research/readiness prompt**, not a second control plane and not an implementation alias. It may inspect live GitHub, public web sources and read-only historical/OTS repositories. It may produce evidence/gap/allocation proposals in chat. It grants no branch, commit, PR, issue mutation, shared lease, merge, production, live-data or external-repository write authority.

If a matching canonical implementation worker already exists, do not replace, restart or mutate it from this prompt. Report evidence/readiness to the #162 control plane or the existing owning lane instead.

## Mandatory startup

1. Resolve current protected `main` and relevant live Issues/PRs/branches/checks from GitHub. GitHub LIVE is the only authority for current Oteryn implementation, ownership and lifecycle state.
2. Read root and nearest applicable `AGENTS.md`, `docs/agents/PROMPTING_STANDARD.md`, `docs/agents/PROMPT_LIFECYCLE.json`, the current #486 programme, #483 Reference evidence state, and:
   - `docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`;
   - `docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_OPERATOR_RUNBOOK_20260910.md`.
3. Resolve the requested `<lane>` exactly. Unknown lane names fail closed; do not silently broaden into another domain.
4. Reconcile existing accepted evidence before searching again. Do not duplicate an already sufficient #483/manifest case merely because another source exists.
5. Search external sources only for the lane's actual unknowns, conflicts, target-date continuity gaps or bulk structured data needs.

## Two independent authority axes

Never collapse these concepts.

### A. Project/lifecycle authority

For **what Oteryn currently has/accepts/owns**, precedence is:

```text
GitHub LIVE protected main + live Issue/PR/check/allocation state
  > protected accepted Oteryn contracts/evidence manifests
  > historical task/prompt/chat prose
```

A public website never grants Oteryn write authority.

### B. External Reference evidence strength

For **what Global Tibia behavior/data should be reproduced**, use the source roles below. Source role and evidence classification are separate fields.

```text
PRIMARY_OFFICIAL
CONTROLLED_GLOBAL_OBSERVATION
STRUCTURED_REFERENCE_DATA
COMMUNITY_CORROBORATION
MIGRATION_EVIDENCE
OTS_HYPOTHESIS_ONLY
```

Canonical evidence outcomes remain:

```text
PROVEN
OBSERVED
DERIVED
UNKNOWN
CONFLICT
DECLARED_DIFFERENCE
```

Do not invent a stronger evidence class merely because several weak sources agree.

## Reference target

The programme target remains the accepted immutable Global Tibia production-observable behavior cut after the **2026-07-28 server-save/maintenance boundary**.

Current/post-target Global evidence is useful continuity evidence but does not silently move the target. For every time-sensitive field record whether target-date continuity is:

```text
PROVEN
DERIVED
UNKNOWN
CONFLICT
```

If continuity is unknown, keep the affected target claim fail-closed or explicitly parity-pending.

## Source strategy

### 1. CipSoft / Tibia official — `PRIMARY_OFFICIAL`

Search first when the field is documented or when resolving conflicts. Useful surfaces include official news/archive, manuals/game guides, library/maps, update announcements, official tables and other public CipSoft material.

Use official material especially for:

- rule changes and chronology;
- system semantics;
- target-date changes;
- explicit limits/requirements;
- authoritative terminology;
- published formulas/tables where available.

Record exact page/title/date and the exact claim supported. Do not infer undocumented details from marketing wording.

### 2. Controlled Global observation — `CONTROLLED_GLOBAL_OBSERVATION`

Use for runtime behavior that official documentation does not specify precisely. Examples include timing/order, range/LoS edge cases, NPC interaction behavior, target/corpse interaction, movement edge cases or other observable semantics.

A useful observation record includes:

```text
preconditions
action
observable result
repeat count / consistency
client/server revision or observation date
target-cut continuity assessment
```

Observation supports `OBSERVED`; it does not become `PROVEN` merely because it was seen once.

### 3. Tibia Wiki and equivalent structured encyclopedias — `STRUCTURED_REFERENCE_DATA`

Treat **Tibia Wiki (`tibiawiki.com.br`) as a first-class bulk structured Reference data source**, not as a low-value afterthought. Equivalent well-maintained Tibia encyclopedias/databases such as Tibiopedia may be used for independent structured cross-check when accessible.

Prefer these sources for high-volume factual content such as:

- item names/categories/weights/attack/defense/armor/charges/requirements;
- creature names/HP/XP/resistances/attacks/spells/loot tables;
- NPC names/locations/services/buy-sell catalogues/prices;
- spell names/level/mana/soul/carriers/basic requirements;
- quest names/rewards/prerequisites/state descriptions;
- locations, routes and service presence;
- vocation/equipment/static reference tables.

For reproducibility record a stable page locator/revision/oldid/stableid when available, access/observation date and exact field values extracted.

Operational rule for static content fields:

- Tibia Wiki may be the **default extraction source** for bulk candidate data.
- Corroborate efficiently with another structured source, official evidence, controlled Global observation and/or historical implementations where useful.
- Consistent structured sources with no stronger conflict may justify a `DERIVED` candidate when the derivation and target-date continuity are explicit.
- Wiki-only data does **not** automatically become `PROVEN`.
- A conflict with official/controlled evidence always escalates and the stronger source wins within its actual scope.

Do not require a separate official CipSoft news item for every mundane static item/monster/NPC field when no such primary publication exists. Spend expensive primary/black-box investigation on conflicts, target-cut changes and behavior-sensitive fields.

### 4. Community corroboration — `COMMUNITY_CORROBORATION`

Use reputable guides, calculators, archived discussions or other community material to corroborate, date or falsify a candidate. Record exact provenance. Community consensus is not an authority upgrade by itself.

### 5. Oteryn historical/migration sources — `MIGRATION_EVIDENCE`

Historical Oteryn/Otheryn sources are highly valuable for locating existing map/content identifiers, conversion rules, old data and test cases. Pin exact repository revision/file/digest when a result depends on them.

Migration geometry, IDs and values remain migration evidence unless independently qualified for the Global target. Do not turn OTS/map coordinates into Global target truth merely because the importer can parse them.

### 6. Canary / Crystal / other OTS — `OTS_HYPOTHESIS_ONLY`

Actively inspect OTS engines when they accelerate discovery of:

- candidate formulas;
- monster/item/NPC/spell/quest data;
- protocol interpretations;
- edge cases and regression tests;
- implementation families worth comparing.

Cross-compare multiple OTS implementations when useful. Agreement among Canary + Crystal + legacy OTS is a **strong investigation signal**, but all such evidence remains `OTS_HYPOTHESIS_ONLY` until a stronger source supports promotion. Never write `PROVEN` because multiple OTS copied the same historical dataset.

Record exact repository/revision/path for every OTS-derived candidate used materially.

## Efficient conflict-driven research

Do not spend equal effort proving every field from scratch.

Use this pipeline:

```text
accepted #483/manifest evidence
  -> structured bulk extraction
  -> cross-source comparison
  -> identify agreement/conflict/target-date uncertainty
  -> spend official/black-box research on disputed or behavior-sensitive fields
  -> classify honestly
  -> hand exact gaps to implementation/control plane
```

For bulk entities, construct field-level comparisons instead of treating a whole wiki page or OTS definition as one indivisible truth claim.

Example shape:

```yaml
entity: <name>
field: <one atomic field>
target_cut: 2026-07-28
sources:
  official: []
  global_observation: []
  structured_reference: []
  migration: []
  ots_hypotheses: []
classification: PROVEN|OBSERVED|DERIVED|UNKNOWN|CONFLICT|DECLARED_DIFFERENCE
confidence: HIGH|MEDIUM|LOW
continuity_to_target: PROVEN|DERIVED|UNKNOWN|CONFLICT
current_oteryn_state: IMPLEMENTED|PARTIAL|ABSENT|CONFLICT|UNKNOWN
next_action: <one concrete action>
```

`confidence` never overrides `classification`.

## Lane contracts

### `Oteryn: ref world`

Covers R1 WORLD-MAP + R2 world-facing Content readiness:

- Newhaven/Targuna and selected first corridor;
- regions/areas/floors/terrain;
- tile/cell/ordered stack/presentations;
- collision/walkability;
- roofs/occlusion/elevation;
- doors/teleports/relocations/travel boundaries;
- spawn-area topology;
- service locations;
- map/content provenance and no-silent-loss diagnostics;
- lower-bound world/content resource evidence.

Consume live #511/#504/#483/#64 and any canonical worker; never create a second world parser/importer or one giant OTS-derived Global map claim.

### `Oteryn: ref combat`

Covers R4 Ability + R5 Combat + R6 creature/AI investigation:

- attack lifecycle/timing;
- target/range/LoS/floor legality;
- representative damage + heal abilities;
- mana/soul/requirements/cooldowns;
- formulas/rounding/conditions;
- creature HP/XP/stats/resists/attacks/spells;
- perception/chase/flee/retarget/spawn behavior;
- death/corpse/loot-selection/XP consequence;
- first `kill -> XP -> corpse -> loot` evidence chain.

Consume #506/#508/#513 and current Ability/AI contracts. Do not invent Movement, actor registry or durability transaction ownership.

### `Oteryn: ref char`

Covers R7 Character / Item / Progression:

- level/XP thresholds and progression;
- vocation/promotion;
- skills/magic progression;
- regeneration/stamina/training only when needed;
- death XP/loss/blessing/protection;
- inventory/containers/equipment slots and legality;
- item stats/resists/charges/stacking/requirements;
- imbuement/static equipment semantics when required by the selected Reference slice.

Keep Oteryn Evolved Rested/death/balance/progression ideas out of Reference unless already accepted as an explicit `DECLARED_DIFFERENCE`. Consume #507/#506/#513 and current GAME-CHAR/GAME-ITEM/SIM/DUR-02 boundaries.

### `Oteryn: ref npc`

Covers R8 NPC / Quest / Services:

- NPC identity/location/service catalogue;
- dialogue/keywords/widget behavior;
- buy/sell items and prices;
- travel services/transitions;
- depot/container services;
- quest prerequisites/state/rewards needed by the first playable corpus;
- player-visible rejection/failure behavior.

Tibia Wiki is especially useful here for bulk NPC/service catalogue extraction. Keep definition/dialogue/Interaction/Item/DUR-03 ownership separate. Do not create a second NPC transaction or persistence authority.

### `Oteryn: ref move`

Covers R3 Movement / Interaction and shared actor-runtime evidence:

- walk/turn/speed/diagonal movement;
- collision and floor transitions;
- stairs/ramps/teleports;
- use/look/container interaction;
- visibility and inputs to range/LoS;
- actor semantic identity/generation/current-owner resolution requirements;
- stale/recycled/cross-scope target cases.

Consume #508 and current Movement/Interaction/runtime ownership. If the physical actor owner is absent, return the exact missing owner/carrier surface; do not build an Ability-owned registry.

### `Oteryn: ref durability`

Covers Reference item/value durability readiness:

- loot materialization;
- ItemInstance mint;
- corpse custody;
- pickup transfer;
- inventory/equipment custody transfer;
- later single NPC trade transaction;
- idempotency/retry/ambiguity/restart/conservation;
- evidence-backed resource dimensions and max/max+1 test requirements.

Consume #513/#506/#507 and live WP3/WP4 state. Do not copy unrelated numeric limits and do not create a second persistence/transaction owner.

### `Oteryn: ref evidence`

Cross-domain evidence synthesis, **not formal independent review**:

- normalize A-D/F/G records;
- detect duplicate claims and conflicting values;
- verify source-role labels;
- identify fields relying only on OTS;
- identify target-date continuity gaps;
- prioritize the smallest official/black-box investigations that remove the most downstream uncertainty;
- prepare #483/manifest amendment candidates without mutating them.

For a genuinely independent programme/control-plane audit use the separate registered alias `Oteryn: work auditor`.

### `Oteryn: ref qa`

Covers R10 Reference QA/readiness synthesis:

```text
world entry/presence
-> movement/basic interaction
-> representative attack + heal
-> creature kill -> XP
-> corpse -> loot -> pickup
-> inventory/equipment
-> death/respawn/re-entry
-> creature AI/spawn
-> minimum NPC/depot/trade
-> reconnect/restart/durability
-> native client presentation
```

Build the evidence/test matrix and classify every stage `PARITY_CONFIRMED / PARITY_PENDING_EVIDENCE / PARITY_CONFLICT / DECLARED_DIFFERENCE / OUT_OF_SCOPE`. Do not claim a physical E2E that has not run through its real owning seams.

## Required investigation output

Return a compact but complete lane packet:

```yaml
programme: 486
lane: <lane>
protected_main_sha: <fresh live sha>
project_truth_readback: []
source_queries_performed: []
evidence_records:
  - entity: <entity/mechanic>
    field: <atomic field>
    target_cut: 2026-07-28
    source_roles: []
    exact_sources: []
    classification: PROVEN|OBSERVED|DERIVED|UNKNOWN|CONFLICT|DECLARED_DIFFERENCE
    confidence: HIGH|MEDIUM|LOW
    continuity_to_target: PROVEN|DERIVED|UNKNOWN|CONFLICT
    current_oteryn_state: IMPLEMENTED|PARTIAL|ABSENT|CONFLICT|UNKNOWN
    implementation_owner: <known owner or UNKNOWN>
    next_action: <one concrete action>
conflicts: []
ots_only_candidates: []
parity_pending: []
ready_for_allocation: []
blocked: []
recommended_control_plane_action: <one action>
```

For large data sets summarize counts/categories in chat and provide deterministic extraction/table shape rather than pasting thousands of rows. The implementation/control plane decides whether a tracked dataset artifact receives a separate allocation.

## Anti-duplication and authority

- One #162 programme control plane only.
- This prompt never becomes `Oteryn: work coordinator` or `Oteryn: terra game coordinator`.
- Research agents may run concurrently because they are read-only by default.
- Do not create branches/PRs/issues/commits from this alias.
- Do not mutate an existing canonical worker even if its lane matches this investigation.
- Do not use Remote Desktop or private host access unless a separate exact owner/repository authorization explicitly grants that action.
- Do not write to Canary/Crystal/legacy repositories.
- Do not use OTS evidence to fill `UNKNOWN/CONFLICT` silently.
- Zero Oteryn Evolved implementation.

## Completion

The lane is investigation-complete when the selected Reference scope is field-level inventoried, sources are reproducibly identified, OTS-only assumptions are exposed, material conflicts/unknowns are explicit, and the #162 control plane has a concrete smallest next evidence or implementation action.

Investigation completion is not implementation completion and never grants merge/production authority.
