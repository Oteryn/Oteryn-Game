# First batch compared with current TibiaWiki BR

Latest-page API read: **2026-09-26 19:32:03 UTC**; the API's server timestamp agrees.
No historical cutoff or revision-date selector was sent. Each page's returned
revision ID, revision timestamp, URL and wikitext digest are retained in
`wiki-current-comparison.json`. A latest page can legitimately have an older edit date.
Only structured facts, Item/spell references and numeric ability ranges are retained;
Wiki narrative prose and artwork are excluded.

The 225 comparison rows yield **164 MATCH, 22 CONFLICT, 20 UNKNOWN and 19
NOT_COMPARABLE**. These classifications describe source observations, not verified
live-client or runtime parity. Canary remains the original pinned sample; no values
are overwritten from Wiki.

| Monster | Values differing between pinned Canary and latest Wiki (Canary → Wiki) |
|---|---|
| Rat | defense 5 → 1; mitigation 0.07% → 0.12% |
| Cyclops | defense 20 → 17; mitigation 0.62% → 0.97% |
| Orc Spearman | defense 10 → 6; mitigation 0.30% → 0.48% |
| Scorpion | defense 5 → 14; mitigation 0.13% → 0.20% |
| Orc Shaman | defense 10 → 8; mitigation 0.25% → 0.40%; push Items false → true; player-blocked respawn false → true |
| Necromancer | defense 25 → 50; mitigation 1.04% → 1.65% |
| Fire Elemental | defense 15 → 18; mitigation 0.51% → 0.80% |
| Dragon | defense 30 → 25; mitigation 0.99% → 1.56%; player-blocked respawn false → true |
| Ghost | defense 5 → 10; mitigation 0.51% → 0.80% |
| Demodras | maximum HP 4500 → 3750 |

The same field name is not independent proof of identical engine semantics; these
conflicts need source/target qualification before native gameplay decisions.

## Loot and ability limits

Six base-loot name sets match case-insensitively. The other four have:

- Orc Shaman: Canary `book`; Wiki `Grey Small Book`.
- Dragon: Wiki additionally lists `Dragon Trophy`.
- Ghost: Canary `book`; Wiki `Orange Book` and `Stone Skin Amulet`.
- Demodras: Canary `book`, `great mana potion`, `life crystal`; Wiki `Gemmed Book`, `Stuffed Dragon`.

Book names may represent aliases or a different appearance/Item. No automatic fuzzy
identity mapping is performed. The older `loot` field and current frequency-band
fields are both handled. Event/raid loot is separate and excluded from base-loot comparison.
Wiki frequency labels do not prove Canary's exact probabilities or quantities.

Raw speed values use unqualified source representations and are NOT_COMPARABLE.
Wiki damage modifiers are **received damage**, whereas Canary element percentages
are **reduction**; comparison uses `100 - reduction`, preserving vulnerabilities.
Blank Wiki fields stay UNKNOWN. Ability references and numeric ranges are captured,
but intervals, probabilities, formulas, condition timing and geometry are UNKNOWN
unless separately qualified. No complete creature or Global parity claim is made.

## Reproduction

```text
python wiki_current_comparison.py --cache <local-cache.json> --refresh
python verify_wiki_current_comparison.py
```

Without `--refresh`, the same local revision-bound cache regenerates the comparison
deterministically. Raw Wiki content is local scratch only and is not committed.
