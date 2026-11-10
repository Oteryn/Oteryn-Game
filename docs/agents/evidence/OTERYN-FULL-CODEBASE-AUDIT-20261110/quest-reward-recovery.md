# Bounded NPC → quest/reward → retry audit

**Base:** `e2e5ced306040113dc9c395251a802f50eb5776a`.
**Result:** No additional confirmed defect or reachable P1 established in this bounded pass. This is a static source result, not a native test pass or a claim that every NPC adapter is correct.
**Scope:** The shared `npc_command/reward_callbacks` → `quest_dialogue` production path, its quest-specific durable transaction, and the relevant same-session reconnect and quest-obligation refresh boundaries. The generic MINT implementation remains primarily owned by the durability audit lane.

## Concrete production flow examined

`connection/routing/npc.rs:11-54` decodes a command-7 talk intent and invokes the actual admission authority. `connection/authority/composed.rs:433-448` passes it through vocation/NPC handling; the selected branch in `npc_command.rs:713-823` requires a successful ordinary conversation, uses the current quest copy to select a shared callback, and invokes `quest_dialogue::apply`.

The callback selection is bound to NPC key, conversation topic, exact keyword, and eligible storage/parent conditions (`npc_command/reward_callbacks.rs:74-149`). The selected transition is server-authored. As a concrete retained row, `content/quests/shared-reward-callbacks.json:497-529` binds `oteryn:quest-reward/l1-wayfarer-cape` to Cranky Lizard Crone, `yes` at topic 1, storage range -1 through 0, a required item `i11701`, and the Wayfarer addon. Its committed storage becomes 1, leaving that eligible range. This row was inspected; the whole content catalog was not enumerated in this pass.

`quest_dialogue.rs:268-355` acquires the character revision slot, reloads the current session/character fence, checks the live actor, current conversation and content generation, and retains the exact transition request and `CommandRef` before awaiting the durable writer. `character_revision_sequencer.rs:459-486,852-915` substitutes the current revision cursor, retries a revision mismatch only once, and invalidates the cursor on unknown outcomes.

## Retry and reconnect counterguards verified

| Failure or retry point | Observed behavior and source |
|---|---|
| Failure before the durable attempt is retained | The caller returns rejection or unknown without claiming committed value. The input outcome remains subject to the connection's command sequencing. |
| Commit result is unavailable | `quest_dialogue.rs:453-455` keeps the pending request. Its `resume` path accepts only the same session and command cause (`463-482`). |
| The original transaction might still be completing | Recovery uses the same receipt advisory lock (`quest_state/storage.rs:68-96,324-341`), so an absent receipt is not inferred from an unlocked racing read. |
| The transaction committed but its acknowledgement was lost | `quest_state/writer.rs:535-549` finds the historical receipt and compares its request binding before fresh gameplay authority is checked. `storage.rs:301-321` excludes replaceable connection generation and character revision from the binding. Recovery returns history, not a new award. |
| Same-session connection replacement | `connection/serve/setup.rs:267-279` enables retained-operation reconciliation. `connection/serve/command.rs:188-211,248-264` resolves it before replacement dispatch; another unknown closes the connection without manufacturing a terminal result. `connection/authority/composed.rs:404-430` retries unavailable lookup under the command budget. |
| The conversation changed or ended after commit | `quest_dialogue.rs:182-195,496-524` preserves logical success while suppressing stale speech. It does not reopen the old conversation. |
| Quest refresh fails after a known commit | `quest_admission.rs:31-98` marks retry and fails quest actions closed when the copy is unavailable; it does not reverse the reward receipt. `composed_admission/quest.rs:18-60` retries on the owner cadence, preserving the deadline while the actor is in House. |
| A fresh login or resume needs outstanding quest work | `composed_admission/admission.rs:458-459` and `composed_admission/resume.rs:120-127` reload quest state. `quest_state.rs:424-531` re-requests pending quest and XP obligations using their original causes/occurrences. |

The direct NPC shared-reward route reconciles the **quest receipt**. It does not have to re-run the generic MINT reservation work budget merely to learn whether this quest transaction committed. The durability lane's broader generic-MINT budget hypothesis is therefore not established by this caller.

## Transaction boundary and value guards

`quest_state/writer.rs:506-549` starts one Type-2 transaction for a reward and performs historical receipt lookup. Fresh work then checks the durable gameplay fence, progression context, minimum level/Premium predicates and pending respawn (`564-629`). It locks and re-evaluates the relevant quest tracks and parent requirements (`660-747`); the conversation's cached quest copy cannot authorize value by itself.

The shared quest reward participant uses this transaction for negative/positive outfit predicates, quest participants, fee, required-input consumption, appearance grants, item MINT, achievement progress, and the source record (`reward_claim_mint/quest.rs:127-409`). A participant returning false causes an explicit rollback at `quest_state/writer.rs:904-906`. Quest revision, transition receipt, changed tracks, state, and any XP obligation are written before the same transaction commits (`981-1177`). These are source-level transaction composition facts; this audit did not execute SQL or prove deployed schema/role behavior.

For item rewards, `reward_claim_mint/quest.rs:470-601` preserves the callback claim key for a nonrepeatable reward. The distinct per-command key is used only for the explicitly repeatable paid-exchange case. `quests/src/quest/reward_callbacks.rs:195-202` requires that case to have no primary/participant storage changes, nonempty consumed inputs, and an item output. Capacity is checked before reservation/application. The generic item owner and its custody/claim internals were not independently re-audited here.

## Test evidence read, not executed

- `postgres_tests/nx6_wayfarer.rs:150-275` contains a real NPC finale fixture that refuses stale fences, forces achievement failure to test whole-transaction rollback, interrupts after commit, and calls the production recovery seam without repeating the achievement.
- `postgres_tests/dark_trails.rs:160-257` contains conversation-generation, stale-lease and retained-command checks, including rejection of a different command during reconciliation.
- `postgres_tests/qengine_rewards.rs:827-952,1040-1115` contains real reward rows, missing-input and capacity cases, exact item/appearance effect assertions, and a transport replay count assertion.

**Native tests executed in this pass: 0.** Reading existing tests provides regression locations and intended assertions, not evidence that those tests pass at this baseline.

## Remaining limits

This pass does not cover all legacy or specialized NPC adapters, the generic authored NPC-program interpreter, every content row, all quest kill/step/timer callbacks, the XP owner's complete SQL implementation, process-crash recovery on a different node, or concurrent live database fault injection. No new P1 is reported from this pass; the earlier GP-001 remains the gameplay lane's own confirmed finding. Exact source ranges are included in `gameplay-ledger.json`.
