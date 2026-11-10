# CL-004 — Store browser confirmation can describe a different purchase than its frozen retry

Status: **CONFIRMED P2 by independent lead source review.**
Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

## Directly inspected source and concrete trigger

1. A user confirms a Store purchase A. `crates/client-ui/src/features/0154-store/protocol.rs:120-138` freezes the complete encoded request in `confirmed_purchase`. A transport-level rejected or undecodable result releases pending but preserves those bytes (`186-200`, `234-246`). This is intended to permit retry of an uncertain outcome.
2. The same session opens the classic Store view (the new browser exposes this action at `apps/client/src/features/0154-store/native/commands.rs:27-30`). The classic controls still allow category reload/next page while a frozen purchase exists: `native/classic.rs:170-179` checks only `!pending`, may clear `catalogue`, and calls `store::catalogue`. UI `input.rs:42-49` similarly permits `select_category` while a frozen request exists. Accepted catalogue replies install the new catalogue B (`client-ui/.../protocol.rs:218-222`) without clearing A.
3. In the new browser, `confirmation_quote` immediately returns the current catalogue B whenever frozen A exists (`client-ui/.../protocol.rs:295-299`). It does not decode the frozen request or compare its offer/revision/price/currency with B. The browser stores this value as `displayed_quote` (`apps/.../native/browser.rs:260`).
4. The browser's actual purchase handler checks B against current B and opens a modal saying **Buy B for B's price/currency**, including B's revision (`apps/.../native/commands.rs:55-89`). Its `store_confirm_purchase` call rechecks B (`apps/.../input.rs:94-106`). The native helper invokes the Store purchase function with confirmation true (`apps/.../native.rs:5-36`).
5. `client-ui/.../protocol.rs:83-86` then emits frozen request A unchanged, before consulting current B. Thus the final modal's concrete terms and the emitted request disagree.

## Counterguards and severity limits

The new browser labels its button **Retry original purchase…** when A is frozen (`native/browser.rs:229-235`). The classic view itself uses an explicit original-purchase retry question (`native/classic.rs:196-208`), so its own confirmation does not have this mismatch. The problematic modal is the new browser after catalogue replacement through the classic controls (or the public category adapter).

`select_product` blocks changing a selection while a frozen purchase exists; ordinary browser-only browsing preserves the old catalogue. This narrows the trigger to a path that replaces `catalogue` despite `confirmed_purchase`. The catalogue snapshot comparisons prevent asynchronous B→C replacement during the modal, but they never compare B with A. The old operation id is retained, so this report does **not** claim a double debit under a new id, an authority bypass, a server quote-validation flaw, or confirmed loss of funds. The independently reviewable defect is inconsistent final confirmation text for a possible real coin purchase retry. A did receive a prior confirmation, which further bounds severity.

## Concrete nonterminal refusal path

Independent root scrutiny requested a real path that retains A while the admitted game session remains usable. This is provided by a temporary error from the subordinate Game→Platform Store API, rather than by closing the client's game transport:

- `crates/gameplay/native-admission-source/src/store.rs:337-365` maps HTTP429 with empty body and Retry-After to `TransportFailure::RateLimited` and HTTP503 with empty body to `Unavailable`. `StoreClient::request` at389-399 maps these to `ProtocolError::RateLimited` or `Transport`. Its `purchase` request at505-523 propagates the error.
- `crates/gameplay/dispatch/src/gameplay_transport/store.rs:359-364` maps every post-send error except local InvalidInput to `None`; `purchase_retained_or_new` uses that result with `?` when invoking Platform at493-503. The Store executor at187-195 maps missing result to `(CommandStatus::Rejected, empty payload)`.
- `connection/routing/store.rs:10-20` wraps this in the ordinary `Dispatch::Reply`, and `connection/serve/result.rs:196` copies its status/payload. The client Session `gated_command.rs:19-38` returns the decoded CommandResult as `Ok`; the appearance validator is a no-op for Store185. Thus the game session is not poisoned solely because the subordinate Store API temporarily returned429/503.
- The UI handles that Rejected result by clearing pending and retaining frozen A. Once the subordinate API recovers, classic catalogue reads use a separate balance operation (`dispatch/.../store.rs:259-302`, `native-admission-source/src/store.rs:553-572`) and may install B in the same session. No reconnect persistence assumption is needed.

These are directly inspected error mappings, not an assertion that an outage occurred in this environment. No fault injection was executed. Ordinary business refusals encoded as Accepted plus StorePurchaseCode clear the frozen request and are excluded from this trigger.

## Validation

Static source trace only. No native UI reproduction, Rust suite or SQL test was executed. The core state transition is directly visible: frozen(A), catalogue(B) → confirmation_quote(B) → confirmed purchase emits(A). The lead independently confirmed P2 with the counterguards above.

## Independent lead verification

The lead independently read the exact HTTP429/503 classification, purchase propagation and ordinary Dispatch::Reply, Session gated command and appearance validator, the actual client server-mechanic adapter/result loop, and both UI quote/request paths. A Store rejection returns Ok from Session and the native loop publishes it and continues (session_loop.rs125-153); ordinary decoded business refusals clear A and are excluded. The final B confirmation versus frozen A discrepancy is confirmed statically; the original A was previously confirmed and idempotency remains intact. No native Windows test or debit/loss incident is claimed.
