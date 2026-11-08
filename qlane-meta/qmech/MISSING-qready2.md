# qready2 missing-mechanic report

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| All 18 qready2-owned reward-only quests listed in the two packets below | none in the checked reward claim/use scope | Definition claims: `content/quests/definitions/quests-00000-00099.json` and `quests-00100-00199.json`. Cases/selectors: `tools/content-schema/quest-authoring/samples/completion-matrix/qready2-cases.json` (17 titles, 21 claims) and `qready2-purple-cases.json` (Purple Tome, 3 claims). Step: world-object USE → `crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs::use_bundle_chest` → existing durable reward claim mint owner. Tables `game_reward_claims`, `game_reward_claim_mint_receipts`, and `game_item_audit_outbox` are asserted in `postgres_tests/qready2_claims.rs` and `postgres_tests/qready2_purple_claims.rs`: exact Item/quantity, second-claim refusal, duplicate command and cold-owner replay, and stale-fence rejection. No missing hook/table/command was found along this path. | neither for this checked scope |

## Evidence and boundary

- The committed `samples/completion-matrix/reward-use-proofs/qready2-*.json`
  packets pin the test files and source claims and retain positive PG durations.
  These are existing qualification records, not a new PG run for this report.
- `tools/content-schema/world-authoring/test_qready2_chest_bindings.py` checks
  all 24 native source selectors against the map; the existing startup overlay
  and `tools/content-migration/qready2_native_profiles.py` supply the selectors
  and six canonically admitted reward Items. No new runtime engine is required.
- The fetched qmech seed `aca848557` already contains the qready2 implementation,
  its integration updates and all 18 `REWARD_CLAIM_USE_VERIFIED` matrix rows.
  Do not replay the older qready2 commits over those newer integration fixes.
- Knight Outfits belongs to qengine2 and is excluded. Full quest journeys and
  unrelated client mechanics are not certified by the reward-only cases.
- Decision: no mechanic claim or migration is warranted for this lane's checked
  entries. Retain the original qready2 migration range 1440–1449; none are used.

## Coordinator M09 assignment (supersedes initial scope/range)

The later binding assignment adds the shared multi-grant MINT participant and
Rookie Guard, Treasure Hunt and Tower Defence reward boundaries to this lane.
Use QMECH migrations **2100–2109**, superseding the earlier retained range.
The live claim is recorded in `CLAIMS.md`; exact checked reward distinctions and
URLs are in `docs/reference/content-prep/qmech-m09/reward-sources.md`.
