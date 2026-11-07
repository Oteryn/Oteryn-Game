QREADY2 owns 18 reward-only quests. Knight Outfits remains owned by QENGINE2.
The case packet contains the 17 quests with source-compatible native placements.
Purple Tome remains blocked: the third claim expects appearance 2436, while
`region-z15-x126-y123.b3` has appearance 2438 at (32421,31594,15).
Its source claim and reward definitions are preserved pending the shared binding owner.

Reuse: the production bundle chest route and durable reward claim mint owner,
QUESTPC2/3/4/7 source selectors and once/replay scenario shape, and the existing
world `quest_access_bindings` startup overlay. No migrations or new reward engine.
The native Item provider gains only the already canonically admitted Items 3310,
21982 and 3587 through `quest_callback_profiles`' existing admission checks.

Regeneration:

```
python3 tools/content-schema/world-authoring/qready2_chest_bindings.py --apply
python3 tools/content-migration/regenerate_content.py --native-only
python3 tools/content-schema/quest-authoring/grizzly_rooms.py
python3 tools/content-schema/quest-authoring/quest_reward_use_proofs.py --packet tools/content-schema/quest-authoring/samples/completion-matrix/qready2-cases.json --log <postgres-log> --batch <1|2|3> --output tools/content-schema/quest-authoring/samples/completion-matrix/reward-use-proofs/qready2-batch<N>.json
python3 tools/content-schema/quest-authoring/quest_completion_matrix.py
```

Stage inputs and use the repository world bundle compiler's `pin-refresh` and
`pin-check` commands. Each imported proof pins claim and test bytes and requires
a passing 21-case PostgreSQL run and a non-skipped completion marker for every
qualified claim. The matrix's
`REWARD_CLAIM_USE_VERIFIED` records reward execution, not the full quest journey.
