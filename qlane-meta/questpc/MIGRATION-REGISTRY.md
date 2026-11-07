# questpc-pc2 migration registry

Owner reservation: questpc-pc2 uses1290–1299; QBASE1200–1209;
QENGINE2 owns0999–1006. PR3020 was OPEN when this mapping was applied.
Shared engine files are byte-identical to
`origin/claude/happy-davinci-ijz3wy-qengine2` at69609ef78 (reverified unchanged at1b6001748).
The six QBASE files match the retained canonical questpc6 source at53d5648b8.
No copy or variant occupies a second filename for any shared engine migration.

| Active migration file | Owner |
| --- | --- |
| `0999_quest_reward_callbacks.sql` | QENGINE2: exact shared content |
| `1000_quest_reward_payments.sql` | QENGINE2: exact shared content |
| `1001_quest_reward_participants.sql` | QENGINE2: exact shared content |
| `1002_quest_reward_state_operations.sql` | QENGINE2: exact shared content |
| `1003_quest_reward_target_sets.sql` | QENGINE2: exact shared content |
| `1004_quest_paid_item_exchanges.sql` | QENGINE2: exact shared content |
| `1005_quest_carried_use_proofs.sql` | QENGINE2: exact shared content |
| `1006_quest_owned_base_predicates.sql` | QENGINE2: exact shared content |
| `1200_quest_finish.sql` | QBASE: exact shared content |
| `1201_corruptor_lifecycle.sql` | QBASE: exact shared content |
| `1202_questpc_npc_revision.sql` | QBASE: exact shared content |
| `1203_questpc_viola.sql` | QBASE: exact shared content |
| `1204_questpc_viola_book.sql` | QBASE: exact shared content |
| `1205_fathers_burden_items.sql` | QBASE: exact shared content |
| `1290_marlin_trophy_exchange.sql` | questpc-pc2 |
| `1291_raid_token_exchange.sql` | questpc-pc2 |
| `1292_map_key_binding.sql` | questpc-pc2 |
| `1293_quest_npc_item_sources.sql` | questpc-pc2 |
| `1294_quest_stateless_paid_receipts.sql` | questpc-pc2 |
| `1295_quest_paid_sale_mint_occurrences.sql` | questpc-pc2 |
| `1296_fathers_burden_xp_binding.sql` | questpc-pc2 |

The former lane files1005/1006/1007/1008 now occupy1290/1291/1292/1293.
Former1096 payments is the exact shared1000 file. Former1097/1098 paid-sale
extensions now occupy1294/1295 and delegate non-monetary cases to the shared
proofs. The local Father XP-variable ambiguity fix lives only in1296;
shared1205 remains unchanged. Father's eleven-track write remains atomic.

Live Rust filename references and schema-ID assertions use this registry.
Historical raw PG logs retain their original version numbers and checksums;
they describe the tested historical commits, not the current migration list.

Validation: staged-tree duplicate check, embedded sqlx migration identity,
shared hashes, six actual PostgreSQL18.6 cases, fmt, both-crate cargo check and
strict all-target/all-feature Clippy PASS. Full proof cloud-migration-registry-validation.json.
No checksums or world pins are edited by hand.
