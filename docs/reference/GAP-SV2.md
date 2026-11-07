# GAP-SV2 — implementation choices (base a9db6a76)
1. I04 opens the existing text writer as a Type2 transaction; text, CommandRef receipt, PRESERVE_INSTANCE transform and audit commit together (WRITE-0 §5.2–5.3).
2. The content-qualified policy carries the fixed target; unknown, conflicting or writable targets fail closed, and MAIL retains its item-specific limits.
3. Replay resolves the original text receipt before consulting transformed definitions/custody; no second transform, revision or audit is written.
4. A same-XID transform proof extends the existing item guard chain; the audit contains definitions/identity/cause, never player text, its hash or writer name.
5. I05 reuses the bounded stage-indexed spell-field schedules and completion receipts for ordinary items, avoiding a parallel timer or a second item store.
6. Complete chains are frozen at creation; an immutable revision-indexed handoff carries the previous stage through existing owner-proven custody moves, and each due stage advances the same ItemInstance once.
7. This general source-absolute chain owner resolves custody afresh; ground and active/reconnectable Character container/equipment scopes execute, other storage retains pending stages for withdrawal; equipment budgets and continuous tile clocks keep their separate accepted owners.
8. Both mechanics share forward migration 0958; the client text dialog remains outside this follow-up.
Evidence: Crystal `201189f871d23597b82caf88fd611454a2ddefb8` (audit-pinned revision; OTS_HYPOTHESIS_ONLY), `src/game/game.cpp:4957–5018` (write-once) and `:13028–13041` (decay); accepted contracts WRITE-0 §5 and TIMED-ITEM-0B §4–§12 govern fencing, storage and audit differences.
