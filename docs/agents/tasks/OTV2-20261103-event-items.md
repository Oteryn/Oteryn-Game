# Event items and toys (W19)

## Task

Implement W19a-W19e through the existing item-use, reward-bag, appearance and
timed-item owners. The delivery remains a draft; no merge or review request.

## Verified starting point

- Base: 128ebc4f6aebf264d01c54003d269fa798e9524c.
- Primary source: Crystal 201189f871d23597b82caf88fd611454a2ddefb8.
- Cross-check: Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446.
- Existing source catalogues: event-item-actions.json, novelty-item-actions.json,
  item-cosmetic-actions.json and event-item-profiles.json under
  tools/content-schema/native-gameplay/.
- Existing durable owners: item_craft, consume_transform, character_appearance,
  account_achievement, spell_item_temporal and terrain_tool.
- Runtime source data does not prove a playable path. The existing re-audit is
  read-only and will not be changed in this task.

## Work allocation

| Slice | Integration |
| --- | --- |
| W19c | Existing consume/transform routing, novelty effects and messages |
| W19a | Existing reward-bag conversion and item-craft writer |
| W19d | Existing appearance grants inside the fenced item-use transaction |
| W19b | Existing terrain and absolute item deadline owners |
| W19e | Reward-box conversion and house balloon transformations |

Migrations 0880-0884 are reserved by the task. Prefer current command and
capability IDs. One writer owns commits and publication; builds are serialized.

## Validation

Pending implementation. Focused rule, refusal, replay and PostgreSQL dispatch
cases must execute before a completion claim. A route-less PostgreSQL case is
reported as skipped, never as database evidence.

## Open dependencies

- Canonical outfit content on the starting main is READY_UNPOPULATED. Missing
  canonical grants must refuse before consuming items or advancing progress.
- Source-defined temporary effects, timers and presentation require live
  consumers; recording source data alone is insufficient.
