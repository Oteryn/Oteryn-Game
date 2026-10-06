# Oteryn Game ListCharactersForAccount Projection v1

- Status: **Accepted**, revision 2 (GAME-LCFA-PROJECTION-CONTRACT-1), 2026-10-06: owner acceptance
  relayed by CP, D828 (after owner answer **1a** to D821, #162). It authorizes only the Platform
  consumer packet and the Game enablement packet of
  `docs/architecture/reviews/OTERYN_GAME_ARCH_LCFA_PROJECTION_CONTRACT_2026-10-06.md`, for
  `testing` and `preproduction` only.
- Revision 2 changes no member, type, path, bound or response of revision 1. It adds the
  multichannel, fencing and authority rules (§2.2–§2.4), rules U-LC1, U-LC3, U-LC5 and U-LC6 for
  testing and preproduction (§10), folds in the "content" clarification (§4) and records the
  producer status (§11).
- Contract ID: `oteryn-game-list-characters-for-account-v1`
- Coordination: #162 (owner decisions Q14–Q18, comments 5899892092 and 5899942821); Oteryn/Oteryn-Platform#1419.
- Owner decision: **Q17a** — a Game-owned `ListCharactersForAccount` projection gives Platform the account's Characters.
- Producer and authority: `Oteryn/Oteryn-Game` Character Authority. Consumer: `Oteryn/Oteryn-Platform`, read-only.
- Parent contract: `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` (accepted) §3 read contract. This document fixes the schema and transport that §3 and §15 left deferred, and changes none of its semantics.
- Platform consumer rules: `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §5 (Oteryn/Oteryn-Platform PR #1420, candidate); Platform ADR 0030 portfolio semantics.

## 1. Purpose

The native Gateway lets a player pick a Character and binds that `CharacterId` and its current `WorldId` into the FND-04 grant. Platform needs the account's Characters for that choice and for its issuance pre-check, without becoming a second ownership authority. Final admission still revalidates ownership and world (FND-04 steps 13 and 14).

## 2. Semantics

```text
ListCharactersForAccount(AccountId) -> CharacterSummary[]
```

- Derived only from authoritative Character Authority state.
- Platform may store and serve it only to the authenticated account owner and to the native issuer. It is never public, never used to show one account's Characters to another, and never proof of current ownership (boundary §3, §4).
- Stale, unavailable or contradictory state fails toward less disclosure and toward no issuance.

### 2.1 CharacterSummary v1

| Field | Type | Rule |
|---|---|---|
| `character_id` | string | canonical lowercase non-nil UUIDv7 (`CharacterId`) |
| `world_id` | string | canonical lowercase non-nil UUIDv7; the Character's authoritative **current** world membership |
| `name` | string | current display name; wire rule 1..64 bytes, NFC-normalized UTF-8; no control (Cc) or format (Cf) characters, which excludes bidi embeddings and overrides U+202A–U+202E, isolates U+2066–U+2069 and zero-width characters; no `"` or `\` (so it serializes without escapes) |
| `availability` | string | `AVAILABLE` when the current lifecycle permits a fresh admission attempt to `world_id`; `UNAVAILABLE` otherwise (pending deletion, transfer in progress, locked or any other non-admissible state) |

- The Game producer emits only CHAR-NAME-1 names (2..29 ASCII letters, single inner spaces), a
  strict subset of the wire rule. The consumer validates the wider wire rule above, never the
  CHAR-NAME-1 rule, so a later name policy that stays inside the wire rule needs no Platform
  change.
- Terminally deleted, retired or finalized Characters are omitted.
- `AVAILABLE` is not a promise of admission; presence, lease, route and revisions are checked at admission.
- Deliberately absent in v1: vocation, level or other progression summary (product policy not decided), `ChannelId`, session or presence state, lease or fencing data, the owner of any other Character, creation or deletion timestamps. Adding a field is a new `contract_version`.

### 2.2 Multichannel: WorldId only, never ChannelId

- The world model is multichannel: a World has one or more Channels, and `WorldId` and
  `ChannelId` are distinct identifiers that are never derived from each other or interchanged.
- A Character belongs to a World. Its Channel is not Character state: it is chosen per admission.
  The projection therefore carries `world_id` and no `ChannelId`, last Channel, preferred
  Channel or Channel availability.
- Platform chooses the Channel at issuance from its route records and runtime status (Platform
  contract §7.4) among the Channels of the entry's `world_id`, optionally narrowed by the
  request's `channel_id`. The grant binds `world_id` from the entry and `channel_id` from the
  route record; the projection never supplies or vouches for a `channel_id`.
- A world transfer changes `world_id` and advances the revision (§5). Moving between Channels of
  one World changes nothing in the projection and publishes nothing.
- `availability` describes the Character, never a Channel: a Character is `AVAILABLE` even when
  every Channel of its World is full, in maintenance or unrouted. Route availability is decided
  by §7.4 and reported as `NATIVE_LOGIN_ROUTE_UNAVAILABLE`.

### 2.3 Session-generation fencing

- Character writes stay session-generation fenced. The projection carries no session
  generation, lease, owner token, fencing value, presence or connection state, and nothing in it
  can be used to take, extend, compare or forge a fence.
- The projection is derived in the Character Authority transaction that commits a Character
  write (§5). A write rejected by its fence commits nothing, so it advances no revision and queues
  nothing; the projection only ever reflects committed, fenced writes.
- The publisher reads the Character roots, the projection tables and the outbox in one read
  statement under the reconciled Character authority, so a restored or unreconciled store never
  publishes. It never writes Character state and never takes a Character lease or session. Its
  only writes are outbox deletions after an acknowledgement. The one lock a Character write can
  wait on is the account's projection revision row, held by the same-transaction revision touch
  or by an operator resync (§5).
- Admission does not trust the projection for fencing or ownership: FND-04 steps 13 and 14 and
  the FND-04A fenced admission revalidate ownership, lifecycle, world and the session generation
  from authoritative state. A stale entry can at worst produce a grant that admission rejects
  (`ADMISSION_ACCOUNT_CHARACTER_CONFLICT`, `ADMISSION_GRANT_WORLD_STALE`), never an admission.

### 2.4 Owner and authority rules

| Fact | Owner and truth | Platform role |
|---|---|---|
| `CharacterId`, the Character, its name, lifecycle, current World and owning `AccountId` | Game Character Authority, only | read model only; never proof of current ownership |
| `projection_epoch`, `projection_revision`, outbox, resync | Game Character Authority | orders and invalidates; never sets or requests them in v1 |
| `AccountId` | Platform (ADR 0028) | Game copies it from authoritative ownership; never mints it |
| `WorldId`, `ChannelId`, route records, `native_login_enabled` | Platform Registry (contract §7) | Game reports runtime facts; the projection carries only `world_id` |
| `availability` | Game, derived from lifecycle | input to the §5.4 issuance check only |

- Game Character Authority is the only producer. A gameplay node that is not a Character
  Authority host, Atlas, the Gateway and every other service hold no projection identity.
- Platform never creates, edits, deletes, renames, transfers or locks a Character from the read
  model, never writes Game tables, and never serves the read model to anyone but the
  authenticated account owner and the native issuer. Atlas never consumes this projection.
- A Platform-side change to an account (disable, deletion, `AccountId` reassignment) is decided
  in Platform and enforced at issuance and redemption; it is never written back into the
  projection, and Game learns of it only through its own accepted contracts.
- A dispute between the read model and Game is always resolved in favour of Game: the
  read-model entry is stale, never the Character.

## 3. Transport and identity (Decision P1: push)

Game delivers per-account snapshots and a liveness watermark to Platform over the existing Game→Platform mTLS client code (`apps/game-server/src/native_admission_source/`, `http1_mtls::exchange`), HTTP/1.1 over TLS 1.3 with a client certificate.

- **Identity:** a dedicated projection client certificate, held only by the publisher running on Character Authority hosts (the hosts whose database role may read authoritative Character ownership). It is separate from the native-evidence and runtime-status certificates; a gameplay node that is not a Character Authority host holds no projection certificate. Platform accepts publications only from its configured projection identities and refuses those identities on every other route.
- Closed operations with compiled paths:
  - `PublishAccountCharactersV1` → `POST /internal/v1/game-auth/native-account-characters`;
  - `PublishProjectionWatermarkV1` → `POST /internal/v1/game-auth/native-account-characters/watermark`.
- Bounds: connect 1 s, handshake 2 s, exchange 3 s; snapshot request at most 16384 bytes (§8); watermark request at most 512 bytes; response at most 256 bytes; one in-flight publication per publisher.

Decision P1: push with the §5.1 watermark is accepted for Q18a internal test builds and for
`testing` and `preproduction` (§10, U-LC1). For the release entry the architect either accepts push on the basis of a measured liveness bound S or requires pull, where Platform queries a Game-hosted endpoint at issuance (exact freshness, but a new inbound Game service with its own listener, identity and deployment). Recommendation: push if the measured S holds, otherwise pull (U-LC1).

## 4. Wire (v1)

Snapshot request, exact member set, no unknown, duplicate or `null` members, nesting at most 3:

```json
{
  "contract_version": 1,
  "operation": "PublishAccountCharactersV1",
  "source_authority": "oteryn:character-authority:primary",
  "account_id": "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6",
  "projection_epoch": "1",
  "projection_revision": "42",
  "source_observed_at": "1790000000",
  "characters": [
    { "character_id": "01934f10-7c04-7001-805b-3b1122334401", "world_id": "01934f10-7c02-7001-805b-3b1122334401", "name": "Aldric", "availability": "AVAILABLE" }
  ]
}
```

- `account_id`: canonical lowercase Platform `AccountId` (UUIDv7, Platform ADR 0028; Game never mints it).
- `characters`: 0..64 entries, sorted by `character_id` ascending, unique. An empty list is a valid snapshot.
- `projection_epoch`, `projection_revision`: canonical non-zero decimal uint64 strings (§5).
- `source_observed_at`: canonical decimal Unix seconds at which the snapshot was read. Every retry of the same `(projection_epoch, projection_revision)` publication is byte-identical: Game takes `source_observed_at` from when that revision was assigned (never later than the read), not from the retry.
- `source_authority`: the configured Character Authority namespace, 1..128 of `[A-Za-z0-9._:/-]`.
- Serialization: raw UTF-8, no string escapes (names exclude `"`, `\` and control characters), no insignificant whitespace.

Watermark request, exact member set:

```json
{ "contract_version": 1, "operation": "PublishProjectionWatermarkV1", "source_authority": "oteryn:character-authority:primary", "projection_epoch": "1", "complete_through": "1790000000", "observed_at": "1790000003" }
```

Success response (`200`, exact): `{"contract_version":1,"result":"accepted"}` or `{"contract_version":1,"result":"superseded"}`. Failures are empty bodies: `400` malformed, `401` unauthenticated, `409` equal `(epoch, revision)` with different content, `429` rate limited, `503` unavailable.

**Content (architect ruling #162 5910360309, carried by ITEM-MOVE-WIRE-0 D212 and normative in
revision 2).** "Content" is the projected character set
only (the `characters` value, compared canonically). `source_observed_at` and the envelope fields
are not content: an equal pair with equal `characters` is an idempotent `200 accepted`, and the
consumer keeps its first stored `source_observed_at`. The same reading applies to the snapshot
ordering rule below.

## 5. Revision, ordering and delivery

- **Snapshot, not delta.** Each publication carries the account's complete current list. Platform replaces its entry, so a lost or duplicated publication never corrupts the read model.
- **`projection_revision`** is per `AccountId`, monotonic, and advanced in the same Character Authority transaction as every mutation that changes the account's list or a listed field: create or bootstrap, rename, lifecycle change, world transfer, and ownership transfer (both the old and the new owner's accounts).
- **`projection_epoch`** is a positive Game-side value raised only after a restore or rollback of the Character store. Raising it **forces a full resync**: the publisher republishes the current snapshot of every account in the new epoch. Platform orders by `(projection_epoch, projection_revision)`; the first time it sees a higher epoch (in a snapshot or a watermark) it invalidates every entry below that epoch, so an account is usable again only once its new-epoch snapshot arrives. A restore therefore cannot leave Platform holding an entry newer than authority. For `testing` and `preproduction` the epoch is stored and raised as §10 U-LC2 rules (migrations 0024 and 0028, operator-only raise); only the production restore runbook stays open. **Restore precondition.** A raise computes `max(restored_current + 1, now in Unix ms)` and cannot see an epoch published after the backup. It is above every published epoch only if the host clock at the raise is later than the clock at every earlier raise. The operator therefore raises only on a host whose clock is synchronized and not behind the clock of the last raise. If that fails, Platform answers `superseded` to every snapshot and watermark of the new epoch, the feed stays stale and nothing is disclosed (fail closed). The publisher logs the `superseded` result class, which is the detection signal. Recovery: correct the clock, then run `game_character_account_projection_resync(true)` again; the new epoch is above the published one once the clock passes it. The production restore runbook must instead keep the highest published epoch outside the restored store as an external fence and raise above it (U-LC2, open).
- **Transactional outbox.** The mutation transaction also records the affected `AccountId` in an outbox. A publisher reads the current list and revision in one read transaction, sends it, and clears outbox rows for that account up to the sent revision on `accepted` or `superseded`. Delivery is at least once; duplicates are harmless.
- **Resync.** An operator command republishes the current snapshot of every account (initial fill, Platform read-model loss, epoch raise). Platform cannot request a resync in v1 (U-LC3). The Game operator path is `game_character_account_projection_resync(p_raise_epoch)` (migrations 0024 and 0028); a raised epoch is `max(current + 1, now in Unix ms)`, so real epochs are large values and the consumer must compare them as uint64, never as 32-bit or floating-point numbers.

### 5.1 Liveness watermark

At least every 10 s the publisher sends `complete_through`: a source time T such that every Character change committed at or before T has been delivered and acknowledged. It is computed as the earlier of (the `created_at` of the oldest undelivered outbox row) and (`now` minus the configured maximum Character transaction duration), so a slow transaction that commits late is still covered. During an epoch's full resync, `complete_through` stays below the resync start until the resync finishes. Platform refuses a watermark with `400` and does not store it when `complete_through` is after `observed_at`, or when either is more than `clock_uncertainty` after `platform_now`; a Game clock ahead of Platform therefore cannot keep the feed live. Platform treats the whole feed as stale when `platform_now - complete_through + clock_uncertainty > S`, when the stored `complete_through` is more than `clock_uncertainty` after `platform_now` (S = 30 s for `testing` and `preproduction`, §10 U-LC5; the release value stays open) or when the watermark epoch is not the highest seen. While the feed is stale Platform refuses native issuance (Platform contract §5.4) and answers the account's Character list read with `503` and no entry; it does the same for an account that is `invalid` or below the highest epoch seen. The watermark is liveness evidence only; it never proves ownership.
- Publication failure never affects gameplay, admission or the Character mutation that caused it.

## 6. Consistency with CHARACTER_AUTHORITY_PLATFORM_BOUNDARY

| Boundary section | How v1 keeps it |
|---|---|
| §1 authorities | Character Authority is the only producer; Platform holds a read model |
| §2 identity relation | ownership stays in the game domain; the projection is derived from it |
| §3 read contract | fields are a subset of the approved examples; private by default |
| §4 admission rule | FND-04 steps 13–14 revalidate ownership, lifecycle and world at admission |
| §11 no direct writes | the transport is a Game-initiated publication; Platform has no Character write path |
| §13 failure rules | stale or conflicting data fails toward less disclosure and no issuance (account invalid, feed stale) |
| §14 rollout | producer first; Platform consumes only after this version is supported |
| §16 scenarios 1 and 8 | stale projection after sale is rejected at admission; no public consumer sees the relation |

## 7. Privacy and logging

The publisher logs only the operation, the result class and timings. It never logs `AccountId`, Character names or the Character list, and neither may Platform (Platform contract §13). The publication carries no credential or secret.

## 8. Proposed limits

Worst-case snapshot size: each entry is at most 17 + 36 + 14 + 36 + 10 + 64 + 18 + 11 + 3 = 209 bytes (keys, two UUIDs, a 64-byte name without escapes, `UNAVAILABLE`, separators), so 64 entries take at most 13,376 bytes; the envelope adds at most about 400 bytes. The request limit is therefore 16384 bytes.

These limits are registered in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`: `LCA-CHARACTERS` 64 entries (hard maximum 64), `LCA-REQUEST-BYTES` 16384, `LCA-WATERMARK-BYTES` 512, `LCA-INFLIGHT` 1 per publisher. The product slot quota per account is not decided here; 64 is a wire bound, not a quota.

## 9. Required tests

- revision advanced in the mutation transaction for each listed mutation, including both accounts in an ownership transfer;
- snapshot ordering: lower pair superseded, equal pair with different content rejected, higher epoch replaces and invalidates lower-epoch entries until resync;
- watermark: `complete_through` never passes an undelivered change; feed goes stale when the watermark stops; issuance refuses while stale; a future-dated watermark (beyond `clock_uncertainty`) is refused and never makes the feed live;
- epoch restore: a raise whose value is not above the highest epoch already published is detected as `superseded` on every publication of the new epoch, and a raise after the clock is corrected recovers;
- identity: only the projection certificate is accepted; gameplay-node certificates are refused;
- names with bidi, format, control, `"` or `\` characters are rejected; a 64-entry, 64-byte-name snapshot fits 16384 bytes;
- deleted Characters omitted; world transfer changes `world_id`; lifecycle change flips `availability`;
- stale projection after sale: Platform issues, admission rejects with `ADMISSION_ACCOUNT_CHARACTER_CONFLICT`;
- privacy: no public route exposes the relation;
- exact wire fixtures shared with the Platform consumer.

## 10. Unknowns

Revision 2 rules each unknown for `testing` and `preproduction`. Every release-entry ruling stays
open and needs its own decision.

| ID | Testing and preproduction (revision 2) | Release entry |
|---|---|---|
| U-LC1 | **push** with the §5.1 watermark | open: push on a measured S, or pull (Decision P1) |
| U-LC2 | the epoch row and the operator-only raise of migrations 0024/0028 (LCFA-1b); a restore is followed by `game_character_account_projection_resync(true)` before the publisher restarts, under the §5 restore precondition (synchronized clock; `superseded` detection; raise again after a clock fix) | open: the production restore runbook, with the highest published epoch kept outside the restored store as an external fence |
| U-LC3 | no Platform-requested resync; the operator runs the resync on Platform read-model loss | open |
| U-LC4 | no progression fields; the selection UI shows name and World | open: product policy, then a new `contract_version` |
| U-LC5 | **S = 30 s**, Platform `clock_uncertainty` 1 s; the producer sends a watermark every 3 s with a gap of at most 10 s (`MAX_WATERMARK_GAP`) | open: S from a measured delivery lag |
| U-LC6 | one projection identity per Character Authority host, issued from the per-run or per-stack development CA of the test stack (the same CA family as the runtime-status identities); the host list is the stack's Character Authority hosts; Platform configures them by certificate subject | open: Platform U15 PKI and the production host list |

## 11. Implementation status

- **Game producer:** implemented and not enabled. LCFA-1 (#1330, migration 0024: projection, outbox
  and epoch tables, the trigger on `game_character_roots`, the publisher in
  `apps/game-server/src/native_admission_source/account_characters.rs`, the `LCA-*` limits) and
  LCFA-1b (#1389, migration 0028: `revised_at`, every Character root listed and every lifecycle
  other than the admissible one listed as `UNAVAILABLE` (a deleted Character has no root and is
  omitted), the millisecond epoch raise, the 10 s watermark gap). The publisher is not
  yet wired to a projection identity or started by any stack.
- **Platform consumer:** not implemented. Platform issuance runs only in mode 33a (D171) until the
  consumer packet lands and the publisher is enabled in the same stack.

## 12. Acceptance

Accepted 2026-10-06: owner acceptance relayed by CP, D828 (D821 answer 1a). Acceptance makes
revision 2 the frozen v1 wire for `testing` and `preproduction`; any change to a member, path,
bound or response after that is a new `contract_version`.
