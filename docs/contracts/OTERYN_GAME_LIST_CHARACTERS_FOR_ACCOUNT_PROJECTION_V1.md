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
| `name` | string | current display name, 1..64 bytes UTF-8, no control characters |
| `availability` | string | `AVAILABLE` when the current lifecycle permits a fresh admission attempt to `world_id`; `UNAVAILABLE` otherwise (pending deletion, transfer in progress, locked or any other non-admissible state) |

- Terminally deleted, retired or finalized Characters are omitted.
- `AVAILABLE` is not a promise of admission; presence, lease, route and revisions are checked at admission.
- Deliberately absent in v1: vocation, level or other progression summary (product policy not decided), `ChannelId`, session or presence state, lease or fencing data, the owner of any other Character, creation or deletion timestamps. Adding a field is a new `contract_version`.

## 3. Transport (Decision P1: push)

Game delivers per-account snapshots to Platform over the existing Game→Platform mTLS client (`apps/game-server/src/native_admission_source/`): same `ProducerDescriptor`, HTTP/1.1 over TLS 1.3 with a client certificate.

- New closed operation `PublishAccountCharactersV1`, compiled path `POST /internal/v1/game-auth/native-account-characters`.
- Bounds: connect 1 s, handshake 2 s, exchange 3 s; request at most 12288 bytes; response at most 256 bytes; one in-flight publication per node.

Rationale: Game is already the mTLS client of Platform and Character Authority stays the only writer. A pull design (Platform queries a Game-hosted endpoint at login) would give exact freshness at issuance but needs a new inbound Game service, its own listener, identity and deployment; the architect may choose it instead (U-LC1). Under push, final admission covers the staleness window.

## 4. Wire (v1)

Request, exact member set, no unknown, duplicate or `null` members, nesting at most 3:

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
- `source_observed_at`: canonical decimal Unix seconds at which the snapshot was read.
- `source_authority`: the configured Character Authority namespace, 1..128 of `[A-Za-z0-9._:/-]`.

Success response (`200`, exact): `{"contract_version":1,"result":"accepted"}` or `{"contract_version":1,"result":"superseded"}`. Failures are empty bodies: `400` malformed, `401` unauthenticated, `409` equal `(epoch, revision)` with different content, `429` rate limited, `503` unavailable.

## 5. Revision, ordering and delivery

- **Snapshot, not delta.** Each publication carries the account's complete current list. Platform replaces its entry, so a lost or duplicated publication never corrupts the read model.
- **`projection_revision`** is per `AccountId`, monotonic, and advanced in the same Character Authority transaction as every mutation that changes the account's list or a listed field: create or bootstrap, rename, lifecycle change, world transfer, and ownership transfer (both the old and the new owner's accounts).
- **`projection_epoch`** is a positive Game-side value raised only after a restore or rollback of the Character store. Platform orders by `(projection_epoch, projection_revision)`; a higher epoch replaces the entry even with a lower revision, so a restore cannot leave Platform holding an entry newer than authority. How the epoch is stored and raised is a Game follow-up (U-LC2).
- **Transactional outbox.** The mutation transaction also records the affected `AccountId` in an outbox. A publisher reads the current list and revision in one read transaction, sends it, and clears outbox rows for that account up to the sent revision on `accepted` or `superseded`. Delivery is at least once; duplicates are harmless.
- **Resync.** An operator command republishes the current snapshot of every account (initial fill, Platform read-model loss). Platform cannot request a resync in v1 (U-LC3).
- Publication failure never affects gameplay, admission or the Character mutation that caused it.

## 6. Consistency with CHARACTER_AUTHORITY_PLATFORM_BOUNDARY

| Boundary section | How v1 keeps it |
|---|---|
| §1 authorities | Character Authority is the only producer; Platform holds a read model |
| §2 identity relation | ownership stays in the game domain; the projection is derived from it |
| §3 read contract | fields are a subset of the approved examples; private by default |
| §4 admission rule | FND-04 steps 13–14 revalidate ownership, lifecycle and world at admission |
| §11 no direct writes | the transport is a Game-initiated publication; Platform has no Character write path |
| §13 failure rules | stale or conflicting data fails toward less disclosure (Platform marks the account invalid) |
| §14 rollout | producer first; Platform consumes only after this version is supported |
| §16 scenarios 1 and 8 | stale projection after sale is rejected at admission; no public consumer sees the relation |

## 7. Privacy and logging

The node logs only the operation, the result class and timings. It never logs `AccountId`, names or the Character list. The publication carries no credential or secret.

## 8. Proposed limits

Registry entries are a follow-up in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (outside this task): `LCA-CHARACTERS` 64 entries (hard maximum 64), `LCA-REQUEST-BYTES` 12288, `LCA-INFLIGHT` 1 per node. The product slot quota per account is not decided here; 64 is a wire bound, not a quota.

## 9. Required tests

- revision advanced in the mutation transaction for each listed mutation, including both accounts in an ownership transfer;
- snapshot ordering: lower pair superseded, equal pair with different content rejected, higher epoch replaces;
- deleted Characters omitted; world transfer changes `world_id`; lifecycle change flips `availability`;
- stale projection after sale: Platform issues, admission rejects with `ADMISSION_ACCOUNT_CHARACTER_CONFLICT`;
- privacy: no public route exposes the relation;
- exact wire fixtures shared with the Platform consumer; 64-entry and byte bounds.

## 10. Unknowns

- U-LC1: push (this candidate) vs pull transport (Decision P1).
- U-LC2: epoch storage and the restore procedure that raises it.
- U-LC3: Platform-requested resync.
- U-LC4: progression fields (vocation, level) for the selection UI; product policy.
- U-LC5: maximum projection staleness Platform accepts at issuance (Platform U12).
