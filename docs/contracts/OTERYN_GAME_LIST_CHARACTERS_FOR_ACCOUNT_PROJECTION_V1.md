# Oteryn Game ListCharactersForAccount Projection v1

- Status: Candidate. Acceptance by the architect and the owner (#162 Q15a). Implementation is not authorized by this document.
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
| `name` | string | current display name, 1..64 bytes, NFC-normalized UTF-8; no control (Cc) or format (Cf) characters, which excludes bidi embeddings and overrides U+202A–U+202E, isolates U+2066–U+2069 and zero-width characters; no `"` or `\` (so it serializes without escapes) |
| `availability` | string | `AVAILABLE` when the current lifecycle permits a fresh admission attempt to `world_id`; `UNAVAILABLE` otherwise (pending deletion, transfer in progress, locked or any other non-admissible state) |

- Terminally deleted, retired or finalized Characters are omitted.
- `AVAILABLE` is not a promise of admission; presence, lease, route and revisions are checked at admission.
- Deliberately absent in v1: vocation, level or other progression summary (product policy not decided), `ChannelId`, session or presence state, lease or fencing data, the owner of any other Character, creation or deletion timestamps. Adding a field is a new `contract_version`.

## 3. Transport and identity (Decision P1: push)

Game delivers per-account snapshots and a liveness watermark to Platform over the existing Game→Platform mTLS client code (`apps/game-server/src/native_admission_source/`, `http1_mtls::exchange`), HTTP/1.1 over TLS 1.3 with a client certificate.

- **Identity:** a dedicated projection client certificate, held only by the publisher running on Character Authority hosts (the hosts whose database role may read authoritative Character ownership). It is separate from the native-evidence and runtime-status certificates; a gameplay node that is not a Character Authority host holds no projection certificate. Platform accepts publications only from its configured projection identities and refuses those identities on every other route.
- Closed operations with compiled paths:
  - `PublishAccountCharactersV1` → `POST /internal/v1/game-auth/native-account-characters`;
  - `PublishProjectionWatermarkV1` → `POST /internal/v1/game-auth/native-account-characters/watermark`.
- Bounds: connect 1 s, handshake 2 s, exchange 3 s; snapshot request at most 16384 bytes (§8); watermark request at most 512 bytes; response at most 256 bytes; one in-flight publication per publisher.

Decision P1: push with the §5.1 watermark is accepted for Q18a internal test builds. For the release entry the architect either accepts push on the basis of a measured liveness bound S or requires pull, where Platform queries a Game-hosted endpoint at issuance (exact freshness, but a new inbound Game service with its own listener, identity and deployment). Recommendation: push if the measured S holds, otherwise pull (U-LC1).

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

**Clarification (architect ruling #162 5910360309), pending on acceptance of ITEM-MOVE-WIRE-0,
which carries it.** "Content" is the projected character set
only (the `characters` value, compared canonically). `source_observed_at` and the envelope fields
are not content: an equal pair with equal `characters` is an idempotent `200 accepted`, and the
consumer keeps its first stored `source_observed_at`. The same reading applies to the snapshot
ordering rule below.

## 5. Revision, ordering and delivery

- **Snapshot, not delta.** Each publication carries the account's complete current list. Platform replaces its entry, so a lost or duplicated publication never corrupts the read model.
- **`projection_revision`** is per `AccountId`, monotonic, and advanced in the same Character Authority transaction as every mutation that changes the account's list or a listed field: create or bootstrap, rename, lifecycle change, world transfer, and ownership transfer (both the old and the new owner's accounts).
- **`projection_epoch`** is a positive Game-side value raised only after a restore or rollback of the Character store. Raising it **forces a full resync**: the publisher republishes the current snapshot of every account in the new epoch. Platform orders by `(projection_epoch, projection_revision)`; the first time it sees a higher epoch (in a snapshot or a watermark) it invalidates every entry below that epoch, so an account is usable again only once its new-epoch snapshot arrives. A restore therefore cannot leave Platform holding an entry newer than authority. How the epoch is stored and raised is a Game follow-up (U-LC2).
- **Transactional outbox.** The mutation transaction also records the affected `AccountId` in an outbox. A publisher reads the current list and revision in one read transaction, sends it, and clears outbox rows for that account up to the sent revision on `accepted` or `superseded`. Delivery is at least once; duplicates are harmless.
- **Resync.** An operator command republishes the current snapshot of every account (initial fill, Platform read-model loss, epoch raise). Platform cannot request a resync in v1 (U-LC3).

### 5.1 Liveness watermark

At least every 10 s the publisher sends `complete_through`: a source time T such that every Character change committed at or before T has been delivered and acknowledged. It is computed as the earlier of (the `created_at` of the oldest undelivered outbox row) and (`now` minus the configured maximum Character transaction duration), so a slow transaction that commits late is still covered. During an epoch's full resync, `complete_through` stays below the resync start until the resync finishes. Platform treats the whole feed as stale when `platform_now - complete_through + clock_uncertainty > S` (S proposed 30 s, U-LC5) or when the watermark epoch is not the highest seen, and then refuses native issuance (Platform contract §5.4). The watermark is liveness evidence only; it never proves ownership.
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

Registry entries are a follow-up in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (outside this task): `LCA-CHARACTERS` 64 entries (hard maximum 64), `LCA-REQUEST-BYTES` 16384, `LCA-WATERMARK-BYTES` 512, `LCA-INFLIGHT` 1 per publisher. The product slot quota per account is not decided here; 64 is a wire bound, not a quota.

## 9. Required tests

- revision advanced in the mutation transaction for each listed mutation, including both accounts in an ownership transfer;
- snapshot ordering: lower pair superseded, equal pair with different content rejected, higher epoch replaces and invalidates lower-epoch entries until resync;
- watermark: `complete_through` never passes an undelivered change; feed goes stale when the watermark stops; issuance refuses while stale;
- identity: only the projection certificate is accepted; gameplay-node certificates are refused;
- names with bidi, format, control, `"` or `\` characters are rejected; a 64-entry, 64-byte-name snapshot fits 16384 bytes;
- deleted Characters omitted; world transfer changes `world_id`; lifecycle change flips `availability`;
- stale projection after sale: Platform issues, admission rejects with `ADMISSION_ACCOUNT_CHARACTER_CONFLICT`;
- privacy: no public route exposes the relation;
- exact wire fixtures shared with the Platform consumer.

## 10. Unknowns

- U-LC1: push (accepted for Q18a internal builds) vs pull for the release entry (Decision P1).
- U-LC2: epoch storage and the restore procedure that raises it.
- U-LC3: Platform-requested resync.
- U-LC4: progression fields (vocation, level) for the selection UI; product policy.
- U-LC5: liveness bound S (proposed 30 s) and watermark period (Platform U17).
- U-LC6: PKI for the projection certificate and the list of Character Authority hosts (Platform U15).
