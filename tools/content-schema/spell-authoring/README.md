# Spell authoring schema candidate v1

Contract, decisions and implementation plan: [`docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md`](../../../docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md).

Player spells and runes. The executed part (Ability, Effect, damage/heal Formula) is the monster
authoring definition in `../monster-authoring/` (monster D11: a player spell a monster also casts is
the one shared Ability); this folder adds the player-casting layer and the `player_expression` Formula.

| File | Purpose |
|---|---|
| `build_formal_schema.py` | Source of the schemas; regenerates the three JSON files below. |
| `spell.schema.json` | Spell bundle: words, vocations, level, costs, cooldown groups, targeting, rune carrier, execution. |
| `spell-dependencies.schema.json` | Ability, Effect (monster definitions) and Formula (monster kinds or `player_expression`). |
| `spell-template.json` | Empty field template; placeholders are deliberately invalid data. |
| `validate_spell.py` | Structural plus semantic/reference validation (words, vocations, groups, formula inputs, formula range over a level/magic-level/skill grid). Import manifests use `../monster-authoring/monster-import-readiness.schema.json`. |
| `verify_formal_schema.py` | Focused positive/negative cases; regenerates the `synthetic-*.json` fixtures (Light Healing, Sudden Death rune and its conjuring spell, from Crystal Server). |
| `spell_census.py` | Loads every `data/scripts/spells/**` and `data/scripts/runes/**` script of a Canary and a Crystal Server checkout in a stubbed LuaJIT sandbox (needs `lupa==2.8`); records registrar calls, Combats, cast tier and the damage/heal formulas as exact expression trees; compares the two sources. |
| `wiki_spells.py` | Fetches TibiaWiki (Fandom) `Infobox Spell` pages and `Category:Runes` as of a given day (default today; owner decision S3), keeps allowlisted facts only, and compares them with the census. |
| `convert_spells.py` | Plan phase P2: turns the census into Spell bundles (Spell, Ability/Effect/Formula, catalog, import manifest) under S1–S5, S11, S13, S14, S16; plain combats go through the monster converter's `combat_ability`. Needs both checkouts and `lupa`. |
| `tibiopedia_spells.py` | Fetches the tibiopedia.pl spell pages (owner decision S12) and keeps single facts with the page URL and page SHA-256, in Fandom field names; no descriptions or comments. |
| `verify_spells.py` | Checks every converted bundle against Fandom, BR and tibiopedia.pl field by field: `agree`, `ours_differs` (the references agree, we do not), `sources_disagree`. |
| `tibiacom_spells.py` | Captures the official tibia.com spell library (S15) in Chromium on a machine tibia.com serves (it blocks the build container and hosted runners); `facts` maps the table cells to Fandom field names; `list-facts` turns an owner copy of the list view into facts. |
| `vocation_vitals.py` | SPELL-D5 candidate vitals: max hitpoints, mana and capacity per vocation and level from Fandom `Formulae`, soul regeneration from `Soul Point`, hitpoint/mana regeneration from Canary and Crystal where they agree; every conflict is listed. |
| `cooldown-groups.json` | S9: the declared closed catalogue of cooldown groups and their roles (primary, secondary); `validate_spell.py` rejects any other group. |
| `official-changes.json` | S11 evidence: official changes that decide a BR/Fandom conflict (fact, date, source URL). |
| `samples/spell-readiness-p2.json` | `convert_spells.py --readiness`: per spell `ready`/`blocked`, its blockers and the bundle SHA-256 (bundles are not committed). |
| `samples/starter-bundles/` | The P3 starter spells converted by `convert_spells.py --only ... --out`; validated in CI with their manifests. The `conversion` CI job checks out the pinned sources and requires the census, readiness, verification report and starter bundles to reproduce byte for byte. |
| `samples/spell-census-canary-99902524-crystal-ff7ede5.json` | Output of `spell_census.py` over the Canary 15.30 branch (S14) and Crystal (one spell per line); the converter input. |
| `samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json` | The census before S14 (Canary `main` at 47dfd51f); input of the 2026-09-27 wiki comparisons. |
| `samples/wiki-spell-facts-fandom-2026-09-27.json` | Output of `wiki_spells.py facts`: page id, revision id, wikitext SHA-256, allowlisted infobox values (including the whole official `librarytext`, S19) and the `Formulae` level curve. |
| `samples/wiki-spell-compare-fandom-2026-09-27.json` | Output of `wiki_spells.py compare`: per-field counts and difference rows. |
| `samples/wiki-spell-facts-br-2026-09-27.json` | `wiki_spells.py br-facts` over the hosted-runner BR capture: mapped short facts with revision ids. |
| `samples/wiki-spell-compare-br-2026-09-27.json` | `wiki_spells.py compare` of the census with TibiaWiki BR. |
| `samples/wiki-spell-crosswalk-fandom-br-2026-09-27.json` | `wiki_spells.py crosswalk`: BR ↔ Fandom per-field agreement and conflicts (S3). |
| `samples/tibiopedia-spell-facts-2026-09-28.json` | `tibiopedia_spells.py facts`: spell and rune facts with URL and page SHA-256. |
| `samples/vocation-vitals-candidate-2026-09-28.json` | `vocation_vitals.py build`: the candidate vitals with both Fandom revisions, the source pins and the open conflicts. |
| `samples/tibiacom-spell-list-2026-09-28.json` | `tibiacom_spells.py list-facts`: the tibia.com list view (words, group, type, level, mana, premium) as copied by the owner; applied as S15. |
| `samples/spell-verify-3-sources-2026-09-28.json` | `verify_spells.py`: our value against the three references, with the disagreeing rows. |

```text
pip install -r requirements.txt
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python validate_spell.py synthetic-valid-light-healing.json synthetic-valid-light-healing-dependencies.json --catalog synthetic-catalog.json
python spell_census.py self-test && python wiki_spells.py self-test
python tibiopedia_spells.py self-test && python verify_spells.py self-test

# evidence regeneration (network / source checkouts)
python spell_census.py --canary <canary@99902524> --crystal <crystalserver@ff7ede5> \
    --out samples/spell-census-canary-99902524-crystal-ff7ede5.json
python wiki_spells.py fetch --cut 2026-09-27 --cache <dir>
python wiki_spells.py facts --cache <dir> --out samples/wiki-spell-facts-fandom-2026-09-27.json
python wiki_spells.py compare --facts samples/wiki-spell-facts-fandom-2026-09-27.json \
    --census samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json \
    --out samples/wiki-spell-compare-fandom-2026-09-27.json
python convert_spells.py --canary <canary@99902524> --crystal <crystalserver@ff7ede5> \
    --readiness samples/spell-readiness-p2.json [--out <dir>] [--only "light healing" ...]
python tibiopedia_spells.py fetch --cache <dir>
python tibiopedia_spells.py facts --cache <dir> --out samples/tibiopedia-spell-facts-2026-09-28.json
python verify_spells.py --bundles <convert_spells --out dir> --out samples/spell-verify-3-sources-2026-09-28.json
python vocation_vitals.py fetch --cache <dir>
python vocation_vitals.py build --cache <dir> --canary <canary@99902524> --crystal <crystalserver@ff7ede5> \
    --out samples/vocation-vitals-candidate-2026-09-28.json
# BR: download the spell-wiki-br-<sha> artifact of spell-wiki-capture.yml, then
python wiki_spells.py br-facts --artifact wiki-spell-infoboxes-br.json --out samples/wiki-spell-facts-br-2026-09-27.json
python wiki_spells.py compare --facts samples/wiki-spell-facts-br-2026-09-27.json \
    --census samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json --out samples/wiki-spell-compare-br-2026-09-27.json
python wiki_spells.py crosswalk --facts samples/wiki-spell-facts-fandom-2026-09-27.json \
    --br-facts samples/wiki-spell-facts-br-2026-09-27.json --out samples/wiki-spell-crosswalk-fandom-br-2026-09-27.json
```

TibiaWiki BR (`tibiawiki.com.br`) answers HTTP 403 (Cloudflare) from the build container, as for
NPCs. `.github/workflows/spell-wiki-capture.yml` runs `wiki_spells.py fetch --wiki br` and
`facts --wiki br --all-fields` on a hosted runner and uploads the infobox fields (cut to 200
characters, with revision ids) as the `spell-wiki-br-<sha>` artifact; the same workflow runs the schema
checks above.
