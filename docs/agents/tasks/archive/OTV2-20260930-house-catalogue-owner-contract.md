# OTV2-20260930-house-catalogue-owner-contract

```yaml
task_id: OTV2-20260930-house-catalogue-owner-contract
title: HOUSES-4 - House catalogue owner contract candidate V1
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/tender-mendel-06tjg2
issue: 162
lane_id: content population (house authoring schema)
pr: null   # recorded in the FREEZE_SHA packet
base_sha: 7d9eb4a1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "house schema worker (claude-code-session-01AGaDHCuMKQ95cqTJBs5XRQ)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1.md
  - tools/content-schema/house-authoring/README.md
  - docs/agents/tasks/archive/OTV2-20260930-house-catalogue-owner-contract.md
public_contracts: [docs/architecture/OTERYN_HOUSE_CATALOGUE_OWNER_CONTRACT_V1.md]
depends_on: [OTV2-20260929-house-authoring-schema-candidate, OTV2-20260929-house-doors]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

A candidate owner contract for the static House catalogue (`content/houses/`), modelled on the Achievement
owner contract (D126) and bound to the accepted housing architecture `EXP-HOUSES-01`. It fixes catalogue
ownership, key and door identity, source precedence for population and how catalogue revisions reach live
Worlds. It authorizes neither population nor runtime; population is the next change after acceptance.

## Architecture and source of truth

- PROVEN: `EXP-HOUSES-01` is ACCEPTED (2026-09-08): one World-global `HouseId` per World, owner `CharacterId`
  (Guildhouse `GuildId`), Premium + `PhysicalHouseEligibility` at acquisition, Premium lapse is not eviction,
  GUI-only ACL (no player-facing `aleta` spells), fine-grained per-door access. The contract adds no housing
  semantics.
- DERIVED: a World's `HouseId` is its instance of the catalogue key; the representation stays persistence detail.
- ASSUMPTION (owner question): `shop` houses are ordinary physical houses under the personal housing slot.

## Owner direction (2026-09-30)

1a prepare this contract; 2a populate `content/houses/` right after acceptance; 3 pending (recommendation b:
keep the TibiaWiki BR comparison as the README summary and the capture workflow, no committed facts file).

## Validation (local)

- Governance, repository policy and architecture semantic audit: see the FREEZE_SHA packet.
- Review: exact-head independent review required (contract candidate).
