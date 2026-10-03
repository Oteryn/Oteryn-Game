# Spell authoring schema candidate v1

The r20 native family adapters complete all 67 formerly unresolved live records:
65 closed native profiles and two blocking-item rune effects. The conversion now
produces 246 authoring-valid records; the six removed spells stay quarantined.
`native_catalog.py` validates each family's strict parameters and retains source
and helper hashes, source disagreements and wiki overrides in manifest evidence.
`build_native_profiles.py` creates the exact 67-profile Rust qualification snapshot;
missing records, extra native identities and unresolved references fail the build.

The Rust `spell::native` reader checks complete cast headers and dependencies against
that snapshot before dispatching to typed actor, combat, companion, delayed, house,
movement and item planners. The planners implement source-qualified mechanic calculations
and transition plans from owner facts. They do not install world commits, persistent
services or wire routes, and do not expand the admitted three-spell V1 book. Thus
authoring completion and native plan tests do not imply production gameplay admission.

The r19 converter resolves field item constants from each pinned server's
`ItemID_t` header and rejects unresolved or nonpositive create-item IDs. Nine
field runes now emit the source IDs 2118 (fire), 2122 (energy), or 105 (poison),
with matching dependency and catalog references. The source notes retain the
constant-to-ID binding. This fixes item identity; world-item execution remains
subject to its existing admission requirements.

`guildstats_spells.py` compares saved, hash-qualified GuildStats character-hits
HTML against authored magic-component bounds. It preserves the calculator's
effective magic level and rounding. The captured calculator uses a legacy
linear level contribution; differences are reported, never applied as fixes.
Its best-hit/PvP labels concern autoattacks, not these spell ranges.

The offline r18 repair candidate applies explicit neutral reference-model proposals through
`formula_corrections.py` and its five vocation/carrier modules. These are independently authored
ASTs qualified against the pinned TibiaTools backend, with alternative Canary/Crystal models
retained in evidence notes. They are not an accepted change of canonical Game truth. Identity,
resolved base power and S5 world level contribution are preserved, except for the explicit
owner decision of 2026-10-01 selecting calculator BP25 for Strong Ethereal Spear over captured
wiki BP38. The captured wiki facts remain unchanged and the disagreement stays attributed;
this is a candidate data decision, not runtime admission. Missing calculator
min/max (`buckets=0`) does not mean deterministic damage: source spread is retained and the
nominal center is checked separately, without treating the midpoint as the RNG mean.

Source-exact `blocked_completions.py` closes extraction for Heal Friend, Paralyze Rune and
Inflict Wound. The new caster presentation timing, target selector and zero-health path fields
remain executor proposals; the core rejects them until implemented. Three monk spender
callbacks are extracted before Harmony, with source pins/blobs checked, leaving existing
actor-owned charge handling intact. BP-only Harmony is a separate contract/core proposal,
not enabled by this converter. `ready` continues to mean authoring-valid, not playable.

The schema verifier also runs the local `test_*.py` regression suites. Rune comparison joins prefer
item ID, then explicit aliases and actual names; ambiguous identities are reported. Missing facts
remain `no_source`, and source agreement does not establish runtime qualification. Party buffs may
use scaled or fixed mana. This offline candidate's Enlighten regeneration follows the supplied
owner handover; the referenced #162 decisions still need live readback before publication.

The owner-requested wiki gap completion uses captured tibiopedia.pl rune `spellrange` only when
Fandom/BR/official resolution and both engine registrars provide no range. Both engines must match
the same unambiguous rune identity. The supplementary page retains its URL, capture date and hash
as `community_capture`; it cannot replace an existing value, an S13 conflict decision, or an area
definition. This fills missing cast-range data and does not resolve native execution blockers.

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
| `official-changes.json` | S11/S24 evidence: official changes that decide a BR/Fandom conflict (fact, date, source URL); field `removed` blocks a spell an announcement removed. |
| `chain-behaviours.json` | S23: accepted chain parameters of player spells (`Ability.chain`), with their sources. |
| `party-behaviours.json` | S27 C.3: accepted `party_buff` parameters of the party spells, with their sources; an entry with `blocked` keeps its spell blocked. |
| `guard-behaviours.json` | S27 B.5/C.5/D.3: the exact Canary `onCastSpell` guards the core now expresses with spell fields (`targeting.allowed_targets`) or extra effects; `spell_scripts.py` evaluates such a script to its single Combat. |
| `wheel-augments.json` | S6/S24 evidence: Wheel of Destiny spell augments (I/II) and revelation perk stages, each with its source (official news, Fandom revision, or Canary as hypothesis) and superseded values. |
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
python build_native_profiles.py --bundles <complete convert_spells --out dir> \
    --out samples/native-spell-profiles.json
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
characters, the `effect` and `notes` mechanics text up to 4000, with revision ids) as the `spell-wiki-br-<sha>` artifact; the same workflow runs the schema
checks above.

### Mystic Repulse II source synchronization (2026-10-01)

`wheel-augments.json` now selects **+60% base damage** for Mystic Repulse II,
matching Wheel authoring and the current official Tibia.com planner
(`MediumPerkInfos.46.Aug2Info`). The former EnglishWiki/Canary +40% row is
retained as superseded evidence. The Oct1 source identity and observation date
apply to this row; other rows retain their recorded target-date sources.
The Wheel evidence check binds this file's digest and refuses a value/unit
disagreement between Spell and Wheel. This does not admit a native spell effect.
