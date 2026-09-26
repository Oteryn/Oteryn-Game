# First batch compared with current Wiki

The owner requested the current state instead of a historical target date.
Both APIs were freshly read on 2026-09-26 without a revision-date selector.
Latest-page revisions, content digests and acquisition/server timestamps are retained.
Only compared structured facts are committed; raw Wiki prose and artwork are excluded.

| Source | Rows | Matches | Differences | Unknown | Not comparable |
|---|---:|---:|---:|---:|---:|
| TibiaWiki Fandom | 259 | 229 | 12 | 8 | 10 |
| TibiaWiki BR | 225 | 173 | 13 | 20 | 19 |

Counts are source-specific: coverage differs, and Wiki observations can disagree.
Canary values are preserved. No Global or runtime parity is claimed.

## TibiaWiki BR findings

Latest-page read: **2026-09-26T19:42:45.227194+00:00**; API server timestamp:
**2026-09-26T19:42:45Z**.

| Monster | Conflicting numeric/boolean observations (Canary -> Wiki) |
|---|---|
| Rat | mitigation_percent 0.07 -> 0.12 |
| Cyclops | mitigation_percent 0.62 -> 0.97 |
| Orc Spearman | mitigation_percent 0.3 -> 0.48 |
| Scorpion | mitigation_percent 0.13 -> 0.20 |
| Orc Shaman | mitigation_percent 0.25 -> 0.40; push_items False -> True; blocked_by_nearby_players False -> True |
| Necromancer | mitigation_percent 1.04 -> 1.65 |
| Fire Elemental | mitigation_percent 0.51 -> 0.80 |
| Dragon | mitigation_percent 0.99 -> 1.56; blocked_by_nearby_players False -> True |
| Ghost | mitigation_percent 0.51 -> 0.80 |
| Demodras | max_health 4500 -> 3750 |

The BR infobox's `defense` field is rendered as **Armadura** (armor), so it is
compared with Canary `stats.armor`, not `stats.defense`. Armor matches all ten.
Damage modifiers describe received damage; Canary element percentages describe
reduction. The comparison uses `100 - reduction`, preserving vulnerabilities.
Blank values stay UNKNOWN. Raw speed is NOT_COMPARABLE without unit qualification.

Six base-loot name sets match case-insensitively. The other four show:

- Orc Shaman: Canary `book`; BR `Grey Small Book`.
- Dragon: BR additionally lists `Dragon Trophy`.
- Ghost: Canary `book`; BR `Orange Book` and `Stone Skin Amulet`.
- Demodras: Canary `book`, `great mana potion`, `life crystal`; BR `Gemmed Book`, `Stuffed Dragon`.

No fuzzy Item identity is admitted from these names. Event/raid loot is excluded from
base loot. Wiki frequency words do not prove exact loot probabilities or quantities.
Ability references/ranges are retained, while formula, intervals, condition timing
and geometry remain UNKNOWN unless separately qualified.

## TibiaWiki Fandom findings

`wiki-current-fandom.json` independently records the latest revision of each page,
including its revision URL and server/read timestamp. Nine mitigation values differ;
the other differences are Fire Elemental pushability and an additional Dragon Trophy
and Stone Skin Amulet. Fandom disambiguated book names are matched to the Canary base
name by the existing collector; BR deliberately retains those name differences.

The BR observations additionally differ on Orc Shaman push-items, Orc Shaman/Dragon
player-blocked spawn and Demodras HP. This is evidence of disagreement/coverage,
not an automatic choice of one source or permission to overwrite gameplay data.

## Reproduction

```text
python wiki_compare.py --canary <pinned checkout> --cache <local Fandom cache directory> --refresh
python wiki_current_comparison.py --cache <local BR cache.json> --refresh
python verify_wiki_compare.py
python verify_wiki_current_comparison.py
python verify_canary_provenance.py
```

Omit `--refresh` to regenerate from the same revision-bound local observations.
Raw content is scratch only. A latest page can legitimately have an older edit date.
