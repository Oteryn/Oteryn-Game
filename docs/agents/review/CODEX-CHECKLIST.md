<!-- Maintained by CP, who keeps this checklist updated. It is the review reference for the automated Claude PR review (.github/workflows/claude-review.yml) and for pre-push self-review. -->

# Pre-push self-review (D1456): catch what Codex would flag, before the push

Run this on your full diff (`git diff origin/main...HEAD`) BEFORE the single push. Fix every hit in the same push.
Every item below is a class that Codex has actually flagged on Oteryn PRs (P1/P2).

## Persistence / SQL / migrations
1. **Unbounded reads.** Every SELECT/collect over players, worlds, rows, history, or a table cut needs a LIMIT, a page, or a proven small bound. (#2168 highscore_snapshot World cut)
2. **Role privileges in migrations.** A job or service role must be NOINHERIT with exactly the listed grants. Migrations must REJECT a pre-existing role that has extra direct privileges or memberships. Check both directions: grants made directly to the role and grants it reaches through a role membership. (#2168 0097)
3. **Lock before mutate.** Lock the rows (`FOR UPDATE`, in a stable lock order) before you delete or update rows that depend on them: ledger and skull rows, receipts. (#2203)
4. **Reject inconsistent inputs.** Empty consequences with a non-empty ledger, a missing receipt key, or an unknown cause: fail closed. Never treat them as a no-op. (#2203)
5. **Every new enum value or cause code needs a schema CHECK update**, plus a migration that adds it. (#2194 item_fee_burn causes)
6. **Idempotency and receipts.** Every value-moving write is keyed on a receipt. Include the epoch/reset id in the key when a reset can replay it (D1451). A retry must return the original outcome.
7. **Reconcile on restart/resume.** Every two-phase or external commit (Premium purchase, NPC bank, transfer) needs a resume/reconcile path that covers "committed, not acknowledged". (#2194)
8. **Migration safety.** Is it re-runnable? Do CHECKs drop or replace the right names? Do grants exist for every new table used by a runtime role?
9. **Validator matches the loader.** A Python or SQL validator accepts exactly what the Rust loader accepts: same fields, same bounds, same rejection of unknown keys.
10. **Indexes for paged twins.** A keyset or paged query and its count/next-page twin both use an index that matches their WHERE and ORDER BY.
11. **NULL in aggregates.** SUM, MAX and COUNT over an empty or all-NULL set: the code handles the NULL result (COALESCE, or an explicit None branch).
12. **Limits enforced atomically.** A cap (slots, stack, depot, offers) is counted and enforced in the same transaction or statement that writes the row, never in a read-then-write gap.

## Session / transport
13. **Deadlines.** Every DB or presence write on the connection path runs inside the existing deadline (e.g. 10 s) or a bounded timeout, never unbounded. (#2164 connection.rs)
14. **Do not drop fields when relaying.** Kind, flags and rank must survive every hop (relay → session → client). Add an assertion test. (#2164 chat_relay online/offline kind)
15. **A capability gate on both sides.** A server frame is sent only when the capability was offered AND accepted. Add a test for a client without the capability. (#2202 pattern)
16. **Reconnect races.** A session prepared before an epoch/publish change must not write after it. Check the generation or fence. (#2167 known risk)
17. **Every end path closes durable state.** Logout, kick, disconnect, timeout, death and server shutdown each close or hand off the durable row a session opened (presence, lease, open trade).
18. **Fence and save outcomes are mapped explicitly.** Every variant of a fence or save result (stale generation, conflict, not found, transient) has its own branch. No `_ =>` that turns a failure into success.
19. **Wire outcomes match the spec.** Every error, rejection and success code sent to the client is the one the protocol document names for that case.
20. **One capability offer.** The capability set offered at boot equals the set offered per session. A capability is advertised only when its data source is loaded and reachable.

## Wiring / completeness (owner rule)
21. **A production caller exists** for each new function. Grep: no new `pub fn` that only tests call. It is composed in serve.rs or node, and registered in the dispatch.
22. **e2e through the real dispatch** in a named test, run by a named CI job. Check that the job's filter actually matches the test path.
23. **Tests deleted or weakened:** none. No `#[ignore]`, no `cfg(any())`, no loosened asserts.
24. **Deploy templates follow config.** A new required config key or environment variable is added to every deploy template, example env file and compose file that starts the binary.
25. **No duplicate test targets.** A test file is compiled by exactly one test target (`tools/repository/check_duplicate_test_sources.py`).

## Hygiene
26. **clippy `-D warnings` on all targets, and fmt.** No `explicit_counter_loop` or similar left in test code.
27. **Protocol codes.** No removed or renumbered codes (E8002). New codes are registered in the protocol registry and docs.
