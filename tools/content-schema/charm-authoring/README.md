# Charm authoring schema candidate v1

The static Charm catalogue: the 25 Bestiary Charms (14 major, 11 minor) with their category, kind, per-stage
cost and value, and a typed effect. Per the full-game tree contract
([`OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`](../../../docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md),
"Bestiary / Bosstiary / Charms"), this is only the content layer:

- Unlock, assignment and proc rules belong to `rulesets/progression/charms/`.
- A character's Charm Points, Minor Charm Echoes, unlocks and assignments are durable Character state.
- Per-creature charm points belong to the Bestiary facts in `content/creatures/bestiary/`.

None of these is modelled here. The `key` values (`oteryn:charm.<name>`) are minted at content population, which is `content` below;
`capture`, `build` and `validate` still write only under this directory.

| File | Purpose |
|---|---|
| `charm.schema.json` | One catalogue (`OTERYN_CHARM_AUTHORING_CATALOGUE/v1`), JSON Schema 2020-12, closed shapes. The effect is a `oneOf` over typed shapes: `attack_proc_damage`, `attack_proc_resource_damage`, `kill_area_damage`, `reflect_damage_taken` (each damage shape says whether it ignores resistances and whether armor reduces it), the timed effects (paralyse, haste, prevent flee) and the parameterless effects (dodge, leech, critical and so on). |
| `charm_authoring.py` | `capture` writes the source facts. `build` derives the catalogue and the comparison report from them, and `build --check` diffs an in-memory build against the committed samples. `validate` runs the schema and semantic checks. |
| `charm_authoring.py content` | Writes (`--check` verifies) `content/charms/index.json` (`OTERYN_FAMILY_INDEX/v1`) and the shard `charms-00000-00024.json` (one `definition` per Charm, identity `{key, revision: definition-r1}`), and registers `Charm` in `content/project.json` (`migrated_families`), `content/manifest.json` (`families`, `managed_files`) and `content/content.lock.json` (`family_counts`). |
| `test_charm_authoring.py` | No-network tests: infobox parsing and its rejections, the pinned Canary digest, the committed build, and one negative case per validator rule. |
| `samples/charm-sources-2026-09-29.json` | The captured source facts (details below). |
| `samples/charms-candidate.json` | The candidate catalogue, which validates. |
| `samples/charm-source-comparison.json` | Where TibiaWiki and Canary disagree. |

## Sources

- **TibiaWiki (tibia.fandom.com), `Derived`, primary.** The 25 pages of `Category:Charms` (`Infobox Charm`).
  - Stored per page: page id, revision id, timestamp, SHA-256 of the raw wikitext, and the infobox name, type, cost,
    the per-stage percent triple and the `implemented` version. No wiki prose is stored.
  - Parameters that do not vary by stage (5% of the creature's maximum health, the damage caps of 2× and 6× level, the
    8% cap, resistance and armor behaviour, the 10 s and 30 s durations, 15%, 2.5%) are written in `CHARMS` in the
    code. Capture rejects the page unless its text contains the phrases listed next to each entry; those phrases are
    stored as `confirmed_phrases`.
  - Currency (major charms cost Charm Points, minor charms cost Minor Charm Echoes) is from the wiki pages
    `Major Charms` and `Minor Charms`.
- **Canary `47dfd51f`, `OtsHypothesisOnly`, cross-check.** `data/scripts/systems/bestiary_charms.lua`, pinned by
  SHA-256 and read with a strict pattern parser.
  - Supplies `kind` (offensive, defensive or passive), which the wiki does not categorise, and `charm_id` (list
    position minus one). Whether that id is the client protocol id is not verified.

## Rules (`validate`)

- The schema passes.
- Keys, names and Canary ids are unique, and each key follows its name.
- The currency matches the category (major: `charm_points`; minor: `minor_charm_echoes`).
- `stage_value` matches the effect type: an effect that triggers carries `trigger_chance_percent`, an always-on
  bonus carries `effect_percent`.
- There are exactly the stages 1, 2 and 3, and both cost and value strictly increase across them.
- A trigger chance is at most 100%.

## Content population

`tools/content-migration/world_project_v2_to_tree.py` registers the committed Charm family (no legacy source) when it
regenerates project, manifest and lock, so `content --check` and the generator agree byte for byte.
Nothing loads `content/charms/` at runtime; `runtime_source` stays `legacy_until_separately_qualified`.

## Wiki and Canary comparison (2026-09-29)

All 25 charms agree on name, category, stage costs and stage values. The catalogue follows the wiki, including its notes:

- **Overpower and Overflux:** shown as physical, but the damage ignores the creature's resistances (`ignores_resistances`). This
  matches Canary's neutral damage.
- **Parry:** displayed as physical and ignoring resistances. The catalogue follows the wiki and accepted CHARM-0 choice that armor reduces it. The 2026-10-01 audit supersedes the original handler-based conclusion: both OTS handlers can check armor, but audited callsites omit the flag and its default is false, so those paths bypass armor. This remains an OTS conflict rather than a change to accepted behavior.
- **Carnage:** capped at 6× the character's level and reduced by armor. Owner answer 13a accepts physical damage with resistances; Canary and Crystal neutral damage is a rejected source variant. Controlled live-game confirmation remains pending without changing the accepted choice.
- **Bless:** the extra `percent = 10` in the Canary table is unused by the audited death-loss handler. The accepted choice uses only the 6/9/12% stage reduction, with no additional fixed 10%.

Crystal (`00ce02a`) agrees on catalogue costs and values, but the 2026-10-01 supplemental audit records engine differences in Cleanse eligibility, defensive ordering, critical RNG and leech arithmetic. Numeric agreement does not verify those mechanics in the official game.

```sh
pip install -r requirements.txt
python charm_authoring.py build --check
python charm_authoring.py validate samples/charms-candidate.json
python charm_authoring.py content --check
python test_charm_authoring.py

# evidence refresh (network + Canary checkout at 47dfd51f; local only)
python charm_authoring.py capture --canary <canary checkout>
python charm_authoring.py build
```

## Complete declarative preparation (2026-10-01)

`mechanics.schema.json`, `charm_mechanics.py` and `test_charm_mechanics.py` supplement the original catalogue without changing its definitions or authoring pipeline. The closed JSON Schema 2020-12 covers the source fixture and both preparation documents. `rulesets/progression/charms/index.json` is populated with `mechanics.json` and `progression.json`; this marker describes preparation, and does not connect a runtime loader.

All 25 catalogue keys bind to their exact effect types and conceptual event hooks. The catalogue remains authoritative for stage costs, stage bonuses, base damage parameters and existing durations. Supplemental fields describe condition speed coefficients/input offset/truncation/floor, Cleanse immunity and eligible sets, equipment gates, additive versus relative modifiers, death loss, products, skinning and kill-area variants. Every parameter has its own evidence status, immutable file locator and line bounds. `activation: false` applies to every captured parameter, including owner accepted choices, because this package is declarative preparation.

The source fixture `samples/charm-mechanics-sources-2026-10-01.json` captures facts and file hashes; it contains no original third-party engine code. It binds the existing catalogue SHA-256, the previously captured 25 wiki revision/hash records and independently extracted Canary, Crystal and TibiaPal cost/value triples (75 comparisons, all agree). External engine agreement is a hypothesis from related forks, not controlled official observation. The source digest in the validator prevents changing captured statuses or resolving uncertainty without a reviewed researched capture and explicit repin.

Statuses distinguish `OWNER_ACCEPTED`, `STRUCTURED_REFERENCE`, `OTS_HYPOTHESIS_ONLY`, `UNKNOWN` and `CONFLICT`. Source records also distinguish project implementations from accepted decision records and `CIPSOFT_OFFICIAL_CAPTURE` restatements. Official manual notes are pinned Oteryn restatements of a private capture; the unavailable underlying attachment hashes are explicitly recorded as a limitation. No live-game observation is claimed.

Accepted common rules retain physical Carnage with resistance and armor, separate attack-proc commits after nonlethal health reduction, no proc chaining or proc leech, per-hit spell/rune targets and main-target auto-attack procs. D186 keeps Low Blow, Savage Blow and leech on secondary auto-attack targets. Hook names identify coordinator integration needs; they do not assert that every runtime seam already exists. [INTEGRATION.md](INTEGRATION.md) records the coordinator handoff and known runtime mismatches.

Open variants remain reference only: exact official speed formulas and immunity duration; Cleanse Agony eligibility; defensive proc ordering; OTS Parry armor bypass; critical RNG sharing; leech chance/AoE reduction/rounding; Gut probability versus quantity; Scavenge fractional RNG rounding; Carnage geometry, summons and mitigation order. Shared defects (Canary critical reroll, Crystal leech unit arithmetic, decreasing Scavenge success on upgrades and Carnage mutable cap leakage) are captured rather than copied into accepted behavior.

Progression separates accepted storage/derived balances, promotion-only initial echoes, 2/6/unlimited shared slots and assignment thresholds from excluded reset, potion and Store lifecycle references. Unassign remains blocked on the accepted Character–Item boundary; no free substitute or external reset behavior is introduced.

```sh
python tools/content-schema/charm-authoring/charm_mechanics.py check
python tools/content-schema/charm-authoring/charm_mechanics.py validate
python tools/content-schema/charm-authoring/test_charm_mechanics.py
python tools/content-schema/charm-authoring/test_charm_authoring.py
# Optional offline verification against exact pinned source trees:
python tools/content-schema/charm-authoring/charm_mechanics.py verify-sources \
  --canary /path/to/canary --crystal /path/to/crystal --tibiapal /path/to/TibiaPal
```

`check` and `validate` verify the same committed package: schema, catalogue binding, complete key/effect/hook coverage, source classes, immutable captured facts, 75 numeric comparisons, current index and deterministic derivation. `verify-sources` additionally hashes files in supplied checkout roots and independently parses their catalogue numbers. It states the verified repository count; omitted external checkout roots remain unverified. It performs no network access. The existing authoring test entry invokes the new suite so its current CI entry also covers this package.

`samples/charm-global-parity-2026-10-01.json` supplements the pinned fork capture with public
official documentation, current community references and historical player tests. All 25
profiles bind the catalogue and distinguish full content from indexed snippets. It corrects
Cleanse immunity provenance, documents the 2026 Hex changes, and withdraws four earlier
Global recommendations that new evidence contradicts. It also records the scoped absence of
Charm payloads in the supplied 15.30 client assets. The existing offline checks validate this
packet and its evidence boundary; they do not execute Global Tibia. See `INTEGRATION.md` for
the corrected consumer choices and remaining distinguishing tests.

At browser completion, the supplement contained 97 source records and 149 qualified claims,
including 24 full-page browser captures and 66 additional assessments. Normal Tavily search
preceded public Chrome/CDP fallback. Its 89 literal quote checks retain capture/text/excerpt
hashes, dates and revisions; repeated access to one revision is not independent evidence.
The packet also retains the TibiaMaps and Exevo calculator-model comparisons. Browser
research closes documentary gaps; it does not claim connected-runtime qualification.

## Executed TibiaPal evidence

`samples/tibiapal-*-2026-10-01.json` retain the independently observed planner, description and
calculator results. The offline evidence tests bind those captures to source hashes and the
current catalogue. They validate recorded observations; they do not rerun a browser in CI.
The manual harnesses execute the original external source without copying it into this repo.

At the pinned TibiaPal revision, native Chromium passed all 25 cards, 150 stage transitions,
42 major budget boundaries and 66 calculator cases. Node VM execution passed 412 planner
checks. [INTEGRATION.md](INTEGRATION.md) explains the observed rounding differences, absent
elemental level cap and limits of these tests. The live domain was blocked by the session
proxy; no deployed-site or official-game parity is claimed.

Use a clean TibiaPal checkout at `61ffa3e0502879ccec44e59ead859e92b6d88531`. Browser reruns
require Python Playwright and a working Chromium executable; these are optional manual-test
dependencies and are not added to the authoring CI requirements.

```sh
node tools/content-schema/charm-authoring/samples/test-tibiapal-planner.cjs \
  /path/to/TibiaPal /path/to/Oteryn-Game /tmp/tibiapal-planner-execution.json
python tools/content-schema/charm-authoring/verify_tibiapal_browser.py \
  --checkout /path/to/TibiaPal --chromium /usr/bin/chromium \
  --screenshots /tmp/tibiapal-browser --output /tmp/tibiapal-browser-verification.json
python tools/content-schema/charm-authoring/test_tibiapal_evidence.py
```

All 25 descriptions match the catalogue's three costs and bonus values. They leave detailed
combat behavior untested; for example, Gut's “more products” does not establish probability
versus quantity, and Cleanse's prose does not establish exact immunity time or removal semantics.

## Execution of every remaining reference question

The owner-requested continuation executes the 19 remaining question groups across five
lanes. The final packet contains 141 source records and 168 qualified claims. `samples/charm-global-parity-2026-10-01.json` now retains a `gap_closure` inventory,
per-question claims, source bindings, lane assessments and actual input/output artifacts.
Compiled extracted C++ bodies and Lua loot functions run under explicit mocks; mathematical
candidate probes retain separate scopes. Current Global measurements are never inferred from
those executions. All 25 profiles link their applicable executed-question claims.

The supplement includes five historical reported Life Leech traces, two new full official
announcements and explicit Agony-condition documentation. It closes the unnecessary demand
for hidden RNG identity when candidate outcomes are observably equivalent. The integration
packet records reproducible commands, resolved reference facts and the precise observations
still required for connected/current-server qualification.
