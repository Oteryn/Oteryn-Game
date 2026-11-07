# Achievement authoring (ACHIEVEMENT lane)

Schema and validator for the Achievement catalogue of
[`OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md`](../../../docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md) §2.
`build_catalogue.py` writes the catalogue in `content/achievements/` (contract §5, step 2).

| File | Purpose |
|---|---|
| `achievement.schema.json` | One catalogue record: `identity` (`oteryn:achievement/<slug>`), `name`, `description`, `grade` 1-4, `points`, `secret`, `premium`, optional `retired` and `provenance` (client `source_id`, TibiaWiki page and revision). |
| `validate_achievements.py` | JSON Schema plus the rules it cannot state: points lie in the grade's range (1-3, 4-6, 7-9, 10) or are 0 for a retired record, keys are unique across all given files. The key is checked for format only: `allocate_key(name)` gives the key of a new record, and later revisions keep it when the name changes (contract §2.1). With `--chest-claims`, every `placements[].achievement` ref of a RewardClaim document must bind to a catalogue key: by slug, or explicitly (`the_professors_nut`, §2.2); an unbound ref is listed and fails (§3.3). |
| `synthetic-valid-achievement.json` | A valid record (Allow Cookies?). |
| `test_validate_achievements.py` | No-network tests, including slug parity with `quest-authoring/ots_chests.py` and the binding of the chest sample's refs to `content/achievements/`; run in CI by `.github/workflows/achievement-authoring-schema.yml`. |
| `build_catalogue.py` | Builds `content/achievements/achievements-*.json` from the staticdata and TibiaWiki observations and `owner_resolutions.json` (contract §2.2); keys are allocated once and kept on rebuild. `--check` regenerates byte-identically. |
| `owner_resolutions.json` | The owner's 2026-09-29 resolutions of the join anomalies: overrides and exclusions. |
| `requirements.txt`, `requirements-dev.txt` | Pinned `jsonschema`, `referencing` and `ruff` for CI. |

```sh
python validate_achievements.py RECORDS.json [...]   # a record, a list or a catalogue shard per file
python validate_achievements.py ../../../content/achievements/achievements-*.json \
  --chest-claims ../quest-authoring/samples/chests/claims.json   # chest refs bind to the catalogue
python build_catalogue.py [--check]                  # write or verify content/achievements/
python test_validate_achievements.py
```

Fit to the evidence (2026-09-29): building candidates from the client staticdata and the TibiaWiki facts by the
contract's §2.2 precedence, with the owner's resolutions of the 10 join anomalies, gives 571 records, all valid and
with unique slugs (one retired, The More the Merrier); only the inferred-id page `Achievement 563` stays out. Of the
28 `…:achievement/<slug>` refs in the quest samples, 27 match a catalogue slug; `the_professors_nut` binds
explicitly to `the_professor_s_nut` (contract §2.2).
