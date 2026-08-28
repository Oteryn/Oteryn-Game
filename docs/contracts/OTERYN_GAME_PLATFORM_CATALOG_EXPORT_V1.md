# Oteryn Game → Platform Game Catalog Export v1

- Contract ID: `oteryn.game-platform-catalog`
- Schema version: `1.0.0`
- Native authority ID: `oteryn-native`
- Producer owner: `Oteryn/Oteryn-Game`
- Consumer owner: `Oteryn/Oteryn-Platform`
- Coordination ID: `OTERYN-GAME-PLATFORM-CATALOG-V1`
- Status: **PROPOSED until this exact contract is merged to protected `main`**
- Production publication: **NOT ENABLED**

## 1. Purpose

This contract freezes the first deterministic Game-owned physical snapshot profile that Platform may implement as an inactive native Game Catalog consumer.

It implements the producer-side choices intentionally deferred by Platform's accepted native semantic boundary at `Oteryn/Oteryn-Platform@20f8aac95ae1b890ec6ebe8a705dda7dfb6674d4`, `docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md`.

This contract does not make Platform authoritative for gameplay content and does not make Atlas, Canary, CrystalServer, Wiki, migration evidence or display names native content truth.

## 2. Decision timing

**Must decide now:** yes. Platform cannot implement an exact native consumer while producer bytes, capability vocabulary, identity form and failure semantics remain unspecified.

**Not decided here:** final Oteryn World Project/Bundle encoding, production signing/auth transport, deployment, retention service, broad content population, or production activation.

A successor version is justified by measured scale, new native content families, authenticated transport requirements, or a proven semantic incompatibility.
## 3. Authority and source boundary

Only canonical native Oteryn content may populate authoritative records.

For schema `1.0.0`, `source_revision` is an exact lowercase 40-hex commit SHA from `Oteryn/Oteryn-Game`. It pins the Game source state from which the normalized native catalogue input was derived.

`authority_epoch` is a Game-issued opaque stable token. It is not inferred from a branch, timestamp or release label. The committed fixture value `preproduction-unallocated` is deliberately non-production and grants no activation authority.

`ruleset_id` and `content_profile_id` are stable namespaced Game identifiers. A consumer must not replace either with protocol version, display version or legacy datapack identity.

The v1 producer input boundary is normalized `NativeCatalogInput` derived from the canonical native semantic graph/registries. The current task implements validation, canonicalization and artifact production; a broad runtime/content adapter is a later Game task.

## 4. Stable identity

Entity and relation identity uses native namespaced keys compatible with accepted `ContentKey` semantics, for example:

```text
oteryn:item.currency.crystal_coin
oteryn:creature.dragon
oteryn:relation.dragon_loot_crystal_coin
```

Legacy numeric IDs, Canary/TFS IDs, client IDs, file paths and display names are never canonical native identity. They may appear only inside explicitly provenance-scoped data fields in a later reviewed capability contract.

Schema v1 rejects non-namespaced canonical keys.
## 5. Capability taxonomy

Schema v1 has this closed capability vocabulary:

- `item`
- `creature`
- `creature_loot`
- `npc`
- `npc_shop`
- `spell`
- `quest`
- `achievement`

Each capability has separate `support` and `completeness` state.

`support` is one of `supported | unsupported`. `completeness` is one of `complete | partial | unknown`.

An unsupported capability MUST have `unknown` completeness. A required capability MUST be declared `supported` or production fails closed.

`partial` and `unknown` never prove authoritative absence. Only a `supported + complete` capability may carry authoritative tombstones for that capability.

An empty array under `partial`, `unknown` or `unsupported` means only that no records are present in this snapshot; it does not mean the native game has none.
## 6. Snapshot envelope

Canonical JSON contains exactly these top-level fields:

```text
contract_id
schema_version
snapshot_id
content_authority_id
authority_epoch
source_revision
generated_at
ruleset_id
content_profile_id
required_capabilities
capability_manifest
completeness_manifest
entities
relations
tombstones
payload_digest
```

`generated_at` is caller-supplied RFC3339 UTC to whole seconds. The producer never reads wall clock implicitly.

`payload_digest` is `sha256:<64 lowercase hex>` over canonical JSON of every field above except `snapshot_id` and `payload_digest`. Therefore provenance, including `generated_at`, is integrity-protected.

`snapshot_id` equals `payload_digest` in v1 and is content-addressed identity for the exact immutable snapshot semantics.
## 7. Record shapes

Entity records contain exactly:

```text
type
content_key
capability_id
data
```

Relation records contain exactly:

```text
type
relation_key
capability_id
source
target nullable
data
```

Tombstones contain exactly `content_key`, `capability_id`, and a bounded `reason`.

Entity keys are unique across one snapshot. Relation keys are unique. Every relation source and non-null target must resolve to an entity in the same snapshot.

An entity and tombstone for the same `content_key` is contradictory and rejected. Duplicate tombstones are rejected.
## 8. Canonical serialization

The physical v1 profile is UTF-8 JSON with:

- object keys sorted lexicographically;
- arrays canonicalized by semantic identity before serialization;
- compact separators `,` and `:`;
- Unicode emitted directly rather than ASCII escaping;
- no NaN/Infinity;
- no floating-point values in schema-v1 record data;
- signed 64-bit integer range only;
- exactly one LF after the serialized document when written as an artifact.

Equivalent unordered source collections must produce byte-identical snapshot bytes when all declared inputs, including `generated_at`, are equal.

Duplicate JSON object keys are invalid at the file-input boundary and fail before semantic processing.

## 9. Hard bounds

Producer v1 enforces these absolute maxima:

- input or output snapshot file: `268,435,456` bytes;
- capabilities: `256`;
- entities: `200,000`;
- relations: `1,000,000`;
- tombstones: `200,000`;
- one UTF-8 string: `2,048` bytes;
- nested record-data depth: `16` levels;
- one record-data object: `4,096` members;
- one nested record-data array: `200,000` entries.

A successor schema is required to increase an incompatible limit.
## 10. Fail-closed rules

Producer v1 rejects at least:

- missing or unknown top-level fields;
- unsupported contract/schema/authority identifiers;
- malformed source revision, timestamp or namespaced identity;
- unknown v1 capability IDs;
- duplicate required capabilities or manifest entries;
- mismatched capability/completeness manifest coverage;
- unsupported required capabilities;
- records assigned to unsupported/undeclared capabilities;
- duplicate entity/relation/tombstone identity;
- dangling relation endpoints;
- contradictory entity/tombstone identity;
- tombstones for incomplete capability coverage;
- floats, oversized strings/integers/collections/depth;
- malformed UTF-8/JSON and duplicate JSON object keys;
- digest or snapshot-ID mismatch.

No validator may repair or infer missing authoritative facts.

## 11. Transport and integrity

The first supported transport is an offline/local build artifact:

```text
game-platform-catalog.json
game-platform-catalog.json.sha256
```

The sidecar is lowercase SHA-256 over the exact JSON artifact bytes including final LF. It protects transport bytes independently of the in-document semantic payload digest.

This profile is suitable for repository/CI compatibility work and inactive Platform import. It is **not** an authenticated production publication channel.
## 12. Production gate

Schema support does not authorize production activation.

Production publication remains blocked until a separately reviewed Game transport/publication profile defines producer authentication, artifact origin trust, retention, publication atomicity and rollback evidence, and Platform separately authorizes the exact inactive candidate.

An authenticated transport profile may wrap these exact snapshot bytes. It must not rewrite their semantic identity or silently substitute another authority.

## 13. Current implementation and fixture status

Reference implementation:

`tools/game-platform-catalog/producer.py`

The committed `fixtures/unsupported-native-input.json` intentionally declares every v1 capability `unsupported/unknown` against Game main `a2a5da955dd8f580c9e768c8ac6a741db388cb22`.

That fixture proves the negative semantics: current absence of a broad native catalogue is represented as unsupported/unknown, never as authoritative empty.

Unit tests may use synthetic supported records solely to exercise validation and determinism. Synthetic fixtures grant no native availability/completeness claim.

## 14. Consumer obligations

A Platform consumer for schema `1.0.0` must verify contract/schema/authority identity, exact payload digest, capability/completeness manifests, stable keys, endpoint integrity and its own resource limits before persisting an inactive candidate.

Unknown required capabilities fail closed. Unsupported, partial or unknown capabilities must remain visibly non-authoritative and cannot create public not-found/removed truth.

Automatic import does not mean automatic activation. Exact candidate activation/rollback remains Platform-owned and separately gated.
## 15. Capability payload evolution gate

Schema v1 freezes the envelope and bounded record carriers, not every future capability payload field.

The `data` object is a bounded transport carrier. A capability may be promoted from `unsupported/unknown` to a production-relevant `supported` state only when Game owns an explicit capability payload contract and Platform has matching fail-closed validation for that payload revision.

Until that evidence exists, the capability remains `unsupported/unknown`; the generic `data` carrier is not permission to publish ad-hoc authoritative fields.

A capability-specific additive contract must preserve stable native identity, provenance, completeness semantics and the v1 no-fallback rule. A breaking payload change requires a new schema/capability revision and explicit consumer compatibility evidence.

## 16. Cross-repository operational semantics

### Authentication, audience, expiry, replay and revocation

For this offline/local v1 artifact transport these session/credential concepts are `NOT_APPLICABLE`: there is no network endpoint, bearer credential, user session or replayable authorization token.

This is not permission to omit authentication from production distribution. Any production publication/transport successor must authenticate producer origin and define audience, freshness/replay handling and revocation/retirement behavior before it may be enabled.

### World/channel and revision fences

The artifact is content evidence, not Game Session admission authority. It does not bind or authorize a player `WorldId`/`ChannelId` session. Its authoritative fence is the exact `content_authority_id + authority_epoch + source_revision + ruleset_id + content_profile_id + snapshot_id` tuple.
A Platform profile may activate a snapshot only when its own profile/ruleset selection explicitly matches that fence. Platform must not infer world/channel eligibility from catalogue presence.

### Observability and redaction

The reference CLI prints only the payload digest on success and a bounded validation error on failure. It does not log raw entity/relation payloads, source files, credentials or environment values.

Future telemetry may record contract/schema, snapshot/digest, authority epoch, source revision, capability states, validation category and bounded counts. It must not make raw rejected payloads ordinary log material.

### Stable failure categories

Consumer/producer diagnostics map to these semantic categories:

- `INVALID_INPUT` — malformed schema, identity, record or contradictory evidence;
- `CAPACITY_EXCEEDED` — a registered hard bound is exceeded;
- `UNSUPPORTED_CAPABILITY` — required schema/capability semantics are not supported;
- `INTEGRITY_MISMATCH` — snapshot/payload/transport digest verification fails;
- `PROVENANCE_CONFLICT` — authority/revision/profile facts conflict with the intended candidate.

Implementations may use more specific local codes but must preserve these failure classes and never downgrade them to warnings when they gate authoritative interpretation.

### Rollout, mixed versions and rollback

The rollout classification is `server-first-safe` for contract/producer availability: merging Game v1 does not change runtime or Platform behavior. Platform then implements an inactive consumer for this exact schema.

Required order:

1. merge and validate Game contract/producer v1;
2. lock the exact merged Game revision/digest in the cross-repository registry;
3. implement Platform inactive import/validation for schema v1;
4. prove producer/consumer fixture compatibility and rejection cases;
5. add native capability adapters only after authoritative content evidence exists;
6. activate an exact candidate only through the separate Platform production gate.

An older consumer must reject an unknown schema or unknown required capability; no silent downgrade or field dropping is allowed. A newer producer must preserve v1 bytes while claiming v1, or emit a successor schema.

Rollback never edits a snapshot. Platform selects a previously retained validated compatible immutable snapshot. Production transport rollback/retirement remains deferred to the later authenticated publication profile.

Named negative scenarios include malformed/oversized artifact, duplicate native identity, dangling relation, unsupported required capability, incomplete-capability tombstone, contradictory tombstone, digest tamper, provenance fence mismatch and unknown successor schema.
