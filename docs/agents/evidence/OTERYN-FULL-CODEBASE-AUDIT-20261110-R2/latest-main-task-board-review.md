# Successor review: Task Board at 43f48d1

Status: **REVIEWED — source analysis; runtime qualification pending**.

This is a separate review of ten changed files at `43f48d1f3bf932bfaef6ca47334aa72117db6c81`, not an alteration of the full-audit input `4f0b6c62a789489fca6c37dd36d5917d096f328e`. All ten files were read in full: **5,495 lines**. The accompanying ledger records exact paths, complete ranges and SHA-256 digests. Inventory and equality checks alone do not supply substantive review credit.

## Contract and actual owners

The revised contract requires one durable Reroll Token for every bounty reroll. The former clock-based free reroll and its persisted `next_free_reroll_unix` field are removed. The historical wire scalar `daily_reroll_at` is retained and projected as zero. Daily Rewards and a completed bounty remain distinct token grant sources. The rules preserve checked balance arithmetic, bounded offers and lists, vocation/difficulty gates, current catalogue eligibility and immutable selected bounty rewards.

The pure action entry point clones board and balances before settling the committed reset period and applying the requested action. A refusal does not publish either clone. Reroll rejects zero tokens before generating replacements and returns one spent token as an effect; it does not debit a client projection. Bounty completion grants a token only after the rule-level balance limits pass. The legacy timer deserialization test explicitly checks that an old JSON field is ignored and is not re-emitted.

The actual unchanged dispatcher validates command kind, selected capabilities, session/connection/lease/scope identity, owner fence and current actor control, drains pending kill work and enters the existing revision sequencer. It passes the original payload and typed action to the durable owner. The durable writer loads the token balance under the current Character transaction, binds receipt identity to Character, cause, ordinal, original payload and typed action, and checks history before a new effect. The custody participant locks the token row, uses checked signed arithmetic and appends a linked token entry and source before publishing the new balance. Savepoint rollback covers participant refusal; the board, revisions, effects and immutable outcome are committed together. These facts were checked directly in `gameplay_transport/task_board.rs`, `durability/task_board/writer.rs`, `custody.rs` and selected `state.rs` ranges.

Eighteen owner/dispatcher/migration files were verified byte-identical between the audit input and this successor. Fourteen already have a full delegated owner review. That reuse is recorded in `latest-main-task-board-owner-equality.json`; it is not eighteen newly reviewed successor files. No SQL was executed.

## Projection and client

The server builds replies from the committed receipt view. The retired timer is always zero, including when an older stored view is decoded; the other board data comes from the immutable view. This review does not assert byte-identical wire output across an upgrade that intentionally changes the retired field. Elite/weekly extension data remains capability-gated, and the older display representation retains its existing grade compatibility rule without changing a frozen reward.

The native window displays the one-token reroll cost and current token balance. Weak mailbox ownership, bounded queued intents, checked one-based row conversion and pending-state handling remain visible in the complete file. The UI sends intents and renders server results; it does not become the token authority. The native play tests exercise the real protocol codec with a mock transport and do not constitute execution on Windows or against PostgreSQL.

## Tests and limitations

All 1,609 lines of the revised pure-rule tests and all 1,473 lines of the two changed PostgreSQL test sources were reviewed. The pure tests cover zero-token refusal at several timestamps, a funded reroll, old JSON compatibility, reward/overflow boundaries, weekly settlement, lists, talisman/shop rules, frozen rewards and Elite state. The PostgreSQL source exercises actual command dispatch, a committed token grant, a paid reroll, identical same-command replay and zero-token refusal without a value delta. Its Elite case explicitly seeds selected fixture state and uses the real claim owner to obtain a reroll token; the privileged SQL setup is not evidence of an ordinary player producer. Other fixture funding, deterministic rare draws and selected premium inputs are likewise identified as fixtures.

**No new test execution was performed for these ten files.** PostgreSQL, native Windows UI and full Cargo tests were not run. Existing test assertions are evidence of intended coverage, not successful runtime results. Live deployment policy, a database upgrade with historical receipts, production content qualification and the actual UI interaction remain unverified by this source review.

## Adjudication

No additional confirmed defect was established in this ten-file change. The paid-reroll change is consistent across the revised contract, pure rule effect, unchanged durable token participant, reply projection and visible client text. A clock-based free reroll is deliberately absent from the revised contract. Existing full-audit findings are neither closed nor reclassified by this limited successor review.
