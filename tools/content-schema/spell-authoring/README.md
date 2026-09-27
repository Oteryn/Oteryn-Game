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
| `samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json` | Output of `spell_census.py` (one spell per line). |
| `samples/wiki-spell-facts-fandom-2026-09-27.json` | Output of `wiki_spells.py facts`: page id, revision id, wikitext SHA-256, allowlisted infobox values and the `Formulae` level curve. |
| `samples/wiki-spell-compare-fandom-2026-09-27.json` | Output of `wiki_spells.py compare`: per-field counts and difference rows. |
| `samples/wiki-spell-facts-br-2026-09-27.json` | `wiki_spells.py br-facts` over the hosted-runner BR capture: mapped short facts with revision ids. |
| `samples/wiki-spell-compare-br-2026-09-27.json` | `wiki_spells.py compare` of the census with TibiaWiki BR. |
| `samples/wiki-spell-crosswalk-fandom-br-2026-09-27.json` | `wiki_spells.py crosswalk`: BR ↔ Fandom per-field agreement and conflicts (S3). |

```text
pip install -r requirements.txt
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python validate_spell.py synthetic-valid-light-healing.json synthetic-valid-light-healing-dependencies.json --catalog synthetic-catalog.json
python spell_census.py self-test && python wiki_spells.py self-test

# evidence regeneration (network / source checkouts)
python spell_census.py --canary <canary@47dfd51f> --crystal <crystalserver@ff7ede5> \
    --out samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json
python wiki_spells.py fetch --cut 2026-09-27 --cache <dir>
python wiki_spells.py facts --cache <dir> --out samples/wiki-spell-facts-fandom-2026-09-27.json
python wiki_spells.py compare --facts samples/wiki-spell-facts-fandom-2026-09-27.json \
    --census samples/spell-census-canary-47dfd51f-crystal-ff7ede5.json \
    --out samples/wiki-spell-compare-fandom-2026-09-27.json
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
