# Achievement authoring (ACHIEVEMENT lane)

Schema and validator for the Achievement catalogue of
[`OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md`](../../../docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md) §2.
It writes nothing under `content/`: populating `content/achievements/` is a separate step (contract §5).

| File | Purpose |
|---|---|
| `achievement.schema.json` | One catalogue record: `identity` (`oteryn:achievement/<slug>`), `name`, `description`, `grade` 1-4, `points`, `secret`, `premium`, optional `retired` and `provenance` (client `source_id`, TibiaWiki page and revision). |
| `validate_achievements.py` | JSON Schema plus the rules it cannot state: the key slug equals `slug(name)`, points lie in the grade's range (1-3, 4-6, 7-9, 10) or are 0 for a retired record, keys are unique across all given files. |
| `synthetic-valid-achievement.json` | A valid record (Allow Cookies?). |
| `test_validate_achievements.py` | No-network tests, including slug parity with `quest-authoring/ots_chests.py`. |

```sh
python validate_achievements.py RECORDS.json [...]   # one record or a list per file
python test_validate_achievements.py
```

Fit to the evidence (2026-09-29): building candidates from the client staticdata and the TibiaWiki facts by the
contract's §2.2 precedence, with the owner's resolutions of the 10 join anomalies, gives 571 records, all valid and
with unique slugs (one retired, The More the Merrier); only the inferred-id page `Achievement 563` stays out. Of the
28 `…:achievement/<slug>` refs in the quest samples, 27 match a catalogue slug; `the_professors_nut` binds
explicitly to `the_professor_s_nut` (contract §2.2).
