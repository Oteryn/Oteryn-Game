# Owner decision batch D165-D173

- Decision: `OWNER-DECISION-BATCH-D165-D173-V1`
- Status: owner decisions recorded; docs only. Acceptance requires validation and protected integration.
- Origin: the owner answered the control-plane questions directly on 2026-09-30. The previous batch is `OTERYN_GAME_OWNER_DECISION_BATCH_D159_D164_2026-09-29.md`.
- Numbering: continues from D164
- Runtime, schema, registry, Platform and production authority: **NONE**. Each decision names its own limits. Nothing here is a merge, release or production grant.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Decisions

| # | Topic | Owner choice (2026-09-30) | Source | Q |
|---|---|---|---|---|
| D165 | Q13d disputed item families (41 ids, ITEM-ID-1b register) | 22a: promotion scrolls 43946-43950 are `progression_material`. 23a: the 12 Thais museum replicas (49272, 49273, 51314, 51316-51319, 53633-53637) are `event_collectible`. 24a (delegated to the control plane): 50335, 51030, 44527 and 44528 are `quest_item`. Exact TibiaWiki pages show these are quest variants; the common items 5941 (wooden stake), 5884 (spirit container) and 8008/8009 (guardian statue) are unaffected. 25: the "alternative" option for all 18 rows listed in #162 5905214525. 26a: 44044 "Test Voodoo Skull" (ts-only) is excluded from live content. 27a: 9132 frost cannon (Shadows of Yalahar quest mechanism) becomes a WorldObject with a new route reason `non_pickupable_blocking_prop` | #162 5905015838, 5905190571, 5905214525, 5905229985 | Q22-Q27 |
| D166 | Character name source for `ListCharactersForAccount` | A Character-name slice comes first (CHAR-NAME-1): a name column, reservation and a bootstrap-intent field, cross-repository with Platform. LCFA-1 then follows as written. LCFA-1 stays parked and migration 0021 stays reserved for it | #162 5905015838 | Q28a |
| D167 | New workflow in #1287 | The owner explicitly authorizes `.github/workflows/achievement-authoring-schema.yml` in #1287 at head `023c16e4`. This covers that exact control-path change only | #162 5905015838 | Q29a |
| D168 | PROFICIENCY-0 wire (§4.4) | Accepted: a capability-gated snapshot and delta, plus `PROFICIENCY_SELECT_PERK` with revision verification | #162 5905161470 | Q30a |
| D169 | Charm slot limits | 2 for free accounts, 6 for Premium, unlimited with Charm Expansion (Tibiopedia "Bestiariusz i uroki (charms)") | #162 5905161470 | Q31a |
| D170 | Charm unassign and reset | Supersedes the scope of CHARM-0 answer 3c. Unassign (level x 100 gp, 25% refund with Charm Expansion) and the full reset (100k + 11k x level from level 101; the first reset is free) are planned as in Tibia before charms are released to players. They form task CHARM-6, which depends on `GAME-ITEM-01`/`DUR-03`. Player-facing charm release is gated on CHARM-6 | #162 5905188176 | Q32b |
| D171 | N4-P ownership check before LCFA | Testing/preproduction only: the Platform native issuer does not verify AccountId->CharacterId ownership (§5.4). It relies on the Game FND-04A §5 admission check, which fails closed. This mode is config-gated and impossible in production. Full §5.4 via CHAR-NAME-1 and LCFA-1 is a release blocker | #162 5905264083 | Q33c |
| D172 | N4-P native route record and TLS trust | Testing/preproduction only: the route record (host, port, `tls_server_name`, `route_revision`) lives on `game_channels` and trusts the configured `oteryn-dev-client` root. U3, D3 and U7 (the production gameplay CA) are a release blocker | #162 5905264083 | Q34b |
| D173 | Control-plane migration order | 0018 DEATH-1a runtime GRANT and 0019 CHARM-2 are merged. 0020 is reserved for CHARM-3 and 0021 for LCFA-1. CHAR-BUILD-1, PROF-1 and CHARM-6 take the next free number at allocation | #162 5901531201, 5902830393 | control plane |

Not given a D number: question 32 was first withdrawn and then answered 32b; D170 records the answer.

## 2. Notes

- D165: rows with an engine binding go into `owner-item-family-decisions.json`, and the rest stay `PENDING_MINT`. This is applied by the task ITEM-Q13D-APPLY. D165 27a also needs a new world-object route reason with tests.
- D171 and D172 do not lift any production requirement, and production still needs separate explicit owner authority. The Platform contract (`OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`) receives a small amendment recording both testing-only modes in the N4P-3 PR.
- D167 authorizes the one workflow file only.

## 3. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
owner_decisions: [D165, D166, D167, D168, D169, D170, D171, D172, D173]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D165_D173_2026-09-30.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
```
