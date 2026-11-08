# Integrated producer contract notes

Package GPROD-PKG composes all three deliveries below on current main. The delivery
sections retain their original scope/history; statements about another producer being
absent describe that delivery's base, rather than this combined package.

The configured Bazaar and deletion producers and the default T94 section retain their
delivered ON settings. All supervision, authority and recovery paths remain composed.
The internal `PublishAccountClosureCommandReceiptV2` transport tag uses the existing
Character command certificate, receipt URL and wire operation; it selects deletion's
2048-byte request bound while CC v1 retains its 1024-byte bound. Bazaar's separate
receipt extension keeps its delivered bound. There are no new public wire codes.

The shared HTTP parser selects header requirements and retained retry-delay limits
independently: Store keeps its required header and 30-second limit, Bazaar retains
up to 30 seconds before its consumer applies the 10-second backoff cap, and T94
keeps optional headers and a 10-second limit. Pending deletion jobs reserve the same
Character operation namespace as transfers and creation. Bazaar/T94 client identities
and deletion/T94 epoch files remain separate when these producers run together.

## Delivery gbaz

# CONTRACT-NOTES — GPROD-BAZAAR

## Owner instruction and rollout

The latest owner instruction for this task supersedes the candidate contract's
original default-off rollout requirement: a configured Game Bazaar producer is
**enabled by default**. `platform.bazaar.enabled = false` remains an explicit
operational pause for the dedicated Bazaar command feed. Retained receipts,
custody and funding continue to reconcile; a pause never releases value.
`game_bazaar_configuration.enabled = false` is the durable authority pause across
both command feeds, including lock-bound CC v1 ownership transfers. Both switches
default true. Pause that durable gate when stopping all new authority effects;
the node switch alone does not disable the separately configured CC consumer.

An omitted `[platform.bazaar]` table means the deployment has not supplied the
required dedicated transport identity. It is not an implicit product feature-off
setting. Existing node configurations without Bazaar credentials remain parseable;
the shipped Synology node template now supplies the enabled Bazaar and Character
command sections. A deployment that selects those sections must provision valid
credentials before startup. No identity or private key is fabricated or silently
borrowed from another purpose.

## PLATFORM CONFIG

The Game-initiated connection uses existing `[platform]` settings:

| Key | Meaning |
|---|---|
| `endpoint` | Configured private Platform mTLS socket address; no request-selected endpoint |
| `peer_name` | Expected TLS server name and HTTP Host |
| `trust_roots_file` | Existing provisioned Platform CA roots |

The shipped `deploy/synology-game/node.toml.template` uses these purpose-specific
settings. `<BASE>` is the existing deployment substitution, not a runtime default
secret location.

```toml
[platform.character_commands]
client_certificate_file = "<BASE>/node/secrets/character-commands.crt"
client_key_file = "<BASE>/node/secrets/character-commands.key"
source_authority = "oteryn:character-authority:primary"

[platform.bazaar]
enabled = true
client_certificate_file = "<BASE>/node/secrets/bazaar.crt"
client_key_file = "<BASE>/node/secrets/bazaar.key"
source_authority = "oteryn:game:primary"
```

`enabled` defaults to `true` when omitted from a configured Bazaar table.
Certificate/key paths and `source_authority` are required values. Paths must be
absolute and purpose-separated. Certificates are checked for shared public keys
against the evidence, Character receipt, runtime-status and projection identities;
changing a path cannot evade that check. Existing private-file ownership and mode
checks apply. Platform must independently authorize the dedicated Bazaar peer for
its six routes, and the Character peer for the existing receipt route.

Bazaar `source_authority` is the Game projection/snapshot namespace. The durable
snapshot policy's `snapshot_source_authority` must equal it. Receipt
`source_authority` remains the configured Character command namespace. Neither
field selects trust. The actual provisioned namespaces may differ from the
synthetic fixture examples, provided both sides configure the same bindings.

The authentication names are purpose/authority identifiers, not configurable
endpoints or credentials:

| Binding | Exact identifier |
|---|---|
| Dedicated Game Bazaar mTLS descriptor purpose | `OTERYN_GAME_BAZAAR` |
| Existing Game Character command/receipt mTLS descriptor purpose | `OTERYN_GAME_CHARACTER_COMMANDS` |
| Bazaar intent issuer | `OTERYN_PLATFORM_BAZAAR_AUTHORITY` |
| Bazaar intent audience and snapshot issuer | `OTERYN_GAME_BAZAAR_AUTHORITY` |
| Snapshot audience | `OTERYN_PLATFORM_BAZAAR_AUTHORITY` |
| Existing CC v1 transfer intent issuer | `OTERYN_PLATFORM_CHARACTER_AUTHORITY` |
| Existing CC v1 transfer intent audience | `OTERYN_GAME_CHARACTER_AUTHORITY` |

The pending-command feed and operation-state comparison feed are the HTTP
operations named below, not broker topics. The contract allocates no new named
broker/Gateway topic. Its registered audit family remains
`BAZAAR_AUTHORITY_DECISION`, event type `6`, schema `1`, in the existing ANL
`EventEnvelope`/`TransactionEventRef` facility. Deployment authorizes the actual
provisioned mTLS certificate subjects for these purposes; these identifier strings
never select a certificate, public key or peer allowlist from request data.

All methods below are POST; Game initiates every connection. There is no inbound
Game listener or Bazaar receipt endpoint.

| Operation | Compiled Platform path | Request / response cap |
|---|---|---|
| `ListPendingBazaarCommandsV1` | `/internal/v1/bazaar/commands/pending` | 512 / 8192 bytes |
| `ReadBazaarCommandIntentV1` | `/internal/v1/bazaar/commands/read` | 512 / 4096 bytes |
| `ListBazaarCommandStatesV1` | `/internal/v1/bazaar/commands/reconcile` | 512 / 12288 bytes |
| `PublishBazaarTransferabilityV1` | `/internal/v1/bazaar/transferability` | 262144 / 256 bytes |
| `PublishBazaarListingStatusV1` | `/internal/v1/bazaar/status` | 2048 / 256 bytes |
| `PublishBazaarWatermarkV1` | `/internal/v1/bazaar/watermark` | 512 / 256 bytes |
| `PublishCharacterCommandReceiptV1` (CC v2 body) | `/internal/v1/game-auth/character-commands/receipt` | 270336 / 256 bytes |

The larger CC v2 receipt cap is selected by an internal transport operation tag;
it changes neither the wire operation name nor the endpoint or identity. Existing
CC v1 publication retains its 1024-byte request cap and bytes.

CC receipt acknowledgements remain exactly
`{"contract_version":1,"result":"accepted"}` as CC §4.3 requires. The Bazaar
proposal explicitly preserves that acknowledgement. A v2 receipt body does not
invent a v2 receipt acknowledgement. Bazaar projection/watermark acknowledgements
are exactly v2 `accepted` or `superseded`; command receipts never accept
`superseded`.

Connections use TLS 1.3 and HTTP/1.1. Connect deadline is 1 second, handshake 2
seconds, command exchange 3 seconds, and ingestion 5 seconds. Both discovery and
reconciliation cursors rescan after at most eight full pages or 30 seconds. One
exchange is in flight per consumer. Poll cadence is 1 second and retry backoff is
bounded at 10 seconds. The HTTP parser preserves a valid positive `Retry-After`;
the Bazaar consumer applies the contract's 10-second ceiling. All responses
require `Cache-Control: no-store`.

Receipt 404/409 retains the Game receipt and stops that key's automatic
publication with an alarm. Malformed/authentication/over-bound refusals require
repair and also retain evidence. Network errors, 429 and 503 retry the same bytes;
none supplies a terminal Game refusal. A lost acknowledgement replays stored
bytes. Watermarks come only from durable completeness/resync evidence, never from
an empty pending feed or an HTTP success.


### Authenticated intent-read conflicts

The candidate inherits CC §4.2/§5.1 operation-identity decisions, while the pinned
Bazaar proposal says an HTTP read failure does not prove that a Game effect is
absent. The implementation preserves both boundaries: an authenticated empty-body
read `409` becomes typed internal `Input::ReadConflict`, passed to the Game durable
authority. The transport itself never manufactures a receipt. Under current
process/recovery fences and operation serialization, Game first preserves any
existing immutable decision. Only an unseen operation can receive a new durable
`CHAR_CMD_OPERATION_CONFLICT` refusal, with no effect, so that attempt cannot
later commit. It does not release or reverse pre-existing asset custody or funds.

An oversized read `200` similarly becomes typed `Input::Oversized` and can produce
Game's own durable `CHAR_CMD_INTENT_INVALID` refusal after the same identity
checks. These technical inputs use separate internal binding tags; no
caller-supplied byte marker is interpreted as trusted transport evidence. A read
`404` leaves the attempt PENDING/unavailable and alarms; it never supplies
no-late-commit or asset-absence proof. Receipt-publication `404`/`409` continues to
retain the original Game receipt and stop automatic publication for that key.

## Qualification and reference identities

The identifiers, key IDs and public keys under
`docs/contracts/fixtures/bazaar/` are synthetic conformance inputs. They are not
production account allocations, peer subjects or signing trust. Provisioning real
mTLS and snapshot signing credentials remains a normal deployment dependency;
the implementation does not promote fixture keys into deployment trust.

The transport tests drive the actual Consumer with a deterministic exchange and
exercise exact schemas, closed routes, existing CC v1 cap preservation, explicit
pause/drain behavior, receipt-conflict retention, no-store responses and bounded
Retry-After handling. The mounted dispatch qualification invokes the same
production `dispatch_bazaar_intent` helper used by the node adapter after fresh
account-evidence refresh. PostgreSQL and full fixture execution evidence belongs
in the final task report; these notes make no separate test-pass claim.


### Durable producer policy defaults

No new environment variable is read. `OTERYN_TEST_POSTGRES_ADMIN_URL` is the
existing private test route, not a deployment setting. Deployment policy is the
single operator-owned row `game_bazaar_configuration`: `enabled` defaults **true**;
`policy` defaults `{}`, interpreted through the following typed defaults. Runtime
and Platform SQL identities cannot change that row.

| JSON policy key | Default |
|---|---|
| `obligations_revision` | `1` (the conservative implemented obligation matrix) |
| `cooldown_seconds` | `2592000` (30 days) |
| `snapshot_survives_transfer` | `true`, supported Character-owned snapshot profile only |
| `escrow_account_id` | `null`; a real Platform-issued AccountId must be configured |
| `escrow_capacity` | `64`, checked again when transferring into escrow |
| `inbox_capacity` | `10000`, exact durable Inbox entries; no carried-inventory preflight |
| `item_classes` | `["nonstackable","stackable","empty_container"]`, further narrowed by current admitted Content |
| `content_artifact_file` | `null`; configure the actual server-authoritative reference artifact |
| `content_artifact_sha256` | `null`; exact SHA-256 of that file, independently checked against current world activations |
| `gold_fee_bps` | `250` |
| `reserve_incoming_headroom` | `true` |
| `common_restore_scope` | `true`; all Bazaar, item, bank and Character state uses the same Game database/recovery fence |
| `snapshot_source_authority` | `"oteryn:game:primary"`; match `platform.bazaar.source_authority` |
| `signing_key_file` | `null`; configure an external regular private 32-byte Ed25519 seed file |
| `signing_kid` | `null`; explicitly provision an immutable key identifier |
| `signing_trust_generation` | `1`; explicitly re-provision current keys/revocations with the new admitted generation after recovery |
| `signing_public_keys` | `{}`; at most two explicitly provisioned current/retiring kid → lowercase 32-byte public-key hex bindings |
| `revoked_signing_kids` | `[]`; revocation is persisted monotonically and cannot be undone by a later configuration |
| `proof_retention_seconds` | `7776000` (P90D audit policy; unsettled authority/idempotency proof is never deleted by audit expiry) |
| `v2_receipt_bytes` | `270336` |

The signer loads the seed from `signing_key_file`, derives its Ed25519 public key,
and requires that exact key under `signing_kid` in `signing_public_keys`. Key IDs
are explicitly configured; they are not inferred from a payload or reused after
revocation. Platform loads the same public-only allowlist. No deployment public
key or AccountId is invented. Missing provisioning leaves the corresponding
operation PENDING while the enabled consumer continues other work.

Synthetic E2E signing identity: `fixture-example-1`, public key
`e8c30390441a54642382f15a784087a42dc75f85f775fdfc9e2a25b0f08a9806`.
Use it only for the synthetic fixture verifier, never as deployment trust.
Snapshot signing input is `oteryn-v2-bazaar-character-snapshot-v1.` followed by
unpadded base64url of the exact snapshot bytes.

Platform E2E stub fixtures:
`docs/contracts/fixtures/bazaar/README.md` and
`docs/contracts/fixtures/bazaar/decisions.json`. Exact strings exclude fixture JSON
framing and the file's final newline. Existing CC v1 settlement bytes remain on
the existing Character command feed/receipt route.

ANL feed/event family is the existing `BAZAAR_AUTHORITY_DECISION` (event type 6,
schema 1), in the existing `EventEnvelope` with `TransactionEventRef`,
`DURABLE_AUDIT`, `RESTRICTED_PLAYER_LINKED`, and
`PLAT3_AUTHORITY_AUDIT_RETENTION_V1`. There is no newly allocated Gateway HTTP
path or topic. The Game retains complete bounded envelope bytes atomically with
the decision through the same durable read/ack discipline as existing audit
owners. The six Bazaar POST routes above carry custody/policy/completeness;
terminal receipts use the separately authenticated Character receipt route.

### Compatibility decisions

- The source tree had lifecycle `1` only despite the candidate referring to an
  existing locked lifecycle. Migration 0994 adds internal lifecycle `2` for the
  contract's `locked` state, with exact same-transaction transition proof and
  frozen Character writer guards. No registered wire ID changes.
- Accepted CC v1 transferable lifecycle semantics remain available. Crossing a
  Bazaar-locked lifecycle requires the exact stored binding; no generic locked
  bypass is introduced. Bazaar listing/settlement uses that lock-bound path.
- Bazaar snapshot/status/policy/watermark epochs use the reconciled nonrollback
  recovery generation. CC v1 receipt projection epochs retain their existing
  LCFA scope and field semantics.
- The native mint owners store the known typed item-family tag `Item`; the
  candidate wire spells that enum tag `item`. The producer serializes only these
  two known representations as wire `item`. It preserves the complete original
  database definition in custody/split proofs and never rewrites the instance.
  Unknown families and out-of-grammar keys/revisions are refused. Current known
  Content transferability and exact full attribute conservation remain required.
- The supported Character capture profile refuses equipment, allocated Wheel,
  active condition or unresolved obligation state it cannot faithfully export.
  Optional unsupported tabs remain explicit null. Derived required fields come
  from the existing gameplay owners, not configuration-supplied statistics.
- The candidate's default-off instructions and lock rollout wording are
  superseded for this Game producer by the owner's 2026-11-06 default-on ruling.
  Contract candidate status and Platform qualification are not fabricated.

### Recovery evidence

Exact retained receipts replay across recovery generations without re-signing or
rewriting their bytes. New disposition and payout still require current-generation
evidence, as contract section 3 requires; an old receipt alone cannot authorize a
new debit, transfer or release. Common database restore scope is mandatory.

Signing trust is explicitly bound to `signing_trust_generation`. Restoring a backup
cannot re-enable its old signing policy under a newer nonrollback recovery fence:
fresh capture and snapshot trust remain PENDING until Operations re-provisions the
current external key/revocation policy for that generation. Database tombstones are
monotone within a generation; they are not claimed to survive arbitrary database
rollback independently of that external provisioning. Destroy retired seed files
and retain current revocation policy outside the restored Game database.

Restore admission independently checks current escrow custody and frozen attributes,
retained lock/ownership transitions, hold disposition and bank headroom, and exact
linked payout ledger conservation against Game-owned receipts. Historical terminal
items may subsequently move, transform or change attributes through their native
owners; recovery validation uses their immutable transition history rather than
requiring them to remain in the original Inbox. Active-only unique indexes prevent
double custody while preserving prior terminal listings when an asset is resold.

Local Content acquisition is bounded to one blocking reader per process and a
10-second total queue/read deadline. The blocking worker retains its permit after
timeout, so an unavailable filesystem cannot accumulate readers. Timeout leaves
PENDING without any SQL transaction held.

Gold Character payout uses the exact retained ownership-transition proof and its
unchanged settlement receipt, rather than the listing's mutable last-operation
field. Unlock may precede payout; it does not release the delivery-bound hold or
make a valid payout depend on the order of those two saga steps. Listing escrow
receipts never qualify as settlement. Current generation, sale, buyer, seller,
world, Character, reservation, revision, gross and receipt-digest checks remain
mandatory.

Snapshot public keys are also checked against retained admission signing-trust
keys, including retired admission keys. Provision distinct keys for these purposes.

### Publication and restore qualification

Retained terminal receipts are historical decisions, not fresh sale authorization.
Lock receipt publication extracts the exact embedded signed bytes with `RawValue`
and verifies their provenance using the immutable recorded kid/public-key binding,
including a subsequently revoked key. It does not re-sign, reorder or require a
new current signing policy to replay a historical receipt. Current eligibility,
Bind and settlement still require current trust and recovery evidence.

After recovery, retained terminal custody statuses publish under the current
projection floor with their historical snapshot hash unchanged. An active lock
whose capture belongs to an older floor publishes `INVALID` with
`RESTORE_RECONCILING`; other lost policy/trust qualification also publishes an
explicit invalid reason. Neither a terminal status nor that historical hash makes
an old capture eligible for a fresh sale. This permits complete bounded resync
without suppressing terminal history forever.

Publication acquires the same Content and complete admission serialization gates
before projection state, outboxes, custody rows or signing-trust reads. This fixes
the cross-node source/outbox lock inversion while retaining authoritative writer
fences. Acknowledgements still compare exact epoch/revision/body digests.

A verified terminal refusal clears the mutable disposition fields when releasing
all or part of a pending GOLD reservation. Its remaining absolute maximum can
then be replaced normally; immutable refusal/binding/release history is retained.
Internal `PAID` remains distinct to prevent a second disposition; the normative
GoldReleaseFacts wire state remains `ACTIVE` for every nonzero remainder and
`RELEASED` exactly when zero.

Restore qualification checks both directions: surviving holds must have their own
receipts, and retained GOLD handoffs/bindings must have the exact compatible hold,
delivery or payment/refusal history. A surviving delivery cannot reopen over a
missing hold or an earlier unfenced `ACTIVE` hold. Qualification tests simulate
those invalid partial restores only in isolated test databases using ordinary
administrative SQL, restore the exact original row, and requalify before any
further producer action. No trigger is disabled and no accepted fixture funds are
created by those corruption probes.

The durable snapshot and actual node adapter share the same 24-record batch
constant. The dispatch qualification invokes that production snapshot boundary,
reads its exact persisted receipt and verifies a durable digest-matched ACK; a
negative control with the former 32-record caller fails before command polling.

## Delivery gdel

# GPROD-DELETION — Game producer

Base: `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`. Game implementation only;
Platform adapters and cross-repository E2E qualification remain Platform work.

## CONTRACT-NOTES

The owner's 2026-11-06 decision supersedes the candidate's default-OFF rollout
wording: Game's producer is enabled by default, with an explicit disable switch.
This does not assert deployed Platform qualification or authorize deployment.

The candidate requires independently current lifecycle evidence but supplied no
wire for it. Game adds the authenticated lifecycle read below. Its response is
independent of immutable intent fields; expected lifecycle/security/recovery values
are never treated as current evidence. A failed read invalidates cached evidence.
Platform must mirror this small extension. Security remains the existing S1/S2
source; known credential revocation during termination is not a financial blocker.
The lifecycle response separately attests `financial_obligations_clear` (required
boolean; false holds CLOSE/FINALIZE pending; CANCEL can compensate).

Immutable creation roots remain private quarantine backing. The closure membership
relation detaches active ownership without falsifying creation receipts. All active
Character reads, LCFA, profile, online and highscore projections exclude detached
members; root and custody writes are fenced. CANCEL restores the original relation,
revisions and custody hash, without reconstructing items. A runtime owner mints an
unforgeable absence capability only after scanning all actor slots; durability also
requires terminal session, released claims and current scope/node ownership. A kick,
disconnect, timeout or expired lease alone never proves absence.

Unknown reversible domain operations stay PENDING. Defaults accept only independently
vacant house/guild/market/bank domains; occupied house/guild rights, market escrow or
nonzero bank balances or outstanding guild disband claims cannot produce a successful close. No destructive substitute,
automatic succession or bank redistribution is selected. `refuse` for house/guild
turns a confirmed held right into registered OBLIGATION. CANCEL keeps all admission
closed until the entire original manifest is restored. Nonempty FINALIZE requires
configured physical-retention acceptance, name retention and public handoff completion;
empty FINALIZE can complete and records a pending erasure job. Erasure jobs do not
claim the owning domain has physically erased records.

Authoritative receipts and minimal fences are retained independently of P90D family-7
audit. `proof_retention_seconds` is reserved policy metadata: setting it cannot expire
replay evidence. Physical erasure remains `PENDING_DOMAIN_RETENTION`; no deletion of
financial evidence is inferred from account closure. Restoration conflicts remain
closed/PENDING; `restoration_operator` identifies an owner, never grants a bypass.

Migration: **0996** (checked free against main/open PRs). Existing event family **7**,
schema **1**, and errors **5014–5016** are reused. No new capability bits, numeric
command IDs or protobuf message numbers. Resource fragments **1698–1713** register
bounded producer envelopes; no pins or goldens were edited by hand.

## PLATFORM CONFIG

There are **no new runtime environment variables**. Game reads TOML via the existing
`serve --config <file>` interface. All keys below have prefix
`platform.character_commands.account_deletion.`:

| Key | Shipped default |
|---|---|
| `enabled` | `true` (false refuses new effects/PENDING; terminal replay remains available) |
| `epoch_fence_file` | `/var/lib/oteryn/account-closure/epoch-fence` |
| `lifecycle_source` | `platform_pull` (`pending` disables positive evidence; `control_observation` uses independently supplied control observations) |
| `house` | `vacant_only` |
| `guild` | `vacant_only` |
| `market` | `vacant_only` |
| `bank` | `vacant_only` |
| `preserve_custody` | `true` |
| `permit_retirement` | `true` |
| `name_release_after_seconds` | `2592000` (30 days) |
| `physical_erasure_after_seconds` | unset (`None`, domain acceptance pending) |
| `proof_retention_seconds` | unset (`None`, no authoritative replay expiry) |
| `restoration_operator` | unset (`None`, conflict remains closed) |

TOML omits optional unset keys (no `null`). The supplied `producer.example.toml`
installs the command identity section and explicitly enables deletion. The enclosing
Character command identity must be provisioned; a missing identity cannot be replaced
by the security-evidence identity. Existing required keys have no fabricated defaults:
`platform.endpoint` (IP:port), `platform.peer_name` (TLS DNS name),
`platform.trust_roots_file`, `platform.character_commands.source_authority` (configured
Game Character authority namespace), `platform.character_commands.client_certificate_file`
and `platform.character_commands.client_key_file`. Other required server settings remain
those of the existing node config. The optional old CC identity section is unchanged;
when installed, deletion is ON even when its subsection is omitted.

**Public key IDs:** no JSON/JWS `kid` or application public-key ID exists in this
contract. TLS leaf identity is loaded from the configured PEM chain; the PEM private
key must match that leaf. Platform authorizes its pinned/provisioned certificate/SPKI
under purpose `OTERYN_GAME_CHARACTER_COMMANDS`; the descriptor rejects certificate
reuse across other installed purposes. Platform server identity is validated with the
configured trust roots and `peer_name`. Synthetic fixture UUIDs/hashes are not keys.
No key is generated, inferred from a UUID, or embedded in fixtures. Platform must use
the same provisioned Character identity mapping as CC, rather than invent a new kid.

**Transport:** HTTPS/mTLS POST to `platform.endpoint`, Host/TLS name
`platform.peer_name`, purpose-separated Character command identity. All requests use
canonical ordered UTF-8 JSON. Existing CC pending/read/receipt routes carry Close and
Resolve. The command pending-list/read wrappers remain v1; immutable deletion intents,
receipts, private closure snapshots and watermarks are v2.

| Operation | Method/path | Maximum body |
|---|---|---:|
| List pending | POST `/internal/v1/game-auth/character-commands/pending` | existing CC limit |
| Read immutable intent | POST `/internal/v1/game-auth/character-commands/read` | intent response 4096 |
| Publish terminal receipt | POST `/internal/v1/game-auth/character-commands/receipt` | 2048 |
| Read current lifecycle | POST `/internal/v1/game-auth/account-closure/lifecycle/read` | request 512; response 2048 |
| Publish private snapshot | POST `/internal/v1/game-auth/account-closure` | 2048 |
| Publish watermark | POST `/internal/v1/game-auth/account-closure/watermark` | 512 |

Feed/family name: **`account_closure`**. No Kafka/AMQP topic or additional broker auth.
ANL durable outbox family7 uses existing bounded collector read/ack discipline,
`PLAT3_AUTHORITY_AUDIT_RETENTION_V1`, P90D and case-scoped legal holds. It is separate
from the Platform private feed and authoritative receipt retention.

Lifecycle request operation is `ReadAccountClosureLifecycleV1`, source is configured
Game namespace, audience `OTERYN_PLATFORM_CHARACTER_AUTHORITY`. Response source is exactly
`OTERYN_PLATFORM_CHARACTER_AUTHORITY`; canonical
field order and examples are in `lifecycle.json`. Required phase is CLOSE/CANCEL/FINALIZE,
positive decimal lifecycle/source/recovery counters, UUIDv7 account/termination IDs,
`observed_at <= Game now < valid_until`, and validity at most **5 seconds**. Platform
uses its independently locked current lifecycle, echoes the requested current Game
recovery only after reconciliation, and advances observation `source_revision` for a
new observation/time. This revision namespace is independent of immutable intent
`source_revision`. Exact duplicates are usable only within their original validity;
unknown, mismatched, stale or malformed evidence remains PENDING. No source deadline
or retention counter is extended by retry.

Projection ack: exact `{"contract_version":2,"result":"accepted"}` or `superseded`.
Receipt ack requires `accepted`; receipt404 or conflict409 quarantines that publication until explicit
resync. Epoch high-water file is external to Character backup: provision `0\n` only on
a genuinely fresh deployment, then retain it across recovery. Missing/malformed files
block publication. Control `game_account_closure_resync(new_epoch)` must select an epoch
above Platform's accepted floor after current recovery reconciliation; snapshots and
watermarks refuse a recovery generation until this explicit control resync. Historical
receipt requery remains byte-identical. Bounded full enumeration republishes original linked
receipts before new outer-epoch snapshots. Proof bytes are never rewritten.

**Platform E2E stub fixtures** (relative to repository root):

- `docs/contracts/fixtures/account-deletion/closure.json`: all original exact wire strings
  and SHA-256 values, including Close proof, private projection and cancellation-before-close.
- `docs/contracts/fixtures/account-deletion/lifecycle.json`: exact request/response vectors
  and hashes for independent lifecycle reads. Times are synthetic; use a controlled clock.
- `docs/contracts/fixtures/account-deletion/producer.example.toml`: ON config fragment;
  PEM paths are provisioning placeholders, not credentials.

## Evidence and remaining owner choices

Focused PostgreSQL cases exercise durable restart, fixture-byte output, replay,
exclusive resolution, stale node/recovery, real actor retirement/released claims,
partial detach then restart/CANCEL, LCFA lag, epoch resync, FINALIZE, audit expiry and
legal holds. Exact filters and measured durations are recorded in the delivery report.
No full PostgreSQL suite or full nextest is required.

Open choices remain configurable: occupied house/guild reversible adapters; market/bank
final disposition; physical retention acceptance; authoritative proof-retention policy;
restoration operator; request-time Platform blocker relaxation (unchanged); PKI/deployment
provisioning and cross-repository exact-head qualification. They are not bypasses.

Read-only donor fetches succeeded for Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`
and CrystalServer `201189f871d23597b82caf88fd611454a2ddefb8`, both
`src/io/iologindata.cpp`. Their legacy persistence does not define Platform authority.
Wiki checks: **wiki-pending**. NEED-FETCH: none.

Read-only comparison against `claude/happy-davinci-ijz3wy-srvpkg4` at
`5e60c19701fe3acdda0a3d1331da1239440a9d1e` found no deletion producer to duplicate.
Shared host/authority/carrier files receive only additive deletion hooks; no NX, SRX,
WORLD-SRC, quest, casting or other gameplay-row implementation is imported.

The Game-side review clarified three edge cases without weakening fences: an early
FINALIZE stays PENDING until its matching Close commits; reissuing a completed
CANCEL after recovery reconfirms its historical outcome/current LCFA pair without
undoing subsequent gameplay; and pre-existing Market owners may finish their
conserved positive-credit settlement before detach. The bank exception requires
current-transaction Market settlement/entry evidence and a pre-existing owned offer;
new offers and unrelated bank operations remain refused. These are Game producer
corrections, not new Platform decision authority.

Private publication polls independently once per second, with one in-flight mTLS
exchange in that publisher and bounded 429/503 backoff up to ten seconds. Normal
watermark attempts become due after five seconds (within the contract's ten-second
cadence); blocked changes never advance completeness. The separate CC consumer
retains its own one-exchange bound. A slow/unavailable pending list cannot starve
private receipt replay or heartbeat attempts.

LCFA, profile and affected-world highscore publication hold restored members while
CANCEL is partial. This prevents publishing a revision whose contents would change
without a revision increment when the final restoration transaction reopens admission.
Unreconciled nonterminal leases remain PENDING; no new timeout/absence inference
bypasses their existing admission/session release owner.

The 256-job pending envelope reserves 128 slots per command family. A full Close
backlog cannot consume CANCEL/FINALIZE capacity. Receipt/snapshot acknowledgements
and conflict quarantine bind both the indexed identity and exact stored body;
paged delivery/expiry/source/work queries have matching partial indexes.

The original `scheduled_for` is durably bound per account/termination/Close generation,
separately from technical attempt TTL. A fresh operation key cannot move that deadline;
a contradictory retry receives `CHAR_CMD_INTENT_INVALID`. A delayed watermark conflict
from an older epoch cannot quarantine an explicitly resynchronized newer epoch.

The shipped fragment also explicitly enables/provisions the existing dependent
feeds; their acknowledgements are required before closure freshness can advance.
For each prefix `platform.account_characters.` and `platform.public_projections.`,
keys `client_certificate_file`, `client_key_file`, `source_authority` and
`epoch_fence_file` are required (no fabricated credential defaults); exact delivery
values are in `producer.example.toml`. The public prefix also sets `enabled=true`.
Its older standalone contract's omitted-field default is false; this delivery's
shipped fragment overrides it ON. LCFA runs when its identity section is installed.
These credentials are distinct from CC and native evidence. Their mTLS purposes are
`OTERYN_GAME_ACCOUNT_CHARACTERS_PROJECTION` and `OTERYN_GAME_PUBLIC_PROJECTIONS`,
respectively; neither has an application `kid`. Their high-water files are separately
retained outside Character backup, with the same fresh-only provisioning rule.

Dependent transport, all HTTPS POST to the same configured Platform endpoint:
`/internal/v1/game-auth/native-account-characters` and
`/internal/v1/game-auth/native-account-characters/watermark`;
`/internal/v1/game-public/character-profile`,
`/internal/v1/game-public/highscores`, `/internal/v1/game-public/world-online`,
`/internal/v1/game-public/world-totals`, and
`/internal/v1/game-public/watermark`. They retain their existing v1 contracts and family names
`character_profile`, `highscores`, `world_online`, `world_totals`, and LCFA's account-character feed.
They do not use the CC identity. Closure does not bypass a disabled/unavailable
or unacknowledged dependent feed; it remains PENDING/stale.

## Delivery t94

# GPROD-T94 — Game delivery notes

Base: `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`. Game only; no Platform files, commits or pushes.

## CONTRACT-NOTES

The owner's 2026-11-06 instruction supersedes the candidate's default-OFF rollout text: the shipped node template, deserialization default, producer and database control start enabled. Explicit disabling refuses publication. The contract lock remains candidate; this change does not assert that Platform has qualified its adapter.

The fixture sequence omits LOCKED between SCHEDULED and SETTLED. All supplied wires remain byte-exact standalone delivery goldens. Production inserts LOCKED and advances the revision before settlement; the disconnected schedule→settlement fixture pair is refused as an authority transition. No fence is relaxed to accommodate the fixture numbering.

There are no concrete owner-approved positive scoring rules or advance aggregation values in the open questions. Configuration retains these choices. Defaults publish actual reset/raid/boss history as ineligible VOID, never retrospective predictions. All target Channels are required for an approved advance binding; no first-Channel winner. `EventBinding.channel_policy` defaults to `UNANIMOUS_OR_PENDING`; an explicitly approved `ALL_TARGETS_OR_VOID` policy can withdraw a mixed terminal aggregate as SOURCE_UNAVAILABLE. Native cancellation and source loss intrinsically yield their VOID reasons, independently of positive option mappings. Boss settlement requires the actual placed boss-death receipt. Placement failure, timeout and lost ownership cannot stand in for a boss kill. Native despawning completion without the promised boss-death receipt produces native SOURCE_UNAVAILABLE evidence for a VOID fallback; a retained actual death receipt is preserved. No generic completion authorizes a killed outcome. Persistent bosses keep their source until actual death or source loss; closing a non-despawning raid schedule is not boss loss. An advertised event can lock and await a native receipt indefinitely; scheduled_end alone never settles it.

Default seasons use a persisted rolling UTC-second calendar: first start is bootstrap time +3 seconds, duration 86400 seconds (configurable range 60..31536000), freeze at end +1 second. Missed calendar periods are backfilled one per bounded pass; completeness is withheld until the calendar is caught up. Expired native tombstones suppress recapture rather than reopening old reset questions. The default challenge is `world:gathering`, ruleset/scoring revision `t94:zero-score-season-v1`, max_score=1, no positive source rules and no eligible guilds. This produces a real OPEN/FROZEN empty standings record, not fabricated guild points. Positive `SeasonBinding.rules` and eligibility must be approved before starts_at. A committed native kill uses a fixed rule award, deduplicated per season/challenge/occurrence/guild; paid/XP modifiers are not score inputs. Attribution defaults to `PRINCIPAL_ONLY` (the fenced native kill principal); an explicit pre-season `SeasonBinding.attribution=QUALIFIED_PARTY_ROSTER` can select the qualified native party roster. This does not use premium or final XP amounts as a participation test. Membership is captured inside the fenced source transaction and never reattributed by later joins/leaves. Source acceptance time is the database cut, rather than a delayed wall-clock combat timestamp.

Bindings are Game-owned database configuration, not Platform input. `SeasonBinding.attribution` defaults to `PRINCIPAL_ONLY`; allowed explicit alternative is `QUALIFIED_PARTY_ROSTER`. `game_t94_bindings.binding` stores the strict `EventBinding` / `SeasonBinding` JSON types; `approved` defaults false. Approval cuts are stamped by Game database triggers at insertion/false-to-true approval and cannot be backdated after approval. Correction acceptance after a declared closed deadline receives an immutable refusal; accepted earlier requests retain their approval cut on replay. The runtime creates conservative default bindings. Administrative positive bindings must supply exact native content, occurrence, source, immutable rules/calendar, options and complete target Channels. Published bindings and started seasons are sealed against rule/calendar/eligibility rewrites. `game_t94_corrections` accepts explicitly approved withdrawal/restoration of an existing contribution, never arbitrary scores; its terminal receipt is immutable. Final freeze refuses further corrections.

Unresolved decisions remain configurable: positive event mappings/windows/aggregation, eligible guilds, positive challenge rules/maxima, season calendar, correction deadlines, disband tombstone and actual Fury realization availability. Missing names withhold display (`[]`); they never zero scores or change authority to VOID. FROZEN records are immutable; incompatible disband display is withheld pending a versioned policy. A missing Fury realization produces SOURCE_UNAVAILABLE bound to an actual reset receipt; setting availability true does not invent a realization, and the world-family watermark remains PENDING for missing declared gate receipts. Source metadata cannot authorize an outcome.

Native receipts, immutable revisions, evidence, terminal receipts and publication/audit outbox commit under the existing semantic transaction and sealed recovery authority. A source capture semantic error rolls back its savepoint and quarantines T94 without fabricating a result. Projection ACK and audit ACK are independent. Completeness also waits for unprocessed disband display withdrawals. The Bazaar producer is absent from this base; integration must add its installed leaf to the mutual purpose-separation checks when its configuration is introduced, preserving the contract's dedicated-key rule. Event IDs 10/11 and payload tags from PR #3010 are unchanged. No capability bits, gameplay commands or public protocol message numbers are allocated. Migration: 0998 only. Resource fragment ordinals start 1698 and are integration-renumberable.

Delivery uses a durable ten-second family lease with a six-second remaining-time check before a five-second HTTP exchange. Restart retries the same retained bytes. A later LOST cleanup ACK replays the immutable native loss commit timestamp, rather than changing source evidence with ACK time. Native event decision time must be present and positive; observation time is never substituted for a missing native decision clock. A 409 quarantines the authority. Missing/unreadable epoch fence refuses delivery; it is never initialized by the producer. First-time empty-store bootstrap requires an explicitly provisioned trusted F=0 and sealed current Character authority; F>=1 refuses empty-cache bootstrap. Positive guild seasons cannot begin before native capture starts. The T94 file wrapper locks the stable parent directory and rechecks before every persist, refusing stale writers that would lower F. Explicit resync first quiesces both durable family delivery lanes (a busy lane yields a retried PENDING/refusal without changing epoch), requires a UUIDv7 request and advances beyond both database epoch and nonrollback F, persists F before sending, enumerates the complete retained correction lineage, and withholds completeness until staging is acknowledged. Ordinary restart does not mint an epoch. A restored nonempty store must first be reconciled by the Game recovery operator: `game_t94_control.recovery_generation` must equal the sealed Character generation and `reconciled=true`; the resync operation refuses to self-certify that restoration. Default policy configuration is hashed when first installed for a World; changing it requires an explicitly versioned binding/calendar rather than rewriting an active season. P90D expiry clears audit/source/display payloads in bounded batches while keeping minimal identity, hash, nonreuse and terminal controls.

## PLATFORM CONFIG

There are **no T94 environment-variable overrides**. The server uses TOML keys below. PostgreSQL tests use the existing `OTERYN_TEST_POSTGRES_ADMIN_URL` harness variable; it is not a production transport setting.

| Config key | Default |
|---|---|
| `platform.world_event_outcomes.enabled` | `true` (absent section also enabled) |
| `platform.world_event_outcomes.source_authority` | `oteryn:game:primary` |
| `platform.world_event_outcomes.client_certificate_file` | `/etc/oteryn/credentials/t94-client.crt` |
| `platform.world_event_outcomes.client_key_file` | `/etc/oteryn/credentials/t94-client.key` |
| `platform.world_event_outcomes.epoch_fence_file` | `/var/lib/oteryn-fences/t94.epoch` |
| `platform.world_event_outcomes.resync_request_id` | absent; explicit UUIDv7 requests resync |
| `platform.world_event_outcomes.defaults.fury_gate_realization_available` | `false` |
| `platform.world_event_outcomes.defaults.season_seconds` | `86400` |
| `platform.world_event_outcomes.defaults.season_advance_seconds` | `3` |
| `platform.world_event_outcomes.defaults.freeze_delay_seconds` | `1` |
| `platform.world_event_outcomes.defaults.event_window_seconds` | `1` |
| `platform.world_event_outcomes.defaults.correction_seconds` | `1` |
| `platform.world_event_outcomes.defaults.max_score` | `1` |
| `platform.world_event_outcomes.defaults.eligible_guilds` | `[]` |
| `platform.world_event_outcomes.defaults.tombstone_name` | absent (withhold unknown disband display) |
| `platform.endpoint` | required deployment SocketAddr, shared existing Platform endpoint; no invented host/port |
| `platform.peer_name` | required existing verified TLS server name |
| `platform.trust_roots_file` | required existing PEM trust roots path |

The Synology template explicitly enables T94 and uses `<BASE>/node/secrets/t94-client.crt`, `<BASE>/node/secrets/t94-client.key`, and `<BASE>/t94/epoch-fence`. These are deployment substitutions for the loader defaults above. Credentials and the initial nonrollback fence must be provisioned by the deployment owner; no embedded credentials, auto-issued trust or fallback identity exist. Keep the fence outside Character database backups. All publishers of the same source namespace must share the same nonrollback fence and a stable lock-capable parent directory; a node-local copy is not an HA substitute. An empty T94 store initializes under the sealed current Character generation only with trusted F=0. New T94 stores use projection epoch 1; provision the initial trusted F=0 file (explicit `0` or `0\n`, meaning no publication has occurred) under the existing EpochFenceFile ownership/durability rules.

Public key ID is exactly `sha256:` followed by lowercase hexadecimal SHA-256 of the **leaf certificate DER SubjectPublicKeyInfo**, exposed as `LeagueDescriptor::public_key_id()`. It is derived at credential load, not an arbitrary configured kid, and changes on key rotation. The matching private key loads from the dedicated key file. Its SPKI must differ from evidence, runtime status, account characters, character commands and public projections. Purpose name is `OTERYN_GAME_WORLD_EVENT_GUILD_SEASON`. Platform must authorize that dedicated mTLS client identity for source namespace `oteryn:game:primary` and these operations. There is no JWT/bearer signing kid, Authorization header, browser route or inbound Game listener.

| Method | Endpoint path | Feed/family | Maximum body |
|---|---|---|---|
| POST | `/internal/v1/game-public/world-event-outcome` | `world_event_outcome` | 16384 bytes |
| POST | `/internal/v1/game-public/guild-season` | `guild_season` | 262144 bytes |
| POST | `/internal/v1/game-public/league-watermark` | watermark for either family | 512 bytes |

Transport is Game-initiated HTTP/1.1 over TLS1.3 with dedicated mTLS and verified Platform peer/trust roots, JSON canonical bodies, one in flight per family. These are HTTP feeds, not broker topics. Responses require `Cache-Control: no-store`, at most 256 body bytes; non-200 bodies are empty. Statuses are 200/400/401/409/429/503. The existing standard PLATFORM error codes 5002–5007 are reused. Retry 429/503 with retained identical bytes and bounded 1/2/4/8/10-second backoff. Connect 1 second, TLS handshake 2 seconds, complete exchange 5 seconds. No per-subject evidence or payload is logged.

Platform E2E stub inputs (unchanged):

- `docs/contracts/fixtures/world-event-guild-season/snapshots.json`: every canonical wire and SHA-256, including schedule, settlement, VOID, correction, final freeze and tied frozen guilds.
- `docs/contracts/fixtures/world-event-guild-season/README.md`: normative replay/recovery/retention scenarios and conformance requirements.
- `docs/contracts/game-events/v1/plat3_authority.proto`: audit payload definitions; family IDs remain in `GAME_EVENT_FOUNDATION_REGISTRY.json`.

Fixture publication tests replay every supplied wire through the publisher and retry/ACK path byte-exactly. They do not turn fixture hashes into native evidence or treat the omitted LOCKED revision as an allowed production transition.

### Game-owned binding configuration

These JSON keys are local Game configuration in `game_t94_bindings.binding`; they are not additional public wire members. Platform mirrors the resulting ruleset meaning and published snapshots.

| Key | Default / requirement |
|---|---|
| `EventBinding.channel_policy` | `UNANIMOUS_OR_PENDING`; explicit alternative `ALL_TARGETS_OR_VOID` |
| `EventBinding.advance_approved` | `false` |
| `EventBinding.channels` | `[]`; a raid/boss advance mapping must name every actual target Channel |
| `EventBinding.outcomes` | `{}`; positive native phase to approved option mapping |
| `EventBinding.void_reason` | absent; explicit no-advance history uses `INVALID_SOURCE`; native cancellation/loss intrinsically carry their VOID reasons |
| `EventBinding.corrections_closed` | `false` |
| `EventBinding.correction_deadline` | absent; no final freeze without a finite configured deadline and complete cut |
| `EventBinding.withdraw_display` | `false` |
| `EventBinding.occurrence_id`, `question_key`, `question` | required actual occurrence, permanent question identity, immutable strict event template |
| `SeasonBinding.attribution` | `PRINCIPAL_ONLY`; explicit alternative `QUALIFIED_PARTY_ROSTER` |
| `SeasonBinding.rules` | `{}`; fixed unpaid award per qualified native creature key |
| `SeasonBinding.eligible_guilds` | `[]` |
| `SeasonBinding.tombstone_name` | absent; withhold unknown disband display |
| `SeasonBinding.corrections_closed` | `false` |
| `SeasonBinding.correction_deadline` | absent; no final freeze without closure |
| `SeasonBinding.complete_through` | `0` (PENDING until a qualified complete cut exists) |
| `SeasonBinding.withdraw_display` | `false` |
| `SeasonBinding.season` | required immutable calendar/challenges template; initial standings empty |
| `game_t94_bindings.approved` | `false`; approval time is stamped by Game and sealed |
| `game_t94_corrections.approved` | `false`; approval cut is stamped, terminal receipt immutable |

`platform.world_event_outcomes.enabled=false` stops publication and interpretation. Already-qualified native capture can retain source inputs for later replay; retention runs independently. Source configuration that lacks accepted evidence stays PENDING. Positive seasons that start before capture bootstrap are refused rather than assigned invented zero scores.

## External evidence

Read-only donor evidence fetched successfully:

- https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/world_changes/fury_gates.lua
- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/world_changes/fury_gates.lua
- https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/creature/raids.cpp

Fury Gates selects/realizes at startup; donor content is not an accepted Game realization receipt or an advance prediction window. Wiki checks: **wiki-pending**. NEED-FETCH: none.

# qengine2 contract notes

Owner rule: bound each investigation or missing-fact search to ten minutes.
Use the documented conservative behavior, implement and test it, and continue.
Push each finished green quest/mechanism slice immediately; do not hold green
work longer than thirty minutes. Only focused checks, no full workspace suite.

Accepted implemented decisions (all PostgreSQL qualified and pushed):

- Dedoras: charge exactly five Sturdy Books or twenty Epaulettes, matching his
  dialogue and the selected canonical recipe. Repair donor strict-greater-than
  checks and the unreachable epaulettes keyword branch. Crystal9f5a72c,
  data-global/npc/dedoras.lua:198-224,
  sha25675b3c6e0215ce23ab7f2f6812a77d71a80d3f4ff1565a33eb3308f3c21310fd1.
- Muriel: his source offers require the addon flag that only those offers set.
  The corpus has no other writer. Bootstrap stage1 only after base completion
  and matching carried evidence, without consuming or granting; the next exact
  hand-in earns the addon. Completed stage2 cannot restart. Crystal9f5a72c,
  data-global/npc/muriel.lua:97-149,
  sha2567bd9180ba903509cb3a4a6fe01a7b73d9038eff99d15c68bf95fed5ec697d4cc.
- Oramond VotingPoints: declare only the missing read-only catalogue fact,
  retaining Oramond ownership. Donor absence is-1; use the conservative signed
  32-bit ceiling2147483647. No player progress is written by this declaration.
  Exact thresholds3/6 and second-addon priority come from Crystal9f5a72c,
  data-global/npc/ezebeth.lua:93-103,
  sha256280ef0cf3d3342ed33e7f156a029f096e32e7a593981099e522256fe47c14e82.

All source blobs remain in the committed pinned donor corpus; no donor copies
or binaries are added. QUEST-STATUS.json retains exact tests and case markers.

- Nobleman: the donor implements the150000 carried-money choice for each addon.
  Its10000-instalment text has no executable payment branch; preserve that
  implemented one-shot behavior. Dress/coat are aliases for the same canonical
  first-addon effect; the appearance owner uses immutable sex for the actual
  look. Hat is the second addon. No bank balance pays a carried-only request.
  Crystal9f5a72c, data-global/npc/irmana.lua:50-135 and male counterpart.

  Exact Nobleman donor citation: crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/irmana.lua:sha256:08e9d63f771a43f2fa147e58f7fb474deef6b541a3a749258ec9a040c8993c03.

- Native Quest payments reuse the complete bounded carried-coin proof profile.
  The new cause must also have its exact same-transaction Quest fee/source
  receipt; no legacy custody, ordering, epoch or Character proof is skipped.

- Royal Costume: donor XML tags looks1456/1457 as store, but the pinned source
  NPC and owner-selected quest recipe explicitly grant them for in-game tokens.
  Admit only Royal Costume as a native quest path; leave other store looks out.
  Both client appearance records exist in the admitted corpus. Exact source
  costs are15000 Silver Tokens22516 +12500 Gold Tokens22721 for base, then
  7500+6250 each for shield and crown, in that order. Do not use the total-package
  30000/25000 dialogue number as the base-only price. Source Crystal9f5a72c,
  data-global/npc/percybald.lua:149-228,
  sha256b617d51111abedf07a4b5de72acc938153152695517ef46b894b7b37873fdadc.

- Pirate keeps the existing shared OutfitAndAddon quest ownership of its source
  base/sabre/hat and DefaultStart tracks. Do not reassign them to the title alias.
  Reuse Morgan's already-delivered firebird stage4->5; implement Duncan's exact
  three100-item hand-ins and both NPC introductions before claiming the whole
  outfit story. Only the Shattered Isles parent objective flags are arranged as
  external prerequisites in the test; no internal outfit stages are seeded.

- L1-05 Oriental: Crystal9f5a72c NPC ishina.lua:60-94 (sha2569eb2e869db4d0dfad8f04fc171773dcf9045e35647bd4069f9e9b18dc0513c71), habdel.lua:60-93 (0666e7207a7fd9565f5311ffc3d87f9b138c858117d802c089a7d00c727c7b78), miraia.lua:57-155 (7114cc53c7119cd7a42947a5be07611956e11e7f7c8833f68197f136be5e353f), razan.lua:57-149 (c12e695654ada3a5f4b1775657835fa18e5957b0870e735803a5ead2e7483acb). Exact comb5945x1; then ape fur5883x100,fish fins5895x100,wings5891x2,blue cloth5912x100. Both gender paths retain the existing shared addon owner. The door retains Oriental owner; source simultaneous write uses a generic atomic participant, never a reassigned track or detached follow-up.
- Source first introduction requires Explorer Society OrcPowder>=35; only that other lane's prerequisite is arranged by the test. Immutable sex guards all reachable authored stages; wrong-sex advisory offers confer no authority and the native commit rejects before writes/costs. Existing physical profiles for four materials are reused; only comb custody was missing. No new donor fact was inferred.

- L1-05B Norseman/Formorgar shared outfit reward: Crystal9f5a72c data-global/npc/hjaern.lua:107-155,189-197 sha2564e9e42e8daf616fe8c5ef71cfd0a4ac0ae0ae5d7bd786c80ccb2c26286d40466. At Questline39 and all four Obelisks5, take frost charm7289x1;write Questline40,Mission12=6,YakchalDoor1 and shared NorsemanOutfit1/DefaultStart1 atomically. Then exact5shards7290 lead40->41,reward41->42/addon1;exact10shards lead42->43,reward43->44/addon2. General owns NorsemanOutfit and DefaultStart;IceIslands retains other flags. Only pre-outfit parentQuestline39/Obelisks5 are arranged;no outfit stage is seeded. Father11 write remains untouched.
- Item name correction: imports/canary/items-xml/items.xml:18927 at pinned sha2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2 names7289 frost charm,19oz,no charges/duration/fluid. Earlier scratch note called it spirit container;IDs/costs were unchanged. Reuse existing7290 physical profile;promote only7289 custody.

- Canonical identity clarification: the admitted source catalogue has Pirate/Norseman outfit paths under `oteryn:quest.outfit_and_addon_quests`;there are no separate native quest records with the earlier scratch dedicated Pirate/Norseman URI spellings. Original L1 title keys/counts are retained;metadata now names the verified shared owner rather than implying a separately admitted record. Norseman progress also uses existing IceIslands owner. This changes no runtime data or finished reward behavior. Evidence: content/quests/missions/quest-state.json source track ownership and the qualified PG receipt/state assertions.

- Nobleman does have dedicated catalogue identity `oteryn:quest.nobleman_outfits_quest`;its source state still uses General. Pirate/Norseman lack separate dedicated records in the admitted definition shards. Native ownership evidence: content/quests/missions/quest-state.json:1 sha25665b74015fd8af6c4f1e4074d5166a5cc56e45d52e0e564da9a83fa202e7f6100. Catalogue identity and state owner are distinct;both are now recorded accurately.

- Hjaern continuous conversation: donor hjaern.lua:129-137 permits the root shard keyword independently of prior topic;rewards at141-155 leave topic3/4. Admit the root question at all reachable L1 topics0/3/4 with disjoint exact Questline40/42 guards. Preserve source next topics and fresh cost guards;never replicate the donor stale-topic yes branch that could charge/rewind after reward. A published-flow regression fails before the additional root offers and passes after.

- L1-06A Rookgaard Druid body: Crystal9f5a72c data-global/npc/tom.lua:230-250 sha2565079fd40c677dcee45941eac3eb4961c1260c5889635585d9d3441829de800b4. Source addon word startsBody0->1;paws/bear paws question and yes hand-in consume5896x50+5897x50,Body1->2,grantDruid addon1 for both sexes. Existing dedicated DruidRook state owner retained;existing material profiles and Tom placement reused. This source path has only body addon1;do not fabricate a second reward. Story independently expects mask1/two account facts and verifies the unearned addon refuses ordinary appearance selection.

- Generic greeting fallback: Tom.lua:61-87 selects four quest-specific greetings and sex-specific farewells. Shared reward greetings now use the pinned NPCHandler neutral defaults whenever a source NPC has no unique static message;functional conditionals retain their owned selectors. Each emitted default cites its exact handler source line/hash. No arbitrary quest branch or gender is guessed,and reward eligibility remains independently fenced.

- L1-06B Citizen Rook: Crystal9f5a72c data-global/npc/amber.lua:69-122 sha25600a81c0a699f019923ec1d906166c7dae13f7f60b4e3fcc321aff41c9b32a7f9;norma.lua:80-118 sha256c1eb0cbfdf9520fe110dd90b07674729d70aeaf69926f8f40a5f65d2983e2105. Amber takes100minotaurleathers5878,thenstrictlymorethan7200seconds before backpack addon1. Internal timer is normalized to start timestamp plus ElapsedAtLeast7201;the public wait is unchanged. Ref lacks native declaration:conservative sharedGeneral owner/initial0/nonnegative i32 bounds,source-backed writable declaration. Increment is native checkedAdd1;finish resets0 as donor math.min does forpositive refs,while retaining0 ifalreadyreset,nevercreatingnegative UI sentinels. Exact quest stages remain independent of Ref.
- Norma takes1legionhelmet3374,50honeycombs5902,100chickenfeathers5890 foraddon2. Source finalMissionHat0 could allow repeated intro;retain completedHat2 guard to prevent restarting/charging a finished outfit. No source costs,body waits or required stages are skipped. Clock-only fixture transition simulates elapsed time after proving fresh stamp/refusal;all outfit stages run literally. Existing materials/NPC placements reused.
- State-operation declarations:default remains read-only;writable requires explicitflag,provenance and every declared writer matching the named owner. Existing native definitions must match exactly. Combinedowned/participant writes remain<=8. Core checkedAdd/SetNow/Elapsed comparison,sortedlocks,capacity and source proofs are reused;SQL1002 validates exact operation preconditions/results andsame physical source transaction.

Beggar source: Crystal9f5a72c data-global/npc/hugo.lua:54-199 sha25660069ed647e4379a2e1177d32652602d86d8d50ad8da066f7dc65529b92fa55d;data-global/npc/simon_the_beggar.lua:84-222 sha2567ad2665ab2a053412536f0dd15762c98fb07122f0db20e020180fecd89003e74. Hugo's strict deadline os.time()+86400 becomes native start+ElapsedAtLeast86401,Simon male uses exact432000>= after beard request;only clocks advance in test fixtures. Native General remains original owner. FirstAddonDoor is absent from the native catalogue:explicit source-backed writable declaration General/initial0/min-1/max1 treats native0 as source-unset-1;both refuse once1. Simon's female gypsy branch grants addon1 but omits advancement,blocking his staff branch;conservative completion writes stage10/door1 like the male completed reward. Shared paid hand-in stamps the existing addon clock for both sexes;female never reads it,male beard request resets it exactly as source. Source staff replay lacks a completion guard;retain its written SecondAddon2 as a one-shot bound. Premium Current is required for Hugo introduction/base and Simon paid/staff hand-ins;Simon first introduction requires the native earned/starter outfit proof. No premium or outfit ownership is inferred from conversation cache.

L1-07 pinned Crystal9f5a72c source ranges/hashes: morgan.lua:115-148 fbef2b0d61842368915e506ff5d2e1e96f00a3f55bb3cad4ee97178824832610;gregor.lua:63-211 f56640cfe0501c564d092d3572b7b065aebaa6e22cb8f5af561a524079c8db98;sam.lua:67-88 ed713aa63d597133355328a16e9c36d1b382c0e1d7e40dc199211f4fc4d14e6b;ustan.lua:63-85 441ed6570ee072e3e75ffa98af95aa653b3d6ec8836327f6a5927833c5faed89;ceiron.lua:60-187 fb4cc1a978148a1ea552764b78930c098afd735af56738433797f20aa17da780;a_majestic_warwolf.lua:64-75 9fa8cee3622cacee85aec7770960d7d1bf46d25569f3a6ef857d0605bc842572;vescu.lua:56-189 af26eb8e9e31d41d3d00f08d03a971cf3136c4c2ffc83317af414e1dcb40262b;erayo.lua:57-114 dc6c54417c8bfdd32abcea9f068322f17d029c4d7722f1acfdeb6cd795a4c48f;atrad.lua:60-105 ffdc0e1b8d0d5455be4be4c6875b3f832dca3a488440ed08020bfa20bd7bde01. All paths data-global/npc/. General remains original owner; missing primary AddonHelmet/RamsayDoor/DruidAmuletDoor/Atrad flags receive source-bounded metadata with native-unset0. Existing declared Ref nonnegative rule also applies to Gregor increment/Sam finish0. Sam strict2h deadline normalizes native start+7201. ApeCity predicate>15 admits16..18,the complete native track domain. Assassin Shipwrecked2 remains read-only under its original parent owner. Source Vescu initial greeting requiresDrunk,Atrad greeting requiresFire: bounded proof samples both existing native condition owners at the value command's admission and binds the exact command/callback/itemfence/nativegeneration. This conservatively rechecks Atrad on each value command rather than retaining an expired greeting condition; no runtime locks are acquired inside SQL. Greeting prose may use the pinned neutral NPCHandler variant,never eligibility truth. PG fixtures apply/clear real native owned conditions;no outfit stages are seeded. Source hand-in prerequisites,quest progression and gifts run literally through NPC dispatch. Knight's existing source hand-in wording is retained;new unowned-addon requirement matches Morgan's explicit hasOutfit check,including account facts from another character.
L1-07 correction from actual PG83.573871312s Druid/79.114808667s Knight failures:two omitted primary door flags inherited the generic unknown-storage sentinel-1,but offers incorrectly required0. Source assignments are unconditional under the main stage guard:explicit writable General declarations now admit initial0/min-1 and assignments accept full-1..1 domain. Knight MissionHelmet assignments likewise admit its full native domain rather than invent an extra stage predicate. Python regression checks assert both door ranges and explicit defaults. Assassin full source path already passed actualPG92.674685234s before this metadata correction. The unowned-addon check takes the existing account appearance grant lock before reading and keeps it through costs/grant,so another character cannot race the negative ownership predicate and still charge. No duplicate account or condition owner is introduced.

## L1-08 source rules and compatibility

- `data-global/npc/elane.lua:55-148`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `beb84839e9b7b878fc276659bcbc0577bd82210f3cf900f53e5ddeb25dcc2cef`.
- `data-global/npc/legola.lua:60-110`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `26084a8748e9d3a01d59745bb30e67fb3610924430ce86034828c893c9b8dfc2`.
- `data-global/npc/angelina.lua:53-80`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `9c9d66498d3880f1c587aea1b43fd44f2c4ceaa9b8b839a4192aaa3dd572ca98`.
- `data-global/npc/lynda.lua:60-245`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `c02b3072ac121d52ccd9e4e6d911942633bddbd9489c9325f66e375e8f5207a9`.
- `data-global/npc/sandra.lua:68-124`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `566fd1c69a69ed447fbf6f56d08603631983cfbc9f203a11c41b71b9bd47ae53`.
- `data-global/npc/myra.lua:62-330`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `5939c5f235ac5f58bff2b2bf6668145e6da0a7ded222d46c3a792c02a386f94b`.
- `data-global/npc/zoltan.lua:60-150`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `c184ead30d167e379ccd628713028fd9cc8acc24713bb7c6943deefce14032d1`.
- `data-global/npc/eruaran.lua:153-199`, crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, sha256 `6da2bd991078e0b7ac9cf96f855f4e931800e5609ba82280ea19bd749ae49936`.

Hunter grants preserve raw male/female addon permutations. Mage grants preserve each explicit source looktype, including cross-family Summoner grants and the female Summoner hat in Zoltan. Legacy neutral account facts remain compatible for both sexes. New qualified facts require matching sex in normal appearance projection and selection.

The male Mage hat has no source quest flag: qualification uses the actual unowned addon predicate under the existing account lock, exactly one carried 5903, and a same-transaction native quest/source/grant receipt. No invented progress flag or completion is recorded. All ten wand/rod types are consumed atomically; the existing bounded recipe limit grows from eight to sixteen.

Conservative donor correction: Eruaran repeats looktype 577 twice for each Dream Warden addon; the canonical outfit family includes male 577 and female 578. The family grant therefore supports both canonical sexes. Premium and each distinct source flag remain required. Missing Lua storage -1 is normalized to native zero for initial one-shot addon flags. Winning lottery tickets are supplied through ordinary native item custody, like other required rare materials; unrelated repeatable vial trading is not an outfit quest stage.

## L1-09 source item grants and paid exchanges

- Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d` `data-global/npc/gnomally.lua:75-169`, sha256 `e598782edbd947acf0de2a13e42b4791536b5b8923d5132841891a77604133b1`:20Major16129 for16252/16255,70Minor16128 for16253/16256; private config[message] selection normalized to distinct ephemeral topics101-104. These are repeatable trades, not quest-progress storage.
- Same revision `data-global/npc/ottokar.lua:61-78`, sha256 `fe8c4a6740a75448c3af035f27c3b2783b9fe91ecad7ba4d255544003370a951`: one Medicine Pouch12517 buys belongings12413 and advances Doctor! Doctor! toward100. Reuse existing achievement owner, with per-command replay protection from the quest/source receipt.
- `data/scripts/actions/items/usable_outfit_items.lua:1-10`, sha256 `69b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37`; `usable_afflicted_outfit_items.lua:1-61`, sha256 `d4a9f639be295771cf2d6f5c19c21435e0ec0ca0cb1684682ce4ad09717cb391`: byte-identical in the existing201189f item-action policy packet and9f5a72c corpus. Preserve existing item-action provenance and exact canonical look-pair matching. No synthetic cosmetics catalogue.

The chosen finite quests accept ordinary legal possession of rare addon vouchers and plague accessories; tests do not assert a loot probability or turn supply packages/belongings into a guaranteed rare drop. The full selected token trades, six-piece combination, Ottokar trade and native USE stages are exercised. A cost-backed value-only receipt is valid only with nonempty positive input and an exact native MINT in the same fenced transaction; the original source completion trigger continues to prove every consumption and MINT. A free gift or detached receipt is not admitted by this arm.

Paid exchange identity: the existing RewardClaim MINT owner remains one-shot per claim, including its pending/reservation exclusion. A source exchange with no storage/state operations/participants and positive inputs plus a MINT gets a deterministic per-command claim key: SHA256(version prefix, fixed-width Character UUID, Session UUID, colon-terminated decimal unsigned CommandId, source callback key). SQL derives the same key and still proves exact same-transaction command, input count and MINT item/quantity. Ordinary phase-bound quest claims retain their existing identity. No claim ledger row is deleted or made reusable.

Scope clarification for L1-09B: the selected Soil Guardian s7 and Crystal Warlord s6 explicitly require supply-package acquisition. Whole-entry qualification therefore also requires ordinary USE of15698 to produce the rare voucher. The original chance table is `data-global/scripts/actions/valuables/random_items.lua:49-77`, Crystal9f5a72c, sha256 `14fc3cf10ca10338aa4479a5371bb3dd7b9b0dede3353a4dec92ffdd11c7023b`. Preserve its1..10001 draw and exact intervals; a deterministic test witness is not a changed drop probability. The earlier legal-possession discussion does not waive these selected package steps.

L1-09B package implementation preserves every one of the25 contiguous source intervals, their exact quantities, and the10001-point roll domain. The existing generic reward-bag consume/MINT owner executes the captured row. Existing richer potion/ammunition profiles are reused; only six missing physical custody profiles are added. Gnomally's source `supply` trade costs2 Minor Crystalline Tokens, gives15698, and is selected by normalized ephemeral topic105; its existing canonical parent is `oteryn:quest.bigfoot_s_burden_quest`.

The PG acquisition witness uses the actual NPC-minted package UUID. Read-only evaluation finds a future ordinary CommandId with the rare outcome, and real harmless NPC commands advance admission to that ID. Neither the source item identity nor the production roll/probabilities is rewritten, and no command admission is skipped. The package is then consumed by ordinary USE; replay cannot consume or mint again.

L1-10 Golden commission: the selected definition in
content/quests/definitions/quests-00100-00199.json#/records/52/definition/oteryn_recipe/payload/recipe/stages
explicitly chooses 100000 gold base, 50000 boots/gloves, then 50000 helmet,
with no Premium requirement. Preserve those authored values and sequence instead
of donor 500000000/250000000/250000000 and helmet-before-boots. Donor
Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
data-global/npc/king_tibianus.lua:117-216, SHA256
e6509e41311669a22f9c7ac6c1787fcf5b63120d06d802a1ad268e0adac9d9bd,
supplies King dialogue topics, paired looks1210/1211, addon1 boots and addon2
helmet, existing GoldenOutfit storage, and carried-or-bank monetary semantics.
The donor-only decoration kit is outside the selected outfit programme and is
not an authored reward. Existing fee owner accepts equivalent coin denominations;
no synthetic inventory grant or duplicate monetary owner is introduced.

L1-10B Brotherhood/Nightmare: reuse the existing account appearance and Quest
transaction owners for ordinary carried USE; never select the branch from item
number alone. Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d source:
- data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua:1-69,
  SHA256 1e3d15bb73efa6f3b26bdd1d0882a579279c0cff39c2fb166d98d03b3ed6be16:
  follower768/base, officer769/addon1, leader770/addon2; rank1/2/3;
  Brotherhood rank greater than Nightmare selects Brotherhood. Joining remains
  mutually exclusive, making that comparison unambiguous. Owned grants retain
  the document; addon use requires the base.
- data-global/npc/the_bone_master.lua:61-119, SHA256
  65a6ff562c38b78383ad4e9a4ba13ede2e875cd07686955f7c7741308d8fe3b1.
- data-global/npc/the_dream_master.lua:61-121, SHA256
  eac3bc09ddfe9b4fb83bb9751fc684f1687cc2f940365892cac5755ab50fe6f7.
  Join/yes/yes creates follower; subsequent advancement consumes500 then1000
  demonic essences6499 and creates officer then leader. No fixture-created
  documents count as the selected acquisition programme. Donor-only shield and
  rank4 advancement are outside these selected three outfit milestones.
The named Dreamer's Challenge prerequisite is the completed puzzle route, chosen
stage s4 (Tic Tac Toe). Its chosen stages s5-s7 already include joining Nightmare
and a document reward, so requiring s7 before joining would create a circular
condition and break the source's actual initiation. Read s4 through the original
Dreamer owner, without writing it or its Knightwatch/world effects here. PG only
arranges that separate parent's qualifying fact. Both order ranks and account
ownership are rechecked before costs. New per-character document claim facts
record the successful one-shot USE; immutable account ownership still prevents
another character paying for an already owned outfit or addon.

L1-11 selected level programmes (no missing-fact hold):
- Dream Warrior: Crystal9f data-global/npc/vanys.lua:83-157 SHA256
  fbc0bb272ed67594e0d385ae0baf41271c81aca511b28455bd896c51f083e59a:
  WardStones.Questline>=3 and SummerCourt=1 grant1146/1147; five
  pomegranates30169 and one ice shield30168 grant addons1/2. The bounded
  claim selector "dream warrior outfit" addresses that completed-task reward
  without intercepting the separate parent's talk/task initiation menu. Its
  readonly original tracks remain owned by The Dream Courts. The selected
  programme explicitly adds level250/Premium. Refuse already-owned grants
  before charging, correcting the donor's repeat-addon-charge behaviour.
- Citizen of Issavi: the authored recipe names an unresolved attire-keeper
  ROLE, not a source NPC. Bind it conservatively to Eshaya, the actual Wanted
  report contact. NPC identity/greeting: Crystal9f data-global/npc/eshaya.lua
  SHA256 0b84914661498605630d5e98bfb74579760e035d280e5395862c14b975e749db.
  The original Kilmaresh selected s21 reports the three outlaw victories;
  use that existing parent's chosen completion signal read-only. Do not
  invent independent outlaw kills or rewrite the parent. Selected base level250,
  Current Premium; eye-embroidered veil37003 addon1 then tagralt-inlaid
  scabbard37002 addon2, one each. Item identities are pinned Canary items.xml
  lines68951-68958 SHA2561cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2.
  The addon-slot order is an explicit conservative binding of the selected
  ordered accessory programme (the source recipe leaves native encoding open).
- Makeshift: the recipe explicitly authors a Gnome Deep Hub qualification desk.
  Bind that role to the admitted Gnomadness; reaching this physical NPC within
  normal interaction range witnesses the selected exploration. Salvage request
  and confirmation execute the authored apprenticeship before the base reward.
  Crystal9f data-global/npc/gnomadness.lua SHA256
  e3411f8704927cc665340418f3b65fa40fdafa6d607364fde90827516b9f7279
  supplies NPC identity, not a falsely recovered outfit callback. The recipe's
  requirements.min_level=100 wins over its contradictory adaptation text saying
  zero; preserve the stricter explicit requirement and Current Premium.
  Crude wood planks27657/addon1 and tinged pot27656/addon2 match
  Crystal9f usable_outfit_items.lua:14-15 SHA256
  69b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37.
  Enforce the chosen first-before-second accessory sequence and award
  Cobbled and Patched only when the first addon is already owned.
Shared implementation: current persisted character level is read while its root
is locked; required addons use the existing compatible account facts. Storage-free
base claims require an exact neutral target/unowned-base predicate under the
existing account grant lock, with SQL1006 retaining all original proof arms.
Carried addons use the existing one-shot account predicate and exact input/ingress
proof without manufacturing quest progress flags.

L12 native hunting trials (AUTHORING; not yet qualified): reuse generic native
kill settlement from questpc3 commit5e72a3f0b0e16803bc5cd999298e4ec83f05eed9,
without importing its other-lane quest bindings. Frozen native death eligibility,
original party observation, occurrence identity, live lease and mission epoch
checks use the existing bounded kill queue and durable revision sequencer.
Dragon Slayer: selected20dragons +20dragon lords,10green/10red scales; base only.
Cave Explorer: selected20Spike creatures is conservatively a fixed Crystal Spider
assignment. Chosen level80 overrides source lower thresholds. Retain actual source
prices1000base/2000first/3000second (5000both addons): Crystal9f
9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d data-global/npc/gnommander.lua:75-97,
SHA25662e3088421dd6f06c1decf2f2ce0c3c296d4a106b670cfa8927eb936e0bb69eb.
Fire-Fighter: selected10fiery creatures is conservatively a fixed Lava Lurker
assignment; Gnomadness is the physical Gnomprona qualification contact, as actually
located in Gnomprona Gardens by the same pinned source npc/gnomadness.lua:102-105,
SHA256e3411f8704927cc665340418f3b65fa40fdafa6d607364fde90827516b9f7279.
These fixed trials are explicit authored rules, not claims of recovered donor
outfit scripts. Firefighting axe39544 and portable flame39545 retain existing
canonical identities and native profiles. NPC dialogue accepts trials before any
counter credit; no direct counter writes are used in the positive PG stories.
Dragon Slayer appearance: pinned Crystal9f outfits.xml marks admitted looks
female1289/male1288 as store. The owner explicitly selected an authored Dragon
Slayer quest and authorizes missing bindings; promote this pair for its quest
base grant as an explicit exception, retaining exact captured appearance hashes.
Do not infer a recovered donor quest or enable unrelated store grants. The
canonical outfit now participates in the existing account grant/appearance owner.
A unique NPC exchange owns its exact keyword even with unmet predicates: after
eligible conversation offers, route that unique exchange to the locked durable
writer for a stage refusal instead of generic legacy-dialogue fallthrough.
Cyclops identity evidence: same Crystal9f data-global/npc/a_sweaty_cyclops.lua:1-14,
SHA256bc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a.
The authored20+20trial is an owner-selected supplement, not a donor script claim.

L13 endgame accessories: pinned Crystal9f9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d
 data/scripts/actions/items/usable_outfit_items.lua:26-31 SHA256
69b5b133b329a76628dcd678f13d86bc3e5bd97902dc79998340c886e860af37 binds
Revenant skull34075 to native bit2 and bracelet34076 to bit1. Keep the chosen
skull-before-bracelet sequence using those actual slot bits; do not swap the
native encoding because the recipe calls its ordered steps first/second.
Cursed wood43899 bit1 then darklight geode43900 bit2. Min250/Current Premium
and original compatible native base facts precede consumption. Parent Soul War
and Rotten Blood grant mechanisms and their account provenance remain unchanged.
New PG variants compose at their earned native base reward checkpoint; the
original parent cases retain all their assertions and historical-state checks.
Rift Warrior: Cledwyn Lua:168-204 SHA256
915bab4d2c1e44b43cc4855c7b896c64e31cb3abe6598af461155bcbf8143431 demands
100silver tokens22516 for EACH addon, source looks845/846. Conservatively bind
the chosen completed-Ferumbras base attestation to Cledwyn as an explicit role;
his donor script itself only supplies addon exchanges. Parent completion remains
read-only under the original Ferumbras quest owner; do not duplicate its world
or boss mechanisms. Source NPC identities: flickering_soul.lua:1-6 SHA256
67a704c084d2490ea37ad337d472a7f3075375f0a3a043b1a615be12d5ac2405;
bloodshade_rotten.lua:1-6 SHA256
153105638eeaa55321f4e30caf4f5796291c89329092b32b3d4e6d6f9103df7c.

## QENGPR merge validation (2026-11-07)

Material merge: c3211f597e600054eb9496f804d6a0638dd90ccf; parents QENGINE2
e7867e061 and main 9668e0a04. Lane bookkeeping is removed; its notes remain above.

Conflict resolutions preserve both sources: sequencer creation fixtures plus starter
bindings; monk cosmetics rebinding plus main's activation helper; both native Item
admissions; both quest callback hooks (daily gates remain step-in only); both
quest-authoring documentation additions. NX-5's maintained authoring is retained
byte-for-byte as a generator input, with its outfits, mounts and grants combined
with QENGINE2. Eleven shared mappings explicitly retain both outfit keys through
source-validated aliases. Repository tools regenerated the cosmetics descriptor,
Item profiles/index, callbacks, lock, manifests, world pin and deployment digest.

Passed on the material merge tree:
- fmt --check: 25.771s.
- cargo check, all targets/features of the ten changed crates: 468.083s.
- clippy -D warnings, all targets/features of those crates: 393.305s.
- Quest-authoring: 1,084 tests discovered (8 pre-existing skips); the two stale generated-output modules
  passed after regeneration (28 QENGINE2 tests: 56.153s; Grizzly: 6.000s).
- Cosmetics authoring: 27 tests, 5.809s including process startup.
- QENGINE2 generator reproducibility: 19.607s; native provider freshness: 43 files.
- pin-refresh: 201.293s; pin-check: 200.637s, digest
  d528b2dfce1bedf9db643ff78c45060ef6c08f69459c9edbf632cdc4ca73eb95.

Content integration and focused real-PostgreSQL execution are still running.

QENGPR validation `integration`: 123 passed, 0 ignored; command wall time 125.665s.

QENGPR validation `world-integration`: 5 passed, 0 ignored; command wall time 118.875s.

QENGPR validation `alias-unit`: 2 passed, 0 ignored; command wall time 74.507s.

## PR #3020 CI repairs (2026-11-07)

The cosmetics provisioning authoring document is 804,946 bytes after QENGINE2,
so its old 200,000-byte sample limit rejected the canonical manifest before any
gameplay could activate. Its limit now matches the authoring producer's 8 MiB
document budget. Descriptor, source-shard, section and overall artifact limits
remain independent. A real materialization test accepts exactly 8 MiB and rejects
one byte more, with valid hashes at both boundaries; the test failed before the
limit fix. Existing input-tampering checks remain intact.

Regenerated islands and their capture sample with convert_islands.py. The normal
world pin-refresh command now also refreshes the client placement-index digest,
using the client's LF-normalized SHA256 convention, and refuses ambiguous pin
source declarations. This replaces the obsolete client digest without editing a
generated digest by hand.

The Content tree family-count assertion is unchanged. The lock was incomplete:
raid_catalogue.py restores Encounter.Raid=239 from content/encounters/raids/index.json
and restores its manifest/project registrations. Every family count now equals
origin/main. Provenance of the exact inventory (counts are index entries unless
noted): Ability=7392 abilities/definitions; Behavior=3145 behaviors; Charm=25
charms; Creature=1863 creatures/definitions; Dialogue=836 dialogues/definitions;
Document=1609 documents; Effect=5797 abilities/effects; Encounter=108
encounters/definitions; Encounter.Raid=239 encounters/raids; Formula=6126
abilities/formulas; Item=34111 items; Loot=1290 loot; Mount=252 cosmetics/mounts;
NPC=1282 npcs/definitions; Presentation=3145 presentations/definitions;
Proficiency=443 proficiencies; Quest=352 quests/definitions; RewardClaim=336
interactions/reward_claims (231 ordinary claims plus 105 variants); Service.Bank=1,
Service.Blessing=4, Service.Promotion=4, Service.Trade=324 and Service.Travel=56
from their services indices; StarterKit=1 starter; item_conversion=129 craft.
QENGINE2 adds runtime profiles/reward bindings, not additional indexed family
identities. The exact native-overlay output inventory is 43, including its
QENGINE2 reward-item proof. The chest-profile identity inventory is 134: the prior
130 plus ITEMHOLD-Q items 2859, 2949, 3053 and 3083 (also Triangle Tower).

FeeBurnCause already assigns QuestRewardPayment tag 18. Added its missing exact
golden tag and independent wire bytes; all existing tags and byte fixtures remain
unchanged. All five audit wire tests passed in 0.081 seconds.

The generator sweep also refreshed creature level-rating/power-score reports,
monster-attack source authoring/pins, and explore-area candidate input hashes.
The frozen Wave 1 market observation stays unchanged; its checker now recognizes
only the accepted market packet's explicit WAVE1_WIKI_MARKETABLE_FALSE
supersession. Positive and negative owner checks retain exact staged bytes.

Validation at the owner's requested immediate CI push:
- Cosmetics provisioning boundary and existing tamper tests: 2 passed, 8.44s.
- All five fee-cause/operation golden and wire tests: passed, 0.081s.
- Full requested world-metadata command sequence: passed.
- pin-refresh and its new source-pin unit test: passed.
- cargo fmt --all --check: passed, 33s.
- Client placement digest and all five requested client scene/map cases: passed
  in the ongoing 190-case run, including the 126 requested cases.
- Cargo check/Clippy, final pin-check, requested Rust/real-PG run and focused
  Python reruns remain in progress at this push; no success is claimed yet.
- Exhaustive Python discovery ran 450 files in 3114.43s: 417 returned success,
  33 returned nonzero initially. These include unavailable historical workspace
  fixtures, old source-stage assumptions, stale reports since regenerated, and
  a runner error invoking the script-only tree test through unittest (its script
  assertions and child commands reached their final PASS before unittest exited
  with no discovered classes). Focused corrections are being rerun.

## PR #3020 main merge and guard regression repair

Merged main d34807f5b (SRVWLD) while retaining QENGINE2's encounter callback
branch. Dryad watersplash remains after successful commit_source_step and uses
the committed destination. docs/reference/SRVWLD-CONTRACT-NOTES.md is taken
byte-for-byte from main; existing QENGINE2 entries remain in this file.

The receipt no-TRUNCATE probe explicitly includes the FK closure added by
0999-1001: game_quest_reward_sources, game_quest_reward_fees and
game_quest_reward_participants. No CASCADE is used. The exact 23514, guard
message and unchanged-state assertions remain intact. The original repository
guard module, linked against freshly built production dependencies in a focused
source-module harness, passed on real PostgreSQL in 14.741s.

The reward-bag catalogue has 28 playable bags: exactly 27 existing bags plus
QENGINE_GNOMISH_SUPPLY (15698), captured from Crystal 9f5a72c64
random_items.lua:49-80. The test preserves the 27-bag legacy inventory and
one-item mint assertions, and independently reaches all 25 new supply outcomes
with their exact source quantities and total weight 10001.

Validation on the merged tree: dispatch cargo check, all targets/features, passed
in 84s. The fresh dispatch binary ran 22 focused callback/real-PG regressions: all
passed, execution 545.989s (1123.039s including compilation). Dryad enter/leave
reported real-PG completion markers at 62.125s and 62.870s. The strengthened
reward-bag case passed in 52.05s. PR #3022 was still open at this batch; its two
main-red fixes are intentionally left to its owner and a subsequent main merge.

## QENGINE2 source profile count and cosmetic voucher repair

Main d34807f5b has 3572 raw native Item profiles; QENGINE2 e7867e061 has
3619 and is missing six main-owned keys (2949, 12369, 14156, 33330, 44179,
52732). QENGINE2 adds 53 other keys. The merged raw file correctly has 3625;
materialization adds the same 38 main-owned update profiles, giving 3663 rather
than the old main-only pin 3610. No Item data is removed or duplicated.
All 3572 existing main records compare equal. The update packet's 59 records
are unchanged (34 overlays plus 25 insertions); only its base SHA is rebound.
The 13 starting-loadout records are also unchanged, completing the 38 additions.

The 53 QENGINE2 IDs are:
133, 139, 768, 769, 770, 875, 3244, 3723, 4867, 5886, 5903, 5921, 5924, 5937, 5938, 5939, 5940, 5945, 5947, 6099, 6100, 6101, 6102, 6107, 6126, 7289, 9041, 10198, 10199, 11700, 11701, 12413, 12517, 12599, 12601, 12803, 15698, 16103, 16128, 16129, 16165, 16167, 16242, 20275, 20276, 22516, 22721, 28792, 28793, 30168, 30169, 37002, 37003.

They exactly equal qengine_reward_item_profiles.py's source IDS and generated
qengine-reward-item-profiles.json records. All 53 qualify speed_bonus=0.
Source: Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446, items.xml SHA256
1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2; appearance
data SHA256 2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.
The speed test derives the cohort from that generated proof, checks its exact
53 unique keys and every zero-speed admission, then retains all existing raw
attribute, equipment-slot and unknown-speed assertions. Native regeneration
--check reproduces all 43 registered files without changing generated content.

USE 16255 reaches the existing cosmetic owner. Its ContentUnavailable refusal
came from two equally matching outfit keys (the QENGINE2 primary and retained
NX-5 alias); it was not routed through Dryad callbacks. Item grants now resolve
the explicit authored primary target before checking the source look pair and
existing provenance/kind/addon compatibility. Unqualified ambiguous look lookup
remains refused. Historical aliases remain compatible; no grant or receipt is
created by this lookup. Negative wrong-look, missing-grant and wrong-provenance
checks accompany the production catalogue regression. SRVWLD watersplash and
QENGINE2 callback execution code remain unchanged.

Plain-main ownership check (bbb3e06d04875fc1e63f024b853194f7580cc875):
the speed catalogue case passed (96.990s), NX-5 global rules (13.582s),
NX-5 outfit unlock/replay (132.953s), Atelier split dismantle (94.773s),
and Forge pending revision refusal (227.200s), all against real PostgreSQL
where applicable. QENGINE2 voucher/outfit story modules are absent on main;
zero selected stories are not counted as passes.

Forge's initial fixture rebinds both training and authored cosmetics to
content-1. Its independently reconstructed successor previously rebound only
training, so restoring the presentation bytes could not restore the original
server digest. Initial and successor activation now share the same prepared
native input; only the deliberate whitespace repack changes identity. All
three original digest equality assertions and the retained refusal stay intact.
The additional Atelier runtime/receipt case also passes on plain main
(14.863s). The reported vector mismatch is in three later coin-proof functions,
not a different copied assets-guard body: replaying 0314/0624/0744–0747 removes
Quest cause 18 from coin admission/completion and duplicates the Atelier-only
Character proof arm because 1000 broadened that arm. The repair keeps that arm
Atelier-only and reapplies the later Quest coin/receipt extension from its
migration; exact live function definitions, stored state and receipts remain
the test's independent equality expectations.

The Royal Costume failure reproduces as a PostgreSQL transaction timeout in
the original two-second semantic pass (backend log), not a missing grant CHECK.
The source recipe still consumes exactly 15,000 silver and 12,500 copper tokens,
leaving the test's extra token of each kind untouched. Reward hand-ins now lock
locations and check plain-item exclusions in bounded sets, insert every original
consumption proof before mutation, update stacks together, and remove only
fully consumed custody entries. All existing BEFORE/deferred guards still run.
The carried membership projection walks the same bounded UUID tree before
serializing selected location rows; its guard overload avoids projecting every
inventory row for each input. The 500/501-node, depth-eight/nine, cycle and
foreign Character/World/absent-item cases were compared against migration 0268's
actual function on real PostgreSQL (six fixtures, full/selected equality).
No deadline, recipe quantity, custody exclusion or receipt assertion was relaxed.

QENGPR follow-up: VIP immutable replay retains its exact byte assertion.
The failed payload decodes to Unavailable (9), revision/number/wait zero;
the expected payload is RateLimited (8), revision 25, wait two seconds.
The unchanged dispatcher emits the former when prepare_vip cannot acquire
the ready holder. The transcript directly replayed through the same root as
the still-serving wire owner, whose periodic work can hold it after delivery.
The replay now opens an independent admitted holder against the same database
and uses the same session, command, payload, fencing and stored receipt.
No frame layout, refusal code, rate bucket or equality assertion is relaxed.
