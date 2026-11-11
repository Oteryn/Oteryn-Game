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

## SEASON lane, 2026-11-07: exact completion boundaries

Base `9668e0a04`, audit fetched from
`origin/claude/happy-davinci-ijz3wy-aud1107`; branch
`claude/happy-davinci-ijz3wy-season`. No gameplay behavior is promoted to DONE.

- W05d: Cake bridge Item 48227 remains canonically unavailable under the live
  identity/map-owner answer 6549960114. Crystal
  `data-global/scripts/lib/a_piece_of_cake_config.lua:249-292` is the producer;
  source facts cannot grant an Item reference. Details and fresh reachable wiki
  revisions: `docs/reference/content-prep/season-20261107/W05d.md`.
- W12: the real pacing consumer has no source-qualified discovery contribution.
  Crystal `data/scripts/discovery_system.lua:528-563,605-615` and
  `src/creatures/players/player.cpp:7222-7231` supply no formula; fresh BR wiki
  revision 443653 supplies only the 100–195 envelope. Mapping, units and stacking
  need an exact source or authored compatibility rule. Details: `W12.md` beside
  the note above.
- W18: full furniture rotation closure contains unavailable targets 34277,
  37883 and 23397 in both donors. Crystal `data/items/items.xml:65377,70954,51370`;
  Canary `data/items/items.xml:65175,70749,51180`. The source checker rejects
  substituted donor snapshots and reports absent identities without repairing
  them. Generated evidence and reproduction: `W18.md` beside the note above.

Donor pins: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446` (`OTS_HYPOTHESIS_ONLY`).
No migration, command ID, capability bit, mechanic ordinal or registry entry is
allocated. Migration range 1160–1169 and lane offset +60 remain unused.

### Continued implementation after owner feedback

The preceding whole-row BLOCKED conclusions are superseded; independent runtime
components are being implemented. No specific owner prohibition was found for
34277, 37883 or 23397. The normal pinned source loader has no usable ItemType for
them; Crystal `src/items/items.cpp:419-422` rejects an uninitialized/reserved type,
and `src/game/game.cpp:3054-3057` returns the original Item unchanged for such a
transform target. The captured recipes preserve those exact no-ops rather than
inventing target identities or refusing all furniture.

W18 source catalogue and policies now cover 160 carpets, 104 windows, 33 music
items, eight mirror IDs, eight magic trees, five bed kits, and 722 XML furniture
transform entries including the three engine-backed no-ops. Migration **1161**
contains generated exact source/target pair helpers. No command, capability or
mechanic ordinal is allocated. Runtime integration is work in progress; the
published source-capture step does not yet wire the new native policies. Its
remaining red point is
`test_source_carpet_and_mirror_have_real_native_policies` against the historical
provider (missing carpet 37365). Source capture/admission preservation tests pass.

All future notes for this lane live in this file, not repository-root notes.

### Native policy integration checkpoint

The native provider now admits the captured decoration policies through
`register_spell_families.py`, preserving earlier specialized Item semantics.
SRX Item updates retain the source-qualified top-order facts. Santa's existing
canonical present container 6510 (five slots) and snowball 2992 are captured by
the separate seasonal native importer; no replacement identities are invented.

The actual combined provider has 4,958 records. Item admission now has a
separate 8,192-record cap; the existing 16 MiB Item byte cap and 4,096-record
Creature/Presentation caps remain intact. The source-qualification and overflow
regression `w18_native_item_top_order_and_furniture_cohort_keep_input_bounds`
passes (18.15 s), and all six decoration/seasonal source policy tests pass
(1.513 s). This fixes the previous missing-carpet policy red point.

This is still a WIP integration checkpoint, not W18 completion. Published
House USE remains the earlier balloon-only owner until the pending decoration
receipt, static overlay and presentation implementations are validated and
committed. No new command, capability or mechanic ordinal is allocated here.

### W05d native Undefined damage consumer

The native creature parser and scheduled player-hit reader now preserve the
explicit `undefined` family through the existing typed HP receipt owner.
Crystal `src/game/game.cpp:8142` excludes Undefined from mana shield;
`src/creatures/creature.cpp:842-845` retains mitigation. Avatar/final reductions,
Gift and the existing lethal/death lifecycle remain with their current owner.
Crystal `src/items/functions/item/item_parse.cpp:528-536` also explicitly excludes
Undefined from `absorbpercentall`, so no new Item resistance kind or ID is needed.
Presentation uses the existing Untyped damage element, without an Earth alias.

The native parser and actual HP receipt/replay/lethal tests pass: two tests in
0.14 s. The modified source files pass rustfmt. This component compiles; full-row
calendar, producers and focused crate validation remain in progress.
The exact restriction for bridge 48227 is retained at
`tools/content-schema/native-gameplay/event_item_identity.py:3-4,18,34-46`;
it does not stop this independent combat consumer. No ID or migration allocated.

### W18 House effect and speech publication

The existing bounded presentation/chat owners now support current named-House
actors and recipients. Effect entries retain exact House scope and ownership
generation; other Houses and Channel observers cannot receive them. Generic
Channel spell outboxes remain Channel-scoped even if drained after a handoff.
Only the explicit, current-session House Item producer can attach House scope.
Crystal `wall_mirror.lua:17-24`, `music.lua:76-80`, `fish_tank.lua:3-8` and
`birdcage.lua:3-9` are the decoration speech/effect consumers.

All 13 presentation publisher tests pass (0.01 s), including the real delayed
Channel-outbox handoff regression, House/Channel and cross-House isolation,
current-session rejection, and House speech. The new House observation adapter
uses a nonblocking state lock under the runtime lock. No wire ID is introduced.
Full decoration owner and expiry integration remains in progress.

### W18 explicit furniture rotation allocation and native client

Base main `9668e0a04` next-free capability 385, command 670, mechanic ordinal
643 and authored offer 640 receive the SEASON offset +60: capability **445**,
command **730**, mechanic **703**, offer **700**. The command multiplexes bounded
catalogue queries and explicit rotation of an authoritative Item or base placement.
It leaves ordinary USE available for native cupboard opening. The native form
shows furniture names and offers Rotate; it discards the catalogue after a result
instead of guessing a new revision. Crystal `src/game/game.cpp:3054-3057`
returns the original Item when a transform target has no ItemType.

Current integration is work in progress. Focused source admission tests for the
Cake cohort pass (three tests, 0.031 s). Canonical materialization, decoration and
seasonal PostgreSQL integration, and final focused validation remain outstanding.

The explicit rotation client now decodes named catalogues, selects the exact typed
server target, renders native Rotate controls, rejects concurrent submissions and
invalid replies, and clears stale revisions after an applied response. Ordinary
USE keeps its container-opening meaning. The shipped session now advertises and
routes both existing wrap capability 207/command 402 and new rotation 445/730,
closed over House capability 49. Two UI tests pass (9.184 s build/run), the actual
native client command/decode test passes (40.226 s), and the advertisement/route
test passes (20.506 s). These complete this client component; server decoration,
container, expiry and source-profile integration remain in progress.

### W18 compiling durable integration checkpoint

The durable House decoration/static-base/container owners, normal native source
closure, explicit rotation server route, and their focused PostgreSQL fixtures
are integrated as WIP. Shared consume/transform plumbing also carries the W05d
Baker prerequisites; seasonal completion is not claimed by this checkpoint.
The full focused dispatch test binary compiles. Actual PostgreSQL cases now pass
for nested container moves/replay/owner fences and mixed-subject exit/disposition
(45.514 s), bathtub (45.471 s), conflicting carpet refusal (45.572 s), fish tank
(47.594 s), magic tree (45.329 s), music (45.342 s), and mirror cooldown/replay
(45.367 s). These are real production-caller cases, not schema-only smoke tests.

Exact red points remain: map-target bed-kit USE routing; native rotation through
the House service command gate; timed aqueduct rotation refuses NotPossible
before the new timer owner is installed; static fixture reach/reset setup.
Auto-carpet neighbor occurrences, authored base-container materialization,
source25-slot container negotiation, complete tree-preserving disposition and
remaining seasonal stages are still being completed. No row is marked DONE.

### Compiling seasonal retry/extraction and House timer checkpoint

The shared scoped native source reservation, committed creature machine-step
consumer and acknowledged raid extraction now compile alongside the timer owner,
static/bed routing fixes and seasonal NPC retry/empty-attempt consumers. The
all-target/all-feature dispatch and game-server check passes in 2m58s. Existing
Player reservation APIs remain intact. New source capture closes25 admitted
seasonal Item policies, including the independent Cake map/reward definitions;
bridge48227 stays excluded under its exact owner decision.

This is a compiling WIP, not row completion. Focused caller tests, current native
provider/loot/Prey regeneration, timer/bed greens, remaining seasonal phases and
source-qualified placement, complete furniture projection, clippy and final
pins/architecture qualification remain required.

### Native Creature/Loot successor qualification

Normal Prey regeneration completed15 real runtime model cases with777 creatures
(5m45.797s); normal registration regenerated43 files (1m28.914s). The actual Rust
artifact-loader regression passes in2.18s (25.60s build), including refusal when
the Cake binding is removed. Exact source-backed sugar chance and corpse12141
are retained; no fabricated empty loot table is used.

Further reserved allocations: supplementary House container capability446
(next main sequence386+60) retains legacy capability14's20-entry bound and
negotiates25 for source-qualified House containers; no new command/domain.
Santa delivery audit field66 is next message field6+60, preserving legacy bytes.
Migration1163 is reserved for exact House auto-carpet occurrences;1166 for Cake
phases/bonus. Santa delivery extends this lane's1165 rather than taking either.

### Scoped actor reservation regression proof

All253 entities unit tests pass with the required test-support feature (5.90s
execution,6.076s total). The scoped unknown-outcome fixture uses the actual
native RaidSpawn creature producer; it proves pending movement/lethal refusal,
exact identity resumption, and that the same positive-health lethal writer
succeeds after release. Legacy Player reservation coverage remains green.
The machine predicate now recognizes the actual qualified Terrain ground8886
as well as a top Item source, citing Crystal items.xml23647-23649.

Reserved world raid-effects kind65 is next existing kind5+lane60. It is for
source-qualified Cake notices in the existing outbox, with independent current
Channel recipient/ACK proofs and immutable lure/phase receipt binding; ordinary
kinds1-4 cursor and terminal rules remain unchanged. No notice code is marked
complete by this reservation.

### Complete source wrap and Carpet classification capture

Normal source tooling now captures2015 initialized furniture profiles and1617
XML forward wrap pairs. Reverse unwrap still requires existing purchased/prior
wrap provenance in the durable owner; this catalogue alone grants no provenance.
All161 XML ItemTypeCarpet identities are classified separately from160 Lua USE
handlers: folded26151 remains nonCarpet;51222/51223 have initialized raw id+flags
in both pinned donor appearances and exact existing canonical references. The
existing Carpet enum ordinal2 is reused. Nine source tests pass in1.982s and
source-generator check passes. Shared native provider refresh remains required.

### Native House capacity and Cake phase checkpoint

Capability446 now negotiates the source-qualified25-entry House container
projection through server snapshots, session decoding and the existing native
container window. Legacy peers retain the20-entry bound. The new client case
selects and opens entry25; runtime validation is pending at this checkpoint.

Migration1166 mounts the shared Cake phase/window/bite receipts and native
terrain projection. Source rules and citations are recorded in1166 and the
Cake owner modules. This checkpoint does not claim annual rewards, regeneration
or the complete seasonal row green.

Reserved normal MapMINT House destination field66 is next field6+lane60;
materialization House key66 and scope generation67 are next fields6/7+lane60
in their separate message. These fields admit a named House interior through
the existing materialization owner; no command or mechanic ID is added.

The negotiated House wire-bound regressions both pass (2.74s build/run),
including the exact12,862-byte worst-case payload and legacy20-entry refusal.
The decoder's second non-browse byte guard now follows authenticated446
selection. Protocol all-target/all-feature clippy-D warnings passes12.10s;
full session176 tests pass21.19s execution. Native entry25 selection/open test
passes11.81s build/run. Other adapter follow-ups remain independently tracked.

Native adapter follow-up: depot/trade reuse the authenticated typed session
container projection; shop/browse pass negotiated446 to the extended decoder.
The generic backpack label remains unchanged. Native UI2 tests pass3.76s;
depot regression passes4.59s; native shop+browse2 tests pass35.47s build/run.
These checks preserve the legacy refusal cases and select the actual entry25.
All721 protocol unit tests also pass0.59s execution.

### Ruprecht canonical source capture

The normal seasonal capture now retains48 records, adding all23 input/output
identities used by Crystal Ruprecht's22 exchanges (the exact pinned script
and line citations are recorded by seasonal_item_profiles.py). Existing
canonical identities are reused. Four capture tests pass0.379s; no new number
or migration is allocated. Actual NPC exchange callers remain in progress.
Native provider registration awaits its required normal Prey model refresh.

### Initialized appearance wrap eligibility

The source pipeline now captures2,126 qualified House profiles and1,860
initialized wrap-eligible forms; explicit XMLpairs remain1,617. Crystal
item_parse.cpp203-206 and items.cpp238-241 establish XML and appearance-default
eligibility. Player wrapping uses decoration kit23398 per game.cpp4897-4913;
XML target values do not replace that production rule. Existing legacy207
6355/6371 compatibility remains separately fenced.

Ten House source tests pass2.213s. Actual PostgreSQL initialization plus
positive rack/transient eligibility and negative unknown/reverse assertions
passes4.987s. The normal SRX1 dispenser owner coalesces only its exact captured
profile plus verified source-top metadata; three regressions pass2.265s,
including changed-weight and unverified-top refusals. Required normal Prey
refresh/provider regeneration is still running, so this is source capture only.

The full negotiated-capacity boundary now passes through actual session join
and sequenced delta frames:16 views x25 entries, exact12,862 bytes, all400 typed
entries retained and legacy decoding refused (7.28s build/run). No generic
limits were widened: domain14 already allows15,488 bytes. Four changed client
crates pass all-target/all-feature Clippy-D warnings in1.19s after the test
uses Result propagation. The complete native client345/UI274 tests pass16.57s
and0.01s execution respectively.

### Full-validation owner rule, November7

The owner now requires every inverse normal workspace dependency's complete
Nextest suite, including PostgreSQL, before any push, plus periodic main merges.
The initial inverse closure has32 crates (recorded by cargo tree); this supersedes
selective green-step pushes. Full failures will be collected and grouped before
fixing and rerunning the entire set. No further push is claimed before that gate.

Native provider registration passes92.238s and its check96.226s; the normal
Prey simulation passes15 cases/777 creatures (Cargo2m42s, about6m30s total).
Normal pin-refresh succeeds with digest4730c4d24e4c5c643041c67a38fc5c7202a98576b9189d3f89fb632402eccf54.
Actual PostgreSQL timer cases pass59.738/61.446/67.912s, static aqueduct86.488s,
bedkit85.106s, expanded container53.308s, capacityV253.270s, nativechair56.808s,
nativecupboard84.368s and Cake extraction76.066s. An earlier disk-exhaustion
batch's unexecuted failures were rerun rather than counted as mechanics verdicts.
Cake phase and seasonal NPC/contribution failures remain to be grouped with
full Nextest results. Known regeneration RED tests remain intact pending fix.

### Owner-requested compiling WIP checkpoint,16:07UTC

At the owner's latest explicit request to push compiling WIP immediately, this
checkpoint includes the local main merge41b58cac0 (origin/mainbbb3e06d0).
The complete32-crate all-feature Nextest build passed in7m34s; its9,721 selected
tests across105 binaries are still running with PostgreSQL and no fail-fast.
Existing43 ignored external-topology/capture/measurement tests retain their
repository annotations; no test was weakened or newly skipped.

Exact known red points retained: condition_cake_regen_tests' three source
merge/tick assertions; focusedPG Cake phase Exhausted versus Committed;
Christmas/real Baker invalid journal state; equipment-only Baker parent-capacity
constraint and partial-extra source-count completion. Full failure collection
continues before the prepared fix batch and full rerun. No green row is claimed.
The complete31-case Python source rerun now passes472.453s.

### Owner replacement validation rule,17:48UTC

The owner replaced the full inverse-dependency Nextest requirement with own
mechanic RED/GREEN cases (including real PostgreSQL), workspace all-target
compile checking, changed-crate Clippy, fmt, duplicate-migration checking and
normal generated-content/pin regeneration. CI owns the complete suites and its
failures will be repaired together. The obsolete full run was interrupted at
the owner's explicit direction:4,573 passed,53 reported failed/interrupted,
4,626/9,721 executed; execution6,423.663s, total6,880.135s. This is diagnostic
evidence, not a passing full-suite claim. Existing43 ignored annotations stayed
unchanged.

Merged origin/main1ae39c7fb (QENGINE2/QBASE0999-1008). No1200-1205 migration
copies exist. Collected fixes cover strict source inventory regeneration,
independent regeneration pulse persistence, Cake equipment parent truth and
exact decrements, set-based complete200-input proofs, historical NPC retirement,
normal400-bed source facts and fixtures, and closed guard/read inventories.
The new main's53 qualified reward-item profiles are a separate measured cohort;
no old speed/ability, source authority or immutable-write checks were removed.
Native provider regeneration passed98.227s. Focused build/PG results follow.

Main reintroduced its legacy root contract notes. The merge preserves that
unchanged evidence with git mv at docs/reference/QENGINE2-CONTRACT-NOTES.md;
the repository root remains clear and all SEASON additions stay in this file.


### Owner compiling-checkpoint direction (2026-11-07)

The owner superseded the pre-push test gate: publish the current state after
`cargo check --workspace --all-targets` and `cargo fmt --check`; CI runs the
full suites. This checkpoint is unfinished work, not a DONE claim for any row.
Changed-crate Clippy passed for all 15 packages in 208.907s. The focused five
PostgreSQL cases executed in 123.658s (434.364s including build): three Baker
physical-input cases passed; the real Baker NPC and Santa NPC cases still fail
at their inventory_changed assertions. No test was weakened, skipped or deleted.
The earlier 67-case mechanic run passed 64 cases, with the three failures
subsequently investigated; the nested-stack retirement repair now passes.
Pin refresh passed in 33.208s and pin-check in 29.525s. The normal Python world
tree validation passed in 593.019s. Remaining native NPC failures and private
feature drafts are follow-up work; they are not represented as completed rules.

Publication gates passed on this checkpoint: `cargo check --workspace --all-targets`
243.796s; `cargo fmt --check` 15.405s. Duplicate migration numbers: none.

### Time-bounded checkpoint after upstream movement

Fetched origin/main at e1ca3e003. A merge produced conflicts in the seasonal
world-event test, durability world-event and reset activation, and session chat
tests; aborted under the owner's ten-minute checkpoint direction. Integration
remains pending. Native Santa/Baker test assertions now print the disposition,
unknown-commit flag and conversation to distinguish early refusal from an
ambiguous write or source dialogue. Their inventory-change assertions remain
strict. Regeneration and changed-crate unit/own-PG reruns are pending for this
checkpoint under the owner's explicit PREPUSH-2 steps 2-3 exception.

The seasonal native-NPC fixture omitted first-entry runtime Character lease
binding after replacing its runtime. Santa and Baker both enforce that lease
before their inventory writer. The fixture now binds the exact current item
fence Character/generation through the production runtime API, matching
`monk_save_postgres_tests.rs` first-entry setup. Native refusal guards and all
inventory assertions remain unchanged; own PG confirmation is pending.

Timed publication gates: workspace/all-targets check passed 162.149s;
fmt --all --check passed 14.314s.

### Native NPC lease regression and main integration

The actual native Baker/Santa PostgreSQL regressions now pass after the
first-entry lease repair: Baker 77.772s, Santa 77.801s; nextest execution
77.804s, build plus run 415.548s. These are real database cases, not skipped
environment guards. No mechanics row is claimed complete by these results.

Merged main47db34f09: retain its world-change occurrence validation and
consumer fixtures alongside the seasonal calendar, Cake post-reset hook and
first-entry actor lease. Both active-world-change and active-seasonal event
announcements remain asserted. The normal merge resolver regenerates native
providers, content trees, central authoring lists and all world pins.
Changed-crate unit tests and post-merge native-NPC PG reruns remain pending
under PREPUSH-2's cadence exception; pre-merge PG evidence is recorded above.

The complete normal merge regeneration (including registered freshness checks,
five content repository tests, and world pin refresh) passed in 2089.967s.

### One-push publication and freeze (owner direction, 2026-11-08)

The owner requested this current checkpoint as one merge commit and one push,
then a frozen branch accepting only owner-supplied CI fixes. Complete rows are
not claimed: W12 donor-formula work and the remaining full-podium/carpet
implementation are unfinished; private feature drafts are not mounted.

The merged-state dispatch binary executed all three seasonal PostgreSQL cases:
3 passed, 0 failed, 224.071s wall time (224.02s test harness). Santa recorded
114.534s; Baker 169.719s; Halloween also ran for over60s and passed.
Workspace/all-targets check passed286.277s; fmt15.731s; changed resolution
crates' Clippy passed649.198s including build-lock wait; pin-check33.341s.

The15 changed crates' unit driver was started with dispatch isolated for
memory bounds, then the other14 crates queued. Only PostgreSQL-labelled
cases are excluded from the library-unit phase; own PG runs are separate.
The dispatch batch selects1605 tests. It exposed a stale authored charm
population pin823 versus the admitted824 Bestiary entries. The normal
Bestiary inventory generator now refreshes that strict assertion alongside
its existing reader/validator/golden pins. Its three generator tests passed
0.026s; actual regeneration0.079s. No threshold, refusal or source-validation
assertion was removed. The Rust count-assertion rerun is pending.

PREPUSH-2 explicitly permits publication after steps1,2,4 when step3 exceeds
the cadence budget, with pending tests disclosed. This publication uses that
exception: remaining dispatch library tests, the other14 crates' library
tests and the broader focused House/Cake PG rerun remain pending. Existing
focused House/kernel/protocol/native-client and PG evidence is recorded above.
The running driver may finish read-only after the push; no further source
changes are authorized until the owner supplies the CI failure list.

Final publication gates after charm-inventory regeneration: workspace/all-targets
check passed370.565s; fmt --all --check passed20.524s. Staged-tree duplicate
migration check passed; no own migration renumbering was needed.

### Owner-requested pre-PR integration with main6f61ac15d

The owner lifted freeze for one integration step, then one push. Fetched main
6f61ac15d93bbda9f9d718721233c0d067f40530 and the actual open PR heads:
3034=3a3066edd52bbf6e17b7f18cfdc95bed046b3daa;
3036=8167756115a4ff0834a014496ceed56604e4820e;
3037=ad3a63611138b8949ea6303cb2798d9a0bf5123a.
Enumerated every migration at all four commits: none occupies any of this
lane's1160,1161,1162,1164,1165,1166,1167,1168,1169 numbers. No renumbering
is necessary.1163 remains unused by the committed lane.

Merge compatibility retains the upstream floor-assembly ground path with an
explicit absent House-loop context and initializes Santa's newly required
non-fluid child subtype to None. Neither widens House authority or gift sets.
Native generation composes the fresh Cake/native loot closure with WORLDCHG's
closed admission update before final repository pinning, preserving source
drop tables and both owner cohorts. Derived captures and pins are regenerated
through the normal merge driver, not edited by hand.

The previous frozen-head unit run eventually completed1605 cases:1604passed,
1failed, execution1567.052s/build+run1923.212s. Its sole stale823-vs824 charm
pin was already fixed by the normal inventory generator before publication;
those historical results are diagnostic, not claimed as green for this merge.
Fresh validation for this integration follows below.

### Follow-up main updates within the same unpublished integration

The owner then requested main98d6ae8ce (Soulpit1140-1141), followed by
main47baa7bb8 (key33 floor-transform permit and8MiB cosmetics provisioning
bound). Standard local merge commits preserve each requested ancestry; no
intermediate push was made. All nine SEASON migration numbers remain free
in those main revisions and the three fetched PR heads.

The completed normalized WorldProject package was retained across the
Soulpit merge: its eleven package documents and all promotion facts are
unchanged upstream; source/world/map-owner evidence hashes were refreshed
for Soulpit terrain. The registered legacy/native/authoring generators and
final world-pin refresh ran against the combined candidate, passing631.271s.
The final47baa7bb8 delta changes only floor-transform scope checking, its
caller and cosmetics provisioning validation; pin inputs are unchanged.
Pin refresh and pin-check were rerun:26.118s and27.319s, both passed.

Mutable generated WorldProject declarations/references now load only through
their current hash-bound package manifest, with schema, revision, unique
member, size and duplicate-identity checks. The global immutable HEAD reader
is unchanged; the complete Cake donor packet, bindings, normalized Creature
and Loot source checks remain strict. New tests reject forged pins, altered
canonical damage and duplicate identities. This resolves regeneration of a
legitimate merged package without weakening the donor-source qualification.

All20 Cake/provider/inventory Python tests passed182.408s after updating the
exact QENGINE unique-key count from53 to its source-captured60. Duplicates,
missing keys, exact packet bytes, per-record semantics and source attributes
remain asserted. The six content-independent Rust crates passed1954 unit
cases on the Soulpit revision (120.664s including build); the final four-file
main update changes none of those crates or their inputs.
Final workspace check passed228.464s; fmt passed15.803s. Provisioning-bound
tests passed0.116s. Focused PG and remaining unit/Clippy results follow.

Final isolated focused validation:11/11 passed, 574.687s build+run.
This includes nine real PostgreSQL cases and two strict inventory unit tests;
per-case durations are retained in the validation log. The200-input case
also passed alone45.265s. Earlier runs while other suites created/dropped
databases reached the unchanged2s transaction deadline; no production
timeout, trigger, assertion or source guard was weakened. Read-only rollback
probes measured200 shape checks94.719ms, admission79.089ms and200 full
completion checks488.102ms. The final batch runs without competing DB tests.

Final Clippy across all15 changed crates passed741.166s including lock wait.
PREPUSH-2's cadence exception still applies to remaining full library suites:
six crates passed1954 tests; the nine-crate batch executed418 passing cases
before4 SIGINT cancellations when stopped to isolate PG validation;3560
remained unrun. This is explicitly incomplete unit evidence, not a green
3982-case claim. CI owns the full suite; no tests were weakened or removed.
This single publication includes all requested main updates through47baa7bb8.
After publication the branch returns to frozen status, awaiting owner CI fixes.

## Main integration c77d436d8 (2026-11-08)

Merged origin/main c77d436d8 without rebasing. Own migrations remain
1160-1162 and1164-1169;1163 is unused. No new command, capability,
mechanic, registry or migration numbers are allocated by this integration.

Regenerated QREADY equipment additions through
`tools/contracts/update_equipment_profile_additions.py` and native profiles
through `tools/content-migration/regenerate_content.py --native-only`.
The independently declared cohort sum and actual profile union both equal
5768. The existing53-key QENGINE cohort is counted once; its explicit Rust
subtraction must match the declared cohort or the refresh tool rejects it.
All six equipment refresh tests pass, including wrong-subtraction rejection.

Rechecked NPC service/offer counts as requested. Source catalogue counts
remain1282 NPCs,836 dialogues and399 services. The production model adds
Grizzly's source-qualified shop (`npc_catalogue/service.rs:367`,
`npc_catalogue/grizzly_shop.rs:16` under crates/gameplay/content/src/content).
Its existing fluid/full-charge restoration is in
`crates/gameplay/content/src/content/npc_catalogue/subtype_trades.rs:78-138`.
No admission rule changes here. The old390-service snapshot failed against
400 services; the complete model reports12415 admitted offers,195 routes,
446 generated-reply NPCs,31 non-gold holds and17 price-cap holds, with all
other hold categories zero. Main3209d3d61 supplies the exact strict service/offer snapshot with the
source explanation. Per owner direction its test version is retained.
No computed equipment or world pins were hand-edited.

## PR3039 CI correction, same c77d436d8 merge publication

All nine owner-reported Rust regressions passed together (30.301s execution).
The repeated partition24 Soulpit failure is the same source identity boundary
case: it now requires the exact admitted Cake Golem key and rejects an
unqualified lookalike. The primary raid area is validated even when its
source-backed alternate city bases are present, retaining the original
fail-closed test instead of bypassing its malformed area.

Source evidence is Crystal201189f
`data-global/monster/quests/a_piece_of_cake/cake_golem.lua:1` (identity),
`:11-19` (easy/very-rare Bestiary, [2,3,5] and30 points), `:25` (venom),
`:76-95` (three attacks and one defence), and
`data-global/scripts/lib/a_piece_of_cake_config.lua:394-400` (raid bases).
`src/creatures/combat/condition.cpp:1348-1356,1385-1391` resets each permitted
regeneration accumulator after one pulse; the exact actor test therefore
expects140 HP/114 mana after PZ rather than granting retrospective mana.
The source projection adds no unsupported attack: the pinned gap count
remains116, with1690 supported melee and4200 supported spell attacks.

Normal tools regenerated presentation bindings (Canary CONST_ANI_CAKE=42),
all1864 source-bound creature races, and quest NPC binding evidence/index.
The race builder admits only Cake's exact Creature definition-r1, target,
file and201189f revision as the additional source; forged revisions, targets
and files are rejected. Both new source-boundary tests passed. Presentation
validation passed15 tests17.152s; quest binding-index validation passed6
tests0.305s; the merge-regenerator checks passed14 cases.

`refresh_season_creature_inventory.py` qualifies the full Cake donor packet
and canonical identity closure before refreshing Creature/schedule, Bestiary,
raid and attack-coverage pins. Non-seasonal populations and the116-gap
boundary remain explicit rejection guards. Its source-checked results are
1864 Creatures,7205 schedules,824 Bestiary rows,240 raids/137 active.
Native registration regenerated44 outputs279.462s; only five registry/index
files needed new bytes after the race/presentation refresh. The generated
mechanic profiles remained byte-identical. No computed pins were hand-edited.

Focused seasonal/House validation on the merged source passed11/11, with
608.562s execution (1153.785s including compilation/lock wait). Its nine
PostgreSQL cases ran45.166-88.170s each; strict raw equipment validation
ran59.419s and the complete charm inventory test0.932s.
Six independent library crates passed1954 tests52.485s execution.
Client-assets passed10 unit tests1.49s. NPC catalogue/service integration
tests passed all six cases, retaining1282 NPCs,399 declared services and
400 projected services/12415 admitted offers. Both architecture modes
passed (workspace10.009s, client-structure2.397s).

The seven-crate batch passed1865 tests887.037s execution; the six CI-affected
crates passed1468 tests930.766s execution, including95 explicitly named
PostgreSQL cases with individual durations0.264-170.085s. The server
HOUSE ACL add/remove cases also passed53.918s/23.235s. The full dispatch
and server suites remain pending under PREPUSH-2's cadence exception; their
focused mechanics and integration targets passed. Existing ignored retained
source/host-measurement fixtures are untouched.

The merge resolver had seeded main's pre-Cake root summaries and seven
retired shard paths despite retaining the source-qualified new shards. The
normal raid generator refreshes its240-record index; the seasonal inventory
tool checks exact identity-set equality against the hash-bound WorldProject
before updating central counts, source bindings, source blobs and managed
shards. A same-count Creature substitution is rejected (3.849s regression).
Both normal regeneration modes now invoke the source-qualified refresh.

## Final owner-requested main update3209d3d61

Merged MAINRED47 without rebasing. Its five changed files are taken exactly
from origin/main, including the strict NPC400/12415/17 snapshot and the
read-only Soulpit contention/25P04 classification handling. The temporary
NPC snapshot generator/fixture was removed in favor of the owner's main
version; the original test function and all metrics remain. The normal
dispatch build regenerates PostgreSQL test mounts for the added Soulpit
boot tests. Source content and migrations are unchanged by MAINRED47.

Final3209d3d61 validation: focused19/19 passed, 970.511s
including compilation/lock wait. This repeats all eleven seasonal/House
focused cases and runs both new real Soulpit boot/contention PG cases,
owner-read retry semantics, all four SQLx classification/commit certainty
tests and exact Soulpit native identity lookup. Full source-tree and
Item/Mount goldens passed249 managed paths,69 Item shards, including all
closed-negative relation and Bank cases. Workspace all-target check
519.142s; changed dispatch/durability Clippy
213.266s; fmt 16.224s;
main's NPC snapshot test 192.465s (all green).
Earlier19-crate Clippy passed525.489s; unchanged crates retain that proof.
Full dispatch/server suites and the post-MAINRED47 full durability rerun
remain with CI under PREPUSH-2's cadence exception; focused current-source
cases above passed. No tests, constraints or source-gap guards were weakened.
After this single push the branch returns to frozen CI-fix-only status.

## PR3039 final quest-authoring CI repair (2026-11-08)

Main3209d3d61 was fetched and was already an ancestor of48bfdb692 before
these changes. The QREADY1 requalification and talk-NPC packet failures
were both reproduced locally. House top-order enrichment was adding two
redundant zero/default attributes to all six QREADY1-owned reward profiles.
Those initialized defaults are Crystal201189f src/items/items.cpp:203-208;
the exact reward facts belong to qready1-reward-item-profiles.json. House
admission now preserves those records and rejects nonzero source top-order
conflicts instead of silently discarding meaningful drawing-order facts.
The existing strict QREADY1 comparison is unchanged. A new regression was
red before the fix and checks all six complete records plus conflict refusal.
No pinned PostgreSQL test, reward proof, native identity or numeric
allocation changes. Talk-NPC candidates are regenerated after all content
and quest packets, using quest_talk_npc_candidates.py.

Owner's compact-format add-on: lane-authored packet generators now share
compact_record_json.py. House profiles/actions shrink354115->2141 and
73194->6871 lines; seasonal Item profiles7182->69; Cake admission1552->249.
Archived source evidence and seasonal raid captures use the same record-line
layout. JSON-load equality was checked for all seven packets, allowing only
the generated catalogue/capture-tool hash changes. Migration1161's VALUES
formatter emits eight tuples per line:9860->1286 lines, identical SQL tokens.
House, seasonal Item and Cake source generators pass their --check modes
against pinned donor snapshots. Nineteen compact/House/SRX tests passed.
The committed immutable Cake source packet requires a local checkpoint
before downstream native regeneration; publication remains one final push.

Post-checkpoint generation also refreshes the Cake raid capture hash and
three Cake-golem attack-authoring rows via build_monster_attack_authoring.py.
The current main f5b584ef1 is merged (only repository CI policy changed).
The broad native-gameplay Python discovery exposed an unrelated existing
loot-section byte comparison failure in
test_full_spell_artifact.LootTablesTests.test_checked_in_section_is_generated_from_the_production_profiles.
It reproduces unchanged at published head48bfdb692 (2.821s): the standalone
loot producer lacks the registered WorldChange refinement/serialization.
No WorldChange facts or tests were altered in this frozen quest/format repair.

Final focused Python verification: compact/House/SRX19 tests pass16.442s;
seasonal Item, source audit, raids, Cake successor/admission19 pass125.891s.
House, seasonal Item, Cake and attack-authoring --check modes pass.
Native-gameplay discovery ran130 tests164.191s:115 pass,14 existing retained
source-fixture skips, and only the baseline loot comparison above fails.
Architecture workspace boundaries and duplicate-migration checks pass.
Migration numbers remain1160-1162,1164-1169;1161 formatting only.

Final f5b584ef1-merged tree: exact CI quest-authoring checks runner passes
509.362s; unit-tests runner passes513.597s. Source/content-tree validation
passes315.809s. Both originally failing quest modules are green unchanged.
Talk-NPC candidates were regenerated last after the final main merge.

Real PostgreSQL proof (admin URL present; no PG source/test edits):
qready1_crystal_wand_quest_exact_reward_once passes115.740s, minting3068
through both Crystal Wand reward claims and checking exact-once semantics.
w18_house_container_capacity_v2_retains_legacy_bound_and_preserves_graph
passes52.583s through the compacted migration1161; nextest2/2 pass168.327s
execution (compilation/lock wait excluded). No reward proof re-import needed.

Changed content/native-admission-source crates: clippy --all-targets with
-D warnings passes66.180s. World bundle pin-refresh passes59.305s and
pin-check passes67.413s; final pins are already current (no hand edits).

Both changed Rust crates' complete nextest run passes488/488 tests,531.312s
execution (one existing ignored case;1081.878s including build/lock wait).
Final workspace all-target check passes370.559s and cargo fmt --all --check
passes16.447s. Before publication origin/main was fetched again and remains
f5b584ef1, already merged. No tests, pins or source-gap guards were weakened.

## PR3039 S17 comparison and encounters-main merge (2026-11-08)

The exact requested PostgreSQL test was run without changing its source:
`s17b_native_observer_map_creatures_effects_and_public_speech_dispatch`.
Main64d8bb8b8 passed43.045s test execution; branch ea351ee4b passed47.118s.
The branch's same compiled test passed three additional isolated runs under
main-build load:48.493s,48.264s,48.852s. Admin PG configuration was present.
The CI Stop502 rejection was not reproduced; no S17 implementation or
assertion changes are justified by this comparison.

The owner then requested encounters main c68395ccc (#3034). It is merged
without rewriting history. Derived conflicts are resolved by the normal
world/native generators. Combined Creature and Loot shards are checkpointed
because the native successor's immutable-input check requires their bytes
in HEAD before it can compose profiles. No source gate or test was weakened.
Quest binding evidence, completion matrix and talk-NPC packets are regenerated
after the merged content, with talk-NPC candidates last. No PG test source,
proof SHA or numeric allocation is changed by this lane's merge resolution.

The seasonal inventory refresh now replays main's `primal_reference_overlay`
before exact family membership checks. Crystal201189f
`data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:79,118`
explicitly supplies corpse39530 and loot={}; the existing normal producer and
content-tree validator verify the immutable capture before admitting that
extra Loot identity. The combined source-qualified inventory regression was
red before this composition fix and both positive and unqualified-extra-Loot
rejection tests pass7.891s. Family equality and all existing source-gap pins
remain strict. No new rule or inferred loot admission is introduced.

The all-target workspace check collected five Cake fixture API errors after
#3034: two movement-cell references now need `SharedOwner::Borrowed`, and
three Ground palette initializers need `Terrain.blocks_projectile`. The
fixtures retain their borrowed cells and use None for the additive field,
which preserves legacy terrain-kind behavior (world-bundle/bundle.rs:140-142).
No assertions or scenario preconditions change. Both affected real Cake PG
cases are included with the S17, House-container and QREADY1 focused rerun.

### QMECH M18 selected First Dragon

No migration or command/capability number is allocated. Existing Boss Activity wire
commands and tables own the selected encounter. The explicit chosen-recipe adapter
is `content/encounters/bosses/selected-first-dragon.json`; its generator checks pinned
Crystal donor blobs and leaves the original unresolved full-phase capture unchanged.
Donor file:line details and immutable source URLs are recorded in
`docs/reference/content-prep/qmech-season/M18.md` and the generated packet.
