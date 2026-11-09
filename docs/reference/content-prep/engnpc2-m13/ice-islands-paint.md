# Ice Islands painting: checked source boundary

Source: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua
SHA-256: `599666e12f2960ab22b820a84a2b79c0ea40db7a0e2b39e7e3ef4cb2931272a5`.

The chosen recipe is `oteryn:quest.the_ice_islands_quest`, stage s4, count3, target_items Baby Seal and Vial of Paint. Stage s3 releases ants on a ship mast; it does not require capturing a live Creature.

The pinned paint action checks target Item 7178, transforms it to7252, and restores7178 after30000ms. It requires Questline8 and increments PaintSeal; the third successful operation (legacy unset value -1) advances Questline 9 / Mission04 = 2 and removes Item 7253. The source itself lacks a personal distinct-target guard; the chosen request explicitly requires three different seals.

Thus the Formorgar/Ice Islands consumer needs native map-object target identity and personal distinct progress. It must not fabricate a Creature identity for Item 7178. The earlier live-Creature owner-borrow change remains relevant to the Goblin Merchant live targets.

The initial distinct-target planner is bounded to eight signed masks and504 stable target indices. It rejects repeated targets, invalid population shapes and unknown bits. It is a pure planner, with no gameplay authority or persistence by itself; the fenced Root now owns progression and conditional paint consumption, while production map callback registration remains pending.

Siflind's pinned start action confirms the initial snapshot: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/siflind.lua lines 142-144 grants Item 7253, then sets Questline 8 / Mission04 = 1. It does not initialize PaintSeal. The new personal distinct-target track starts at 0; it does not reuse the legacy counter's -1 sentinel.

The native state rule advances Questline 9 / Mission04 = 2 together on the third distinct target. Its PostgreSQL fixture exercises the real fenced Root with that explicit starting snapshot, duplicate refusal, exact replay after completion and rejection without World context. Production parsing now accepts only the exact checked seal, vial and transform/reset metadata; registration in the World catalogue remains pending, so this is not yet a playable quest qualification.

A complete scan of the current 1,208 native placement regions found 198 top-level Item 7178 roots (193 on floor -7, two on -8 and three on -11). The earlier nearby 184 roots were a partial geographic scan. Source Item 7178 identity, native position and top-level ordinal must all remain part of callback admission; population counting alone grants no target authority.

The exact target table in `ice_islands_seals/targets.rs` was checked against all 198 decoded native roots. Map source: https://github.com/zimbadev/crystalserver/blob/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data-global/world/world.otbm ; materialized placement-index SHA-256: `f770da7cceec09faf63e6086fa099efa001db57045d4e9906d0057ff42c90af6`. IDs are append-only and must not be renumbered when future map content changes. The pure target resolver checks position, native floor, top-level ordinal and unpainted appearance 7178, then delegates to the same progress planner. It is not a substitute for retained runtime authority or paint custody.

Recheck the map qualification with `python3 tools/content-migration/verify_ice_seal_targets.py`. This read-only verifier decodes every indexed region, verifies its recorded digest, rejects provisional seal palette bindings and compares the complete set of native seal identities with the stable Rust table.

The fenced World-consumption participant now receives the exact locked three-track plan. It recomputes the seal transition, rejects incomplete or forged plans, and consumes one plain carried vial only on the third distinct seal. All uses still require positive live quantity and locked current custody. The existing Item consumption owner removes custody and records the burn in the same physical PostgreSQL transaction as the quest receipt; replay performs no second burn.

Real Root cases: `engnpc2_ice_seals_postgres_distinct_atomic_completion_and_replay` (4.972813651s) and `engnpc2_ice_seals_postgres_paint_retained_until_atomic_completion` (4.724492814s). The latter starts with no vial and proves refusal without progress, then uses historical carried fixture stock with every trigger enabled during the tested commits. The red case left quantity1/lifecycle1 after completion; green proves quantity0/lifecycle2, removed custody, one consumption receipt and a matching physical transaction ID on the quest receipt. These cases still do not qualify production map-click routing or the source 30-second map reset.

The closed callback admission covers all 198 checked map identities, Item7253, Item7178-to7252 transformation and the exact 30000ms reset. The source green-rings enum is9: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/utils/utils_definitions.hpp line64 (`CONST_ME_GREEN_RINGS = 9`). The runtime handler reference remains the pinned Lua paint action; this C++ enum reference is supporting content-preparation evidence. Seventeen altered definitions are refused, including renamed keys, alternate target identities, missing/wrong/early-consumed paint, added experience, different transforms and different reset timing.

Catalogue integration now declares the owned `painted_seal_targets` track only when seal callbacks are registered. It requires initial0/minimum0 and the exact packed-storage maximum, rejecting an incompatible existing declaration. Tests reuse the actual canonical Questline/Mission04 metadata, prove the first native transition reads an absent counter as0, and verify unrelated callback sets add no counter. The legacy `paint_seal` track remains separate.
