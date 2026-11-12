# Oteryn Game Bazaar Authority v1

- Status: **CANDIDATE**, revision 1, PLAT3-CONTRACTS. Contract only; no runtime,
  migration, acceptance or enablement claim. Adapters remain off by default.
- Contract ID: `oteryn-game-bazaar-authority-v1`. Producer/authority:
  `Oteryn/Oteryn-Game`; consumer/orchestrator: `Oteryn/Oteryn-Platform`.
- Game baseline: `28c7d9ab7ff85baef47c9947554f623ff8b2a87c`.
- Parents: `OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` (CC) §§2–7, 15;
  `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` §§5, 10, 13, 16; public projections
  (PP) §§3–6; `game-events/v1/foundation.proto` (ANL envelope).
- Platform evidence, read at `efdd058839cca2b922391e9a8323236e94e78834`:
  `portal-rs/crates/bazaar/src/game/{game_wire,game_store,game_snapshot,game_projection}.rs`,
  `portal-rs/crates/bazaar/src/ports.rs`; proposal
  `docs/contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md`, revision 2, §§2–8.
  The proposal's older CC reference at `857f15f3` does not supersede current CC.
  Nine validated immutable shapes and local staging are preparation, not Game execution.
  Six private routes remain 503/no-store; Evidence methods still deny by default.

## 1. Authority and custody

Platform owns account policy, listings, bids, Wallet coins and saga decisions. Game owns
Character identity, item instances, world custody, bank gold and every authoritative effect.
No Platform SQL adapter writes Game state; Reservations methods perform no remote I/O inside
Platform transactions. Accounts::state is current transaction-local policy, never Game proof.

| Stage | Game truth required before Platform advances |
|---|---|
| Offer ready | Selected seller, listing/world, exact deposited item/quantity and attributes match a committed EscrowBazaarItem receipt and current escrow status; Character offers require a current locked snapshot |
| Escrow locked | Character actor ABSENT, no live lease, admission fenced and existing lifecycle `locked`; items removed from playable custody into durable escrow in the same commit as receipt/outbox |
| Delivered/transferred | Exact buyer and sale have a committed delivery or settlement transfer; inventory capacity preflight, local saga state or an HTTP 200 is insufficient |
| Cancelled/expired | Platform requests return/unlock explicitly; only committed return and correct current owner permit terminal cancellation. Listing expiry never releases custody or funding |

Evidence::{offer_ready,escrow_ready,delivered} read verified durable local records only,
matching all parties, asset, listing, world, sale, operation, receipt digest and current recovery
floor. Missing, stale, contradictory, unsupported or rebuilding evidence returns false/error.
Reservations::{lock_accounts,reserve,release,settle,complete} retain sorted account locks,
absolute holds and replay semantics. `complete` requires delivery evidence; a pending seller
credit cannot become spendable on intent issuance. Coin funding is Platform truth; GOLD
funding is Game bank truth. Neither side fabricates the other's balance or reservation.

## 2. Direction, wire and version allocation

All HTTP is Game-initiated mTLS, CC §3. There is no inbound Game listener and **no Bazaar
receipt endpoint**. This candidate allocates CC extension `contract_version: 2` for the nine
Bazaar variants, subject to owner acceptance under CC §15. Existing v1 Create/Transfer bytes,
issuer, outcomes and guards are unchanged. Command names ending V1 do not select a version.

The exact six route bodies, scalar encodings, order, bounds and responses of Platform proposal
revision 2 §§2–4 and 7 are incorporated at the Platform SHA above. They are immutable inputs
of this candidate, not a reference to moving Platform main. In particular:

| POST path (`/internal/v1/bazaar`) | Operation | Request/response bytes |
|---|---|---|
| `/commands/pending` | ListPendingBazaarCommandsV1 | 512/8192 |
| `/commands/read` | ReadBazaarCommandIntentV1 | 512/4096 |
| `/commands/reconcile` | ListBazaarCommandStatesV1 | 512/12288 |
| `/transferability` | PublishBazaarTransferabilityV1 | 262144/256 |
| `/status` | PublishBazaarListingStatusV1 | 2048/256 |
| `/watermark` | PublishBazaarWatermarkV1 | 512/256 |

Dedicated Bazaar purpose certificate, configured source namespace, no request-selected trust;
existing Character receipt identity is separately authorized for publication. Connect 1 s,
handshake 2 s, command exchange 3 s, ingestion 5 s. Poll 1 s; 429/503 backoff at most 10 s.
One consumer exchange in flight; bounded rescans after 8 full pages or 30 s, CC §4.1.
Pending backlog 24. All responses no-store. Disabled/misconfigured surfaces answer 503.
JSON has exact ordered members, raw UTF-8, no escapes/unknown/duplicates, depth at most 12.
Only proposal-explicit nulls are allowed. IDs are lowercase non-nil UUIDv7; counters are
canonical decimal uint64 strings; times are Unix seconds. No ChannelId or lease token crosses.

Intent order: `contract_version,command,variant,issuer_authority,issuer_decision_id,
source_revision,operation_id,listing_id,seller_account_id,world_id`, command members below,
then `issued_at_source,expires_at_source,audience`. Constants: 2, PLATFORM_BAZAAR,
OTERYN_PLATFORM_BAZAAR_AUTHORITY, OTERYN_GAME_BAZAAR_AUTHORITY respectively.

| Command | Exact command members in order (types/bounds: pinned proposal §4) |
|---|---|
| LockBazaarCharacter | character_id,expected_character_revision,eligibility_revision,snapshot_schema_version |
| BindBazaarCharacterTransfer | character_id,lock_id,expected_lock_revision,transfer_operation_id,transfer_binding_digest,purpose,sale_id,funding |
| UnlockBazaarCharacter | character_id,lock_id,expected_lock_revision |
| EscrowBazaarItem | seller_character_id,item_instance_id,expected_state_revision,quantity,transferability_revision |
| DeliverBazaarItem | escrow_id,expected_escrow_revision,buyer_account_id,buyer_character_id,escrow_receipt_digest,sale_id,funding |
| ReturnBazaarItem | escrow_id,expected_escrow_revision,seller_character_id,escrow_receipt_digest |
| HoldBazaarGold | buyer_account_id,buyer_character_id,offer_id,hold_purpose,amount,expected_hold_revision |
| ReleaseBazaarGold | buyer_account_id,hold_id,expected_hold_revision,release_reason,amount |
| PayoutBazaarGold | buyer_account_id,hold_id,expected_hold_revision,sale_id,gross_amount,fee_bps,fee_amount,seller_amount,delivery_operation_id,delivery_receipt_digest |

## 3. Idempotency, fences and decisions

`(issuer_authority,operation_id)` is the command identity, across variants, as CC §5.
Compare SHA-256 of exact intent bytes before expiry checks. Identical retries return the stored
receipt byte-for-byte (already-applied); changed bytes retain the first decision, mutate nothing
and alarm. No ALREADY_APPLIED wire outcome: accepted means COMMITTED; rejected means REJECTED;
already-applied means replay of that same terminal outcome. Source revision orders polling,
not execution authorization. Unseen attempts require `issued < expires <= issued+300` and
future skew at most 5 s. Expiry rejects only the attempt, never its pre-existing custody.

Assert current process incarnation and nonrollback recovery fence before decision/publication.
Take account portfolio locks by AccountId, then admission/Character roots by CharacterId
(CC §6.3); asset/bank locks follow their owning domain's deterministic order. The implementation
must prove that order jointly before activation. Serialize receipt identity, expected revisions,
asset custody, admission and domain writes in one Game transaction. Commit effect, immutable
receipt, evidence and pending outbox together; transient dependency failure rolls everything
back and leaves PENDING. A definitely rejected attempt stores a terminal receipt, no effect.

Lock checks seller ownership, no live lease/actor, Character revision, current eligibility,
housing CC §6.4 and resolved guild/market/mail/depot obligations. Unknown eligibility denies.
The lock blocks admission and mutation through every Character writer, including rename,
delete and transfer. Bind authorizes exactly one existing TransferCharacterOwnership intent
by its entire byte digest, operation and purpose. Under the same admission serialization,
the v1 transfer may cross this particular `locked` state only with that binding; all other
CC §6.2 guards, capacity, housing proof and session-generation fencing remain mandatory.
It preserves the lock through seller → allocated escrow account → buyer/return. No generic
locked-state bypass. Unlock requires no executable/ambiguous bound transfer and the verified
intended terminal owner; an expired transfer is not proof of a terminal decision.

Item escrow removes the exact instance or an explicit conserved stack split, preserving full
attribute-state hash, definition revision, quantity and unique custody. Nonstackable quantity
is one. Unsupported bound/timed/container/attribute cases reject, never normalize. Deliver and
Return serialize on the same escrow revision: exactly one can consume it. Destination is the
recipient's durable same-world Inbox, with current ownership checked under portfolio locks;
full carried inventory is irrelevant. If Inbox cannot accept exact state, reject unchanged.
A deleted/moved seller Character requires a fresh authorized destination on the original seller
account; otherwise keep escrow for owner resolution. Never give an asset to a different account
because the original destination vanished. Suspension alone must not prevent a safe return.

GOLD uses the one Account+World bank authority: held <= total <= 999999999999. Hold replaces
an absolute maximum (expected revision zero creates); no inventory/guild/other-world funding.
Reserve seller incoming headroom atomically before handoff; every ordinary bank debit/credit
must respect holds/headroom. GOLD delivery/binding fences its exact hold against release until
a durable no-late-commit refusal or matching payout. COINS requires Platform to fence its Wallet
reservation before listing the handoff intent. These are sagas, not distributed ACID.
Payout proves Game's own committed delivery (Character settlement, never listing escrow),
matching buyer/seller/world/sale and current generation. Checked wide arithmetic:
fee = ceil(gross*250/10000), net = gross-fee > 0; buyer debit = seller credit + burn.
Debit, credit, burn, receipt and ledger commit once. Excess requires explicit Release.
The fee/headroom policy is a candidate owner decision, not existing bank implementation.

## 4. One receipt facility and verifiable evidence

TransferCharacterOwnership retains exactly CC §4.3's v1 committed/rejected receipt, including
both account projection revisions and recovery_generation. Platform settles that leg only on
COMMITTED CHAR_CMD_OK. The existing `/internal/v1/game-auth/character-commands/receipt` receives
PublishCharacterCommandReceiptV1; it is not one of the six Bazaar routes.

For the nine new variants, this candidate defines the CC §15 v2 exact receipt member order:
`contract_version,operation,source_authority,command,operation_id,outcome,result_code,
result_data,recovery_generation,decided_at`. Version is 2; operation is the existing publication
name; outcome and result codes use CC semantics. REJECTED has present `result_data:null`.
COMMITTED has CHAR_CMD_OK and exactly the corresponding record from pinned proposal §5:
CharacterLockFacts, TransferBindingFacts, UnlockFacts, ItemEscrowFacts, ItemMoveFacts (Deliver
and Return), GoldHoldFacts, GoldReleaseFacts or GoldPayoutFacts. That table's exact members,
types and order are normative here. There is no generic arbitrary object and no v1 field change.
Receipt ceiling for this extension is 270336 bytes (signed snapshot up to 262144 plus framing);
rejected receipts remain <=1024. Platform's current route must be extended and qualified for
this issuer/version/bound before dispatch; existing preparation does not imply support.

`receipt_digest = SHA-256(exact stored receipt bytes)`, lowercase hex, binds the immutable
result_data and intent by durable operation identity. Game's authenticated publication and
retained ANL record make this hashed evidence verifiable; a caller-supplied hash alone is not
proof. CharacterLockFacts additionally contains the signed snapshot: pinned proposal §6's
exact SignedSnapshot/CharacterSnapshot schema, SHA-256 and Ed25519 domain
`oteryn-v2-bazaar-character-snapshot-v1.` plus unpadded base64url(snapshot bytes).
Verify provisioned current/retiring kid, signature, hash, listing/lock/seller/world/revisions
and current trust; no embedded key or signing/admission key reuse. Unsupported snapshot tabs
remain explicit null, not invented values. Key provisioning is an activation blocker.

Platform never pulls an inbound Game receipt endpoint. Its adapters query their own durable
receipt/projection store; Game pulls `/commands/reconcile` and republishes its retained receipt
when Platform reports PENDING/REOPENED. The existing Character receipt route must store the
new issuer's receipt against the exact Bazaar intent, atomically with its local terminal state.
A lost publication is retried from Game's outbox. Publication 200 accepted means stored, not a
new delivery. 404/409 retains evidence, stops that key and alarms; no inferred rejection.

## 5. Evidence and conformance

The normative companion [fixture specification](fixtures/bazaar/README.md) fixes
projection/recovery, registered errors, conformance and rollout.
No runtime conformance or enablement is claimed.

## Open questions for owner

- Accept obligations/cooldown/snapshot survival and the escrow AccountId allocation/capacity;
  unresolved facts deny locking rather than inventing eligibility.
- Accept Inbox destination coverage, exact supported item classes, fee/headroom policy and
  common bank/item/Character restore scope. Existing code is not proof of these new interlocks.
- Assign signing-key custody/rotation/revocation and finite financial-proof retention; the
  proposed audit ceiling does not authorize deleting unsettled authoritative receipts.
- Accept the v2 result_data extension and route bounds, then supply the real review PR number
  for the pending lock entry. No PR/merged canonical SHA is fabricated in this docs-only task.
