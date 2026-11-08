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

## ENCX qualified activity catalogue publication

- Migration 1150 grants the existing runtime catalogue publisher INSERT/UPDATE and
  adds uniqueness for `(room_template_key, content_revision)`; no wire identifiers
  or capability bits are allocated.
- Boot publishes only `QualifiedActivityRoomTemplate` values through
  `ActivityHandoffOwner`, under the current node incarnation and admission lock.
  Captured candidates and qualification refusals cannot be supplied as templates.
- Source identity: Crystal `201189f`,
  `data-global/scripts/actions/bosses_levers/the_primal_manace.lua:2-4` binds
  the lever's boss name to its creature. The captured creature suffix determines
  the boss namespace identity in one mapping on `BossLeverCandidate`.
- The current source-map importer captures Canary Thalom cells
  `[5840,5283,5871,5314,5,8]`; Crystal Primal's source arena is
  `[33547,32749,33570,32769,15,15]` (same donor file:15-17).
  Publishing rejected candidates cannot repair that missing map admission.

### Publication checkpoint validation

At `89af353ae`, catalogue equality/replay PostgreSQL scenario
`qualified_activity_catalogue_postgres_equals_boot_qualified_set` passed in
67.03 seconds. The current qualified set is empty; this proves exclusion and
replay, not positive room publication. The unchanged Primal assertion
`primal_ordeal_postgres_qualified_production_room_is_reachable` failed in
66.66 seconds, with source diagnostic `OutsideQualifiedMap`.
Changed content/durability/dispatch/server crates passed clippy with warnings
 denied (6m35s); the publisher/server check passed (30.81s), and fmt passed.

The pinned Crystal `data-global/world/world.otbm` contains 504 Primal arena
cells and the source boss, entry, lever and exit positions. The current loader
`project/native_spell_world.rs` admits only the pinned Canary Thalom document,
including exact source bounds, frame, tile count (2833), digest and closure.
A separate provenance-qualified Crystal region must be admitted; replacing
those checks with permissive bounds would invalidate the existing source proof.

### Primal source region capture

`capture_primal_region.py` verifies the exact Crystal `201189f` map Git blob
`e61658ec0ab7b70ec35025ae969426d6fc57ae75` before parsing. Bounds and required
placements come from the generated donor lever capture. The generated region
contains 510 tiles: 504 arena cells plus required source entry/exit cells outside
that rectangle. Unknown selected attributes, duplicate cells, substituted bytes,
missing placements and capacity overflow are rejected. This is source capture,
not active map admission or creature qualification.

### Primal item semantics capture

The source-region generator now verifies pinned appearances blob
`37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0` and item XML blob
`ea4196beae8f77be898f05de89db81de134886b6` before reusing the repository's
item-loader projection. It emits 89 item policies for the 510 selected tiles.
All required placements have ground and are not solid-blocked. One dynamic
item remains explicit: source teleport 1949 at `(33567,32758,15)` has donor
destination `(0,0,0)` and requires the encounter exit owner; it is not silently
promoted to ordinary terrain or an invented destination. Three Python tests
pass, including red-before-green item-source substitution coverage.

### Source-qualified Primal map facts

`qualify_primal_region` consumes only the exact generated 510-tile document and
its explicit parent coordinate frame/map revision. It provides scoped collision
facts without issuing an artifact pin, activated room or mutable instance. The
source teleport remains `Unqualified` until the exit owner is composed. The new
Rust test `primal_region_admits_source_placements_and_refuses_pending_teleport`
failed before the qualifier existed and passes after implementation (0.04s),
covering boss/return walkability, pending teleport refusal and document
substitution. This is a source-admission slice; production template integration
and the real-dispatch W10 chain remain incomplete.

### Source region artifact carrier

The native source carrier now accepts an explicit bundle of the unchanged pinned
Thalom document and exact generated Primal document. Server qualification merges
only captured cells; gaps stay absent and the unresolved teleport stays unknown.
The client projection includes both regions, and existing carrier cross-pins bind
that projection and both source documents. Region tests passed (3 tests, 1.42s),
including real artifact encode/decode and client substitution rejection; content
clippy passed with warnings denied. The bundle test was red first with
`InvalidProject("native source world qualification")` (0.21s). The production
Primal fixture still needs to select the new source bundle and resolve encounter
hooks; this is not a completed encounter chain.

Focused baselines on the preceding source-facts checkpoint: Soul War
`soul_war_postgres_native_taints_xp_reset_and_replay` passed in 79.92s; Summer
`summer_population_npc_placements_greeting_lore_and_stale_actor_on_postgres`
passed in 42.36s. Neither proves the full W11/W16 encounter-chain requirement.

### Production map provider checkpoint

`encounter_source_regions.apply_native` is part of the existing native provider
registration tool. It packages the unchanged pinned channel source with the
verified Primal document and regenerates the production manifest's source-world
pin. The Primal PG fixture now activates this packaged source rather than the
Thalom-only fixture. Native provider regeneration and `--check` passed; world
pin-refresh completed without tracked pin changes. Content/dispatch clippy passed
with warnings denied (1m35s), fmt and architecture passed.

The unchanged Primal catalogue assertion ran against PostgreSQL for 71.88s and
remains red. Its exact qualification refusal advanced from `OutsideQualifiedMap`
to `SourceHookUnavailable`. Crystal `monster/quests/primal_ordeal_quest/
the_primal_menace.lua:66-69` requires `ThePrimalMenaceDeath` and
`ThePrimeOrdealBossDeath`; `:339-347` invokes custom `onThink` behavior. Those
callbacks must be composed before the room can be published. No callback is
marked available solely to get a row into the catalogue.

### Canonical boss identity reaches entry gates

The real-dispatch PostgreSQL entry scenario now checks canonical Primal and
Bakragore template keys as well as creature aliases. Before the fix it failed in
4.88s (`ContentUnavailable` instead of the required level refusal). Both identity
forms now resolve through captured source data before catalogue lookup, retained
handoff retry and endgame policy selection. The unchanged assertions passed in
4.59s; dispatch check (5.27s), clippy (7.36s) and fmt passed. This closes an entry
policy routing gap without admitting an unqualified template.

Production map checkpoint pin-check passed in 2m31.825s. The dependent server
crate passed check (1m39s) and clippy with warnings denied (24.12s).

### Pinned Primal hook discovery

The lever importer now discovers Primal's direct source requirements from three
exact Git blobs: monster `76adae125dd1f8eb099878b6fa5f8b96f91445e8`, hazard
handler `3cafb041e31f9f068b2ec118af2b5575aee6d2bc`, and quest death handler
`cb0c34b64010470546b125959f2a63690bedb64b`. Missing or substituted source stays
unknown. Generated requirements are `ThePrimalMenaceDeath`,
`ThePrimeOrdealBossDeath`, and `mType.onThink`. `callbacks_known` describes
source discovery only; native qualification still rejects these unsupported
requirements with `SourceHookUnavailable`.

Five Python provenance/parser tests pass (0.004s), including red-before-green
hook discovery, missing-source refusal and source substitution rejection. All
four activity-template tests pass (68.04s), preserving map bounds and physical
corpse gates. Content check passed (11.51s), clippy with warnings denied passed
(6.10s). This is not native hook composition or a completed W10 encounter.

### Primal Hazard Instance identity consumer checkpoint

Primal's Hazard consumer now accepts the native death owner's existing tagged
Instance receipt framing through `party_kill_xp::decoded_death_scope`, retaining
legacy Channel bytes unchanged. No new encoding, identifier or migration is
introduced. The new scope regression was red on the Channel-only implementation
and is green after reuse of the shared decoder. It rejects cross-namespace UUID
aliasing, generation mismatch and truncated keys. Durability check passed in
1m00s; the legacy Hazard PostgreSQL regression and clippy are still running at
this WIP checkpoint. This does not remove the transport's Channel-only endgame
composition gate or qualify the unsupported Primal hooks.

### Consumer checkpoint validation completed

At `77e1b2067`, the legacy real-dispatch Hazard scenario
`primal_hazard_postgres_kill_unlock_selection_replay_cap_and_fences` passed in
40.46s, covering selection, unlocks, replay, caps and fences. Durability clippy
passed with warnings denied (3m05s); fmt and architecture passed. The fresh full
Primal reachability case ran for 72.94s and remains red with
`SourceHookUnavailable`. Tagged Instance scope matching is covered by the new
unit regression; an end-to-end Instance death/reward case is not yet proven.
The transport's endgame Channel-only cast, Soul War/Rotten Blood consumers,
Instance corpse custody and native Primal hook composition remain unfinished.

### Shared endgame tagged-death receipt checkpoint

Migration 1151 preserves the existing Soul War/Rotten Blood receipt predicates
and additionally accepts the native owner's existing 53-byte Instance framing
only with tag 2. The transport no longer narrows endgame deaths to a 52-byte
Channel array. Soul War and Rotten Blood reuse the same scope/generation decoder
as Primal; native source receipt, membership, transaction-origin and session
fences remain required. Legacy occurrence bytes are unchanged.

The Rotten Blood occurrence regression failed before the slice API (E0308) and
passes after implementation; dispatch/server check passed in 19.62s. This is a
compiling WIP: migration 1151 and focused PostgreSQL regressions still need to
run. Native hook composition and physical Instance corpse custody remain
unimplemented. No new command, capability or mechanic identifier is allocated.

### Shared consumer checkpoint validation completed

At `f8ed17e96`, migration 1151 applied successfully in both focused PostgreSQL
scenarios. `soul_war_postgres_native_taints_xp_reset_and_replay` passed in 80.45s;
`rotten_blood_postgres_native_chain_final_reward_replay_and_cleanse` passed in
107.77s. Changed durability/dispatch/server crates passed clippy with warnings
denied (2m40s); fmt and architecture passed. These are legacy Channel consumer
regressions, not a positive Instance encounter-chain proof. The Rotten Blood
case still explicitly preserves physical corpse refusals for unqualified bosses.
A real tagged Instance death transaction and full production entry/fight/reward/
exit remain to be demonstrated before either W10 or W11 is DONE.

### Instance receipt integration fixture WIP

A new real-handoff component case enters an assigned Instance through command
178, then exercises the captured-death consumer seam for tagged receipt replay
and Channel/Instance alias rejection. It does not seed a Primal catalogue row or
claim a physical Primal fight. Existing Channel captured-death fixtures retain
their default 52-byte key type; the helper now also accepts a tagged-key type.
The new case compiles (test build 1m44s) and is still running at this checkpoint.
The helper's known remaining Channel-only guard is `Err("channel fixture")`;
the runtime red result has not yet been observed. No new numbers are allocated.

### Scoped receipt helper fix

The new Instance receipt case reached `Error: "channel fixture"` after a real
native handoff (205.88s). Selecting `SpellLane::for_scope` from the current fence
instead of constructing a Channel lane turns the same component case green
(205.57s): the native tagged Primal receipt grants one Hazard unlock, replays
without another grant and rejects a stripped namespace tag. Test-target dispatch
clippy passed with warnings denied (3m42s).

The negative case is further strengthened with a fresh XP occurrence and an
explicit assertion that no rejected-key XP receipt remains; that rerun is
pending at this WIP checkpoint. This proves consumer scope composition after a
native handoff, not a physical Primal encounter or template qualification.

### Instance receipt component proof completed

The strengthened `boss_activity_postgres_tagged_instance_primal_receipt_replay_and_scope_fences`
case passed against PostgreSQL in 206.38s. It performs a real command-178 Instance
handoff, supplies a captured death to the existing XP/consumer seam, retains one
Hazard receipt on replay and rejects an untagged alias using a fresh occurrence,
with no partial XP receipt. It does not prove physical Primal combat or qualify
a Primal room. Dispatch check passed in 16.14s; test-target clippy passed with
warnings denied in 3m42s; fmt and architecture passed. No further identifiers or
migrations were allocated.

### W10 Instance death audit prerequisite

Crystal `201189f`,
`data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:79,94`
declares corpse 39530 and `rewardBoss = true`. The physical death in its native
activity must retain its Instance identity through corpse and loot audit records.
The existing Ground and EventEnvelope messages already admit an exact Instance;
the death reference now has additive optional protobuf field 6, `instance_id`.
Exactly one of Channel and Instance is accepted, with the same typed scope
required by the Ground destination and envelope. Corpse-container loot retains
the death's scope even though its destination names only its parent corpse.
Absent field 6 preserves the captured Channel wire bytes and existing bounds.
No command, capability, mechanic or registry number was allocated.

`instance_death_mint_audit_keeps_channel_and_instance_identities_disjoint` first
ran red with `InvalidInput` (0.007s; build/run 80.561s), then passed (0.016s).
The 30 audit tests passed in 0.390s (build/run 80.243s), including the existing
Channel compatibility and resource-bound cases. The audit tag/golden generator
produced no changes. Architecture checking passed. All 708 durability tests
passed, with no skipped tests, in 451.420s (build/run 533.000s), including
`boss_activity_item_consumers_postgres` (6.061s). Cargo check passed in 94.176s.
Clippy identified one complex return type; named scope fields resolved it and
clippy passed with warnings denied in 1m05s. The 30 audit tests passed again in
0.432s after that correction. The 25-package dependent PostgreSQL run at
`2f291e493` is still collecting all failures; it has reported the guild-war
test transport-gate race described by its `pending` assertion.

This checkpoint covers wire validation. The production death-mint owner still
requires Channel identity; Instance corpse custody and native Primal hook
composition remain to be implemented. The Primal production room remains
unqualified with `SourceHookUnavailable`; W10 is unfinished.

### W10 native Instance corpse and loot owner

Donor: Crystal `201189f`,
`data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:79,94`
requires corpse 39530 alongside personal boss rewards. Migration **1152** extends
native creature-death reservations and receipts with exact Instance identity.
It preserves Channel identity, existing row immutability checks, work-unit
charges, materialization stamping, corpse capacity and atomic receipt/audit
proofs. The source key uses the existing assignment encoding: Channel tag 1,
activity Instance tag 3; runtime/custody Instance kind remains 2. Legacy Channel
intent bindings retain version 1; Instance bindings use version 2 so their
business-intent framing is disjoint. No command/capability/mechanic/registry IDs
were allocated.

Combat now passes the committed death's actual scope and native occurrence
bytes into this owner. The same owner mints the corpse on Instance Ground and
its loot into that corpse, using the actual Ground root's item lane and key-33
lock. The narrow Instance placement proof requires the same complete death
cause, frozen reservation, current assignment, fresh corpse and same-transaction
audit. Existing accepted placement causes remain installed.

`physical_instance_death_reaches_the_mint_cause_without_channel_aliasing` ran red
with `AuthorityRejected` after an actual carrier lethal commit, then green.
The PostgreSQL case
`native_instance_corpse_mint_postgres_physical_death_loot_replay_and_scope_fences`
exposed and fixed the generated-key/immutability interaction, the missing
native-corpse Instance placement cause and the Channel-only corpse-root lock.
It passed in 4.816s, then passed again with a real Channel control in **5.053s**.
It checks corpse and loot commits, exact scope, immutable audit, replay, wrong
lane, same-UUID Channel alias, unowned Instance and stale generation. The focused
46-test set passed in 5.062s (build/run 82.480s). Audit tag/golden regeneration and
pin refresh produced no changes; the world pin remains
`6157033be3cbe48fb8eb1261ce13579cb1e02ec40f457025d1b89ac3d2d62671`.

The owner decision replaced full reverse-dependent nextest runs with focused
mechanic tests and workspace compilation. The old 25-package run at `2f291e493`
was stopped as directed after 2,140 cases: 2,137 passed, one client guild-war
transport-gate race failed and two running tests were interrupted. Its complete
log was retained outside the repository. No test was disabled or deleted; the
legacy Channel replay assertions remain.

This is a native corpse-owner component proof. Production Primal hook
composition, qualification and the complete encounter chain remain unfinished;
W10, W11 and W16 are not DONE. The Primal qualification case still reaches
`SourceHookUnavailable` before publication.

Final focused validation for this owner step: workspace/all-target check passed
in 188.849s, changed-crate/all-target clippy with warnings denied in 64.504s,
workspace fmt check in 15.245s, duplicate-migration check against fetched
`origin/main` (`1ae39c7fb`) in 0.367s and pin-check in 28.617s. Pin refresh and
audit tag/golden regeneration used repository tooling and changed no artifacts.


## ENCX integration of main 1ae39c7fb

Merged main with the repository merge drivers. Rebuilt the conflicting spell
manifest through `tools/merge-driver/regen.sh --skip-checks`, then explicitly ran
`register_spell_families.py` and its `--check` mode; both passed. The first final
pin refresh was killed while competing with clippy; the isolated retry passed.
Preserved incoming producer/quest notes under
`docs/reference/QENGINE2-CONTRACT-NOTES.md`, leaving no root contract notes.
QENGINE/QBASE migrations remain 0999–1008; no 1200–1205 copies or new ENCX
migration/command/capability/mechanic numbers were introduced.

Focused validation: 46 corpse/mint/audit tests passed in 5.181s, including actual
PostgreSQL execution of
`native_instance_corpse_mint_postgres_physical_death_loot_replay_and_scope_fences`
in 5.175s. All eight native source-world tests passed in 1.184s, including
`source_region_bundle_admits_primal_and_projects_both_regions` in 0.848s.
The two map-capture and three boss-lever importer Python tests also passed.
Workspace/all-target check passed in 443.842s; all-target clippy with warnings
denied passed for all eleven ENCX-changed crates in 648.620s (both times include
build-lock waits). The redundant smaller clippy run was stopped after the broader
run passed. Fmt check passed in 18.423s, duplicate-migration check in 0.398s,
pin refresh in 31.210s, pin-check in 33.765s and architecture-check in 6.041s.
These component and integration checks do not change W10/W11/W16 completion:
the full Primal production qualification/encounter chain remains unfinished.


## ENCX publication cadence (owner direction)

After the integrated main checkpoint, publish compiling work within fifteen
minutes, then at least every thirty minutes. The pre-push gates are
`cargo check --workspace --all-targets` and `cargo fmt --check`; full suites run
in CI. If a main merge cannot be resolved within fifteen minutes, abort that
merge, publish the last compiling state, and resume the integration afterwards.
Keep follow-ups small for the single SRVENC integration PR. This checkpoint
adds no runtime behavior or allocated numbers. The next W10 production gate
remains the captured Primal callbacks in `activity_template.rs`; neither source
hook qualification nor the unfinished W10/W11/W16 chain tests are relaxed.

Checkpoint gates passed: workspace/all-target compile 220.394s; fmt 14.866s.


## W10 callback qualification diagnostics

Boot now reports the exact unavailable captured source hooks with each
`SourceHookUnavailable` rejection. Qualification and diagnostics use the same
hook filter; no source hook was admitted by this change. Crystal 201189f,
`data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:66-69,338-347`
requires the distinct Primal death/quest callbacks and think owner. Map failures
retain their own reason and do not report a callback rejection.

The new test `primal_hook_diagnostic_names_the_captured_unimplemented_callbacks`
failed first (missing diagnostic API), then passed in 0.010s. All five activity
qualification tests passed in 82.869s. The native Instance corpse PostgreSQL
regression `native_instance_corpse_mint_postgres_physical_death_loot_replay_and_scope_fences`
executed and passed in 5.370s. No IDs, migrations, wire shapes or generated
registrations changed. W10/W11/W16 full production chains remain unfinished.

Follow-up gates passed: workspace/all-target compile 409.527s (including test-build lock wait), changed-crate/all-target clippy with warnings denied 170.204s, fmt 14.531s, duplicate migrations 0.358s.


## PREPUSH-2 integration of main f641d8d6c

Fetched and merged current main with repository merge drivers. The spell manifest
was the sole textual conflict and was resolved by the repository generator.
Main itself supplied an uncompilable NPC caller: `quest_world_callback.rs` called
five-argument `apply_input` with three arguments (E0061). The shared mutation now
accepts an explicit private `InputAuthority`: World callbacks retain the exact
Channel lane/fence check, while the existing current-equipment-admitted
`NpcItemPermit` admits carried custody only. Ground is rejected before any NPC
mutation; no fresh or reconstructed lane permit is manufactured.

Regeneration uses the full content/world materializer and tree tooling, spell
registration, item audit tag tooling, quest binding evidence assembly and client
registration golden tooling. World metadata was checked and regenerated from the
converter's exact hash-pinned Crystal source files: zero metadata files changed.
The two registration fixture changes are generator canonical formatting only.
No ENCX migrations or command/capability/mechanic numbers were added.

The native Instance corpse PG regression executed on merged main and passed in
5.777s. Unit suites are running for all eleven ENCX-changed crates (library unit
selection; the full PostgreSQL suite is excluded), and the real-dispatch NPC XP
hand-in PG case is also queued. Final check evidence is recorded below before
publication. W10/W11/W16 full encounter chains remain unfinished.


Publication gates: full regeneration and pin refresh passed (world digest
`d528b2dfce1bedf9db643ff78c45060ef6c08f69459c9edbf632cdc4ca73eb95`, unchanged
by regeneration). Spell registration check passed in 85.936s and quest binding
check in 19.032s. The fresh workspace/all-target check passed in 999.935s,
including build-lock waits; changed-crate/all-target clippy with warnings denied
passed in 804.094s, fmt in 16.082s and duplicate migrations in 0.414s.

PREPUSH-2 cadence exception: full library unit suites are still pending for
`oteryn-client`, `oteryn-client-ui`, `oteryn-game-server`,
`oteryn-gameplay-combat`, `oteryn-gameplay-content`,
`oteryn-gameplay-dispatch`, `oteryn-gameplay-durability`,
`oteryn-gameplay-entities`, `oteryn-gameplay-items`,
`oteryn-gameplay-monster-ai`, and `oteryn-synthetic-client-harness`.
The first build exposed the repaired main caller error; the second hit a
SIGKILL while compiling the dispatch test binary concurrently with server tests.
A serial build retry is queued; no test case or assertion was disabled.
`catalogue_npc_general_xp_handin_runs_in_real_dispatch` is also pending build.
The completed Instance corpse PostgreSQL case remains the actual 5.777s proof
for this checkpoint, not a claim that the full encounter chains are DONE.


Final generated-content checks passed: World-project/tree equivalence (34,111
items, 252 mounts and all admitted bindings), materialized tree 97/97, and the
publication pin-check in `/tmp/encx-f641-publish-pin-check.log`. A duplicate
optional pin-check process and the NPC dispatch test build suffered memory kills
while running together; the publication pin-check completed successfully, and
the one-job unit retry remains active. No assertion failure is being reported as
a pass. The NPC PG case has not executed yet and remains explicitly pending.

## M29 — qualified Primal corpse deadlines in Instance (QMECH)

Coordinator allocation is now **2200–2209**, superseding the old ENCX range for
new QMECH work; **2200** extends the existing Item temporal receipt owner. No
command, capability, mechanic ordinal or registry number is allocated.

Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data/items/items.xml:73328–73347` defines 39530 → 39529 after 10s, →39528 after
300s, →39527 after 300s, terminal retirement another 60s later. The existing
content overlay `tools/content-schema/native-gameplay/primal_corpse_items.py`
and `content::bosses::primal_closure::qualify` already qualify all four source
Items; the quest's native kill/corpse producer already mints 39530. Migration
2200 connects that exact production key to the existing original-mint schedule
producer. No quest graph or fabricated content row is needed.

Instance due passes now drain the existing frozen field-chain schedules rather
than returning zero. Receipts carry exactly one Channel/Instance identity;
Instance receipts support only physical Ground corpse custody. Original death
scope/generation stays immutable. The current assignment and generation are
resolved independently, with the native due-pass permit, node and recovery
fences still checked at the production transaction boundary. Existing source,
clock, previous-stage, compare-and-change and same-XID audit guards remain.
The existing scoped corpse retirement owner reads the original +670000ms
terminal deadline; stage replay cannot restart that deadline.

Party refusal fidelity was transferred separately in `ed8f218de`: a level-499
teammate yields `Level`, without calling shared admission. Donor:
Crystal `data/libs/functions/boss_lever.lua:183–218`.

Red PG proof: `primal_ordeal_instance_corpse_postgres_deadline_and_scope_fences`
failed in **12.42s**, with zero schedules versus the three donor deadlines.
First green ran in **23.94s**, including the real 10-second source deadline,
physical native death/MINT, immutable schedule, scoped terminal recovery,
wrong permit/generation rejection, exact first-stage transform, audit and replay.

Final focused PG run: four `native_corpse::` cases passed in **32.91s**, with
M29's actual Instance case **15.890s**. The existing Channel takeover PG control
also passed (**5.39s**, timer participant **187.736ms**). Crate library tests
passed **742/742** in **97.26s** in unit mode (PG disabled there; the five focused
PG cases above were separately run against PostgreSQL 18.6). The repository's
`migration_numbers_are_unique_and_match_embedded_migrator` check passed.
Quest-authoring checks passed; all **105** quest-authoring unit-test modules
passed, and materialized content-tree validation passed **97/97**. No content
input changed, so the standing rule requires pin/lock verification, not World
regeneration. Production caller: `dispatch::gameplay_transport::source_item_cycle`
reads the runtime's exact scope/generation and calls `drain_spell_item_deadlines`;
its Ground invalidation is retained when a physical Item changes.

## M02 — first engine slice: exact physical Ground stack predicates

Started INDEX M02 while it was OPEN/none. During pre-push merge ENGNPC2
published the winning claim; their ownership is preserved. This tested engine
slice is available to that lane, without a duplicate claim or DONE assertion.
No new migration or public number.
The optional `world_requirements[].ground_stack` lists one root per entry,
bottom to top, with its exact Item key and quantity. It is read-only and requires
a one-way personal player USE/STEP-IN callback, so it cannot bypass the existing fenced quest writer.
The writer reissues current scope/node/recovery authority in the same physical
transaction, holds the existing Ground-owner lock (advisory namespace33) and locks
the actual Item/custody rows through COMMIT. No overlay is accepted as custody.
No synthetic catalogue, custody table, reward or Item producer is introduced.

Reference: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
`src/items/tile.cpp:1443–1493,1635–1667` distinguishes actual ordered Item roots
from the ground tile/creatures/top projections. URL:
https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/tile.cpp#L1443
This is OTS_HYPOTHESIS_ONLY for Global; Oteryn's existing Item owner supplies the
physical stack ordinals and the 64-root bound, not donor pointer identities.
Nomads Land's accepted authored recipe s4 requests crates/amphorae blocking the
window, with source spoiler revision1002600:
https://tibia.fandom.com/index.php?curid=22842&oldid=1002600
The existing captured recipe is evidence of that selected objective, not a
claim that its currently unresolved exact trigger/placement bindings are ready.

RED: `m02_ground_stack_predicate_preserves_order_and_requires_a_durable_player_callback`
failed with unknown `ground_stack` (0.02s); the same case now passes (0.02s).
All340 quests library tests passed (41.57s). Real PostgreSQL18.6 case
`m02_ground_stack_postgres_exact_order_quantity_and_scope` passed (6.25s total,
312.548ms for the native MINT/predicate/lock scenario), including nonconsecutive
ordinals, swapped roots, wrong quantity/key/cardinality/map/content/frame/address,
and a competing backend denied the Ground-owner lock. All source guards stay on.
This is an engine slice; canonical Nomads window binding and full quest passage
remain the next piece. M02 is not claimed DONE by this checkpoint.

Next queue row: M03 was OPEN/none on the pre-push qmech head; ENCX selects it
for live blessing/promotion/Premium world admission after this engine hand-off.

### M02 coordinator reconciliation against qmech 891284f07

The earlier ENCX `GroundStackItem/quantity` proposal is superseded and removed.
The sole world-callback wire contract is SRV7's `GroundStackEntry { item, count }`;
SQL observations are fed into the same `ground_stack_matches` comparator via
`GroundStackFact` with real UUID identities. Native/static anchor and cache
admission remain exactly SRV7's implementation. The only additional production
participant is the locked PostgreSQL re-check inside the existing personal
quest transaction, retaining scope/node/recovery and current content proofs.
The PostgreSQL fixture now mints canonical gold roots through the existing
MINT writer; no fixture-only key circumvents the shared comparator's validation.

Reconciled validation: all359 quests tests passed in43.46s, including both
upstream Ground models, SRV7 schema tests and preserved M07 tests. The final
PG case passed in5.46s (255.811ms measured participant scenario), now also
rejecting use of a prior physical transaction's sealed scope token. Clippy
for quests/durability/dispatch passed with warnings denied in2m17s; fmt passed.
No upstream M02-specific PostgreSQL cases were found under authority_cases;
the added PG case is the actual durable readset/locking coverage.
