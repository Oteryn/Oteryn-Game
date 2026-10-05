# Experience table evidence (2026-10-05)

`evidence.json` is the formula record for `rulesets/character/experience/experience-table.json`
(ARCH-PROGRESSION-SOURCE-0 section 1.1, amendment A1).

- **Source.** The public closed-form formula `exp(L) = 50/3 * (L^3 - 6L^2 + 17L - 12)`
  (`source_kind` `closed_form_formula`), with the TibiaWiki "Experience Formula" page as
  provenance. No third-party table and no capture is committed.
- **Generation.** `tools/content-schema/character-progression/build_progression.py` holds the
  formula once as structured constants, evaluates it in exact integers (no float), and renders
  the `formula` text from the same constants. It refuses an `evidence.json` whose `formula` or
  `source_kind` differs.
- **`levels_sha256`.** The SHA-256 of the generated normalized `levels.csv` (`<level>,<experience>`
  LF-terminated lines for levels `1..=last_level`). It is a regression pin, not independent evidence.
- **Not checked against the official table.** The values have not been compared with the official
  tibia.com table; this is a declared difference. Existing support: the OTS formula reproduces
  official Experience Table samples including levels 23 and 24 (Global Reference checkpoint
  2026-09-09, section 9.3), which also asks that the formula be treated as a regression hypothesis.
- **Runtime.** The formula is producer input only. The runtime reads the committed,
  revision-bound table through the native gameplay `progression` section.
- This README is prose and is not digest input; the evidence revision is computed from the
  canonical `evidence.json` alone.
