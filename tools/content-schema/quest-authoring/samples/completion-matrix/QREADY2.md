QREADY2 owns 18 reward-only quests. Knight Outfits remains owned by QENGINE2.
The primary case packet covers 17 quests; the Purple Tome follow-up packet covers
its three independent book claims. All 24 source selectors match the native map.

Reuse: the production bundle chest route and durable reward claim mint owner,
QUESTPC2/3/4/7 source selectors and once/replay scenario shape, and the existing
world `quest_access_bindings` startup overlay. No migrations or new reward engine.
The native Item provider gains only the already canonically admitted Items 3310,
21982, 3587, 2822, 2823 and 2848 through the existing admission checks.

Regeneration:

```
python3 tools/content-schema/world-authoring/qready2_chest_bindings.py --apply
python3 tools/content-migration/regenerate_content.py --native-only
python3 tools/content-schema/quest-authoring/grizzly_rooms.py
python3 tools/content-schema/quest-authoring/quest_reward_use_proofs.py --packet tools/content-schema/quest-authoring/samples/completion-matrix/qready2-cases.json --log <21-case-postgres-log> --batch <1|2|3> --output tools/content-schema/quest-authoring/samples/completion-matrix/reward-use-proofs/qready2-batch<N>.json
python3 tools/content-schema/quest-authoring/quest_reward_use_proofs.py --packet tools/content-schema/quest-authoring/samples/completion-matrix/qready2-purple-cases.json --log <3-case-postgres-log> --output tools/content-schema/quest-authoring/samples/completion-matrix/reward-use-proofs/qready2-purple.json
python3 tools/content-schema/quest-authoring/quest_completion_matrix.py
```

Stage inputs and use the repository world bundle compiler's `pin-refresh` and
`pin-check` commands. Each imported proof pins claim and test bytes and requires
a passing PostgreSQL run and a non-skipped completion marker for every qualified
claim. `REWARD_CLAIM_USE_VERIFIED` records reward execution, not the full journey.
