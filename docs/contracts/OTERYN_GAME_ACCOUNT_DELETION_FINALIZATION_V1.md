# Oteryn Game Account Deletion Finalization v1

- Status: **CANDIDATE**, revision 1, PLAT3-CONTRACTS; contract/fixtures only.
- Contract ID: `oteryn-game-account-deletion-finalization-v1`.
- Producer/authority: Game for Characters and Game admission closure; Platform for web account
  lifecycle, cancellation, credentials and `finalized_at`. Game producer defaults ON by owner decision 2026-11-06; Platform qualification remains separate.
- Game baseline: `28c7d9ab7ff85baef47c9947554f623ff8b2a87c`.
- Parents: `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` §§5, 8, 11, 13;
  `OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` (CC) §§2–7, 15;
  LCFA and `OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` (PP) §§4–7;
  `game-events/v1/foundation.proto` and its registry (ANL).
- Platform evidence read at `efdd058839cca2b922391e9a8323236e94e78834`:
  `portal-rs/crates/identity/src/{termination,termination_tests,account_lifecycle_http}.rs`.
  Boundary at termination.rs:99 has authorize/check_blockers/revoke and returns Result<()>;
  it is not presently a Game close/proof protocol. Request/cancel already check sudo/current
  password. The request and final worker reject wallet reservations, open Bazaar listings,
  pending email changes and any account_characters row. Tests require unknown dependency
  denial, revocation rollback, repeated request preserving deadline, one finalization and one
  winner in cancel/worker races. This candidate does not claim those files are implemented.

## 1. Handshake and cancellation window

Use **detach then final erasure**, so cancellation remains possible until Platform's worker
commits finalization, including after the grace deadline (current termination.rs semantics).
All network work runs outside Platform account-row locks.

1. Platform records an authorized termination_id and deadline, revokes credential families,
   increments its security generation and serializes all lifecycle decisions on the account.
   This does not authorize Game destruction during the cancel window.
2. Once due, an asynchronous adapter stores one immutable CloseAccount intent for that
   termination/generation. Game pulls it in the CC direction. Game closes admission, kicks
   sessions, and detaches every Character into private reversible quarantine. The proof means
   **zero account-owned playable Characters and Game account closed**, not physical erasure.
3. Game publishes a terminal CC receipt and the complete private account-closure snapshot.
   Platform stores both, verifies hashes, generation and LCFA convergence, then rechecks its
   blockers and pending termination under the account lock. Boundary::revoke(finalizing=true)
   may return Ok only using this verified local proof; it must not perform remote I/O there.
4. In that same Platform transaction, the worker closes credentials/account, sets finalized_at,
   and enqueues ResolveAccountClosure with decision FINALIZE. A concurrent cancellation uses
   the same lock: either cancellation wins and the worker cannot finalize, or finalization wins
   and cancellation gets NoPending. No window ends merely because Game detached the Characters.
5. If cancellation wins, it atomically invalidates the proof for local use, advances the
   lifecycle generation and enqueues ResolveAccountClosure with decision CANCEL. Game restores
   quarantined bindings/obligations only after that authenticated decision. Until acknowledged,
   Game remains closed and UX says restoration pending. No old session/grant is resurrected.
6. FINALIZE permanently retires quarantined Characters, releases names through accepted
   lifecycle rules and starts physical erasure under domain retention. CANCEL and FINALIZE are
   mutually exclusive for one termination. No later command may reverse FINALIZE.

The present `any account_characters row` guard is retained until a separately implemented
adapter consumes current complete LCFA snapshots and removes detached rows. Unknown/stale
rows remain blockers. Request-time rejection of accounts with Characters is unchanged; closing
an existing Character portfolio needs a separately accepted request-gate change. The handshake covers Characters acquired after request. This contract grants no bypass of current wallet/Bazaar/email blockers.

## 2. Immutable commands and identity

This is a further CC §15 candidate extension within version 2, sharing its operation identity,
not a new command transport. Extend the existing Character pending/read/receipt routes for CloseAccount and ResolveAccountClosure under the
existing dedicated Character identity/issuer. Keep finalization pending until exact version/variant support is qualified.

Exact intent order for CloseAccount:
`contract_version,command,variant,issuer_authority,issuer_decision_id,source_revision,
operation_id,account_id,termination_id,lifecycle_generation,expected_recovery_generation,
scheduled_for,issued_at_source,expires_at_source,audience`.
Constants: 2, CloseAccount, PLATFORM_ACCOUNT_CLOSE, OTERYN_PLATFORM_CHARACTER_AUTHORITY,
OTERYN_GAME_CHARACTER_AUTHORITY. ResolveAccountClosure uses command ResolveAccountClosure,
variant PLATFORM_ACCOUNT_CLOSE_RESOLUTION, and adds `close_operation_id,decision` immediately
before scheduled_for. Decision is exactly CANCEL or FINALIZE. Every member is required/non-null.
The fixtures give exact wire bytes.

Use CC exact member validation/encoding: no unknown/duplicate/null members, raw UTF-8 without
escapes/whitespace, canonical lowercase non-nil UUIDv7 IDs, canonical positive decimal uint64
strings for generations/revisions and Unix-second times. Max intent 4096 bytes, depth 3;
pending/read limits, cadence, identity, mTLS timeouts and rescan remain CC §§3–4. scheduled_for
is the original Platform deadline, not attempt expiry. A fresh Close/FINALIZE cannot execute
before it. CANCEL may execute before it. Attempt TTL <=300 s, future skew <=5 s. An expired
unexecuted attempt gets CHAR_CMD_INTENT_EXPIRED; fresh attempts use fresh operation IDs bound
to the same termination. Idempotency binds exact bytes at `(issuer_authority,operation_id)`.
Same bytes replay before checking expiry; changed reuse never overwrites the first receipt.

lifecycle_generation is a monotone Platform per-account lifecycle fence, distinct from native
security/session/recovery generations. Close is issued at generation G. Exactly one resolution
is issued at G+1 under Platform's account lock; no wrap. Game accepts a resolution only against
that termination's stored close at G, records its exclusive decision, and rejects late Close G
once resolution G+1 is known. If CANCEL arrives first, store the cancellation tombstone at G+1;
subsequent Close G cannot detach anything. FINALIZE arriving without committed Close is PENDING,
not guessed safe. A fresh termination requires a higher generation and a new termination_id.
Current Platform lifecycle/security evidence is required before a first effect; missing evidence leaves PENDING. Game asserts expected_recovery_generation equals
its current reconciled generation before any new work; stale expected value is terminal
CHAR_CMD_ACCOUNT_CLOSE_STALE_GENERATION. Reissue only after reconciliation, with a new key.

## 3. Offline fence, bounded work and domain consequences

Take the account portfolio lock first (CC §6.3). Mark closing, block create/transfer/admission
for this account and persist the full portfolio manifest (at most LCFA's 64 Characters).
For each Character, serialize on its lease/root, revoke the live admission/session generation,
kick the controller and wait for durable actor ABSENT/no live lease. A kick request/timeout is insufficient. Old writers stay fenced; detach only confirmed durable state.

A durable resumable job processes at most one Character per transaction, with per-domain
idempotency `(close operation,CharacterId,step)` and its owning fences. No terminal success
until every step is accounted for. Each partial commit remains private, closed and quarantined;
it is never a completed proof. Resuming cannot repeat a debit, refund or asset movement.
An unavailable owner stops the step, not skips it. Portfolio writes in other flows must honor
the closing fence, so no late creation/incoming transfer escapes the manifest.

| Domain | Required disposition before proof |
|---|---|
| Character root | Detach owner linkage from active portfolio, quarantine original binding for CANCEL; no playable/queryable Character remains under account |
| House/residence | Owner-domain reversible prepare and content fence, per CC §6.4/EXP-HOUSES-01; quarantine custody/tenancy rights. No silent donation, auction or disposal during cancellation window |
| Guild | Quarantine membership, invitations and leadership under guild revision; freeze affected leadership/disband timers. If reversible successor/obligation handling is unavailable, keep PENDING. No silent leader succession or disband payout |
| Market/Bazaar | Block new work; executable/ambiguous deliveries, escrows, bids, holds and settlements are blockers. Finish their owning sagas and prove no late commit before detaching; no timeout refund |
| Depot/stash/Inbox/inventory | Preserve exact instance/quantity/attribute/custody and pending delivery references in quarantine; disable claims. Never mint substitutes or drop assets into the world |
| Bank/account entitlements | Not transferred to another account; unresolved financial obligations block. Permanent disposition requires owner-approved domain policy before FINALIZE |
| Public/LCFA | Publish complete empty account portfolio, profile/deaths tombstones and absence from guild/house/online/highscores as applicable. Retain public name-handoff safeguards; no account linkage in public exports |

CANCEL restores only quarantined facts under current ownership/content fences, publishes fresh
revisions and permits fresh admission after completion; it never reuses old session generations.
A restoration conflict remains closed for explicit owner repair, not partial login. FINALIZE
requires all owning domains to have accepted final disposition before it can be COMMITTED;
physical erasure may follow without reopening the Game account. No unimplemented domain rule
is assumed to succeed. Names stay reserved during quarantine; cancellation cannot steal a name
that another account legitimately acquired. Account IDs/Character IDs are never reused.

## 4. Receipt, closure proof and requery

Use CC's single PublishCharacterCommandReceiptV1 on its existing receipt route. v2 exact order:
`contract_version,operation,source_authority,command,operation_id,outcome,result_code,
result_data,recovery_generation,decided_at`. COMMITTED uses CHAR_CMD_OK;
REJECTED uses null result_data and a terminal code. No partial success.
Close result_data is exactly, in order:
`account_id,termination_id,lifecycle_generation,closure_state,detached_characters,
manifest_sha256,projection_epoch,projection_revision,proof_sha256`.
closure_state is CLOSED_DETACHED; detached_characters is decimal 0..64 (zero allowed).
manifest_sha256 binds the ordered quarantined Character/disposition manifest, not an empty
query result. The LCFA pair is the complete empty portfolio revision committed by the last
close transaction. Resolve result_data has the same fields, closure_state FINALIZED or
CANCELLED, and references the resulting complete LCFA portfolio pair. The count/hash still
bind the original detached manifest, including for CANCELLED. Receipt bound 2048 bytes.

proof_sha256 is SHA-256 of the exact canonical result_data object with the proof_sha256 member
omitted; receipt_digest is SHA-256 of the whole retained receipt. A hash is accepted only from
the authenticated Game publisher, bound to the exact immutable intent and current recovery
floor. Closure state, proof, receipt, audit and outboxes commit together; names stay private.

Private projection operation PublishAccountClosureV1 uses a new Platform ingest route
`POST /internal/v1/game-auth/account-closure` under the Character identity. Exact member order:
`contract_version,operation,source_authority,account_id,projection_epoch,projection_revision,
source_observed_at,closures`. Version 2; closures is 0..1 records ordered as:
`termination_id,operation_id,lifecycle_generation,closure_state,receipt_digest,proof_sha256`.
Its projection pair is separate from LCFA. Max 2048 bytes, depth 4, responses/ordering PP §§4–6 (version echoed as 2).
An empty list withdraws usable proof; it never certifies closure. Higher epochs invalidate
old proof. A dedicated account_closure family uses PP's watermark shape/503 stale behavior,
version 2, on `POST /internal/v1/game-auth/account-closure/watermark`; max 512 bytes.
Watermark every <=10 s, uncertainty 1 s, S=30 s; it cannot pass unacknowledged closure/receipt
or LCFA changes. These are new candidate routes, not claims of existing Platform handlers.

Platform re-queries its local durable projection/receipt by account, termination and operation.
Game's CC pending rescan republishes a stored terminal receipt until acknowledged. A missing
receipt does not mean an absent close; no finalized_at until receipt/hash, current closure
snapshot, current lifecycle generation and LCFA empty pair all match. Request a full Game
outbox resync on a revision gap/higher epoch: enumerate closed and cancelled subjects,
republish full snapshots and linked receipts before advancing watermark. No page-render call
to Game. Equal pair/content duplicates accept; unequal content quarantines/409.

## 5. Evidence and conformance

The normative companion [fixture specification](fixtures/account-deletion/README.md) fixes
recovery, retention, conformance and rollout.
No runtime conformance or enablement is claimed.

## Open questions for owner

- Accept reversible quarantine and domain-specific restoration/final disposition, including
  freezing guild/house side effects. If any domain cannot do it, closure stays pending; no
  shorter cancellation window is silently substituted.
- Should request-time any-account_characters blocking be relaxed later? It remains enforced
  in the pinned Platform behavior and needs its own explicit product decision.
- Choose physical erasure and minimal authoritative-proof retention, resolve account bank
  funds and entitlements, and assign operator handling for unresolved quarantine conflicts.
- Accept CC v2 lifecycle variants/private projection routes and provide the actual review PR
  number before the pending lock can become validator-complete or canonical.

## Game producer clarification (2026-11-06)

The owner selected default ON for Game, with an explicit disable switch. See
[CONTRACT-NOTES / PLATFORM CONFIG](fixtures/account-deletion/CONTRACT-NOTES.md)
for the independently authenticated lifecycle-read extension, conservative configured
owner policies, private quarantine backing and exact Platform fixture/config handoff.
This supersedes candidate default-OFF wording for Game only; it does not claim
Platform acceptance or deployment qualification.
