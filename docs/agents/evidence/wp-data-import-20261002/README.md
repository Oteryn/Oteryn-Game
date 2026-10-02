# Native Weapon Proficiency data import qualification

`actual-import-qualification.json` records the actual native CLI output comparison against every
committed definition, ordered perk, binding and the entire progression ruleset. Raw source JSON
was independently read with Python Decimal; every imported numerator/denominator was checked
using Fraction. All 443 definitions / 3671 perks / 665 bindings are exact, including source metadata.
No activation, Character progress or provider admission is claimed.

`independent-review.md` is the source-pinned advisory read-only review, with both schema defects
and complete-ruleset preservation repaired before final checks. It separately confirms all 665
Item references resolve against 34032 committed Items. Final CI/review remains coordinator-owned.

To run the native import: `cargo run --locked -p oteryn-game-server --bin oteryn-game-import-proficiencies`.
The JSON export is a data-only result, not an authority-bearing runtime generation or checkpoint.
