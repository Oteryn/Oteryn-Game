# L2 missing mechanics

Checked against qmech `e278b09a75f13e089d3e903bfb2ee399b1fdc9dd` plus this lane's M39 implementation.
This is a mechanic report, not a claim that every quest in the older packet is qualified.

M39 now implements `any_topic` in `npc_command/reward_callbacks/offer_topic.rs`,
inclusive inventory guards in `quest/reward_callbacks/inventory.rs`, and their
transactional enforcement in `durability/quest_world_callback/reward_inventory.rs`.
Scoped refusal replies are handled by `npc_command/quest_dialogue.rs`.
Into the Bone Pit is wired through the two `m39-stories.json` shards; both native
PG journeys passed (78.255105046 s / 78.718498797 s, PostgreSQL 18.6), including
canonical-writer rejection of a false loss claim and literal command replay.
Tomes of Knowledge now wires two NPC books, ten one-shot source chests, twelve
incremental hand-ins and additional paid XP books. Native PG stories passed
(86.111770788 s and 95.356697343 s); the source mission refusal is exercised
without injecting callback rows. Merchant/access consumer verification is not
a full-quest certification from these NPC-offer tests.

| quest | missing mechanic (server or client) | evidence (quest file and step, missing hook/table/command) | blocks fully or partly |
|---|---|---|---|
| Unnatural Selection | M40 implemented: negotiated eight-way STEP across server and native client. | `postgres_tests/m40_native_dance.rs::m40_native_diagonal_dance_full_sequence_and_replay`: PostgreSQL 18.6, 69.40 s; all sixteen source steps, four diagonals, x3 receipts, wrong-step reset, capability refusal, replay and one achievement. Existing source dance callbacks are reused. | Assigned engine boundary proved; final clippy/workspace/node-boot validation is pending. |

Current coordinator-assigned migration range: **2020-2029**; M39 uses **2020**
for the existing reward trigger's quantity-variable collision with the independent-output SQL alias.
