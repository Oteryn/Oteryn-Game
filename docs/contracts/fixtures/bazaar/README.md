# PLAT3 bazaar evidence and conformance

Normative companion to `../../OTERYN_GAME_BAZAAR_AUTHORITY_V1.md`. Fixture JSON is documentation only;
wire string values exclude JSON fixture framing and its final LF. Hash exact UTF-8 wire
strings, never pretty-printed parsed objects. Synthetic identifiers are not deployment trust.

## 5. Projection watermark, recovery and resync

Policy/status use PP's full-subject snapshots, `(projection_epoch,projection_revision)` and
accepted/superseded responses, with proposal §7's exact fields. Status includes custody state,
last_operation_id and snapshot_hash; it is insufficient without the matching terminal receipt.
Revisions increase on each committed subject change. Lower pairs are superseded; equal pair
and different content is 409/quarantine, never overwrite. Revision jumps detect a missed subject
publication and require its full snapshot, not replaying an asset command. A jump is permitted
only because the replacement is complete. Full resync must also cover missing subjects.

Watermark is monotone per epoch: `complete_through <= observed_at`; no future observations
beyond 1 s uncertainty. It advances only after all policy, status and prerequisite receipts
through that cut are acknowledged, even when the outbox was empty. Publish at most every 10 s;
serve only while `Platform_now-complete_through+1 <= 30` seconds. Stale means unavailable,
never empty/eligible. A higher epoch invalidates all older evidence and suspends every adapter.
Full resync enumerates every policy world and every retained listing (including terminal
returns/deliveries), publishes complete snapshots and replays linked receipts under the current
floor. No watermark until the enumeration and outboxes are acknowledged. Unknown listing is
not evidence of cancellation; unmatched Platform listings stay unavailable for reconciliation.

On restore, use CC §5.4 generation-selected reopening, never wall-clock selection. A surviving
receipt republishes unchanged; erased operations are redecided from restored state with their
original TTL. Reconcile assets, bank holds and receipts under one proven recovery scope before
opening admission. No independent restore may leave delivery surviving while funding vanished.

| Failure | Required result |
|---|---|
| Restart before commit | No effect or receipt; consume same intent again |
| Restart after commit, before publication | Replay stored receipt/outbox, one transfer |
| Duplicate delivery versus return | One escrow revision winner; loser terminal stale-state refusal |
| Stale session/recovery fence | No effect; stale session is refusal, unreconciled recovery is PENDING/unavailable |
| Missing acknowledgement or timeout | Keep funds/custody fenced; requery/replay same operation |
| Rollback/flag off | Stop new work; preserve and drain/reconcile outstanding custody, never auto-release |

## 6. Error vocabulary, evidence and conformance

Existing CC errors remain authoritative: UNSUPPORTED, INTENT_INVALID/EXPIRED,
OPERATION_CONFLICT, OWNER_MISMATCH, CHARACTER_NOT_FOUND/IN_SESSION/NOT_TRANSFERABLE,
ACCOUNT_NOT_ELIGIBLE and HOUSING_NOT_PREPARED all retain their CHAR_CMD_ prefix.
Additional terminal result codes registered at next-free Platform integration numbers:
5008 CHAR_CMD_BAZAAR_STALE_REVISION; 5009 CHAR_CMD_BAZAAR_OBLIGATION;
5010 CHAR_CMD_BAZAAR_ITEM_NOT_TRANSFERABLE; 5011 CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE;
5012 CHAR_CMD_BAZAAR_HOLD_CONFLICT; 5013 CHAR_CMD_BAZAAR_PAYOUT_INVALID.
Insufficient gold/headroom and forbidden release map to HOLD_CONFLICT; unsupported attributes
or quantity map to ITEM_NOT_TRANSFERABLE; guild/market/cooldown blockers map to OBLIGATION.
Missing dependencies are PENDING, not terminal DELIVERY_NOT_COMMITTED guessed from absence.
New-key stale expectations reject; changed reuse never replaces the original receipt.

Registered BAZAAR_AUTHORITY_DECISION payload uses the existing EventEnvelope and
TransactionEventRef, receipt hash and intent digest. Exact receipts stay in the authoritative
decision store; the audit does not duplicate a possibly 270336-byte receipt. Retention is
PLAT3_AUTHORITY_AUDIT_RETENTION_V1; no names, credentials, payloads or account IDs in logs.
Private raw evidence is owner/staff restricted, never a public projection.
Fixtures: `fixtures/bazaar/decisions.json` (wire strings exclude fixture framing/newline).
Required producer/consumer qualification: duplicate and conflicting keys; offline/admission race;
all nine exact schemas; attribute/split conservation; delivery/return race; unfunded handoff;
headroom race; stale fence; crash on both sides of commit; receipt 404/409; watermark stall,
gap/full resync, higher epoch and restore. Hashes must be recomputed over exact wire strings.
No runtime conformance is claimed by these documentation vectors.

## 7. Rollout

Review/accept this candidate and CC v2 extension, lock actual merged SHA/digest, build Game
consumer/domain guards/outboxes first, then Platform receipt/verification adapters behind
explicit default-off flags. Shared exact-wire and interrupted-saga qualification precedes
separate testing/preproduction enablement. No production authority is granted.

Before producer implementation, register measured family limits through resource-limit fragments
(as PP §8 requires); existing ANL hard ceilings remain mandatory. Candidate bounds here do not
change runtime configuration or authorize a larger envelope.
