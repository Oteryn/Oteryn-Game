# Migration registry after main integration

Main `1ae39c7fb9bef09c3cd50d16410374f4134b1676` owns the shared migrations below; this branch consumes their exact bytes. The former 1200–1205 files are removed.

- `0999_quest_reward_callbacks.sql`
- `1000_quest_reward_payments.sql`
- `1001_quest_reward_participants.sql`
- `1002_quest_reward_state_operations.sql`
- `1003_quest_reward_target_sets.sql`
- `1004_quest_paid_item_exchanges.sql`
- `1005_quest_carried_use_proofs.sql`
- `1006_quest_owned_base_predicates.sql`
- `1007_quest_carried_payment_reapplication.sql`
- `1008_quest_carried_membership_projection.sql`

Lane-owned files (1290–1299):

- `1290_questpc_source_contracts.sql`
- `1291_marlin_trophy_exchange.sql`
- `1292_raid_token_exchange.sql`
- `1293_map_key_binding.sql`
- `1294_quest_npc_item_sources.sql`
- `1295_quest_stateless_paid_receipts.sql`
- `1296_quest_paid_sale_mint_occurrences.sql`
- `1297_fathers_burden_xp_binding.sql`

1290 contains only the source-specific contracts absent from main: fluid/charge initialization, NPC response receipts, Falcon cycles, Corruptor lifecycle, Viola and Father proofs. It composes the exact Father eleven-track exception with the installed main constraint; it never replaces main’s paid/carried arms. Existing lane extensions follow in their original order, renumbered with git mv.

Earlier registry proof files describe their retained historical commits. Current file hashes and the explicit absence review are in `cloud-main-migration-consolidation.json`. PostgreSQL markers and exact durations are retained in cloud-main-qengine-consolidation-proof.json; the repository duplicate check passed on the staged tree.
