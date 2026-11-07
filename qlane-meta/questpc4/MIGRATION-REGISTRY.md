# Questpc4 migration registry

Coordinator assignment: questpc4 owns1250-1269; next free1259. Main owns through0998 and1095. QENGINE2 owns0999-1006; never create lane variants. QBASE owns1200-1209. Its listed files are absent in this checkout. Do not split the Father's Burden11-track operation if imported later.

Nine lane migrations were moved with `git mv`; SQL bytes and therefore SQLx content checksums are unchanged. The embedded migrator regenerates version/name metadata from the renamed files. PostgreSQL qualification compares all nine installed versions/checksums against that owner and refuses obsolete versions.

| Previous | Current | Unchanged SQL SHA256 |
|---|---|---|
| 1060_floor_assembly.sql | 1250_floor_assembly.sql | 42fbd6a8f2240a311084fb3e7d492fa7f7c8c5ef66854d3f64936b01736933bc |
| 1061_reward_container_fluids.sql | 1251_reward_container_fluids.sql | c7c6de7ab084a241b2fa6d2105212a3699c22de16453e0459edce2328a5123d8 |
| 1062_floor_transform.sql | 1252_floor_transform.sql | 08c7802570eb31362fc25bd92fc7a63855cbfe21096eb66ede469b70d4fd1ffa |
| 1063_floor_transform_expiry.sql | 1253_floor_transform_expiry.sql | 906ab935283d9ff47487edced287e3b11dc70a95a37450cd2b2f93acf98fcacc |
| 1064_quest_reward_completion.sql | 1254_quest_reward_completion.sql | afd7a287ab9417e02a7b3215cd4478790ee8139ed634fb3cdab8e581592f9b0c |
| 1065_carried_step_branches.sql | 1255_carried_step_branches.sql | bbd022eaba15376c453656baaebc9bfc93c845fa4b75f7dd50fe216c2fbdf4bc |
| 1070_remaining_tomb_carried_recipes.sql | 1256_remaining_tomb_carried_recipes.sql | 6b4e9ae12137c2544e437f0d16161b1761686b90df982fa012001f826a45b5a4 |
| 1071_shared_world_switch_leases.sql | 1257_shared_world_switch_leases.sql | be638f6ca262e5e36fa32c685f9f959b4a2edd3686d3817eed52078b4be75e05 |
| 1072_quest_source_expiry.sql | 1258_quest_source_expiry.sql | af4b339c3f71f5a3bef88f605da6c5bb04b0663997229c0de6aad29e4aa7f925 |

QENGINE2 reference69609ef781c4e66d9c25fa61392c1ec0b6680033: local0999_quest_reward_callbacks,1000_quest_reward_payments,1001_quest_reward_participants,1002_quest_reward_state_operations are byte-identical.1003-1006 are not present or consumed yet; no variants exist. Fresh main4508fd2f1 has0998 and1095 but no0999-1006, so the post-PR3020 main-merge condition has not occurred. Recheck before publication and future batches; once integrated, merge main and retain only its canonical shared files. Any lane additions belong in1259 onward.

The owning world pin-refresh passed with unchanged f970b97d1976c5b587f04448aac5828729c0819dd76b113a66a5bc196101dbd7. No pin is edited manually. Filename references are updated; historical numeric prose in earlier trace records is history, not current allocation.

VerificationGREEN: seven actual PostgreSQL18.6 cases, including five checks of all nine installed versions/content checksums and zero obsolete versions; embedded-registry pureunit; durability/dispatch all-target/all-feature check and Clippy-Dwarnings; fmt, duplicate versions, diff. SQL hashes above reverified before commit. Fresh main4508fd2f1 at12:06UTC still lacks PR3020 migrations.
