<!-- Maintained by CP, who keeps this checklist updated. It is the review reference for the automated Claude PR review (.github/workflows/claude-review.yml) and for pre-push self-review. -->

# Pre-push self-review (D1456): catch what Codex would flag, before the push

Run this on your full diff (`git diff origin/main...HEAD`) BEFORE the single push. Fix every hit in the same push.
Every item below is a class that Codex has actually flagged on Oteryn PRs (P1/P2).

## Persistence / SQL / migrations
1. **Unbounded reads.** Every SELECT/collect over players, worlds, rows, history, or a table cut needs a LIMIT, a page, or a proven small bound. (#2168 highscore_snapshot World cut)
2. **Role privileges in migrations.** A job or service role must be NOINHERIT with exactly the listed grants. Migrations must REJECT a pre-existing role that has extra direct privileges or memberships. (#2168 0097)
3. **Lock before mutate.** Lock the rows (`FOR UPDATE`, in a stable lock order) before you delete or update rows that depend on them: ledger and skull rows, receipts. (#2203)
4. **Reject inconsistent inputs.** Empty consequences with a non-empty ledger, a missing receipt key, or an unknown cause: fail closed. Never treat them as a no-op. (#2203)
5. **Every new enum value or cause code needs a schema CHECK update**, plus a migration that adds it. (#2194 item_fee_burn causes)
6. **Idempotency and receipts.** Every value-moving write is keyed on a receipt. Include the epoch/reset id in the key when a reset can replay it (D1451). A retry must return the original outcome.
7. **Reconcile on restart/resume.** Every two-phase or external commit (Premium purchase, NPC bank, transfer) needs a resume/reconcile path that covers "committed, not acknowledged". (#2194)
8. **Migration safety.** Is it re-runnable? Do CHECKs drop or replace the right names? Do grants exist for every new table used by a runtime role?

## Session / transport
9. **Deadlines.** Every DB or presence write on the connection path runs inside the existing deadline (e.g. 10 s) or a bounded timeout, never unbounded. (#2164 connection.rs)
10. **Do not drop fields when relaying.** Kind, flags and rank must survive every hop (relay → session → client). Add an assertion test. (#2164 chat_relay online/offline kind)
11. **A capability gate on both sides.** A server frame is sent only when the capability was offered AND accepted. Add a test for a client without the capability. (#2202 pattern)
12. **Reconnect races.** A session prepared before an epoch/publish change must not write after it. Check the generation or fence. (#2167 known risk)

## Wiring / completeness (owner rule)
13. **A production caller exists** for each new function. Grep: no new `pub fn` that only tests call. It is composed in serve.rs or node, and registered in the dispatch.
14. **e2e through the real dispatch** in a named test, run by a named CI job. Check that the job's filter actually matches the test path.
15. **Tests deleted or weakened:** none. No `#[ignore]`, no `cfg(any())`, no loosened asserts.

## Hygiene
16. **clippy `-D warnings` on all targets, and fmt.** No `explicit_counter_loop` or similar left in test code.
17. **Protocol codes.** No removed or renumbered codes (E8002). New codes are registered in the protocol registry and docs.
