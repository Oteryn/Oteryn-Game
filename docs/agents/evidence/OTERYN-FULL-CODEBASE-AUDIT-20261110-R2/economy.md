# Economy package audit

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: REVIEWED — all 28 tracked package files, 3,978 actual lines, including tests and manifest. Per-file hashes/ranges are in ledger.json. No production edits.

## Entrypoints and invariants

The crate consists of pure bank, guild bank, coin conversion, market, NPC trade, player trade and Store planners. Durable owners must supply current server facts; these planners alone never authorize account, Character, World or Channel mutation. Examined the relevant dispatch and durability adapters rather than accepting arbitrary helper parameters as a reachable request.

Bank and guild transfers use checked arithmetic, denomination/value conservation and closed amount/rank/junior rules. Output mint ordinals are checked before each nonempty output; max ordinal is valid only if no successor is needed. NPC trade fees consume eligible current default-state coins before a permitted bank shortfall; free trades do not inspect unused coins. Sell-all sums eligible complete stacks with checked arithmetic and performs the same exact output-room check. The actual writer validates item units, capacity 1..20 and identities; fences the character's World and Channel; locks occurrence/replay and the backpack entries in descending ordinal order; and compares current backpack definition before using the pure plan.

Coin exchange checks ratios and quantity overflow without charging failed converter use. The durable implementation traverses a bounded backpack tree, resolves current item stack definitions, fills existing output stacks, splits the remaining quantity across actual per-bag room and checks final total carried weight. Generic planner room booleans are therefore not the only capacity guard. Existing DUR-001 Inbox weight finding is not duplicated here.

Market quotes use current 2.5-percent fee, floor/min/max, checked amount×price and fixed expiry. Match settlement binds same World/key, opposite sides, different Account and maker ordering; conserved escrow/refund remains explicit for partial fills. Cancel/expiry and held credits preserve custody until the hard-cap credit can apply. Definition revision admissibility belongs to the durable owner. No prior 2-percent-reference divergence was misreported as a bug.

Player trade restricts inputs to direct pickupable leaf entries and checks receiver space and weight without crediting the outgoing item's future removal. Ready/transferring lifecycle prevents timeout/cancel from revoking a frozen transfer. Actual owners check live actor control, assignment, account/distance and current backpack snapshots; durable fences separately retain World and Channel identity and acquire ordered roots. Existing cross-module transaction reviews remain separate from this package credit.

Store quote/delivery helpers use deterministic typed effects, UUIDv7 request identities, a length-delimited semantic binding digest and same-request-ID retries after transport loss. The production catalogue admits only supported positive-priced offers; unlock/cosmetic ownership and one-purchase constraints are reserved durably before the Platform purchase call. Returned operations apply through a current gameplay fence and retain an outbox receipt when publication fails. The actual Platform intent contains account but no character/world target fields: its exact operation ID/kind are checked, while the purchase response binds the complete outgoing request digest (including requested target), request ID, currency/quantity/unit-kind/total. The durable delivery validates admitted character/world/session/account, repeats authority inside the value transaction, serializes operation receipt and rejects mismatching identity replays. A losing concurrent mint is rolled back and only the exact retained receipt is returned. The missing target comparison suspicion was refuted at the actual contract/caller boundary; no finding is asserted.

## Refuted hypotheses

- Guild withdrawal receiver cap is intentionally the BANK0 soft ceiling, despite the higher account hard ceiling used by disband returns; accepted GUILD0 section 5.2 explicitly requires this. It is not arithmetic loss.
- Bank recipient names with spaces are not rejected at the valid display-name layer: CharacterName comparison keys deliberately strip spaces and lowercase.
- Bulk coin output does not mint an oversized stack merely because pure qualification accepts one available slot; durable materialization splits by current stack maximum and rejects remaining unplaced output.
- NPC planner mint order does not depend on arbitrary caller ordering in production: locked SQL entries are explicitly descending.
- Store default quote view does not permit a duplicate permanent unlock purchase unchecked: current durable reservation gates cover those admitted product types.

## Findings and verification

No new confirmed P0/P1 or other package-local finding. All source and test branches were substantively analyzed; tests were inspected, not executed. No full Cargo, SQL, Platform or live game integration test is claimed. Existing standalone character source tests are unrelated and are recorded separately.

Remaining package files: none. External owner code beyond the exact dependency ranges in ledger.json is not credited as fully reviewed by this package report.
