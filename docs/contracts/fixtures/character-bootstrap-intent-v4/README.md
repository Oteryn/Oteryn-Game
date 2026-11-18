# Character bootstrap intent v4

Byte-exact intent bodies served by
`POST /internal/v1/game-auth/character-bootstrap-intents/read`, contract
version 4. Contract:
`docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md`. A
change to any file here is a contract change.

- Version 4 adds `requested_vocation`, right after `requested_sex`: exactly one
  of the case-sensitive JSON strings `Knight`, `Paladin`, `Sorcerer`, `Druid`,
  `Monk`. No numeric form, no `None`, no default, no case normalization.
  Versions 1-3 are retired.
- Agreed with the Game consumer lane on 2026-11-17. The consumer at
  `Oteryn/Oteryn-Game@e2152d050038cecdb3728c222af2ac184e1cac5a`,
  `crates/gameplay/durability/src/character_bootstrap_intent.rs` still pins
  `CONTRACT_VERSION = 3` and moves to 4.
- Newhaven is the existing `starter_template_revision` value
  `newhaven-starter-r1`; there is no new member.
- Proof: `portal-rs/crates/characters/src/bootstrap/tests.rs`,
  `v4_fixtures_match_the_serializer_and_validator`. It checks every file's
  length and SHA-256 against `manifest.json`, that each valid intent is the
  issuer's own serializer output for the manifest values and passes the
  reader's validator, and that each rejected intent differs from
  `intent.knight.json` only in its vocation and fails that validator.

## Cases

`manifest.json` lists the files with byte length and SHA-256, the values shared
by every vector (`fixed`) and one entry per case. Files have no trailing
newline. Identifiers and times are synthetic.

| case | file | `requested_vocation` | outcome |
|---|---|---|---|
| intent-knight | `intent.knight.json` | `Knight` | accept |
| intent-paladin | `intent.paladin.json` | `Paladin` | accept |
| intent-sorcerer | `intent.sorcerer.json` | `Sorcerer` | accept |
| intent-druid | `intent.druid.json` | `Druid` | accept |
| intent-monk | `intent.monk.json` | `Monk` | accept |
| vocation-lower-case | `rejected.vocation-lower-case.json` | `knight` | reject |
| vocation-none | `rejected.vocation-none.json` | `None` | reject |
| vocation-missing | `rejected.vocation-missing.json` | member absent | reject |

The rejected bodies are ones a conforming Platform never sends; they pin the
consumer's fail-closed behaviour. The issuer refuses the same values as bad
input before any transaction.
