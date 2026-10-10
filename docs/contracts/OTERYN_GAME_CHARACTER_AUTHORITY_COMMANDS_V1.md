# Oteryn Game Character Authority Commands v1

- Status: **Candidate**, revision 1 (GAME-CHAR-CMD-0), 2026-10-08. Not accepted. It authorizes no
  implementation, no Platform change and no enablement until the owner accepts it after the
  independent review of the frozen head.
- Contract ID: `oteryn-game-character-authority-commands-v1`
- Coordination: #1622 (plan comment 6056637549, packet P4, inventory rows C7 and C8);
  Oteryn/Oteryn-Platform#1476.
- Owner rulings:
  - **D963 Q2=A:** Platform keeps Character creation and Bazaar ownership transfer disabled until
    this contract and its Game implementation exist. No temporary migration bridge (boundary §11)
    and no Canary write.
  - **D965 Q5=A:** a player creates a Character on the Platform account web page, which calls the
    Game `CreateCharacter` command. The Game client Character list has a "Create character" entry
    that opens that page (§9). The client has no creation form.
- Producer and authority: `Oteryn/Oteryn-Game` Character Authority. Consumer and orchestrator:
  `Oteryn/Oteryn-Platform`.
- Parent contract: `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` (accepted) §5
  (mutation family), §6 and §6.1 (creation, CHAR-NAME-1), §10 (Bazaar ownership transfer), §13
  (failure classes) and §16 (conformance). This document fixes the command envelope, transport,
  idempotency, receipts and result codes that §5 and §15 left to an owning later contract. It
  changes none of the boundary's semantics.
- Related: `docs/architecture/reviews/OTERYN_CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_DECISION_2026-09-23.md`
  (intent model; this contract is the separate explicit contract that decision reserves for
  Platform user creation); `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md`
  (LCFA, the read side); `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`
  (CharacterLease and admission);
  `docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md` §3 ruling 10
  (restore reconciliation, §5.4); `docs/architecture/EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md`
  §§7, 13, 14.7, 22.2 (Bazaar housing disposition, §6.4).

## 1. Scope

Two commands of the boundary §5 family:

| Command | Platform caller | Boundary |
|---|---|---|
| `CreateCharacter` | account web page, for the authenticated account owner (D965) | §6, §6.1 |
| `TransferCharacterOwnership` | Bazaar saga only, never a direct user request | §10 |

Not in scope: rename, deletion, restore, finalization, world transfer, the operator bootstrap
variant (unchanged), any Game code, migration or Platform change.

## 2. Model: Platform-authored intents, Game-initiated consumption

Platform never calls Game. Every exchange is a Game-initiated request to Platform over the
existing Game→Platform mTLS client, the same direction as the LCFA publication (LCFA §3) and the
operator bootstrap-intent read (`apps/game-server/src/native_admission_source/`). This contract
adds no inbound Game listener; LCFA §3 Decision P1 records that such a listener is a separate
architecture decision.

```text
1. Platform authorizes the user or saga request and stores one immutable command intent,
   keyed by a fresh operation_id, in state PENDING.
2. The Game command consumer lists PENDING intents       (ListPendingCharacterCommandsV1).
3. It reads each exact intent                            (ReadCharacterCommandIntentV1).
4. Character Authority decides it in one PostgreSQL transaction and stores a durable receipt.
5. The consumer publishes the receipt                    (PublishCharacterCommandReceiptV1).
6. Platform stores the receipt, marks the intent terminal and continues its UX or saga.
```

- Platform owns the user authorization, the Bazaar saga, the intent and its PENDING/terminal
  state. Character Authority owns the decision, the receipt and every Character write (boundary
  §1, §11).
- An intent stays PENDING on Platform until Platform holds its receipt. Platform never marks an
  intent failed on a timeout, an expiry or a missing receipt (boundary §10: a timeout proves
  nothing). Game always eventually publishes a terminal receipt for every intent it can see (§6).
- Delivery is at least once in both directions; duplicates are harmless (§5).

## 3. Transport and identity

HTTP/1.1 over TLS 1.3 with a client certificate, through the existing `http1_mtls::exchange`
client, exactly as LCFA §3.

- **Identity:** a dedicated character-command client certificate, held only by the command
  consumer on Character Authority hosts. It is separate from the native-evidence,
  runtime-status and projection identities; a gameplay node that is not a Character Authority
  host holds none. Platform accepts the three routes below only from its configured
  character-command identities and refuses those identities on every other route.
  - Testing and preproduction follow LCFA U-LC6: one identity per Character Authority host,
    issued from the per-stack development CA, configured by certificate subject. Production PKI
    stays open (U-CC5).
- **Issuer trust:** Game accepts intents only from the configured issuer
  `OTERYN_PLATFORM_CHARACTER_AUTHORITY` over that authenticated channel. No request field can
  select or broaden trust.
- Closed operations with compiled paths, all `POST`:
  - `ListPendingCharacterCommandsV1` → `/internal/v1/game-auth/character-commands/pending`;
  - `ReadCharacterCommandIntentV1` → `/internal/v1/game-auth/character-commands/read`;
  - `PublishCharacterCommandReceiptV1` → `/internal/v1/game-auth/character-commands/receipt`.
- Bounds: connect 1 s, handshake 2 s, exchange 3 s; a read request at most 448 bytes (it carries a
  `issuer_authority` of up to 128 bytes and a `command` of up to 64, 402 bytes at most), a
  pending-list request at most 512 bytes and a receipt at most 1024 bytes (the largest receipt,
  a committed transfer with 20-digit counters, is 699 bytes); a pending-list
  response at most 8192 bytes (a full page of 32 entries with 64-byte commands and 20-digit
  revisions is 5602 bytes); an intent response at most 4096 bytes; a receipt response at most 256 bytes; one in-flight exchange per consumer.
- Cadence: while enabled the consumer lists pending intents every 1 s, and backs off up to 10 s
  on `429` or `503`.
- The internal reverse proxy of a stack must route these three paths behind `internal-mtls`
  like the LCFA routes. Adding them to a stack is the enablement packet's change, not this one.

## 4. Wire

Common rules, as LCFA §4: exact member sets, no unknown, duplicate or `null` members, nesting at
most 3, raw UTF-8 without string escapes, no insignificant whitespace. Identifiers are canonical
lowercase non-nil UUIDv7 strings. Counters and times are canonical non-zero decimal uint64 strings
(times in Unix seconds); compare them as uint64.

### 4.1 Pending list

Request:

```json
{"contract_version":1,"operation":"ListPendingCharacterCommandsV1","source_authority":"oteryn:character-authority:primary","after_source_revision":"0","after_operation_id":"00000000-0000-0000-0000-000000000000","max_entries":32}
```

Response `200`, 0..32 PENDING entries whose key `(source_revision, operation_id)` is strictly
above `(after_source_revision, after_operation_id)`, sorted by that key ascending (`source_revision`
as uint64, then `operation_id` as bytes), unique `operation_id`:

```json
{"contract_version":1,"pending":[{"command":"CreateCharacter","operation_id":"0192a000-0000-7000-8000-000000000001","source_revision":"17"}]}
```

- PENDING means "no receipt stored on Platform", whatever the intent's expiry.
- Paging: the continuation key is `(after_source_revision, after_operation_id)`, because the two
  variants can share a `source_revision` (§4.2). `after_source_revision` is a decimal uint64
  string and the only counter that may be `"0"`; `after_operation_id` is the only identifier that
  may be the nil UUID. The consumer starts at `("0", nil)`, continues from the last returned
  entry's key, and starts again at `("0", nil)` after a response with fewer than `max_entries`
  entries. An intent that stays PENDING (a transient dependency failure, §6) therefore never hides
  later intents, including ones that share its `source_revision`.
- Rescan: §5.3 permits Platform to commit a lower `source_revision` after the consumer has passed
  it, and a continuous stream of full pages would keep the cursor from returning to that key. The
  consumer therefore runs a bounded rescan: after at most 8 consecutive full pages, or 30 s since
  its last rescan, whichever comes first, it lists again from `("0", nil)` and pages forward until
  it passes the key it had reached, then resumes from that key. A rescan lists only PENDING
  entries, so its cost is the number of unreceipted intents behind the cursor. Re-listing an
  intent is safe because the per-operation receipt makes deciding idempotent. The 30 s bound is
  well inside the 300 s intent TTL, so an intent that commits behind the cursor is read before it
  expires.
- `source_authority` is the configured Character Authority namespace (LCFA §4 rule).

### 4.2 Intent read

Request:

```json
{"contract_version":1,"operation":"ReadCharacterCommandIntentV1","source_authority":"oteryn:character-authority:primary","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","command":"CreateCharacter","operation_id":"0192a000-0000-7000-8000-000000000001"}
```

`issuer_authority` is a string of 1..128 bytes of ASCII uppercase letters, digits and `_`; Game
sends its configured constant. Platform keys intent reads by `(issuer_authority, operation_id)`,
the uniqueness scope of §5.1, and checks the requested `command` against the stored intent.
Response `200` is the exact stored intent bytes, byte-identical on every read. `404` means
Platform holds no intent with that `(issuer_authority, operation_id)`. `409` means Platform holds
that pair for another `command`; Game stores a `REJECTED` `CHAR_CMD_OPERATION_CONFLICT` receipt
(§5.1) and raises an operator alarm.

`CreateCharacter` intent:

```json
{"contract_version":1,"command":"CreateCharacter","variant":"PLATFORM_USER_CREATE","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a1","source_revision":"17","operation_id":"0192a000-0000-7000-8000-000000000001","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","target_world_id":"01934f10-7c02-7001-805b-3b1122334401","requested_name":"Aldric","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

`TransferCharacterOwnership` intent:

```json
{"contract_version":1,"command":"TransferCharacterOwnership","variant":"PLATFORM_BAZAAR_TRANSFER","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000b1","source_revision":"18","operation_id":"0192a000-0000-7000-8000-000000000002","character_id":"01934f10-7c04-7001-805b-3b1122334401","expected_from_account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","to_account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c7","purpose":"BAZAAR_SETTLEMENT","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

| Member | Rule |
|---|---|
| `command`, `variant` | closed pairs: `CreateCharacter`/`PLATFORM_USER_CREATE`, `TransferCharacterOwnership`/`PLATFORM_BAZAAR_TRANSFER`; anything else is `CHAR_CMD_UNSUPPORTED` |
| `issuer_authority`, `audience` | exactly the constants shown |
| `issuer_decision_id` | the immutable Platform authorization decision (bootstrap decision §Authenticated intent) |
| `source_revision` | positive, unique per `(issuer_authority, variant)`; ordering hint only (§5.3) |
| `operation_id` | the idempotency key (§5) |
| `account_id`, `expected_from_account_id`, `to_account_id` | Platform `AccountId`s; Game copies them and never mints one |
| `target_world_id` | requested World; Game decides whether it accepts creation there |
| `requested_name` | 2..29 bytes of the CHAR-NAME-1 repertoire (boundary §6.1); Game validates it, Platform's check is advisory |
| `character_id` | the Character to transfer |
| `purpose` | closed set, for audit only: `BAZAAR_LISTING_ESCROW`, `BAZAAR_SETTLEMENT`, `BAZAAR_ESCROW_RETURN` |
| `issued_at_source`, `expires_at_source` | `issued < expires ≤ issued + 300` (the bootstrap intent's technical TTL); `issued` at most 5 s after trusted Game time (§5.3) |

Creation choices other than name and World (sex, outfit, vocation) are not v1 inputs: Character
sex is not yet a creation input (`OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md`)
and the canonical starter state applies (boundary §6). Adding a choice is a new
`contract_version` (U-CC2).

### 4.3 Receipt publication

Committed `CreateCharacter`:

```json
{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"CreateCharacter","operation_id":"0192a000-0000-7000-8000-000000000001","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","character_revision":"1","projection_epoch":"1","projection_revision":"43","recovery_generation":"3","decided_at":"1790000002"}
```

Committed `TransferCharacterOwnership`:

```json
{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"TransferCharacterOwnership","operation_id":"0192a000-0000-7000-8000-000000000002","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":"01934f10-7c04-7001-805b-3b1122334401","from_account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","to_account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c7","character_revision":"9","projection_epoch":"1","from_projection_revision":"44","to_projection_revision":"12","recovery_generation":"3","decided_at":"1790000002"}
```

Rejected, either command:

```json
{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"CreateCharacter","operation_id":"0192a000-0000-7000-8000-000000000001","outcome":"REJECTED","result_code":"CHAR_CMD_NAME_UNAVAILABLE","recovery_generation":"3","decided_at":"1790000002"}
```

- The three member sets above are exact per `(command, outcome)`. The rejected member set also
  serves a `command` that Game does not know: Game then echoes the pending-list `command` string
  with `CHAR_CMD_UNSUPPORTED` (§7). Platform accepts that string when it equals the stored
  intent's `command`. A `command` string is 1..64 bytes of ASCII letters, digits and `_`; an entry
  outside that form is a malformed list response, not an intent.
- `projection_epoch` and the projection revisions are the LCFA values the commit assigned to the
  affected accounts (LCFA §5); Platform uses them to know when its read model shows the result
  (§9).
- `decided_at` is the Character Authority transaction time.
- `recovery_generation` is in every receipt, committed or rejected: the Character recovery
  generation (`game_character_recovery_admissions.recovery_generation`, 1..2^64−1) whose fence
  the deciding transaction asserted (`assert_recovery_fence`) when it stored the receipt.
  Platform stores it with the receipt; it selects the receipts that a Game restore reopens (§5.4).
- Every publication of one `operation_id` is byte-identical: Game publishes the stored receipt
  bytes, never a recomputed receipt.

Responses: `200` `{"contract_version":1,"result":"accepted"}` when Platform stores the receipt or
already holds the identical bytes. Failures are empty bodies: `400` malformed, `401`
unauthenticated, `404` no such intent, `409` Platform holds different receipt bytes for that
`operation_id` and that operation is not reopened by a restore (§5.4), `429` rate limited, `503` unavailable. On `404` or `409` Game keeps its receipt,
stops republishing that operation and raises an operator alarm; Platform keeps the intent PENDING
and raises its own. Neither side rewrites the other's record.

## 5. Idempotency, ordering and anti-replay

### 5.1 Operation identity

- Platform mints a fresh UUIDv7 `operation_id` once per user submission or saga step and stores
  it with the intent before any Game exchange. A browser double-submit or a saga retry reuses the
  same `operation_id`; a new attempt after a terminal receipt uses a new one.
- Uniqueness scope: `(issuer_authority, operation_id)`, across both commands. Platform keys its
  intents and stored receipts, and Game keys its receipts, by that same pair. An `operation_id`
  is never reused for another command.
- Game checks before any mutation that the decoded intent's `command` equals the pending-list
  and read-request `command`, and that no stored receipt holds that `operation_id` for another
  command. A mismatch with no stored receipt gets a stored `REJECTED`
  `CHAR_CMD_OPERATION_CONFLICT` receipt and an operator alarm; a mismatch against a stored
  receipt follows §5.2 (no write, no new receipt, alarm).
- Platform intents are immutable: the stored intent bytes never change.

### 5.2 Game receipt store

- Character Authority stores one durable receipt per operation identity, with the canonical
  binding of the complete intent (every member of §4.2), in the same transaction as the decision.
  Deterministic rejections (§7) are stored too, so a rejected `operation_id` can never commit
  later.
- Reading an intent whose operation identity already has a receipt: if the binding is equal, Game
  republishes the stored receipt and writes nothing; if it differs, Game writes nothing, publishes
  nothing new, keeps the stored receipt and raises an operator alarm (`CHAR_CMD_OPERATION_CONFLICT`
  is logged, never published over a stored receipt).
- A committed exact retry returns the same `character_id`, revision, projection revisions and
  bytes. It never allocates a second Character, advances a revision or moves ownership twice
  (boundary §16 scenarios 2 and 4).

### 5.3 Source revision and replay

- Within `(issuer_authority, variant)`, equal `source_revision` with a different `operation_id` or
  binding is a source contradiction: the newcomer gets a stored `REJECTED`
  `CHAR_CMD_OPERATION_CONFLICT` receipt and Game raises an operator alarm.
- Unlike the operator bootstrap variant, a lower `source_revision` than one already accepted is
  **not** refused: concurrent Platform issuance commits revisions out of order. Replay is closed
  by the durable per-operation receipt, the per-revision uniqueness check and the 300 s expiry.
  The consumer still processes pending intents in ascending `source_revision`.
- Expiry is checked against conservative trusted Game time inside the deciding transaction. An
  expired intent gets a stored `REJECTED` `CHAR_CMD_INTENT_EXPIRED` receipt, so a late consumer
  can never commit it.
- Future-dated intents: an `issued_at_source` more than 5 s (the permitted clock skew) after
  trusted Game time, or Game time that is unavailable or ambiguous, never commits (bootstrap
  decision: future or ambiguous intents are rejected). A future-dated intent gets a stored
  `REJECTED` `CHAR_CMD_INTENT_INVALID` receipt; unavailable Game time is the transient case of §6.
  An intent can therefore never stay valid for more than 305 s of Game time.
- The Character restore fence (`CHARACTER_RESTORE_NONROLLBACK_FENCE_V1`) is a prerequisite of
  every decision: a store that is restored and not yet reconciled decides nothing and publishes
  nothing.

### 5.4 Restore reconciliation

OPERABILITY §3 ruling 10 applies to both commands; this section fixes how it uses this wire. The
restore notice and its acknowledgement are PLATFORM-RESTORE-RECONCILE-P1's wire, not this one's.

- After a Game restore notice with restored generation H, Platform reopens as PENDING every
  intent whose stored receipt carries `recovery_generation` ≥ H. It selects only by generation,
  never by comparing a Platform time, `decided_at` or `issued_at_source` with the restore point T.
  It keeps the superseded receipt as history and stores exactly one new receipt for that
  `(issuer_authority, operation_id)`; it settles or compensates only on that new receipt and
  never infers a Game outcome from its own saga row.
- Game consumes reopened intents through the unchanged §4 routes, after the fence reconcile and
  before admission opens (ruling 9 step 11), and decides each from restored state by operation
  identity:
  - a receipt that survived in the restored store is republished byte-identical, a no-op;
  - with no stored receipt, the intent is decided again by §6 under the new generation. The
    expiry and future-skew rules of §5.3 apply unchanged against the original intent times, so
    an operation lost by the restore is in practice decided `CHAR_CMD_INTENT_EXPIRED`; a lost
    create never silently reappears with another `CharacterId`, and a lost transfer never
    commits outside its 300 s window;
  - an erased or terminally deleted Character, or a Character created after T, is always a
    bounded rejection (`CHAR_CMD_CHARACTER_NOT_FOUND`); an erased account is
    `CHAR_CMD_ACCOUNT_NOT_ELIGIBLE`.
- On any `REJECTED` new receipt Platform compensates its own state (refund, reverse the escrow
  or settlement step, tell the user). Platform acknowledges the restore notice only after every
  reopened intent has a new receipt; Game opens admission only after that acknowledgement.
- A reopened intent's republished or new receipt is accepted with `200`; `409` stays the answer
  for different bytes on an operation that no restore reopened.

## 6. Decision procedure

Before any authoritative write, Character Authority proves independently, as the bootstrap
decision §Consumption requires:

1. the authenticated issuer over the character-command identity, a supported `(command, variant)`
   and an exact, unexpired intent;
2. current allowed S1/S2 Platform account-security evidence for `account_id` (create) or
   `to_account_id` (transfer), under its existing freshness rules;
3. current Game process incarnation (#415);
4. a current valid Character recovery fence;
5. valid current World and interpretation context; Game binds its current profile, ruleset,
   content and starter-template revisions at decision time and stores them with the receipt (the
   user variant does not carry them on the wire).

A failure of 2–5 because a dependency is unavailable is transient: no write, no receipt, the
intent stays PENDING and the consumer retries. A definite answer is a stored receipt.

### 6.1 CreateCharacter

In one transaction, holding the account portfolio lock (§6.3) of `account_id`, or not at all:

- name: CHAR-NAME-1 repertoire, comparison key and global reservation (boundary §6.1); a taken
  key is `CHAR_CMD_NAME_UNAVAILABLE` with no Character write; concurrent same-key creates have
  exactly one winner (boundary §16 scenario 3);
- account limit: the account may hold at most 64 listed Characters, the LCFA wire bound
  (`LCA-CHARACTERS`); the product slot quota stays open (U-CC1);
- allocate a fresh `CharacterId`, apply the canonical starter state, write the root, revision 1
  and all dependent state;
- advance the account's LCFA `projection_revision` and record the outbox row (LCFA §5);
- store the receipt with the complete binding, the audit event and its outbox state.

Creation takes no CharacterLease and creates no GameSession.

### 6.2 TransferCharacterOwnership

In one transaction, holding the account portfolio locks (§6.3) of `expected_from_account_id`
and `to_account_id` and then the Character's admission serialization (its CharacterLease row),
or not at all:

- the Character exists and is not terminally deleted;
- current owner equals `expected_from_account_id`, and `to_account_id` differs from it;
- the lifecycle is transferable (not pending deletion, not locked, no world transfer in
  progress);
- **session-generation fencing:** no live CharacterLease exists and the actor is `ABSENT`
  (FND-04 §4). Otherwise `CHAR_CMD_CHARACTER_IN_SESSION`. Because the transfer and fresh
  admission serialize on the same row, either admission commits first and the transfer is
  refused, or the transfer commits first and admission's ownership revalidation (FND-04 steps 13
  and 14) refuses the former owner (boundary §16 scenarios 1 and 5). Ownership never has two
  holders, and no stale grant, lease or session generation issued before the transfer can admit
  or write for the former owner;
- `to_account_id` holds fewer than 64 listed Characters, checked under its portfolio lock;
- **housing proof** (§6.4): when the Character's housing requires it, a current housing prepare
  for this operation is consumed; otherwise `CHAR_CMD_HOUSING_NOT_PREPARED`;
- rebind the owner, advance the Character revision and both accounts' LCFA projection revisions
  (LCFA §5), store the receipt, the audit event and its outbox state.

Platform usage note (Platform's choice, not a Game rule): a Bazaar listing can move the Character
to a Platform escrow `AccountId` (`BAZAAR_LISTING_ESCROW`), so it cannot be played during the
auction; settlement moves it to the buyer (`BAZAAR_SETTLEMENT`); a cancelled auction returns it
(`BAZAAR_ESCROW_RETURN`). Game treats the escrow account as any other `AccountId`. Platform
settles money only on a `COMMITTED` settlement receipt, and recovers a `REJECTED` one through its
own saga (boundary §10).

### 6.3 Account portfolio lock and lock order

- Each `AccountId` known to Character Authority has one portfolio lock (one row locked for
  update). Every command that adds a Character to, or removes one from, an account holds that
  account's lock: `CreateCharacter` for `account_id`, `TransferCharacterOwnership` for both
  accounts. The 64-Character check runs only while the lock is held, so concurrent creates and
  transfers into one account serialize and the account can never exceed 64 listed Characters.
- Lock order, the same for every command: portfolio locks first, in ascending `AccountId`
  order; then CharacterLease rows, in ascending `CharacterId` order. Admission takes no
  portfolio lock, so it cannot form a cycle with these commands. A deadlock or serialization
  failure that the database still reports is the transient case of §6: the transaction rolls
  back, nothing is stored and the intent is decided again.

### 6.4 Housing disposition proof

EXP-HOUSES-01 §§13.5, 14.7 step 4 and 22.2 bind Character Authority to consume current housing
proof before an Account rebinding that changes control of a housing scope. A Character never
carries a house to another account silently (§13).

- **When required:** in the deciding transaction, before any write, Character Authority reads
  the Game housing domain. A housing prepare is required when the Character owns a physical
  house, or when `to_account_id` holds a Residence or a physical house in the Character's World
  and the Character owns a physical house there (the §7 personal-slot conflict). It applies to
  every `purpose`, escrow moves included; an escrow `AccountId` has the same per-World slot.
- **What is proof:** a housing prepare for the same `(issuer_authority, operation_id)`, bound to
  this `character_id`, `expected_from_account_id`, `to_account_id` and the Character's World,
  in a pending, non-terminal disposition state (§13.1, §13.4), whose content fence (§14.7
  step 2) is still held at the revision the prepare recorded, and whose seller disposition
  (§13) and, on a slot conflict, buyer keep-choice (§13.3, §13.4) are recorded. Character
  Authority reads it under lock in the same transaction, so the fence cannot be released between
  the check and the rebinding commit.
- **Fail closed:** missing, stale, mismatched, released or terminal proof is a stored `REJECTED`
  `CHAR_CMD_HOUSING_NOT_PREPARED` receipt with no Character write. Character Authority never
  infers housing state from the intent, a Platform listing or an earlier receipt (§13.5).
- **After the decision:** the transfer never releases the fence and writes no housing state. The
  housing domain finalizes on `COMMITTED` and runs abort restoration on `REJECTED` (§13.1,
  §13.4, §14.7 steps 5–6), keyed by the same operation identity; while no receipt exists the
  housing state stays pending and fenced (§14.7 step 7).
- The housing prepare command and its wire belong to the housing contract, not this one. Until
  the housing domain exists no Character owns housing and the check passes trivially; the
  housing activation packet must add the read before any Character can own a house.
- A house auction bid that nominates the transferred Character (§11.2) is not consulted here;
  the auction's final guards exclude it once the Account binding changes (§11.5).

## 7. Result codes

`outcome` is `COMMITTED` only with `CHAR_CMD_OK`; every other code is `REJECTED`. Each `REJECTED`
code is terminal and stored. The boundary §13 class tells Platform how to proceed.

| `result_code` | Command | Boundary §13 class | Platform action |
|---|---|---|---|
| `CHAR_CMD_OK` | both | success | show the Character / settle |
| `CHAR_CMD_NAME_INVALID` | create | deterministic business rejection | ask for another name |
| `CHAR_CMD_NAME_UNAVAILABLE` | create | deterministic business rejection | ask for another name |
| `CHAR_CMD_WORLD_NOT_ACCEPTED` | create | deterministic business rejection | offer another World |
| `CHAR_CMD_ACCOUNT_CHARACTER_LIMIT` | both | deterministic business rejection | tell the user / recover the saga |
| `CHAR_CMD_ACCOUNT_NOT_ELIGIBLE` | both | authorization rejection | refuse; no retry with the same facts |
| `CHAR_CMD_CHARACTER_NOT_FOUND` | transfer | ownership rejection | recover the saga |
| `CHAR_CMD_OWNER_MISMATCH` | transfer | stale expected state | reread through LCFA, recover the saga |
| `CHAR_CMD_CHARACTER_IN_SESSION` | transfer | session conflict | ask the seller to log out, new `operation_id` |
| `CHAR_CMD_CHARACTER_NOT_TRANSFERABLE` | transfer | lifecycle conflict | recover the saga |
| `CHAR_CMD_HOUSING_NOT_PREPARED` | transfer | stale expected state | let housing restore, prepare again, new `operation_id` |
| `CHAR_CMD_SAME_ACCOUNT` | transfer | deterministic business rejection | Platform defect; alarm |
| `CHAR_CMD_INTENT_EXPIRED` | both | stale expected state | new `operation_id` if still wanted |
| `CHAR_CMD_INTENT_INVALID` | both | authorization rejection | Platform defect; alarm |
| `CHAR_CMD_OPERATION_CONFLICT` | both | ambiguous / contradiction | alarm; no automatic retry |
| `CHAR_CMD_UNSUPPORTED` | both | mixed-version rejection | keep the feature disabled; never fall back to Canary or SQL |

- `CHAR_CMD_UNSUPPORTED` covers an unknown `contract_version`, `command` or `variant`. Game
  publishes it from the pending-list entry, as the rejected receipt of §4.3, even when it cannot
  decode the intent body (boundary §16 scenario 9).
- Dependency unavailability is not a code: it is the transient case of §6.
- Receipts never carry raw database errors (boundary §5).

## 8. Privacy and logging

The consumer logs the operation, the command, the result code and timings only. It never logs an
`AccountId`, a Character name or a Character list, and Platform logs none of them for these routes
(LCFA §7). Intents and receipts carry no credential or secret. The receipt reveals the
account-Character relation only to Platform over the internal identity; Platform shows it only to
the account owner (boundary §3).

## 9. Client "Create character" deep link

D965 Q5=A, normative for the Game client:

- The Character list screen has a "Create character" entry. Activating it opens the system web
  browser at the configured Platform account page for Character creation. The client has no
  creation form and sends no Game command.
- The URL is one complete client configuration value: an absolute `https` URL on the Platform
  web origin the client already uses for account links. It is not part of the versioned wire;
  the expected value ends in `/account/characters/create` (U-CC4). The client
  appends no query, fragment, token, `AccountId`, `CharacterId`, session or ticket. The page
  requires its own Platform web login; no Game credential crosses into the browser. The exact
  Platform route is Platform's to confirm (U-CC4); changing it is a client configuration change,
  not a new `contract_version`.
- The page submits the user's name and World choice to Platform, which stores a
  `CreateCharacter` intent (§2). The page shows "pending" until the receipt, then the result
  code's message, and "created" only on `COMMITTED`.
- **List refresh.** The client never polls Platform in the background. It refreshes the Character
  list when its window regains focus and when the user activates "Refresh", at most once every
  2 s, by repeating the Character list request it already makes in the native login flow
  (Platform `OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §5), which Platform serves from the LCFA
  read model. The new Character appears once Platform's read model holds the account at or above
  the receipt's `projection_revision` in the receipt's `projection_epoch`; before that the list
  is merely older, never wrong. While the LCFA feed is stale Platform answers `503` (LCFA §5.1)
  and the client keeps its last list with a "list unavailable" notice.
- Selecting the new Character uses the unchanged admission path; admission revalidates ownership
  (boundary §4).

## 10. Consistency with CHARACTER_AUTHORITY_PLATFORM_BOUNDARY

| Boundary section | How v1 keeps it |
|---|---|
| §1 authorities | Platform authorizes and orchestrates; Character Authority decides and writes |
| §2, §7 identity | Game allocates `CharacterId`; transfer preserves it |
| §4 admission | transfer serializes with admission; admission revalidates ownership |
| §5 common requirements | operation identity, authenticated caller, atomic decision, durable receipt, no raw DB errors, audit |
| §6, §6.1 creation | Game allocates, reserves the name under CHAR-NAME-1, applies starter state |
| §10 Bazaar | Platform owns the saga; Game owns the rebinding and receipt; no distributed ACID |
| §11 no direct writes | Platform has no Character write path; Game pulls intents |
| §13 failure rules | §7 maps every code; ambiguity waits for the receipt |
| §14 rollout | producer first; Platform stays disabled until Game supports this version (D963) |
| §16 scenarios 2–5, 9 | §11 below |

Related accepted documents: OPERABILITY §3 ruling 10 (generation on every outcome, re-request by
operation identity, rejection for an erased Character, no clock selection) is kept by §4.3 and
§5.4; EXP-HOUSES-01 §§7, 13, 14.7 and 22.2 (no silent house transfer, current same-operation
fence proof, fail closed, abort restoration by housing) are kept by §6.4.

## 11. Conformance scenarios

The Game implementation and the Platform consumer must prove, with shared exact wire fixtures:

1. **duplicate create retry** (§16.2): the same `operation_id` read twice, before and after a lost
   receipt response, yields one Character and byte-identical receipts;
2. **double submit:** two browser submissions with one `operation_id` create one Character;
3. **same-name race** (§16.3): concurrent creates with colliding comparison keys, one
   `COMMITTED`, the others `CHAR_CMD_NAME_UNAVAILABLE`, no partial writes;
4. **name rules:** names outside CHAR-NAME-1 give `CHAR_CMD_NAME_INVALID` with no write;
5. **account limit:** a create or transfer that would give an account a 65th Character is
   refused;
6. **transfer timeout** (§16.4): the receipt publication fails after commit; Game republishes the
   stored receipt; Platform settles exactly once; a retried intent read never transfers twice;
7. **active-session transfer** (§16.5): a live CharacterLease gives `CHAR_CMD_CHARACTER_IN_SESSION`;
   admission and transfer racing on one Character never yield two owners, and the former owner
   is refused at admission after a committed transfer;
8. **stale seller** (§16.1): a wrong `expected_from_account_id` gives `CHAR_CMD_OWNER_MISMATCH`
   with no write; a stale LCFA entry never overrides Game;
9. **expiry:** an intent consumed after `expires_at_source` gets a stored `CHAR_CMD_INTENT_EXPIRED`
   receipt and can never commit later;
10. **operation conflict:** a changed intent under a stored `operation_id`, and equal
    `source_revision` with another `operation_id`, write nothing and alarm;
11. **out-of-order revisions:** a lower `source_revision` that is unexpired and unseen is decided
    normally;
12. **mixed version** (§16.9): an unknown `contract_version`, `command` or `variant` gives
    `CHAR_CMD_UNSUPPORTED`; no Canary or SQL fallback exists on either side;
13. **dependency unavailable:** missing account-security evidence, process proof or recovery
    fence writes nothing and stores no receipt; the intent stays PENDING and is decided later;
14. **restore:** a restored, unreconciled store decides and publishes nothing;
15. **identity:** only character-command certificates reach the three routes, and those
    certificates are refused on every other route;
16. **projection:** a committed create advances the account's LCFA revision, a committed transfer
    advances both accounts', and the receipt carries those revisions;
17. **client deep link:** the entry opens exactly the configured HTTPS URL with nothing appended,
    and a focus regain after a committed create shows the Character once the read model reaches
    the receipt's revision;
18. **privacy:** no log line of either side carries an `AccountId`, name or Character list;
19. **future-dated intent:** `issued_at_source` 6 s after trusted Game time gets a stored
    `CHAR_CMD_INTENT_INVALID` receipt and never commits; 5 s after is decided normally;
20. **portfolio limit race:** an account holding 63 Characters receives two concurrent
    transfers, or a transfer and a create: exactly one commits, the other gets
    `CHAR_CMD_ACCOUNT_CHARACTER_LIMIT`; two transfers between the same two accounts in opposite
    directions never deadlock-stall (lock order of §6.3);
21. **operation id across commands:** one `operation_id` listed or read once as
    `CreateCharacter` and once as `TransferCharacterOwnership`: the first decided keeps its
    receipt; the second writes nothing, stores no second receipt and alarms; a read whose intent
    `command` differs from the requested `command` gets a stored `CHAR_CMD_OPERATION_CONFLICT`
    receipt before any mutation;
22. **pending paging:** with 32 permanently PENDING intents (dependency unavailable), a 33rd
    intent is still listed and decided through the continuation key; a `CreateCharacter` and a
    `TransferCharacterOwnership` sharing `source_revision` across a page boundary are both listed;
    a full page of 32 entries with 64-byte commands and 20-digit revisions (5602 bytes) is
    accepted, and a 33-entry or over-8192-byte response is a malformed list response;
23. **unknown command receipt:** a pending entry `{"command":"RenameCharacter",...}` gets the
    rejected receipt with `"command":"RenameCharacter"` and `CHAR_CMD_UNSUPPORTED`, and Platform
    accepts it with `200`;
24. **restore after a terminalized transfer** (ruling 10): a transfer commits under generation
    3 and Platform settles; a PITR to a T before that commit restores generation H = 3 and
    the fence moves to 4. Platform reopens every receipt with generation ≥ 3, selected without
    any clock comparison, including one decided before T. The receipt that survived T is
    republished byte-identical and changes nothing; the lost transfer has no stored receipt, is
    decided `CHAR_CMD_INTENT_EXPIRED` under generation 4 with no rebinding, Platform accepts the
    new receipt with `200` and compensates; a lost create is not recreated; a Character erased
    after T gets `CHAR_CMD_CHARACTER_NOT_FOUND`; Game opens admission only after Platform's
    acknowledgement; every receipt carries `recovery_generation`;
25. **housing proof** (EXP-HOUSES-01 §25 scenarios 12–15, 29, 36, 37, 41, 42): a Character that
    owns a physical house, transferred with no prepare, with a prepare of another
    `operation_id` or Character, with a prepare whose content fence was released or whose
    content revision advanced, or, on a buyer slot conflict, with no recorded keep-choice, gets
    `CHAR_CMD_HOUSING_NOT_PREPARED` with no rebinding, and housing restores the prior result; the
    same transfer with a current same-operation prepare commits, keeps the fence for housing
    finalization and rebinds once; an escrow move follows the same rule; a Character with no
    housing needs no prepare.

## 12. Limits

| Limit | Value |
|---|---|
| pending entries per list response | 32 |
| request bytes (read) | 448 |
| request bytes (pending list) | 512 |
| intent bytes | 4096 |
| receipt request bytes | 1024 |
| response bytes (pending list) | 8192 |
| response bytes (receipt) | 256 |
| in-flight exchanges per consumer | 1 |
| intent TTL | 300 s |
| Characters per account | 64 (`LCA-CHARACTERS`, wire bound, not a quota) |

The implementation packet registers the new limits in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`.

## 13. Unknowns

| ID | Question | v1 position |
|---|---|---|
| U-CC1 | product Character slot quota | open; only the 64 wire bound applies |
| U-CC2 | creation choices beyond name and World (sex, outfit, vocation) | not inputs; a new `contract_version` |
| U-CC3 | per-account create rate limit | Platform's to set on the web page |
| U-CC4 | exact Platform account page route | client configuration; expected `/account/characters/create` until Platform confirms |
| U-CC5 | production PKI and Character Authority host list | open, as LCFA U-LC6 |
| U-CC6 | rename, deletion, restore, world transfer | out of scope; later contract versions |
| U-CC7 | Platform storage of `recovery_generation` and the restore notice (PLATFORM-RESTORE-RECONCILE-P1, not yet accepted by Platform) | enablement (§14 step 4) requires Platform's acceptance; until then both commands stay disabled |
| U-CC8 | housing prepare command and fence representation (EXP-HOUSES-01 not started) | §6.4 semantics bind; the housing contract fixes the wire and tables |

## 14. Rollout

1. Owner acceptance of this candidate after independent review; the lock entry becomes `LOCKED`
   at the merged commit.
2. Game implementation (consumer, receipt store, decision transactions), disabled by default.
3. Platform consumer: intent store, the three routes, receipt handling, the account page and the
   Bazaar saga change (packets DECANARY-CREATE-1 and DECANARY-BAZAAR-1).
4. With U-CC7 settled, enable both in one `testing` stack with shared fixtures; then `preproduction`.
5. Until step 4, Platform keeps creation and transfer disabled (D963 Q2=A).

## 15. Acceptance

Not accepted. Acceptance makes revision 1 the frozen v1 wire; any later change to a member, path,
bound, code or response is a new `contract_version`.
