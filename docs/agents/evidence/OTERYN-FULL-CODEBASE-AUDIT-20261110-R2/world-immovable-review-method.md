# World immovable catalogue — complete semantic data review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`, `crates/gameplay/world/src/map/spell_tiles/immovable_catalogue.rs`, 572 physical lines. Classification: **full mixed semantic review**, separate from manual source-only reading. The repeated encoded characters were analyzed as a complete indexed relation; no claim is made of manually reading each repeated character or regenerating absent donor bytes.

The complete actual consumer `immovable.rs` (77 lines), generator `build_bundle_immovable.py` (72 lines), shared parser/loader `build_source_world.py` (198 lines) and spell-tile builder/test source (1,102 lines) were manually read. Every distinct encoded semantic class was examined. Exhaustive structural and relationship assertions cover every encoded position; parsing alone was not counted as substantive review.

## Complete representation and meaning

The literal consists of 565 rows of 96 ASCII characters and one final row of 27 characters, indexed by source ID 0 through 54,266. Every row and delimiter is structurally consumed with no unexamined trailing data. Exactly 42,107 positions are known; all other 12,160 positions contain `?`. The complete known-position set equals the committed membership manifest naming the generator's exact appearance-file digest. This authenticates agreement with that committed manifest, without claiming possession of the original appearance bytes.

| Code | Count | Solid | Pathfind | Magic field | Movable |
|---|---:|---|---|---|---|
| A | 14,589 | false | false | false | false |
| B | 14,828 | true | false | false | false |
| C | 838 | false | true | false | false |
| D | 321 | true | true | false | false |
| E | 11 | false | false | true | false |
| F | 1 | true | false | true | false |
| G | 20 | false | true | true | false |
| H | 4 | true | true | true | false |
| I | 9,839 | false | false | false | true |
| J | 827 | true | false | false | true |
| K | 664 | false | true | false | true |
| L | 156 | true | true | false | true |
| M | 9 | false | false | true | true |
| ? | 12,160 | unknown | unknown | unknown | unknown |

The consumer returns three conservative blocking facts for unknown IDs, invalid codes or out-of-range lookup. Known movability applies only when the placement has neither a unique ID nor action 100. Solid and pathfind remain separate. A magic field can block item pathfinding without becoming a nonfield blocker. Action 101 does not inherit action 100's immovability override. Source flags are loader facts, not an independent movement grant.

The complete builder processes only top-level entries, distinguishes ground from tops, joins the actual served definition, aggregates these flags and preserves unknown tiles for missing identities, pickupable entries and usable doors. It validates the total visited base count and interns the complete shape, including PvP precedence and projectile/floor facts. Actual relocation still requires its sealed current actor/session/map proof and landing checks.

## Independent relationships checked

`world_immovable_semantics.py` and `world-immovable-semantics.json` retain exact source SHA, immutable Git equality, every class and every position. The checked-in XML exactly matches the generator's pinned SHA-256. All 32,938 XML-member magic-field classifications and all final explicit blocking/movable overrides were compared with the encoded relation. All 45 encoded magic-field IDs are exactly those established by the relevant XML semantics. Duplicate XML ownership is rejected in the checked subset. Source membership and selected XML relations do not grant full review credit to those large supporting files.

The generator checks both expected input hashes before decoding. Its imported parser's unrelated older source-world revision does not alter this generator's explicit inputs: only the parser and policy functions are used. The bounded protobuf parser rejects unsupported wire types and truncated fields; the loader rejects missing/duplicate selected objects and duplicate XML overrides. XML properties are merged in source order, then the generator applies the final blocking override. This is static source analysis; the generator was not executed.

## Limits

The original `appearances.dat` is absent from the inspected workspace. The protobuf-derived base solid/pathfind/movable facts and regeneration from those exact original bytes remain **UNKNOWN**. No external donor fidelity or live movement result is asserted. No Rust/native/SQL test or complete world import was run. The three original tests in `immovable.rs` were inspected only. The data review establishes the complete checked-in representation, its actual consumer semantics and available pinned relationships, while keeping the missing external input explicit.
