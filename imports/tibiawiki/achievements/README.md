# TibiaWiki Achievement facts

Facts from every Achievement page of the English TibiaWiki (`tibia.fandom.com`, `Category:Achievements`),
extracted from the `{{Infobox Achievement}}` of each page by
`tools/content-census/stage_tibiawiki_achievements.py`, and joined to the 15.30 client staticdata
observations in `imports/cipsoft-staticdata/achievements/`. Each dated directory is immutable; a newer capture
adds a new date directory. Nothing here is an Oteryn identity or gameplay definition; `content/achievements/`
stays unpopulated until the Achievement owner contract (D126) defines the catalogue.

Per page: page id, title, exact revision and timestamp, the SHA-256 of the raw wikitext, and the infobox fields
`achievementid, name, actualname, grade, points, secret, premium, implemented, status, unknown, description,
coincideswith` as the wiki's raw strings. From `relatedpages` and `spoiler` only the linked page titles are kept
(`*_links`); the wiki's own prose (`spoiler`, `history`, `notes`) is not stored. Descriptions are the game's own
achievement text. Authors: the TibiaWiki contributors, CC BY-SA; each page's history is at
`https://tibia.fandom.com/index.php?curid=<pageid>&oldid=<revid>`.

| File | Content |
| --- | --- |
| `2026-09-29/tibiawiki-achievements-facts.json` | 572 achievements (204 secret) and 3 list pages without an infobox; `pages_digest` `0af3a02e…6ecf`, `snapshot_sha256` `8f6af8e5…a931` |
| `2026-09-29/staticdata-join.json` | join `achievementid == source_id`: 368 of 368 staticdata records joined, 203 wiki-only (all secret), 1 wiki page with an inferred id (`563?`), 57 joined records with a differing name or description, 10 anomalies |

The join differences are real source disagreements (wiki typos, spacing, older wording, name capitalisation);
for the 368 joined achievements the client staticdata is the closer source. Anomalies are values outside the
documented grade 1-4 and point ranges or missing fields, reported as found, not corrected.

`pages_digest` is the SHA-256 over the sorted `<pageid>:<revid>:<sha256>` lines; `snapshot_sha256` is the SHA-256
of the canonical raw snapshot. The raw snapshot holds wiki prose and is not committed; it is reproducible from the
pinned revisions:

```
python tools/content-census/stage_tibiawiki_achievements.py refetch RAW   # exact pinned revisions
python tools/content-census/stage_tibiawiki_achievements.py check RAW     # regenerates both files byte-exactly
python tools/content-census/stage_tibiawiki_achievements.py check         # offline: join derived from committed facts
```
