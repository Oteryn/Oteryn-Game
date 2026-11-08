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

## W07/W08 Soulpit shared owner (2026-11-07)

- Allocation base is `9668e0a04e`, lane offset `+40`. Migration numbers
  `1140` (terminal return proof) and `1141` (runtime staging floor) are inside
  the assigned `1140–1149` range. Activity return reason `52` is main's next
  unused reason `12 + 40`. Command `710 ACTIVITY_SCOPE_ACK` is main's next
  unused command `670 + 40`. No capability, mechanic ordinal or offer-order
  number is allocated; the acknowledgement belongs to existing capability 117
  and state domain 99. Existing donor source appearance IDs are retained.
- Crystal `201189f` `data-global/scripts/quests/soulpit/soulpit_fight.lua:37-106`
  validates all five staging positions, consumes one core and starts the fight.
  The native adapter reserves the same shared Instance carrier/session handoff;
  its Type2 transaction commits the debit, craft audit, encounter, participants
  and every fresh destination session together. A failed successor rolls them
  all back. Core species uses the existing reverse-variation/name resolver and
  pinned Crystal roster, then preflights the real native AI closure before debit.
  The exact selected core definition/revision is retained in that request and
  its replay binding; the custody writer refuses a transformed replacement
  before debit, so the species consumed is the species that passed preflight.
- Runtime/LastPosition floor is source z `8`; map `TilePos` is `-8` under
  ADR-0021. Migration 1141 fixes the deferred participant proof forward while
  retaining exact source-position and same-transaction admission checks.
- Crystal `201189f` `data-global/lib/others/soulpit.lua:31-140` and
  `data/scripts/globalevents/encounters.lua:12` supply the waves, independent
  spawn/kick deadlines and terminal cleanup. The existing World owner drives
  registered current Instances even with no connected participant. Private
  Bundle overlays and door facts are independent from the originating Channel.
- The task's missing static-map projection extends the older nonground Terrain
  rule in `OTERYN_GAME_MAP_LOAD_PACKET1_BUNDLE_LOADER_DECISION_2026-10-04.md:124`:
  nonground `walkable` may carry a known boolean, while nonground speed stays
  absent. Old `None` semantics are preserved. The bounded 55-record Soulpit
  profile derives exact ground/layer/unpass flags from the admitted official
  appearance artifact; Crystal `201189f` `src/items/items.cpp:175-176,203-208,227,233,244`
  supplies their physical interpretation. No unknown decoration is invented as
  a wall and no native Item alias is created. Generator and repository-B3 tests
  bind the exact artifact, membership and placement digests.
- Crystal `201189f` `src/creatures/players/player.cpp:6584-6607,12932-12936`
  supplies stack-40 XP and weapon points. The real lethal callback freezes
  contributors and equipment in the existing settlement queue; other stacks
  and summons do not enter ordinary loot or progression callbacks.
- Crystal `201189f` `data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106`
  supplies survival mastery. Completion and Animus grants remain one lifecycle
  transaction. Shared death/logout/exit returns remove membership atomically;
  independently proven owner-loss recovery interrupts the old fight, without
  restarting process-local actors/timers or granting survival mastery.
- Crystal `201189f`
  `data-global/scripts/actions/soulpit/soulpit_arena_exit.lua:8-16` supplies
  USE-only exit. The client consumes pinned core/obelisk/exit handles and the
  existing domain-99 shared-session receipt. Soulpit barriers additionally
  require command 710 bound to the exact handoff and destination session after
  the full destination snapshot. The server preserves partial source frames
  and discards in-flight source commands until this acknowledgement, preventing
  generation-one commands from executing in the fresh destination namespace.

- Passive Soulpit species retain their admitted targeting flags. Crystal `201189f`
  `data-global/scripts/quests/soulpit/soulpit_fight.lua:67,87-88,131` accepts registered
  cores without a hostility filter. `src/lua/functions/core/game/game_functions.cpp:288-310`
  creates the ordinary source type and applies a Soulpit stack;
  `src/creatures/monsters/monster.cpp:2341-2348,2752-2759` changes stack, HP and reward
  policy, while `monster.cpp:258-259` still reads hostility from the original type.
  Preflight therefore validates the native profile closure without imposing ordinary
  hostile-AI eligibility. Only an owner-verified current Soulpit actor bypasses the
  ordinary admission hostility filter; its targeting profile is unchanged. The actual
  native Chocolate Blob/core `49087` regression proves ordinary refusal, Soulpit
  admission, retained passive targeting, and rejection after actor retirement.

- The Soulpit static authoring generator also owns the client's placement-index
  digest in `crates/client-assets/src/world.rs`: it hashes the exact emitted index
  bytes and requires exactly one constant projection. Its check covers the Rust
  consumer together with the JSON inventory.
- Soulpit reads retry only the existing nonblocking root-holder busy result under
  the existing command budget; authority, history and unknown-commit failures
  remain closed. Autonomous traversal filters current readiness before its bounded
  page limit so an earlier unready fight cannot hide a later current fight.

### Real PostgreSQL proof (2026-11-07)

All eight cases ran against the configured PostgreSQL route with migrations
1140/1141 and existing admission/custody/deferred guards enabled:

| Test suffix | Seconds |
| --- | ---: |
| `soulpit_native_item_use_entry_return_and_animus_query_postgres` | 40.424 |
| `soulpit_production_owner_spawns_native_wave_and_timeout_returns_without_mastery_postgres` | 39.750 |
| `soulpit_native_entry_refuses_insufficient_wave_capacity_without_burning_core_postgres` | 40.269 |
| `soulpit_shared_entry_debits_core_and_commits_real_instance_session_postgres` | 44.128 |
| `soulpit_terminal_return_requires_checkpoint_and_interrupt_never_grants_mastery_postgres` | 52.329 |
| `soulpit_survivor_completion_and_animus_commit_together_postgres` | 42.072 |
| `soulpit_failure_after_animus_insert_rolls_back_completion_postgres` | 42.352 |
| `soulpit_shared_entry_uses_current_trusted_gm_group_not_forged_hint_postgres` | 41.241 |

The shared fixture refreshes and holds authenticated Account evidence immediately
before successor admission, exactly like production; content loading must not
consume the existing five-second freshness window. Failure probes use durable
sequence observations to prove execution reached the debit or Animus insertion;
all rollback and replay assertions remain active. The 1,510 frontend tests and
seven runtime orchestration tests also pass. Separate focused validation passes:
21 backend tests (292.201 seconds including compilation), 14 bundle/asset tests,
six terrain compiler/codec tests, four architecture registration tests, five
repository-content tests (158.48 seconds execution), and 123 server content tests
(8.26 seconds execution). All-target clippy for all 15 changed crates passes
with `-D warnings` (166.078 seconds); formatting (14.934 seconds), architecture
(3.942 seconds), and pin-check (32.071 seconds) also pass.

The final parity sweep repaired the donor GM level exemption using the existing
current Game staff projection. The staff producer remains outside this lane;
the Soulpit consumer proves the current group at its locked entry. The focused
GM, forged-hint and unconfigured restarted-owner proof passes in 41.241 seconds.
It proves each owner's exact session/fence through the normal durable status
reader before checking exemptions. The immutable signed roster is hash-bound;
only current group/head rows are share-locked. Direct checks under
`oteryn_game_runtime` prove the roster lock is forbidden and the repaired
group/head lock succeeds, preserving the existing read-only roster privileges.

- Crystal `201189f` `data-global/scripts/quests/soulpit/soulpit_fight.lua:54-61`
  exempts current gamemasters from the level-8 entry requirement. The shared entry
  transaction resolves each participant's group from the configured trusted environment,
  activation and current roster, joined to the current control head. It share-locks that
  projection through the core debit and session handoff. The caller's compatibility
  `game_master` hint grants no authority; absent, revoked or mismatched staff proof is
  ordinary-player status. This is a read of existing Game staff authority, with no
  Platform identity or staff-configuration mutation.

### Frozen implementation validation

The implementation is frozen at `267bb10800f4f54fdcfa7f83e01a09f9ddb73543` for
independent review. Final checks on that source pass: `cargo check` for all 15
changed crates (103.344 seconds), all-target clippy with `-D warnings` for those
crates (92.158 seconds), formatting (14.922 seconds), architecture (3.739 seconds),
and refreshed-World pin-check (27.388 seconds; earlier pin-refresh 36.642 seconds).
The native entry/mastery/return PostgreSQL case passes again in 40.424 seconds,
and generic shared-owner enter/return/replay/fences passes in 5.067 seconds.

Client proof includes `soulpit_native_core_obelisk_scope_and_exit_walkthrough` and
`soulpit_same_generation_entry_and_return_reset_revisions_and_map_handles`,
within the 1,510 passing frontend tests (42.219 seconds). W08 completion and
post-Animus rollback use real shared entry rather than fabricated admission.
Primary fixtures use the existing test-admin root; the restarted negative GM
owner uses `oteryn_game_runtime`, with positive lookup privileges checked
separately under that same runtime role. The initial independent review's clock
finding and its completed repair review are recorded below.

### Review repair: serialized encounter clock

Independent review of the first frozen implementation found a queued-tick race:
entry or exit could publish a later encounter clock while a tick waited for the
transition lock. The owner now samples its monotonic clock after that lock and
uses it throughout the pass. The trusted caller timestamp remains a lower bound
for deterministic deadline scenarios; production elapsed time always advances.
This preserves Crystal's serialized encounter/global-event ordering (the donor
files and lines are cited in the shared-owner section above).

The deterministic tests `soulpit_tick_clock_waits_for_entry_transition` and
`soulpit_tick_clock_waits_for_exit_transition` both fail with `InvalidClock` under
the original sampling order and both pass after the fix (80.702 seconds including
compilation). They use a real async mutex, queued waiter and domain encounter,
without timing sleeps. Existing cadence and deadline validation are unchanged.

### W07/W08 acceptance

- **W07 DONE, server and client:** the shared production owner atomically consumes
  the selected core and admits the private Instance, drives physical native waves
  and returns members through the same owner. Native client core/obelisk/exit use,
  full snapshot adoption, fresh handles/revisions and acknowledgement fencing have
  passing tests. Current GM exemption and entry/exit tick contention are covered.
- **W08 DONE, server with retained native presentation:** the real shared-entry
  completion grants mastery atomically; replay cannot duplicate it and injected
  post-Animus failure rolls everything back. Native entry/query/return and client
  presentation paths have passing tests.

Independent repair review of
`c69d75069dbdae2b3f35587420b67ee38ec35c48` against the originally reviewed source
reports no relevant findings and confirms the P1 clock race is resolved. The
production wave/timeout PostgreSQL regression passes again in 39.75 seconds;
post-repair dispatch/server check (103.065 seconds), all-target clippy with
warnings denied (150.826 seconds), formatting (14.660 seconds), and architecture
(3.557 seconds) pass. The other changed-crate, content, generator and pin gates
recorded above remain applicable to their unchanged sources and artifacts.
No additional numbers were allocated after command 710, return reason 52, and
migrations 1140/1141. Final acceptance changes only these lane notes.

### Restored-workspace verification

At 2026-11-07 14:29:32 UTC, local and remote delivery head both matched
`4cf022101326ddb34957c7565893820cbcf2662d`, with a clean worktree.
Fresh checks passed: architecture (3.587 seconds), source_generator (0.864 seconds), diff (0.029 seconds).
W07 and W08 remain DONE; no mechanic source, allocations, or generated
artifacts changed. This checkpoint records verification of the completed delivery.
## PR 3036 generated-owner footprint repair

The clean 78294c081 main packet rebuild passes; the Soulpit successor needs a
fresh Item stat packet because its Terrain owner inputs changed. The 13,414
fields and 6,533 promoted Items remain unchanged. Regenerate with
`lower_wiki_stats_packet.py`, then
`tools/contracts/refresh_item_stats_promotion_pin.py`; neither source facts nor
qualification predicates are relaxed.

The existing 55 static records use canonical shard range 08614–08668. After a
palette qualification, `refresh_palette_capture.py --items-xml <source>` recounts
the committed regions and verifies the XML against the existing source capture
(Crystal 00ce02a5, items.xml SHA-256
13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece).
Also run `convert_appearance_only_ids.py --crystal-root <source-root>` and
`convert_islands.py`, then the complete world-metadata validation job. This adds
no migration, command, capability or mechanic allocation.
