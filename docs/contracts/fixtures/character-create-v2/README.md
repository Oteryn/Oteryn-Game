# Character create intent v2

Byte-exact vectors for the `CreateCharacter` intent that Platform serves through
`/internal/v1/game-auth/character-commands/read` (Issue #1937). Contract:
`docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`, section
*1. Create character*. A change to any file here is a contract change.

- Members, in order: `contract_version` (2), `command`, `variant`
  (`PLATFORM_USER_CREATE`), `issuer_authority`, `issuer_decision_id`,
  `source_revision`, `operation_id`, `account_id`, `target_world_id`,
  `requested_name`, `requested_sex`, `requested_vocation`, `issued_at_source`,
  `expires_at_source`, `audience`. This is v1 with `requested_sex` and
  `requested_vocation` inserted right after `requested_name`.
- `requested_sex`: exactly `Female` or `Male`. `requested_vocation`: exactly one of
  `Knight`, `Paladin`, `Sorcerer`, `Druid`, `Monk`. Case-sensitive, no default,
  no numeric form.
- Game consumer: `Oteryn/Oteryn-Game@e2152d050038cecdb3728c222af2ac184e1cac5a`,
  `crates/gameplay/durability/src/durability/character_command.rs` (`CreateWire`,
  `deny_unknown_fields`, lines 60-78; it pins `contract_version` 1 at line 124 and
  moves to 2 in its own change). Agreed with the Game server lane on 2026-11-17.
- Base: `Oteryn/Oteryn-Platform@1bb15ad3af64a975dd01da79f5d210f5806d6077`.

## Cases

`manifest.json` lists each file with its byte length and SHA-256, and one case per
file. `accept` vectors were produced by the Platform serializer
(`portal-rs/crates/characters/src/commands/wire.rs`) and share the operation,
decision, revision, account, world, name and timestamps of the v1 golden
`portal-rs/crates/characters/src/commands/fixtures/create-intent.json`. `reject`
bodies are canonical otherwise and must be refused by a conforming consumer. Files
have no trailing newline. Digests and outcomes are checked by
`portal-rs/crates/characters/src/commands/create_v2_tests.rs`.

| case | file | sex | vocation | outcome |
|---|---|---|---|---|
| create-knight | `create-intent-knight.json` | Male | Knight | accept |
| create-paladin | `create-intent-paladin.json` | Female | Paladin | accept |
| create-sorcerer | `create-intent-sorcerer.json` | Male | Sorcerer | accept |
| create-druid | `create-intent-druid.json` | Female | Druid | accept |
| create-monk | `create-intent-monk.json` | Female | Monk | accept |
| rejected-vocation-lowercase | `rejected-vocation-lowercase.json` | Male | `knight` | reject |
| rejected-vocation-missing | `rejected-vocation-missing.json` | Male | absent | reject |
| rejected-sex-missing | `rejected-sex-missing.json` | absent | Knight | reject |
| rejected-sex-lowercase | `rejected-sex-lowercase.json` | `female` | Paladin | reject |

Create receipts stay `PublishCharacterCommandReceiptV1` and do not echo sex or
vocation.
