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

## SRVENC W02: Instance corpse mint owner (2026-11-07)

Allocation: migration `1100_instance_creature_mint.sql` (assigned 1100–1109),
plus next-free authored protobuf field 6 on `CreatureDeathOccurrenceRefV1`.
No new command, capability, mechanic ordinal or registry entry.

Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`src/creatures/creature.cpp:711-721,777-782` creates a physical corpse even
for personal-reward bosses. The existing DUR-03 owner now preserves the exact
Channel or Instance cause, lane, assignment, reservation, receipt, Ground
custody and audit envelope. Historical Channel bindings/bytes remain unchanged.
Migration 1100 preserves namespace separation and source-race custody proofs.

Validation: combat 41/41 (3.63 s), durability 707/707 (74.74 s), check
(145.267 s), clippy `-D warnings` (109.135 s); isolated PostgreSQL 18.6
migration/namespace/immutability/race-guard proof (12.530 s). The native
production-boss and ordinary Instance dispatch cases are being validated
separately; no complete W02 production-room qualification is claimed here.

ENCX coordination: fetched and merged `81f2f0414` from
`origin/claude/happy-davinci-ijz3wy-encx`. No SRVENC change to
`endgame_entry.rs`; its shared composition remains ENCX-owned.

## SRVENC W03: production boss to physical personal chest (2026-11-07)

The unchanged Grand Mother Foulscale production profile now has an actual
native lethal -> settlement queue -> physical chest browse/take -> live Item
in the recipient's equipped backpack -> identical command replay test.
Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/monster/raids/grand_mother_foulscale.lua:5-25,40-46,78-97` supplies
classification, HP, corpse and complete loot. Reward chest source:
`src/utils/utils_definitions.hpp:646` (19250). The corridor remains explicitly
test-authored; it does not substitute for boss room source geometry.

Real PostgreSQL 18.6 validation: production boss chain
`boss_rewards_postgres_production_boss_lethal_to_personal_item_custody`
PASS 47.285 s; original rat/native replay chain PASS 52.636 s. No numbers
allocated. Separate Instance corpse cases are currently red: drain returns
Deferred; this row does not conceal or bypass that W02 integration failure.

## SRVENC W02: accepted Instance Ground cause (2026-11-07)

Allocated forward migration `1101_instance_creature_ground_proof.sql`.
Native Instance tests exposed the exact remaining SQL red point at COMMIT:
`game_item_ground_insertion_guard()` rejected the creature corpse because
its Instance whitelist only admitted the previously registered owners.
The new arm proves the full frozen reservation/receipt cause, exact current
transaction audit bytes, actual Instance/world/generation, Item definition
and exclusive custody; the existing mandatory mint consistency trigger and
all other insertion proof arms remain unchanged. Donor: Crystal
`src/creatures/creature.cpp:711-721,777-782` at the pinned revision above.

Real PostgreSQL 18.6 validation applied the migration to an isolated failed
native-case schema, committed its decoded frozen Instance corpse reservation,
audit and receipt with all deferred constraints, and observed one exact corpse.
Foreign Instance, missing audit and substituted audit each raised23514.
Combined executable proof duration115ms; no timing sleeps. Native dispatch
cases will be rerun after the owned Instance-map compilation step.

## SRVENC composition coordination and note ownership

Merged ENCX through `46669330a` (`88795094f` shared endgame prerequisite
composition and `46669330a` production-room regression). SRVENC does not
implement a second `endgame_entry.rs` composition. The next W02 step adds
owned, isolated map assets and a source-qualified ordinary boss template.
All future SRVENC notes live in this lane-specific document; the root
`CONTRACT-NOTES.md` is removed by the requested `git mv`.

## SRVENC W02: isolated bundle-backed activity assets

The existing qualified activity owner now retains owned room/movement handles
through handoff and recovery. A bounded World Bundle view copies only actual
authored tiles, resets scope-local doors/rotations/callback state, filters
floor transitions and gives each real Instance its own overlay. Registration
rejects an origin Channel overlay for an Instance. Immutable compiled content
is shared through Arc; no leaked assets or alternate authority owner.
Crystal `data/libs/functions/boss_lever.lua:63-76,190-225` supplies literal
room/entry positions. Boot qualifies rooms after the bundle combat view and
retains both the issued parent activation frame and physical bundle frame.

Validation: dispatch all-features/all-targets check PASS3m46s; real World
Bundle isolation test `boss_activity_bundle_instances_bound_tiles_and_isolate_door_state`
PASS0.03s (build2m01s), covering2Instances, independent doors, bounded cells,
parent preservation, stale scope and invalid Instance rejection. Independent
source review found no concrete authority/aliasing defect. Native PostgreSQL
regressions remain to rerun after1101; no full-row completion claimed.

## SRVENC W17: native player modifier state

Source: `docs/reference/tibia-manual/combat.md:271-272,274,276-278`.
The actual PlayerSpellState owner now implements nonlethal current-HP
percentage hits, fear2s/root3s/drunk10s, exact keyed incoming damage bonuses
15/25/10percent, expiry/suppression and exit cleanup. The real ConditionStore
and existing movement/fear cast gates remain authoritative; direct percentage
hits preserve mana/shields and unrelated state. No persisted fields or IDs.

Validation: full `oteryn-gameplay-spell` package324/324 passed18.47s,
including5newDrome owner cases (actual shield ordering, exact expiry, cast
gates, overflow/refusal and cleanup). Before implementation the public
DromeStatus API was absent (compile-red). The real encounter engine/cadence
uses these successors in the next step; its behavioral RED showed200HP
where the source25percent corpse burst requires150HP. Full encounter
PostgreSQL GREEN remains pending; this is not a full W17 completion claim.

## SRVENC W17: production modifier checkpoint

The native encounter owner applies corpse bursts once per participant, timed
Lava/Beam/Tanked warnings, seed defusal, Somersault melee hooks and Escalated
working levels. Source: `docs/reference/tibia-manual/combat.md:271-278`.
Terminal grace return retains exact actor/session authority without granting
control to the dead player. No new IDs or migrations.

Validation: actual PostgreSQL exploding-corpse/current-HP case passed42.233s
and actual grace-expiry cleanup passed41.838s; dispatch compiled in the same
473.453s focused run. Remaining red points are the real source map placement,
Instance reward binding/loot lock and item-selection test accounting. Actual
Drome movement steering and late-observed native kill timing remain next steps.
Creature base HP/damage/scaling remain absent from available donor evidence.

## W02 ENCX catalogue integration

Merged ENCX1a70f306d, reusing its qualified-room publisher and migration1150.
Preserved SRVENC parent/physical frame qualification and post-bundle boot
ordering. Both lane-specific note files survive; no root note remains.
The unpublished static Sugar catalogue draft was discarded in favor of this
shared production owner;1102 remains unallocated. No second composition.

## W02 Sugar Daddy source closure

The first closed ordinary rewarded boss uses the existing canonical Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446` Creature, not a fixture
classification: `data-otservbr-global/monster/bosses/sugar_daddy.lua:21-25,76-132`.
Its complete pinned file has four native attacks, two native defenses, no summons
and no scripted event hooks. Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
`data-global/scripts/quests/candia_bosses/sugar_daddy_lever.lua:1-23` supplies
the unchanged room layout, two-hour cooldown and five source stand positions.
The only implicit hook is the implemented `BossLeverOnDeath`, registered in
`data/libs/functions/boss_lever.lua:241`; its complete source is
`data/scripts/creaturescripts/monster/boss_lever_death.lua:1-39`.

The exact existing canonical Item `oteryn:item.tibia.i48416` has no XML override.
Both admitted donor DATs contain its container/corpse flags; Crystal
`src/items/items.cpp:163-174` indexes ItemType by exactly appearance.id.
Canary `src/items/items.hpp:282,307,369` supplies weight0, capacity8 and
non-pausing defaults. New native admission is bounded to that exact binding
and Ground-only, non-pickupable, non-stackable capacity8 corpse policy. It
does not invent an XML row, new canonical identity or a pickup permission.
The existing loot table's 22 entries (21 distinct Item references) keep all
identities, quantities and chances. The native source importer supplies their
custody facts; existing known values and equipment pattern identities retain
their accepted precedence while UNKNOWN movability/classification facts close.

Canary `data/items/items.xml` source Item ID:line witnesses are: `675:1405`, `3029:6788`, `3031:6796`, `3035:6813`, `3039:6829`, `32769:62147`, `45639:77301`, `45640:77330`, `45641:77361`, `45642:77375`, `45643:77388`, `45644:77422`, `48114:80465`, `48249:80485`, `48250:80490`, `48251:80495`, `48252:80500`, `48253:80505`, `48254:80510`, `48255:80515`, `48256:80635`.
The source capture pins complete XML, DAT, loader, monster, lever and callback
blobs; only normalized server facts are committed. Generated profiles, loot
facts, candidate fingerprints and manifests use their repository generators.

No new command, capability, mechanic ordinal or migration is allocated by this
source-data step. The exact source binding48416 reuses its existing canonical
identity. The source qualification census changes precisely from
52 missing /329 inadmissible /1 independently admitted corpse to
52 missing /328 inadmissible /2 (Sugar Daddy and The Primal Menace); the five
shared-body bosses retain their previous separate expectations.

W02 real Sugar Daddy source room physical closure:
`tools/content-schema/world-object-authoring/sugar_room.py` is the owning bounded
supplemental generator (integrated into build_catalogue.py); its capture records all
79 exact source appearance flags including the two `usable` witnesses without
admitting Item interactions. Crystal00ce02a5 data/items/appearances.dat SHA
17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1 and
items.xml SHA13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece
prove all79 have no XML overrides; Crystal src/items/items.cpp:163-174 binds exact
appearance.id to ItemType. Complete admitted official15.30 appearances2dfa943b…
independently supplies identical flags for all79 and exact admitted membership.
Physical classes:10ground(2unwalkable/speed0),38border,26blocking wall,5common.
Only stable palette keys/provisional bits change; B3bytes, palette indices, item
stacking and source geometry remain unchanged. New canonical Terrain keys retain
source ids, shard08614-08692 generated after existing8614records. No Item binding,
command, capability or mechanic ordinal allocated. New sourceprofile:
OTERYN_OFFICIAL_CLIENT_APPEARANCE_ONLY_BOSS_ROOM/v1, exact79-id schema arm.
Tests `SugarRoomTests.test_exact_79_source_flags_and_native_geometry` and
`test_real_b3_room_has_no_provisional_omissions_and_preserves_indices` PASS2.148s:
real sourcezone+6anchors covers1119placements, all79canonical and no omission;
foreign unknown donor palette rows remain refused. `sugar_room.py --check` passes.

## W02 compiled custody and publication checkpoint

Migration1103 replaces the Channel-only corpse child lock with exact typed
Instance ownership; carrier binding retains scope/generation/projection guards.
Migration1104 makes the ENCX publication immutable and scoped by World, current
content revision and physical origin frame/map/server artifact. Current fenced
readers select that publication; Instance returns resolve the frozen origin.
Independent source review found no concrete P0/P1; real-schema1101 substitution
proof reran PASS0.150s. No command/capability/mechanic numbers allocated.

WIP: production libraries compile in the focused dispatch build; its test target
is currently red at an added Terrain optional-field fixture initializer. Actual
PostgreSQL instance custody and scoped publication regressions remain pending.
The source geometry/projectile closure and movement behavior checks continue
in the next checkpoint. No full-row completion is claimed here.

W02 projectile projection correction: Crystal src/items/items.cpp:234 assigns
`iType.blockProjectile = object.flags().unsight()` independently of movement
`unpass`. The Sugar generator therefore emits KNOWN `blocks_projectile` for all79
Terrain records. Ten solid walls(45015,45031,45176,45177,45178,45179,45180,45181,
45182,45250) are projectile-transparent; blocked grounds45196/48121 block shots.
Bundle Terrain now carries optional `blocks_projectile`; absent/UNKNOWN keeps the
legacy interpretation, serialized None is omitted so previous bytes remain stable.
The compiler refuses malformed KNOWN nonboolean values. BundleFacts and SpellTiles
prefer the known flag before legacy fallback; visible RoofFacts consumes the same
resolved flag instead of independently treating every wall as opaque. Physical
movement solidity is unchanged. Mechanical existing Terrain literals use None.
Focused regressions added: `source_projectile_known_false_true_and_unknown_are_distinct`
and `terrain_projectile_overrides_preserve_solidity_and_visible_lookthrough`.
Rust execution pending root Cargo slot; Python Sugar exact79 corpus passes2.231s.

W02 owning format allocation: World Bundle format8 (next free after7, lane offset0).
Per OTERYN_WORLD_BUNDLE_FORMAT_V1.md:56-57,319-321 the new optional known Terrain
projectile field requires a new version even when old UNKNOWN manifests omit it.
Writer header, manifest format, minimum reader and digest domain now coherently
use8; reader accepts only8, consistent with prior format transitions. Existing
v6 viewport semantic-golden helper verifies a layerless/no-override fixture before
reconstructing its old identity; no recorded golden bytes/digests were edited.
Root owning content regeneration and compiler pin-refresh will generate actual
v8 bundle bytes/pins. No generated bundle/pin fields were hand-edited.

Ordinary Instance PG assertion correction after actual90.171s RED: fresh MINT
sets game_item_instances.minted_transaction_id (0010); last_transaction_id is
nullable until later mutation (0011:21-23). Retained real native-rat DB contains
livei3607/i3031, revision1, exact physical parent/placement/Instance and matching
minted transaction, with last_transaction_id NULL. Test now asserts those exact
semantics and additionally matches receipt destination parent+ordinal, exact
visible physical body id and typed World/Instance. Every joined row is explicitly
coalesced false inside bool_and, preventing a missing custody join from being
ignored beside a valid child. Read-only corrected predicate returns2,true; actual
focused production rerun remains required for GREEN.

## W03 Instance personal reward qualification

Real PostgreSQL tests now pass for the unchanged production Grand Mother
Foulscale profile inside an Instance (83.742s), native Instance reward custody
and identical replay (92.657s), physical native chest chain (44.391s), and the
production boss physical chest chain (42.447s). Source remains Crystal
201189f grand_mother_foulscale.lua:5-25,40-46,78-97 and utils_definitions.hpp:646.
Item selection reports remaining quantity for its selected ordinal. Assertions
retain the exact original Collected/1/0 result and additionally prove every
unselected entitlement is byte-identical after collection and replay, with
exactly one collected ordinal. No source chance/count or Item custody rule
changed. No numbers allocated. Source Sugar entry qualification continues
separately; its current red is exact source profile digest mismatch.

## SRVENC W17: Drome movement conditions (2026-11-07)

The actual movement owner consumes canonical Fear/Root conditions only for an
actor whose runtime Drome membership and `PlayerSpellState::drome_practice` both
remain current. `docs/reference/tibia-manual/combat.md:277-278` supplies the 2/3
second durations; Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
`creature.cpp:286-291` supplies the walking refusal. Expiry and suppression remain
with the existing condition owner.

Only the live `drome.drunk` condition supplies Tanked Up steering. Crystal
`creature.cpp:224-260,275-284` maps its normal roll using `(subId + 25) / 5`.
The represented standard Drunk condition has no intensity field, so this consumer
uses default subId 0 and the existing source-normal roll helper. The stronger
"super drunk" intensity in manual `combat.md:276` remains missing source data;
no stronger intensity is inferred. Other Drunk sources and non-Drome movement
retain their existing behavior.

The actor/session, position revision and frozen owner time identify the semantic
roll. Preflight and the common physical movement commit use that identical
occurrence and time, preserving field/floor destination proofs through SQL.
Command replay remains with the existing admitted command owner, before movement
or RNG is re-entered. This consumer adds no actor fields, persistence, protocol or command IDs.

Validation:18 movement/field regressions passed0.17s (132.606s build+test).

## W17 native Drome Highscores filters checkpoint

Allocated Highscores category13 (Character lifetime Drome levels) and14
(current World/rotation Drome levels), reusing command14/capability0009.
Migration1105 is reserved for the server projection; its focused PostgreSQL
qualification remains pending. Source: manual combat.md:262-264.
Native filters, existing response decoding and row presentation pass all4
client-ui Highscores integration tests (1.742s); the new case first failed
with "Drome category filter unavailable" (3.322s). No new command/capability.
This is a compiled UI checkpoint; server projection and full row remain WIP.

## W02 compiled source and physical projection checkpoint

The original Sugar lever geometry, unchanged canonical Creature profile and
complete22-entry personal loot are admitted through the shared ENCX publisher
and isolated owned Instance factory. Native corpse projection now requires its
committed scoped MINT/audit/reservation proof, uses native +z and pinned map ABI,
and invalidates cached Ground on committed/replayed corpse outcomes. Historical
corpse generation remains provenance, while current reads remain fenced.

Validation: dispatch allfeatures/alltargets check PASS181.391s;9Content/World
identity/isolation/projectile cases PASS5.967s (144.292s build+test);6compiler
terrain cases PASS0.02s (5.654s total). Full owning content regeneration
PASS1951.607s including5repository tests71.85s. Format8 pin-refresh PASS134.879s,
pin-check PASS169.669s. Generated artifacts came only from repository tools.
Allocated Terrain records8614-8692, WorldBundle format8; no commands/capabilities.
WIP full production Sugar/publication/native projection PostgreSQL rerun is next;
this checkpoint compiles and does not claim full-row completion yet.

## W02 production source chain PostgreSQL GREEN

`srvenc_source_sugar_daddy_template_creates_instance_and_settles_personal_rewards_postgres`
PASS151.794s: actual source B3 stands/entry, wire178, independently ready owned
Instance factory, unchanged native Sugar profile, committed source corpse48416,
primed Ground cache refresh/real projection, personal entitlements/replay and
literal60second return/wire179 with reward retention. The shared publisher
`srvenc_boss_publication_postgres_scopes_replay_conflict_and_current_reader`
PASS177.012s proves multiple revisions/Worlds, equal replay, current fence/pin,
immutable publication/metadata and conflict refusal. Corpse-generation projection
unit PASS0.016s independently rejects foreign frame/scope and stale fields.
Source comments/captures retain donor file:line evidence; no fixture profile,
source probability, geometry, ownership or current-fence guard was relaxed.

## W17 native arena cadence checkpoint

Real qualified B3 hall/irregular connected arena wave PASS110.382s; timely
native defeat observed after deadline PASS47.429s. Raffle source combat.md:284
now awards exactly20% of remaining participants: actual320participant native
case PASS47.233s with20ranked+60raffle winners, after RED70total/50raffle.
The engine's actual native corpse/effects reservation edge observed RED49.265s
and its one-time clear credit is implemented; focused GREEN is running. Auction
cutoff/three-wave catch-up baselines are running with normal guard-preserving
fixtures. No numeric command/capability/migration allocations in this engine step.
The production library compiled in the Game-server PG target; pending edge-case
qualification and genuine Drome-exclusive profile/scaling data keep W17 WIP.

## SRVENC final-check progress

Whole-workspace cargo fmt --check PASS14.332s; architecture-check PASS3.775s;
registry-fragments --check PASS. Strict Clippy identified only new test-shape
issues in source helpers (identity conversions and large tuple annotation);
those are corrected without suppressions. Final Clippy rerun follows the
remaining genuine-custody auction fixture qualification. Source production
Sugar/publication PostgreSQL GREEN remains151.794s/177.012s unchanged.

## SRVENC core package validation GREEN

Complete selected library suites for Content, World, WorldBundle, Entities,
Combat and Spell:1163 tests PASS629.921s (717.523s including build), no failures;
existing runner exclusions were unchanged. No tests were disabled or weakened.
This covers the admitted source corpus, Item/Creature identity and provenance,
Instance isolation, physical projection, native combat and player effects.
The source boss/physical reward PostgreSQL evidence recorded above remains
GREEN; remaining final checks concern Drome custody/Highscores integration.

## W17 native bulk-clear accounting GREEN

The production native command catch-up now applies exactly the unsaved clear
count once, matching the durable settlement formula. Real three-wave test
`tibiadrome_postgres_three_native_waves_command_catchup_counts_each_once`
PASS50.789s after twelve native kills; old code produced(level,start)=(3,5),
corrected code returns(3,3), waves3. Source manual combat.md:264; no new IDs.
The genuine native Bazaar interaction currently reaches an excluded E11
adapter boundary: its Reference-only loader rejects native OTNSW03 content.
Root requested owner scope direction; no byte limit or artifact pin was bypassed.

## W17 bounded Drome Highscores server checkpoint

Categories13/14 use the existing revision-stable Highscores owner and native
filters. Drome-specific scored metadata includes vocation-none scorers (manual
combat.md:259,262-264; classic vocation filters remain unchanged). Current
auction withdrawal is scoped to13/14, preserving captured ranks/paging and
ordinary categories. Migration1105 extends category checks and the strict
role guard by only the existing score-table SELECT permission.
Actual PostgreSQL: lifetime/current/private-own-row PASS6.209s;51-row retained
snapshot withdrawal/paging/own-rank proof PASS5.514s. Real Reference-profile
public observer proof uses actual Game capture/sign/custody and producer-created
Character; source issuer floor+1 bootstrap is fixed, final rerun pending.
No Platform E11 logic or immutable public projection body policy was changed.

## W17 authentic production data dependency

The pinned complete Crystal/Canary Git trees have no Creature profile paths for
Murmillion, Scissorion, Hoodinion, Mearidion or Domestikion. The reference manual
combat.md:268 defines their roles; :297 explicitly states that HP/damage and the
wave difficulty curve are absent. Source Item/plushie/corpse names do not supply
Creature combat profiles. Existing dispatch/tibiadrome.rs still selects admitted
Svargrond practice profiles; entities/carrier/tibiadrome.rs labels the working
scaling parameters. These are not presented as authentic Drome production data.
Manual :276 likewise does not quantify stronger super-drunk intensity; only the
source-normal steering parameter is represented. No substitute base stats,
attack profile or scaling curve was invented for this lane.

## W17 public-profile custody qualification checkpoint

The genuine public-profile proof compiled successfully (245s build) and ran
32.572s, but its real Game Bazaar lock returned `Unavailable`; it is not
counted GREEN. Investigation preserves capture/signature/custody guards and
the excluded Platform E11 native-artifact boundary. Current-standing coverage
is being qualified through the supported fenced Reference consumer; already
GREEN native encounter, movement, physical rewards and Highscores paging proofs
remain unchanged. Migration1106 is pending qualification and is not yet claimed
as a completed rotation-cut production rule.

## W17 genuine current-auction regression compiled checkpoint

The supported Reference authority fixture now performs actual Game Bazaar
Lock, signed capture/custody and full recovery reopen. The named auction
regression moved to that fixture; native physical/status/UI positives remain.
Build PASS172s; actual PostgreSQL RED33.255s: fenced native-provider Status
returned (rank,points,potions)=(2,250,3), expected(1,250,3), proving an auctioned
rival incorrectly remained in current standings. Command: cargo nextest run
--locked -p oteryn-game-server --test character_authority_postgres
--test-threads 1 -E 'test(srvenc_tibiadrome_postgres_auction_excludes_standing_and_expired_rewards)'.
Account-security observations now continue the registered Account source floor
through its actual API; no custody or recovery guard was bypassed. Obsolete
unsupported actor-auction fixture scaffolding is removed. Migration1106 is the
shared historical lifecycle-at-cut reader (manual combat.md:263); it and the
compiled fixture are checkpointed before production caller correction. Actual
cut tests cover event history, not a simulated fortnight crossing.
Final fmt PASS14.912s, architecture PASS3.540s, registry-fragments check PASS.

## SRVENC final main composition and allocation correction

Merged main1ae39c7fb (QENGINE2/QBASE0999-1008) in73128bf59. No1200-1205
copy exists. The merged generator preserves the eight incoming quest B3 regions,
all79 Sugar Terrain/projectile facts, and both native Item cohorts. QENGINE2's
53 and Sugar's13 new keys are disjoint; the independent full-profile speed
assertion remains exact at3676 and checks every original raw attribute. The
53-item QENGINE2 proof and unknown-speed set remain unchanged.

The earlier13/14 allocation is SUPERSEDED: HIGHSCORES0 section3 reserves13
Loyalty,14 Drome Score,15 Taint and16 Phosphorus. Lifetime Drome uses existing14;
current rotation allocates next-free17 (offset0). Command14/capability0009 are
unchanged. Native Windows combo indices map through explicit category IDs; both
directions and reserved gaps are tested. Game public encoding admits only
1-12,14,17. Platform consumer rollout of the extra subjects remains explicit in
PUBLIC_PROJECTIONS_V1; no Platform pages or E11 native-artifact adapter changed.

Migration1107 retires incorrect13/14 snapshot tokens via normal guarded deletes,
preserves public revision floors, and excludes retired13 metadata from resync.
The actual PostgreSQL regression restores the prior schema/source, applies the
real migration, and checks guards, owner, ACL, search_path, definer mode and
wrapper preservation; it also verifies source scores remain unchanged.
No immutable header/row is rewritten and no guard is disabled.

The imported1150 candidate is absent from origin/main1ae39c7fb and remains
unreleased ENCX branch work. Its unused catalogue UPDATE grant is corrected
in place to INSERT only, matching the immutable scoped publisher. This is not
a new migration allocation; no1151 or range exception is used. Earlier notes
about preserving the candidate checksum are superseded by this pre-integration
correction. The strict test retains UPDATE/DELETE/TRUNCATE denials and additionally
checks column UPDATE, REFERENCES and MAINTAIN; its real RED was13.392s.
The final grant/publication GREEN follows below after execution.

## SRVENC validation after merged content

The owner replaced the full reverse-dependency test rule with own mechanic
checks plus workspace compilation and CI full suites. The old full run was
stopped as directed (4589 passed; ten substantive failures grouped and fixed;
three transient in-progress-manifest failures rerun GREEN; four interrupted).
No test code was weakened, ignored or deleted.

Workspace cargo check --workspace --all-targets PASS200.589s after five
SharedOwner borrow adaptations in incoming QENGINE2 callers. Strict Clippy
15changed crates, all-targets/all-features, -Dwarnings PASS423.288s.
Merged repo-tool pin-refresh PASS138.093s; pin-check PASS161.208s (format8,
32879502-byte bundle); duplicate migrations PASS0.370s. Reused main's owning
client pin-refresh implementation rather than introducing another generator.
Architecture PASS3.691s; fmt --check PASS15.063s.

Focused mechanic batch:39 positive checks GREEN644.834s, with only the now
corrected1150 candidate privilege mismatch RED. Actual PostgreSQL18.6 cases:
Sugar full source entrance/Instance/corpse/personal reward/return PASS150.784s;
scoped catalogue replay/conflict/current-reader PASS173.497s; production
Instance corpse/personal custody PASS123.361s; physical boss-to-personal Item
custody PASS91.977s. Drome genuine auction/public-profile proofs PASS43.367s
and43.467s; migration1107 PASS9.868s; three waves PASS51.895s; reserved victory
PASS76.197s; full raffle share PASS70.658s; irregular source hall PASS108.116s;
current-HP corpse effect PASS75.500s; grace cleanup PASS80.173s.
Focused library/protocol/UI53tests PASS82.966s (272.246s including build).
Native client play/map8tests PASS9.677s (32.433s including build), including
actual Highscores presentation and all four previous client pin regressions.
The zero-test binary selector attempt is not counted as validation.

W17 still cannot be called an authentic full production encounter: the pinned
Crystal/Canary trees contain no native profiles for Murmillion, Scissorion,
Hoodinion, Mearidion or Domestikion; combat.md:297 expressly leaves their
HP/damage and wave difficulty curve unspecified. The existing practice pool
and working scaling are not relabelled as source-qualified production data.
All implemented native mechanics and presentation above are positively tested;
no authentic Creature statistics or stronger drunk intensity was invented.

## SRVENC final gate and row result

The candidate grant correction is GREEN: strict staged boss privilege/dispatch
case PASS6.607s; the same real scoped publication/current-reader/replay/conflict
case PASS176.208s with UPDATE unavailable (2/2,176.211s runtime).
Final cargo check --workspace --all-targets PASS204.573s; strict15-crate
Clippy all-targets/all-features -Dwarnings PASS208.360s; fmt --check PASS15.011s;
duplicate-migration check PASS0.359s; registry fragments check PASS.
The earlier39 positive mechanic cases,53 focused library/protocol/UI cases and
8 native client cases remain GREEN; no unrelated full suite was restarted.

W02 DONE: source Sugar entrance/owned Instance, exact corpse qualification,
physical corpse projection, scoped catalogue and replay/current fence proofs.
W03 DONE: unchanged production reward-boss lethal settlement through personal
physical chest to live Item custody, quantity and replay invariants.
W17 BLOCKED on authentic production data, not on the implemented mechanics:
`dispatch/src/gameplay_transport/tibiadrome.rs:700` still selects admitted
Svargrond practice opponents. The five exclusive Creature profiles and their
HP/damage/wave curve are absent from pinned donors; `entities/src/carrier/
tibiadrome.rs:53` labels its working scale as an Oteryn choice, and manual
combat.md:297 explicitly records the numeric source gap. Full source-native
Drome cannot be claimed until those profiles/scaling are supplied and admitted.

Final allocations:1100,1101,1103,1104,1105,1106,1107;1102 and1108-1109 unused.
Existing unmerged1150 corrected, no1151 allocated. Existing Highscores14 reused,
new17 allocated,13/15/16 reserved. Existing command14/capability0009 reused.
Terrain8614-8692 and World Bundle format8 retained; all pins were generated.

## Combined SRVENC and ENCX delivery (owner direction, 2026-11-07)

ENCX advanced from b5103e7f to 7328791ae after integrating main1ae39c7fb;
this branch merges that head for the owner's single combined delivery.
Exactly one1150_qualified_activity_catalogue.sql retains ENCX's unique
room-template/content index and SRVENC's INSERT-only append publisher privilege
and1104 immutable catalogue closure. Keep ENCX1151 tagged endgame receipts and
1152 native Instance corpse scope extension; no duplicate migration or new
command/capability/mechanic number is allocated by this merge.

1152 preserves SRVENC1100's existing17-byte typed cause primary keys and all
1100/1101/1103 proof bodies, adding equivalent33-byte assignment projections
without UPDATEing immutable accepted rows. Crystal201189f
`data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:79,94`
requires physical corpse39530 plus personal boss rewards. New unified Instance
mint writes use ENCX semanticv2/native53-byte framing; exact accepted SRVENC
semanticv1/61-byte reservations remain resumable with their original complete
hash, frozen audit envelope, identities, budget and physical corpse reference.
Channel52-byte framing is unchanged. Compatibility never treats a hash prefix
or an arbitrary opaque corpse reference as proof of the exact typed death.

Merged-schema RED proof: the original1152 fails on an actual1100 prefix with
`column "death_instance_id" ... already exists`; the adapted1152 populated
prefix preservation proof passes in0.148s. The first full39-test mint run
passed37, with both new preservation cases exposing an incorrect test
assumption about terminal replay work: resource-limit0319-DUR03-RL-08 requires
each commit attempt to charge one unit, including AlreadyCommitted. The tests
must require the exact+1 per replay, preserve every other field, and reject a
fourth attempt with the whole row unchanged; the production cap remains3.

Combined validation checkpoints: focused UI/protocol/content/mechanic56/56
PASS78.083s runtime (277.681s including build); all8 native source-world
projection tests PASS0.937s; boss/source Python8/8 PASS1.849s. Official compiler
pin-refresh PASS145.933s; isolated final pin-check PASS166.356s, bundleformat8
32,879,502bytes and digest196a7f87877c7f1725a04ad008b5768dad4b7848fe99b17d2b92ef49bf8d856d.
Architecture-check PASS4.095s. No full reverse-dependency nextest suite was run.

Final corrected focused mint/audit39/39 PASS30.507s runtime. Real PostgreSQL:
`native_instance_corpse_migration_preserves_committed_causes_and_replay_postgres`
13.315s; `native_instance_corpse_migration_preserves_legacy_v1_corpse_and_loot_replay_postgres`
15.495s; `native_instance_corpse_mint_postgres_physical_death_loot_replay_and_scope_fences`
15.584s; `boss_activity_item_consumers_postgres`13.724s. Canonical and legacy
pending reservations resume before first commit with whole-row equality;
post-migration terminal replay preserves frozen bytes and charges exactly one
work unit; fourth attempts reject with whole rows unchanged. The43-file
spell-provider freshness check passes83.568s.

Production PostgreSQL checkpoints pass:
`boss_activity_postgres_production_reward_boss_instance_corpse_and_personal_custody`
124.294s and `boss_rewards_postgres_production_boss_lethal_to_personal_item_custody`
86.529s. ENCX's captured-death component after a real native Instance handoff,
`boss_activity_postgres_tagged_instance_primal_receipt_replay_and_scope_fences`,
passes340.688s; `endgame_entry_postgres_real_dispatch_cannot_allocate_or_reset_without_w02`
passes13.556s. These are precise component/production boundaries, not a claim
that Primal's complete production combat chain or W17's unsourced archetypes
have become donor-qualified through this merge.

Owner's superseding push rule (2026-11-07): publish once workspace/all-target
check and fmt pass, without holding the compiling merge for remaining tests.
Final `cargo check --workspace --all-targets --keep-going` PASS763.057s
(includes build-lock wait). Focused mint39/39 and unit56/56 already pass;
combined production46-case run and changed-crate clippy continue independently.
Final architecture PASS10.600s, duplicate migrations PASS1.801s, registry
fragments PASS0.176s. No pending test or lint is represented as passed here.

## Completed validation of combined head3621b5ea5

The previously pending runs completed: changed-crate clippy with warnings
denied passed350.854s; final fmt passed14.719s; client8/8 passed27.619s runtime.
The combined46-case production/mechanic run completed923.753s runtime:
45passed, one inherited ENCX failure. Actual source-qualified Sugar Daddy
Instance/personal-reward chain passed170.713s; scoped catalogue publication,
replay/conflict/current-reader proof passed212.408s. Soul War terminal replay
and taint/XP cases passed136.996s and153.267s respectively.

The remaining failure is
`primal_ordeal_postgres_qualified_production_room_is_reachable` (153.445s):
`SourceHookUnavailable`, then missing production Primal catalogue row.
ENCX7328791ae has the identical candidate and callback guard; existing ENCX
notes already record this failure. The guard accepts only BossLeverOnDeath;
Primal requires native owners for ThePrimalMenaceDeath,
ThePrimeOrdealBossDeath and mType.onThink. A name-only whitelist would falsely
qualify the encounter. W10 implementation remains ENCX-owned; no test or guard
was weakened. Main integration and fresh generation/unit/PG checks remain
pending under the owner's emergency publication exception.

Emergency publication gate: fresh `cargo check --workspace --all-targets`
passed (3m22s), and `cargo fmt --all --check` passed. Only this validation
record changes; no mechanic, generated pin or test expectation changes.

## Main integration47db34f09 (2026-11-07)

Merged main's NPC quest programs, world-change calendar and cast capabilities
without changing lane ownership. The three conflicts were generated Item
providers/spell manifest; merged producer code preserves QENGINE, QNPCFX,
SRVENC Sugar13 profiles and ENCX source regions. No new lane number allocated;
main's existing1310-1315/1320-1322 migrations are inherited unchanged.

Confirmed all11 World package documents exactly match main and this lane has
no World materializer/example input changes. Used the documented native-only
regeneration path, then the canonical tree generator, rather than rerunning
the unchanged World package. The initially attempted full materialization was
stopped before copying any output into the repository. Quest binding evidence,
NPC dialogue links, audit tag goldens, registry fragments and changed client
registration goldens were regenerated with their tools. World metadata and
duplicate migration checks pass. Final pins, workspace check, clippy, affected
unit tests and own PG revalidation are tracked separately; none is claimed
passed before its output is available. The inherited Primal hook-closure
failure remains an ENCX-owned gap; no guard or test is weakened.

Main integration regeneration proof: native43-file output PASS86.391s;
canonical tree PASS214.864s; authored family tools PASS17.235s;
pin-refresh PASS31.392s; pin-check PASS29.487s (format8,32,879,502bytes,
digest196a7f87877c7f1725a04ad008b5768dad4b7848fe99b17d2b92ef49bf8d856d).
Item/Mount and complete materialized-tree equivalence PASS123.894s;
world metadata PASS1.504s. The regenerated delta is four content indexes,
manifest/lock and Grizzly room provenance; no pin is hand-edited.

Current-main PostgreSQL execution: all3 native Instance corpse cases PASS in
14.100s runtime: committed-cause migration/replay7.355s, legacyv1 corpse/loot
migration/replay7.279s, physical death/custody/scope fences6.820s. Both Channel
and Instance controls and the three-attempt budget remain strict.

Affected-crate unit revalidation pending under PREPUSH-2's cadence exception:
oteryn-architecture-check, oteryn-client, oteryn-client-runtime, oteryn-client-ui,
oteryn-game-server, oteryn-gameplay-content, oteryn-gameplay-dispatch,
oteryn-gameplay-durability, oteryn-gameplay-npc, oteryn-gameplay-quests,
oteryn-gameplay-social, oteryn-gameplay-world-change, oteryn-protocol-oteryn,
oteryn-session, oteryn-world-bundle-compiler. These run library/binary unit
targets without the full PG suite; the remaining source Sugar/publication,
production boss/reward and Tibiadrome own PG cases remain separately pending.
No pending suite is represented as green.

The full workspace/all-target check collected one merge-compatibility failure:
E0308 in postgres_tests/s17_client_watch.rs, where main's new fixture assigned
&NativeEntryMovementCells to SRVENC's SharedOwner. Wrapped that existing borrow
in SharedOwner::Borrowed, matching the production owner contract; assertions
and behavior are unchanged. A sibling scan found no other raw-reference
assignments, and the keep-going run reported no other compilation failures.
The full workspace check is rerun after this correction; the red log is kept
separately from the green candidate's output.

After the one caller adaptation, fresh full workspace/all-target check passes
128.792s and fmt passes14.385s. The three current-main PG cases remain green
with actual execution above. Remaining own PG cases and unit targets for the
15 affected crates are queued; publication does not hold the compiling merge
for those suites under PREPUSH-2's cadence exception. No whole-workspace or
full PostgreSQL test suite is launched.
At the cadence publication, changed-crate clippy with warnings denied is also
still running; it is not represented as passed. The checked/pinned code is
published without waiting for the remaining validation processes.

## Final combined PR candidate (2026-11-08)

Owner requests ONE publication followed by a FROZEN branch. Merged final ENCX
9da977650 with merge commit dbde2f037; the subsequent main merge reports already
up to date, with main47db34f09 included through the existing merge commits.
No rebase, force push, PR creation or labels are performed by this lane.

Preserved the scoped/current-parent combat-room publisher while relocating
ENCX's unavailable_source_hooks diagnostic to that real production caller.
Quest inputs retain the incoming opaque World/NPC authority sum type: World
keeps its Channel spell-lane check; NPC requires its current admission permit
and rejects Ground before mutation. No bare row-mutation helper survives.
Independent read-only review of both resolutions found no concrete P0/P1.

Exactly one1150 remains INSERT-only with its template/content uniqueness and
1104 closure.1151 and dual-schema1152 are retained; own migrations remain
1100,1101,1103-1107. Duplicate check PASS0.392s, no collision or renumbering.
All11 World package documents continue matching main; native-provider and
central-list tools regenerate the merged data without hand-editing pins.
Final native generation PASS86.999s; NPC binding evidence PASS17.934s;
pin-refresh PASS25.770s; pin-check PASS28.452s; world metadata PASS1.500s.
Final workspace/all-target check PASS252.310s; fmt PASS17.858s;
changed-crate all-target/all-feature clippy with -D warnings PASS260.640s.

Final unit scope is the PR diff against main, not the previously merged-main
scope: oteryn-client, oteryn-client-assets, oteryn-client-ui,
oteryn-game-server, oteryn-gameplay-combat, oteryn-gameplay-content,
oteryn-gameplay-dispatch, oteryn-gameplay-durability, oteryn-gameplay-entities,
oteryn-gameplay-native-admission-source, oteryn-gameplay-spell,
oteryn-gameplay-world, oteryn-protocol-oteryn, oteryn-world-bundle,
oteryn-world-bundle-compiler. Unit targets run separately from the focused
real-PG cases; no full-workspace or full-PG suite is launched. Tests from a
superseded pre-merge candidate were cancelled and are not claimed as evidence.

Final candidate native PostgreSQL proof:3/3 PASS11.318s runtime, with real
committed-cause preservation5.807s, legacyv1 corpse/loot preservation5.845s,
and physical Instance corpse/loot/scope fences5.503s. No route-less execution
is counted as PG proof. Wider own-PG cases (including NPC input authority)
and the15 PR-changed crate unit targets are queued/running separately.
The PR body explicitly retains the known production gaps: Primal custom
callback composition and unsourced Tibiadrome archetypes are not invented or
claimed complete. The inherited required-Primal PG assertion is retained.

## PR3034 authorized collector-main merge

Owner explicitly authorizes main78294c081 despite the PR freeze. This pass adds
no feature work. Pre-merge lane diff:327 files,+14147/-1321; the post-merge
comparison and representative files must remain present before publication.
Main's reset epoch is kept alongside SRVENC's borrowed movement-cell owner.
The newer main InputPermit brand is retained, including its Channel check;
NPC Ground refusal is kept before mutation. No unbranded row helper returns.

Main's new FloorTable teleport_sources also has to survive SRVENC's bounded
activity-region view. The crop now retains provenance only for retained
inside-to-inside teleports. The focused transform/hidden-source/orphan/outside
regression passes, and independent review found no scope widening.

Generated loot is checked semantically, not by taking a side: all12 SRVENC
changed Item facts and all38 main changed Item facts are retained, including
main's i6571 surprise bag. All drop tables and creature bindings remain
unchanged. The repository loot producer and prey simulation regenerate the
admission-bound hash and reports; no generated hash or expected result is
hand-edited. Existing lane migrations remain unchanged and no new number is
allocated; collector migrations are inherited from main.

The collector merge's first workspace compile found two integration families
in newly inherited dispatch fixtures: raw movement-cell references need the
existing SharedOwner conversion (five assignments across four files), and
four Terrain literals need the additive optional blocks_projectile field.
The repair preserves reset epochs and all existing assertions; None retains
the original fixture's unspecified projectile behavior. No tests are removed.
Canonical tree validation passes for 34,111 Items and 252 Mounts; the metadata
check passes separately using its actual world-authoring tool path. A scratch
wrapper's obsolete metadata path failed after the tree check, not in content.

Collector-main merge final local validation (wall times include build-lock
waits; PostgreSQL per-case durations below prove real execution):
- workspace all-target check: PASS 1742.002s wall time.
- format: PASS 15.765s wall time.
- changed-crate clippy: PASS 1409.468s wall time.
- 15 changed-crate unit suites: PENDING; 1663/5424 passed, no failures observed at evidence capture. The owner PREPUSH-2 step-3 cadence exception permits publishing after steps 1, 2 and 4; the run continues and CI remains authoritative.
  Packages: oteryn-client, oteryn-client-assets, oteryn-client-ui, oteryn-game-server, oteryn-gameplay-combat, oteryn-gameplay-content, oteryn-gameplay-dispatch, oteryn-gameplay-durability, oteryn-gameplay-entities, oteryn-gameplay-native-admission-source, oteryn-gameplay-spell, oteryn-gameplay-world, oteryn-protocol-oteryn, oteryn-world-bundle, oteryn-world-bundle-compiler.
- own/ENCX PostgreSQL cases: PASS 1227.695s wall time.
  Summary [ 577.955s] 16 tests run: 16 passed (6 slow), 3432 skipped
- pin refresh: PASS 48.181s wall time.
- pin check: PASS 34.792s wall time.
- duplicate migrations: PASS 1.133s wall time.
- architecture: PASS 3.785s wall time.
- world metadata: PASS 1.977s wall time.
- prey freshness: PASS 83.894s wall time.
- native spell freshness: PASS 84.274s wall time.

PostgreSQL cases:
- PASS [  49.949s] ( 1/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::boss_rewards::boss_rewards_postgres_production_boss_lethal_to_personal_item_custody
- PASS [  90.821s] ( 2/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::boss_activity::boss_activity_postgres_production_reward_boss_instance_corpse_and_personal_custody
- PASS [  90.508s] ( 3/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::npc_program::catalogue_npc_general_xp_handin_runs_in_real_dispatch
- PASS [  91.632s] ( 4/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::npc_program::catalogue_npc_source_acceptance_is_atomic_and_replay_bound
- PASS [  50.056s] ( 5/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::reaudit_arena_lifecycle::srvenc_tibiadrome_postgres_exploding_corpse_hits_current_health_once
- PASS [  49.054s] ( 6/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::reaudit_arena_lifecycle::srvenc_tibiadrome_postgres_grace_expiry_clears_temporary_status_before_save
- PASS [  48.059s] ( 7/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::reaudit_arena_lifecycle::srvenc_tibiadrome_postgres_native_kills_before_deadline_survive_delayed_observation
- PASS [  49.388s] ( 8/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::reaudit_arena_lifecycle::srvenc_tibiadrome_postgres_native_victory_credits_before_reserved_corpse_effects
- PASS [  47.508s] ( 9/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::reaudit_arena_lifecycle::srvenc_tibiadrome_postgres_raffle_awards_one_fifth_of_large_rotation
- PASS [  51.040s] (10/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::reaudit_arena_lifecycle::srvenc_tibiadrome_postgres_three_native_waves_command_catchup_counts_each_once
- PASS [ 102.771s] (11/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::reaudit_arena_lifecycle::srvenc_tibiadrome_postgres_real_source_hall_runs_an_irregular_native_wave
- PASS [ 164.769s] (12/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srvenc_source_boss::srvenc_source_sugar_daddy_template_creates_instance_and_settles_personal_rewards_postgres
- PASS [  13.533s] (13/16) oteryn-gameplay-durability durability::boss_encounter::activity_assignment_tests::native_corpse::native_instance_corpse_migration_preserves_committed_causes_and_replay_postgres
- PASS [ 227.320s] (14/16) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srvenc_boss_publication::srvenc_boss_publication_postgres_scopes_replay_conflict_and_current_reader
- PASS [  15.833s] (15/16) oteryn-gameplay-durability durability::boss_encounter::activity_assignment_tests::native_corpse::native_instance_corpse_migration_preserves_legacy_v1_corpse_and_loot_replay_postgres
- PASS [  13.505s] (16/16) oteryn-gameplay-durability durability::boss_encounter::activity_assignment_tests::native_corpse::native_instance_corpse_mint_postgres_physical_death_loot_replay_and_scope_fences

Diff preservation before this evidence-only commit: 331 files changed, 14199 insertions(+), 1295 deletions(-).
Main is an ancestor (behind=0). No new allocation; all ten lane/ENCX SQL files
remain byte-identical to the pre-merge candidate. After the single push, the
branch returns to owner-requested freeze; CI remains the full-suite authority.

## SRVENC CI repair: qualified Primal encounter closure (2026-11-08)

The CI Primal run could not select its qualified room: the strict callback/data
qualifier lacked a complete native consumer and Item/loot closure. The repair
retains that guard. `bosses/primal_closure.rs` requires the actual Primal Menace,
all eleven source beasts and Fungosaurus policies, every referenced attack and
defense, the real 39530 corpse reward row, its four native decay stages and the
exact immovable, unpickupable, walkable pod policy. Callback names qualify only
through this current-generation proof; unknown hooks and incomplete data still
refuse admission.

`primal_item_profiles.py` captures only the five already-bound Items 39176 and
39527–39530 through the existing source Item interpreter. It verifies the
registered Crystal ff7ede XML/appearance hashes, preserves accepted owner facts,
rejects identity/custody/physical conflicts and regenerates the native provider
manifest. All five retain Ground-only custody. `primal_reference_overlay.py`
separately verifies the pinned Primal Lua blob and its literal `monster.loot = {}`
and corpse 39530 before adding the exact empty `IndependentBernoulliPpm` Loot
definition and Creature loot reference. Unrelated null loot remains unknown.
The materializer and validator apply the same closed overlay; generated records
and manifests are produced by repository tooling.

The native Creature cadence consumes the qualified source roster and schedules
beasts, pods and Fungosaurus replacement through the current Instance owner.
Pods use scope-local physical Item projections, committed STEP ingress and the
existing player HP/death owner; their source damage and hatch deadlines retain
the active scope, generation and artifact binding. The donor's once-earned
Primal-killed quest flag is projected from the immutable, contributor-bound
Primal hazard receipts in `quest_state/storage.rs`, under its existing root lock
and track cap. Its focused PostgreSQL RED executed in 48.381 s: the real credited
kill previously returned no completion track. GREEN evidence follows below.

Donor rules and captures:

- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:27-39,66-79,118,156-347`:
  beast pool, callbacks, physical corpse, explicit empty loot and native cadence.
- The same revision, `data-global/scripts/systems/hazard_primal.lua:23-81` and
  `config.lua.dist:247,255-259`: pod STEP damage, physical creation and hatching;
  `data-global/scripts/quests/the_primal_ordeal/creaturescripts_the_primal_menace_killed.lua:3-18`
  and `data/libs/systems/hazard.lua:116-126`: damaging-player quest flag and
  conditional hazard advancement.
- Crystal `ff7ede593c69d4c658b382c97443e8155926924a`,
  `data/items/items.xml:71977,73314-73335` and `src/items/items.cpp:236-251`:
  pod identity, corpse capacities/decay and appearance-derived physical flags.
  The capture uses the registered XML `c847293e…` and appearance `6adb790d…` hashes.

No migration, command, capability bit or mechanic ordinal is allocated by this
repair. Cargo/PostgreSQL validation is recorded separately after execution.

### PR #3034 CI repair evidence after the PUBPROJ main merge

Normal merge `22b7247bc` incorporates main `b3864f14a`; no migration is edited or
allocated by these repairs. Before merging, exact main `78294c08` passed the
reported Monk scenario (45.959 s) and seeded map walk (6.141 s). The branch's
encounter changes remain present; final diff preservation is checked before push.

The receipt and reservation writers explicitly provide both typed death-scope
columns required by migration 1152, including the Channel fixtures that disable
triggers. The existing discriminator/key checks, immutable causes and 17-byte
legacy scope identity remain intact. Instance assignment keys retain their
separate 33-byte namespace; no default or constraint is weakened.

The Monk oracle now applies the actual qualified target's armor and mitigation
before Wheel/Mantra flats. Canary `99902524`,
`data/scripts/spells/attack/tiger_clash.lua:6` and
`src/creatures/creature.cpp:977-987,1006-1007,911-914` require that order. Rat armor
1 and mitigation 3/25 percent explain `1376 -> 1375 -> 1373`, then 71 flat damage
produces 1444. Both critical branches assert that their accepted interval width
does not increase. Production combat and every other stage assertion are retained.

The map digest is generated by `tools/contracts/refresh_world_map_golden.py`,
which first verifies the v6 wire compatibility recording: 324 updates and
5,431,963 bytes. The intentional bundle format transition changes the recorded
digest to `212408a7b0779ef39613a505339751452e860ead8f1d1252d277c6584515e4db`.
The generator runs the dispatch all-features configuration used by CI.

Metadata and Item packets are regenerated from the qualified placements/Terrain.
The world-base palette receipt recount mode verifies every region and the pinned
XML before updating only its palette summary. Historical Item reconstruction
checks the exact Sugar Terrain producer and retains the immutable original Hit
receipt; nested reconstruction accepts that exact SHA only after terrain checks.
Repository tooling refreshes derived qualification receipts and consumer pins.
All 47 checked Item semantic sections are unchanged. Authoring formatter deltas
retain the same non-import AST after the equivalent `re.M` alias normalization;
the import reordering and unused-import removal were reviewed separately.

Executed final authoring checks:

- World metadata: all 15 workflow commands PASS, 89.652 s; Item authoring: the
  complete workflow command list PASS, 564.687 s (including census, drift and Ruff).
- Item/Mount equivalence: full regression script PASS, 458.743 s; standalone
  validator PASS, 125.347 s. All other local workflow commands pass. Exact donor
  presentation/race checks PASS, 7.584 s / 4.269 s, after filling the local source
  cache; no source check was relaxed.
- Primal producer regressions: 7/7 PASS, 2.740 s; registry-fragment regressions:
  7/7 PASS, 0.386 s. Original Forge failure and historical guards: 4/4 PASS,
  40.344 s + 15.772 s; their full-workflow rerun is also green.
- World pins: refresh PASS, 47.084 s; pin-check PASS, 40.816 s; duplicate-migration
  check PASS, 0.509 s. Quest binding evidence regenerated and checked, 20.458 s.
- Map recording regeneration PASS, 560.243 s including build; its verified walk
  executes in 6.23 s. The separate Primal completion regression first executed
  RED on PostgreSQL in 48.381 s (`None` versus the required `Some(1)`).

Final changed-crate, PostgreSQL, compile, Clippy and formatting results follow
only after their commands complete.

### PR #3034: requested SOULPIT main merge and combined evidence

The requested normal merge includes main `98d6ae8ce5e12d60916532c72353126f534fa20a`
(SOULPIT, migrations 1140–1141) after the PUBPROJ, QENGINE and NPC physical-item
lane merges. It keeps the existing shared ownership of private instance maps,
while carrying main's atomic Soul Core debit, native Soulpit lifecycle, ACK and
return handling. The main merge's duplicate door initial-state declaration is
removed; activity forks retain their actual bounds, and v8 projectile facts
remain explicit in the newly merged terrain fixtures.

Both qualified Terrain additions are replayed by their producers. The combined
historical view verifies and strips the exact Sugar shard/witness, then the
exact Soulpit shard, then Grizzly; the original historical Hit packet digest
remains immutable. Seven previously merged Soulpit qualification receipts remain
byte-identical. `refresh_srvenc_qualification_receipts.py` generates fresh combined
successors (proof v4, stack receipt v12, name v5, Mantra/numeric v10), permits only
byte-identical owned reruns, and keeps all source facts and qualification scopes.
No migration, command, capability or mechanic number is allocated by these CI
repairs. Final validation results are recorded below after the complete run.

### PR #3034: main47 repair validation receipts

Main `47baa7bb872498ba9c5b30cf070c330f85cefba8` is included by a normal
merge. Its floor-transform lane proof and finite 8 MiB cosmetics provisioning
bound are retained; both provisioning regression tests pass. The key-33 inventory
was regenerated with its producer and all ten Rust integration checks pass
(4.127 s execution; 604.608 s including the fresh integration-target build).

All 15 world-metadata job commands pass (96.042 s). The full item-authoring job
passes (535.891 s). All 19 Content tree / Item+Mount equivalence job commands pass
(867.879 s); no CI timeout or workflow limit was changed. Exact equivalence replays
the same hash-checked Primal empty-loot admission as the producer, preserving all
family-count and full-definition equality checks. Fifty original semantic packet
sections across nineteen existing evidence files remain unchanged; all published
qualification receipts remain byte-identical to main.

The official map recorder passes with `LEGACY_WIRE_VERIFIED`: 324 updates,
5,431,963 bytes, and digest
`212408a7b0779ef39613a505339751452e860ead8f1d1252d277c6584515e4db`.
Its walk executes for 6.22 s (785.299 s including the build). The merged gate
inventory's exact comparisons also pass, including the actual single activity
cast gate and the new Primal PostgreSQL scenario's explicit Source-room seam.

Focused PostgreSQL evidence on the combined tree (admin URL present; each case
executes against PostgreSQL rather than the route-less guard):

- `primal_ordeal_postgres_qualified_production_room_is_reachable`: 102.942 s.
- `primal_hazard_postgres_kill_unlock_selection_replay_cap_and_fences`: 74.597 s.
- `c11_monk_real_dispatch_ascetic_party_serene_refund_and_stance_reload_postgres`: 76.748 s.
- `primal_menace_postgres_native_cadence_spawns_and_retires_actual_beasts`: 83.372 s.
- `srvenc_tibiadrome_postgres_real_source_hall_runs_an_irregular_native_wave`: 99.104 s.
- `srvenc_source_sugar_daddy_template_creates_instance_and_settles_personal_rewards_postgres`: 129.716 s.
- `utility_runes_animate_real_dispatch_owns_skeleton_and_consumes_corpse`: 65.296 s.
- `utility_runes_animate_real_dispatch_refuses_black_skull`: 78.286 s.
- `soulpit_native_entry_refuses_insufficient_wave_capacity_without_burning_core_postgres`: 74.700 s.
- `soulpit_native_item_use_entry_return_and_animus_query_postgres`: 67.769 s.
- `soulpit_production_owner_spawns_native_wave_and_timeout_returns_without_mastery_postgres`: 59.600 s.
- `soulpit_shared_entry_debits_core_and_commits_real_instance_session_postgres`: 64.646 s.
- `soulpit_terminal_return_requires_checkpoint_and_interrupt_never_grants_mastery_postgres`: 59.419 s.
- `soulpit_shared_entry_uses_current_trusted_gm_group_not_forged_hint_postgres`: 55.698 s.

Final combined-tree checks are green:

- Changed-crate unit tests: 7,086 passed across all 17 changed crates; 1,872.072 s
  execution (2,647.218 s including compilation). The 28 pre-existing ignored
  tests retain their original annotations; no test was weakened or skipped by
  this repair. The world-change lifted cases and all nine gate inventories pass.
- Focused PostgreSQL run: all 60 selected checks pass, 1,500.695 s execution
  (2,222.387 s including the shared-build wait). Both reported animate-rune cases,
  the reported Primal room and monk cases, Source Sugar personal rewards,
  production Tibiadrome, native Instance corpse replay/fences and the merged
  Soulpit entry/lifecycle/GM/Animus checks executed with the admin URL present.
- `cargo check --workspace --all-targets`: 237.870 s.
- Clippy for all 17 changed crates, all targets/features, `-D warnings`: 272.887 s.
- `cargo fmt --all --check`: 15.598 s.
- `architecture-check workspace .`: 5.314 s.
- Final producer `pin-refresh`: 43.278 s; `pin-check`: 49.653 s.
- `dup-migrations.sh HEAD`: 0.602 s, no duplicate numbers.

The published candidate includes the requested normal main merges and preserves
its substantive diff against main. This repair allocates no new numbers and
changes no merged migration or CI workflow. The branch remains frozen after the
single combined repair push; subsequent writes are limited to owner-sent CI fixes
or requested main merges.

### Requested QREADY main merge (2026-11-08)

Normal merge includes main `c77d436d8be39f348924e66d97928ffd69e61a8c` (#3037 QREADY1+2), preserving both lanes'
reward profiles and all QREADY chest bindings. The independent equipment cohorts
now sum to 3,792 admitted profiles, exactly matching the regenerated provider
union; the incoming-only 3,774-profile intermediate correctly failed the guard
before the boss/Primal producers replayed. No assertion was relaxed.

NPC and service shards are byte-identical to main: 1,282 NPC definitions and
2,747 NPC source bindings; 324 trade, 56 travel, 2 bank, 7 promotion and 10 blessing
services (399 total). There are 12,234 declared trade offer rows: 5,202
BuyFromPlayer and 7,032 SellToPlayer. Eight trade services have no executable
offer array; their held source evidence remains untouched. Exact NPC/service
round-trip validation passes, including complete offer payloads rather than just
counts.

The five new QREADY fixture cell assignments use the existing borrowed
`SharedOwner` conversion. Their test assertions and actual Source cells are
unchanged. World metadata passes all 15 commands (102.570 s), strict Item/Mount
and NPC/service equivalence passes (146.977 s), and all eleven incoming QREADY
Python checks pass (4.814 s). Canonical regeneration: 231.987 s; derived tail:
38.752 s; pin refresh: 25.313 s; pin check: 38.171 s; duplicate migration check:
0.445 s. No new migration, command, capability or mechanic number is allocated.

Publication follows the owner's PREPUSH-2 cadence exception for step 3: the
corrected `cargo check --workspace --all-targets` passes (867.960 s including the
build-lock wait), Clippy passes (856.560 s including its wait), fmt passes
(15.526 s), and architecture-check passes (130.392 s including its wait).
Item-authoring's full job passes (585.425 s). At the publication checkpoint the
focused PG run has passed its first 15/23 cases, including both animate-rune
regressions, Primal entry/cadence, monk, Tibiadrome and personal boss rewards.
The remaining focused PG cases and unit suites for `oteryn-gameplay-dispatch`,
`oteryn-gameplay-content` and `oteryn-client-assets` remain running, not reported
as green. This is the explicitly allowed pending step-3 validation; tests were
not weakened, filtered out of those selected unit suites, or stopped.

### PR #3034: final explore packet / S1 hunt CI batch

The requested main merge is already present: `53b082bea` includes main
`c77d436d8` through normal merge `278631a7a`; no rebase or force update is used.
`test_quest_explore_area_candidates::test_committed_packet_matches_generator`
reproduced locally (8 tests, one failure, 0.230 s), then all eight tests passed
(0.235 s) after `quest_explore_area_candidates.py` regenerated the packet. The
only semantic JSON delta is `/inputs/5/sha256`, the islands-index receipt; all
247 stages, 970 canonical Areas, 20 exact candidates and zero runtime spatial
bindings remain unchanged. No generator or test assertion is loosened.

The S1 hunt failure precedes loot, Prey payout and relogin: the peer must first
land real nonlethal qualifying damage. Its runtime combat facts were Knight
level 1, unarmed, Balanced, starting fist skill 10. The checked-in fist formula
allows raw 1–6 damage, while the actual qualified Rat can block defense 2–5,
then armor 1 and mitigation 3/25 percent. Thus 32 sampled swings do not guarantee
a positive hit. Attack code, the Rat policy and this scenario are unchanged
between the failing head and main; this is an underpowered stochastic fixture,
not a loot or Prey rule regression. A pre-fix local PG execution happened to
pass in 34.656 s, consistent with an intermittent failure.

The leader already explicitly uses runtime combat level 50 to keep its minimum
fist damage above Rat defenses (`full_world_support.rs:499`). The peer now uses
the same fixture level. Its durable progression state, party levels, payout
rules, target health, 32-attempt limit and all existing assertions are unchanged.
An additional precondition computes the minimum from the live peer facts and
production `player_fist_formula`, subtracts the actual qualified target's maximum
defense/armor, and applies production `creature_physical_response`; it requires
strictly positive worst-case damage before the existing real-hit loop.

Donor evidence: TibiaTools `fe745abf` `damage-calc/damage.ts:13-26`, as admitted in
`combat-rules/src/attack/formulas.rs`; Canary `99902524` `creature.cpp:958-1007`
for defense/armor/mitigation, and `creature.cpp:911-914` for response order. The
repair changes no production combat, encounter, loot, Prey or persistence rule,
allocates no IDs/migrations, and weakens no test. Final execution receipts follow.

### CI repair validation and latest main integration (2026-11-08)

Merged main `3209d3d61` normally. Preserved main's exact Soulpit boot,
handoff owner, durability error classification and NPC service-model test files
(400 services / 12415 offers / 17 sell-above-cap). The branch retains its
substantive encounter diff; no migration or identifier was allocated.

The repaired hunt scenario executed against PostgreSQL: PASS 67.087 s
(nextest summary 67.098 s). The explore-area generator's eight targeted tests
passed in 0.235 s after reproducing the stale receipt failure. Workspace
all-targets check after the latest merge passed in 278.622 s; formatting passed
in 15.156 s. Dispatch all-targets/all-features Clippy passed with `-D warnings`
(1084.612 s including build-lock wait); architecture check passed in 281.028 s.

The full quest-authoring run executed 1110 tests in 1340.713 s and exposed
two errors with one cause: the earlier SharedOwner compile adaptation changed
`qready1_reward_claims.rs`, invalidating its source-bound PG evidence. The same
source-binding sweep found all five QREADY1/QREADY2/Purple receipts stale. All
41 real PostgreSQL cases passed: QREADY1 17/17 in 552.21 s (individual
31.885–103.013 s), QREADY2 21/21 in 448.40 s (31.119–52.959 s), Purple Tome
3/3 in 61.68 s (29.443–32.485 s). The existing lane-specific QREADY1 importer
and generic `quest_reward_use_proofs.py` importer regenerated all five receipts
from successful completion markers and the actual passing summaries. Neither
hashes nor durations were edited by hand. The completion matrix was regenerated
and unchanged; the explore packet was regenerated last. All 23 focused quest
proof/matrix/explore tests passed in 1.748 s.

The previous merged-main unit run passed all 3274 selected Rust tests. A fresh
full dispatch unit build was deferred under the owner's cadence exception to
prioritize the latest-main all-targets compile gate; that final unit rerun remains
pending and is not represented as passed. No test was weakened or removed.

The final CI report added the schema-78 Type2 activation backfill failure.
The branch's current-schema reservation helper explicitly supplies death-scope
columns introduced in migration 1152, whereas main's helper supplies the older
columns. Reproduced the exact local PostgreSQL 42703 failure in 1.04 s. The
fixture helper now selects the historical insert shape for schema 78 and keeps
the explicit death-scope writer for current-schema cases. This matches main's
historical SQL, preserves every backfill/immutability assertion, and changes no
production writer or migration. The entire five-case activation PG module is
rerun, rather than only the reported case.

After main integration, repo-tool pin refresh passed in 28.712 s, pin-check
in 30.958 s and duplicate-migration check in 0.465 s. Quest binding evidence
regeneration and its check passed in 17.934 / 18.026 s; equipment's independent
3792-profile union check passed in 0.171 s. No generated values were edited by
hand. The post-backfill-fix workspace all-targets check passed in 59.557 s.

All five Type2 activation PostgreSQL cases passed in 31.76 s after the
schema-aware fixture repair (180.932 s including compilation/lock wait).
The quest-authoring CI `checks` and complete two-worker `unit-tests` runners,
and fresh changed-crate Rust units, were started after proof regeneration.
Their final receipts are kept separate from earlier passing runs; the owner
cadence exception applies if the complete unit runners are pending at push.

Final prepush receipts: the isolated backfill PG execution passed in 6.34 s
(6.352 s wall); the full quest-authoring `checks` job passed in 308.579 s.
Game-server all-targets/all-features Clippy passed with `-D warnings` in
338.781 s; final formatting passed in 32.656 s. Final repo-tool pin refresh,
pin-check and duplicate-migration check passed in 49.325 / 45.135 / 0.398 s.

At the cadence-exception push, fresh full Rust library units (game-server and
the dispatch/content/client-assets set, plus the durability feature-union
package) and the complete quest unit runner are executing, with no failures
reported so far. The queued nextest build was replaced before execution by
`cargo test --no-fail-fast` to share in-process fixtures; no assertions or
tests are omitted. These ongoing runs are not claimed as green at push time.

## PR 3034 head 370423fbc: isolated PG comparison (2026-11-08)

Fetched and merged `origin/main` first at `3209d3d61`, already an ancestor.
After the detached comparison, fetched/merged its successor `f5b584ef1` before
this evidence update. That successor changes only four repository policy/CI
files, with no Rust or content changes. No production, fixture, timeout, pool, security gate, content or test
source is changed for this report. The three reported failures all passed
serially on the SRVENC branch, with a configured real PostgreSQL connection:

- `quest_container_chests_leaf_text_is_frozen_and_replayed`: 29.73 s;
  `W15B_LEAF_TEXT POSTGRES_CASE_COMPLETED`.
- `qengine_l2_battle_mage_whole_outfit_story`: 63.52 s; retained marker reports
  `duration=63.515349151s success=true`.
- `activity_player_death_postgres_partial_loss_recovers_through_empty_origin_lifecycle`:
  6.03 s; `ACTIVITY_PLAYER_DEATH_DEPARTED_PROCESS POSTGRES_CASE_COMPLETED`.

The activity death case was also built and executed from detached main
`3209d3d61`, rather than treating the branch result as a main result: PASS
6.15 s (613.124 s including the separate main build). Returned to the SRVENC
branch and fetched/merged main again before this evidence update. No main
branch ref was modified. The death test, departed-process recovery fixture and
`foundation/fnd04_verifier.rs` are byte-identical between branch and main.

The stale-security failure was not reproduced in either isolated run. The
unchanged verifier (`fnd04_verifier.rs:2284-2317`) rejects observations older
than five seconds; the common `house_transfer::prepare_owned_session` fixture
records its time before publication/commit database operations. This makes
slowness a plausible explanation, not a proven reproduction of CI contention.
The pool/readiness and early-EOF failures likewise did not reproduce alone.
Per owner direction, no pool, serve, node-boot, freshness or gameplay rule is
changed for those passing cases.

The first local attempt stopped before useful execution because old lane
validation databases exhausted disk space. Removed 31 inactive SRVENC/QREADY
databases and obsolete completed lane build variants, then obtained the above
real PG results. No test, assertion or security freshness bound was weakened.

The formerly pending full unit runs on head 370423fbc completed successfully:
client-assets 10 (0.53 s), content 332 (337.50 s), dispatch 2934 (1083.58 s),
durability 737 (52.53 s), and game-server 578 (1866.43 s). All 4591 selected
Rust tests passed; only existing ignored tests remain ignored. The complete
quest checks and 1118-test unit runner also passed on these unchanged content
bytes in 308.579 / 627.564 s, and materialized content validation passed.
This candidate changes only this evidence note, so no crate or pinned test
source requires a new unit or proof import. Final prepush receipts are retained
in the local CI3 validation logs and reported with the push.

An additional optional check of the incoming main-only policy simulator
(`python tools/repository/test_validate_merge_group_pg_sim.py`) passed repository
policy/routing checks but could not execute its native PowerShell canaries:
`AssertionError: pwsh is required for WP1 native failure-propagation qualification`.
This environment lacks that executable; this optional script is not claimed
as passed, and its source/assertions were not changed or bypassed.


## QMECH M24 source-bound death continuations

The shared QMECH package owns this work. The coordinator assigned migration
range **2180–2189**; this implementation allocates no migration, command,
capability or ordinal. The earlier report-only 1108 reservation is released.

The native lethal queue captures exact room recipients for six Heart of
Destruction bosses and Court Warlock, and damaging players for Goblin Leader,
Magma Bubble, Cake Golem and seven Ancient Tombs pharaohs. Source URLs and
file:line evidence are retained in `content-prep/qmech-m24/source-notes.md`.
Primal Menace remains with its existing atomic Hazard writer. Cake uses the
chosen twenty personal kills, with the unavailable bridge unchanged.

Captured scope, generation, character, lease and session are independently
checked by the current fenced quest writer. Replay uses the existing death
receipt identity. Cumulative kills guard the predecessor mission epoch rather
than the changing counter, allowing multiple queued deaths without accepting
a death captured before mission acceptance. Source-bound transition keys
require native evidence; generic transition requests cannot mint this credit.

The real PG red cases exposed an existing rowless-entry asymmetry: without
a qualified corpse Item, the queue constructor discarded `damage_shares`.
Room rules still worked, while damage-only quest steps received no eligible
recipients. The producer now retains the actual native lethal ledger on this
path as it already does on materialized and Soulpit entries. This grants no
XP, corpse, loot or personal reward eligibility. The tests keep their exact
source-stage and receipt-count assertions.

M24 rule-level donor references (Crystal revision
`201189f871d23597b82caf88fd611454a2ddefb8`):

| Rules | Donor file:line | Implemented boundary |
|---|---|---|
| Anomaly, Rupture, Realityquake, Eradicator, Outburst, World Devourer | `data-global/scripts/quests/heart_of_destruction/creaturescripts_heart_boss_death.lua:26`–138 | exact room recipients and source flags |
| Court Warlock | `data-global/scripts/quests/the_order_of_the_stag/court_warlock_fight.lua:323`–329 | room recipients, questline 11→12 |
| Goblin Leader | `data-global/scripts/quests/tower_defence_quest/creaturescripts_goblin_leader_kill.lua:3`–8 | damaging players, questline 2→3 |
| Magma Bubble | `data-global/scripts/quests/the_primal_ordeal/creaturescripts_magma_bubble_death.lua:12`–23 | damage-map recipients and the killed flag only |
| Cake Golem | `data-global/monster/quests/a_piece_of_cake/cake_golem.lua:1` | exact canonical killed creature identity |
| Omruc | `data-global/monster/quests/ancient_tombs/omruc.lua:1` | exact canonical killed creature identity |
| Thalas | `data-global/monster/quests/ancient_tombs/thalas.lua:1` | exact canonical killed creature identity |
| Dipthrah | `data-global/monster/quests/ancient_tombs/dipthrah.lua:1` | canonical spelling for the recipe's Diphtrah label |
| Mahrdis | `data-global/monster/quests/ancient_tombs/mahrdis.lua:1` | exact canonical killed creature identity |
| Vashresamun | `data-global/monster/quests/ancient_tombs/vashresamun.lua:1` | exact canonical killed creature identity |
| Morguthis | `data-global/monster/quests/ancient_tombs/morguthis.lua:1` | exact canonical killed creature identity |
| Rahemos | `data-global/monster/quests/ancient_tombs/rahemos.lua:1` | exact canonical killed creature identity |

The last eight rules intentionally use the already accepted chosen Oteryn
recipe, not a claim of exact donor quest bookkeeping:
`content/quests/missions/quest-state-completion-candidate.json:1`, Cake s2
(personal count20), Ancient Tombs s2/s3/s5/s6/s8/s10/s12 (count1 each). The
generator copies their exact predecessor guards, effects and bounds. Source
URLs, pinned wiki revisions and this approximation distinction are retained
in `docs/reference/content-prep/qmech-m24/source-notes.md`.


## M24 real PostgreSQL qualification

On the own-lane branch after coordinator QMECH `a505e643f`, all13
`srvenc_m24_postgres_*` cases passed against real PostgreSQL18.6:
264.54s test execution, 0 failures, 0 ignored. Each case creates its own
database and drives an actual native lethal projection, the existing kill
queue and fenced quest writes; corpse materialization remains independently
qualified. No ADMIN_URL skip path was used.

| PG case suffix | Executed seconds |
|---|---:|
| `dipthrah_stage` | 57.285 |
| `anomaly_room` | 57.343 |
| `court_warlock_room` | 57.367 |
| `cake_golem_twenty_and_cap` | 69.396 |
| `goblin_leader_damage` | 57.923 |
| `magma_bubble_damage` | 57.923 |
| `mahrdis_stage` | 57.916 |
| `morguthis_stage` | 71.175 |
| `omruc_stage` | 76.087 |
| `rahemos_stage` | 90.675 |
| `thalas_stage` | 97.475 |
| `vashresamun_stage` | 96.262 |
| `world_devourer_room` | 73.221 |

Every case also rejects a generic command for the native-bound transition
and rejects a source context from a different ownership generation. The Cake
case queues simultaneous kills, reaches exactly20, refuses the21st credit and
replays each corpse projection without an extra receipt. Source stage/value
and exact receipt-count assertions remain unchanged.

The initial negative fixture mistakenly supplied a non-UUIDv7 occurrence.
It now uses the existing UUIDv7 fixture helper so the test reaches the actual
wrong-generation check. The earlier personal-credit red cases exposed the
real rowless-ledger loss; retaining the native damage shares fixes those
without qualifying XP, loot or corpse Items. Build retries resolved memory
pressure by stopping concurrent Python runners, and disk pressure by deleting
inactive lane compiler caches; normal repository profiles were preserved.

Final M24 correction-packet checks: dispatch all-feature/all-target Clippy
with `-D warnings` PASS252.836s; workspace all-target check PASS290.055s;
three source-boss catalogue/boundary/counter tests PASS3.19s; native-provider
regeneration through `regenerate_content.py --native-only` PASS228.403s.
Equipment5791/cohort-union, seasonal inventory, Grizzly30/30 room pins,
materialized tree97/97, M24 generated bindings, migration duplication and
format checks passed. The full authoring checks/unit runner also passed on
the unchanged M24 quest inputs (308.581s /489.307s); no pinned reward-USE
test source was modified.

The optional additional Grizzly filter did not execute when its unrelated
normal-profile rebuild exhausted disk; it is not counted as a passing test.
All13 required M24 PG cases above actually executed and passed. Inactive
compiler caches were removed and the required format/tree/pin checks rerun.

## M03: live character requirements, first engine slice

Coordinator correction: M02 already belongs to ENGNPC2/SRV7 and is not SRVENC
work. After merging qmech 5b960aa2a, M03 was OPEN/unclaimed and is now claimed by
SRVENC in INDEX only. CLAIMS.md remains coordinator-owned.

The chosen 25 Years of Tibia recipe requirements are recorded in
content/quests/definitions/quests-00200-00299.json:1, record 67. The retained
Crystal donor data-global/scripts/quests/25_years_of _tibia_quest/actions_time_machine.lua:26-35
supplies the entrance trigger, not these owner-selected prerequisites.
The schema now binds explicit promotion/blessing requirements into the native
world admission digest and rejects unbound or malformed requirements. The
existing fenced world callback consumer reads permanent promotion and currently
held blessings inside the Character writer transaction; consumed blessings
cannot be accepted from session projections.

This is a tested engine slice, not completed M03: premium owner threading and
the two production quest entrance bindings follow. No migration, command,
capability or registry number is allocated.

M03 first-slice validation: schema test RED (unknown character_requirements),
then GREEN. Real PG case
m03_postgres_character_requirements_read_current_transaction executed in 5.87 s
(wall 169.323 s including normal-profile compilation), covering missing promotion,
current promotion, missing/held blessings and consumed blessing refusal.
The qmech M07 fixture required the missing ground_stack:None initializer to
compile; no assertion changed. Full quests lib run:356 PASS / 3 FAIL, all three
failures in untouched reward_callbacks tests; baseline comparison is recorded
separately in the local validation log. No failing test is skipped or weakened.
Baseline qmech 5b960aa2a, with only the ground_stack:None compile repair and
without any M03 source changes: reward_callbacks filter reproduced the same
3 FAIL / 21 PASS in 3.96 s (wall 16.011 s). Thus these reward fixture failures predate
this slice; reward code and assertions are unchanged.
Workspace checking also exposed existing missing creature_use:None fields in
engitem_oramond.rs and questpc5_map_marks.rs admission fixtures. Both are repaired
without changing assertions. M03's PG test uses the existing harness's explicit
noncanonical pre-routing behavior when PG is absent; only the real configured PG
execution is counted as validation evidence.

## M03: transaction-scoped Premium admission slice

Pits of Inferno's chosen Premium requirement is recorded in
content/quests/definitions/quests-00000-00099.json:1, record 89. The native
SpellPremiumCoordinator already implements the transaction-scoped quest Premium
port (spell_premium_coordinator/promotion_gate.rs:48-59); M03 reuses that owner.
The writer resolves the account from the fenced Character root and asks that
owner inside the same transaction. Missing authority refuses admission. Both
immediate world callbacks and queued retries retain the native coordinator.
Legacy callers without an owner remain fail-closed for newly gated callbacks.
Reward callback Premium admission keeps its existing owner through the same
helper. No new Premium projection, migration or numeric registry entry is added.

The new Premium schema test first failed on the absent premium field, then
passed. Its PG probe checks the actual durable account and PostgreSQL transaction
ID, plus current/refused/missing-owner cases. Production entrance content wiring
remains the next M03 slice; this engine slice alone is not marked DONE.
Premium slice proof: both M03 PG tests executed on the configured real database
and passed in 5.09 s (wall 132.715 s including compilation). Premium schema RED:
unknown premium field; GREEN: both M03 schema tests pass. Changed production
crates clippy -D warnings passed in 137.728 s; native regeneration passed in
223.558 s without changing outputs. No content input changed in this slice.
The compiled M03 PG cases were executed again with a clean proof log: 2 PASS,
6.52 s (wall 6.523 s). Workspace all-target check passed in 307.226 s; format
check and duplicate-migration check passed. The initial compile error from a
borrowed Premium gate was fixed before these successful checks.

## M03: production quest entrance wiring

The existing shared-access generator now installs 19 native bindings: the nine
source machine entry positions, nine unconditional return positions, and the
source-selected PoI entrance at32795,32335,8. The retained B3 is read by the
new generator to bind exact appearances and top-level ordinals; return corners
contain extra decorations and therefore use ordinal2. No World tile is edited.
Source URLs and exact chosen-recipe boundaries are recorded in
`docs/reference/content-prep/qmech-m03/source-notes.md`.

The closed chosen-entry profile validates both real recipe guards and source
positions. It is the only new exception to donor-script provenance validation;
arbitrary URLs and altered guards are rejected. Its first-stage counters start
at0 and raise to1 without undoing later progress. Generic quest writers cannot
impersonate these source-bound world entries. Return travel is not gated by
Premium, promotion or blessings after the player has entered.

RED production-catalogue test: missing M03 time-machine entry. GREEN: production
catalogue includes all19 selectors and the real chosen first-stage counters.
Generator tests:3PASS, including return ordinals and refusal after recipe drift.
Real PG production PoI STEP:69.82s, including missing-Premium refusal, generic
caller refusal, a committed stage counter, exact replay and a fresh refusal;
exactly one receipt persists. Actual five-blessing source guard PG:7.00s;
account/transaction Premium PG:6.74s. Content-tree validation:97/97 PASS.
No migration or numeric identifier is allocated (assigned range2180–2189 unused).
Quest-authoring checks passed in329.385s; the full unit runner passed all113
modules in543.359s. Native regeneration passed in295.367s. M03 schema tests:
3PASS; production entry catalogue test:PASS. Changed crates clippy passed
in105.548s. The subsequent workspace check exhausted disk before completion;
inactive compiler caches were removed and it is rerun after the requested
qmech merge. No test or assertion was weakened.

## M51: source Barbarian Arena death and geometry rules (first slice)

Read current qmech INDEX, CLAIMS and ASSIGN without merging: M51 is OPEN,
claim none, and has no assigned lane. Claimed M51 locally for SRVENC; CLAIMS.md
is unchanged. Rules cite Crystal201189f
`data-global/scripts/quests/svargrond_arena/creaturescripts_arena_kill.lua:2-38`
and `data-global/lib/quests/svargrond_arena.lua:7-22,25-115,118-172,193-256,285-313`.
The donor is retained in the source corpus and at
https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/quests/svargrond_arena.lua

The pure rules preserve most-damage credit, tier-wide creature membership,
pillar/portal consequences, the source's 600-second deadline and exact ten-pit
geometry. The last pit's center/pillar differs by one Y tile from its reset
rectangle; spectator occupancy has a different readset from reset. All30 source
creatures were checked against canonical creature keys. Missing pillar still
advances PitDoor but creates no portal, matching the donor's branch ordering.
This API computes proposals only: it is not an ownership or write capability.

RED: absent source arena rules module (E0432). GREEN:3 focused rules tests;
clippy all targets passed in10.977s and format check passed. No SQL or content
input changed, so this slice adds no PG case or generated content. No migration
or registry number is allocated. Exclusive pit custody, production death/map
composition and durable kick/reset remain; M51 is PARTIAL, not DONE.

## M51: atomic shared pit reservation before entry fee (second slice)

Allocated migration2180 (assigned range2180–2189): game_svargrond_pit_leases.
The existing fenced arena_match writer claims shared pit custody before charging
an entry fee or accepting an advance. Transaction advisory locking closes the
empty-slot race; a different live run cannot replace custody. Old-pit release
and new-pit claim share the arena transaction, so a failed composition rolls
back to the old pit. Idle/completed arena state releases custody. Progress,
fees, victories and command receipts remain with the existing owner.

Source: Crystal201189f movements_arena_enter.lua:22-30,
movements_arena_pit.lua:52-62, lib/quests/svargrond_arena.lua:193-256,285-313.
RED pre-migration PG fixture:5.22s, Database(Database(None)); new table absent.
GREEN real PG: m51_postgres_pit_reservation_serializes_contenders_and_releases_atomically,
4.95s (wall23.356s including compilation). It exercises actual simultaneous
transactions, refusal after commit, rollback of an advance and release.
Durability clippy -D warnings:98.792s; fmt check PASS. No generated content or
command/capability ordinal changed. M51 remains PARTIAL pending native
pillar/portal composition, recovery custody checks and kick/reset closure.

## M51: custody checked before restoring a fight (third slice)

The arena reader now proves exact run/session/scope/deadline ownership before
allowing native opponent restoration. A still-current active run can adopt an
unoccupied reservation through the same fenced owner; conflicting custody
prevents restoration and follows the existing stale-run cleanup path.
The dispatcher retains this result rather than respawning from progress alone.
Source: movements_arena_enter.lua:22-30 and movements_arena_pit.lua:52-62.
RED: missing owned predicate, E0425. GREEN real PG:5.10s, testing wrong run,
wrong deadline, exact expiry and the existing contention/rollback checks.
Durability + dispatch clippy -D warnings:122.612s; fmt PASS. No new migration.
M51 remains PARTIAL: native pillar/portal projection and lifecycle closure follow.

## M51: retain the kick timer while waiting at a victory portal

Restored phase-2 matches now retain a live timer without spawning another
opponent. Phase changes update that projection instead of reusing a fighting
projection solely by run/pit. Only active phases before the original deadline
can recover a timer; victory never grants another 600 seconds.
Source: Crystal data-global/lib/quests/svargrond_arena.lua:259-313.
RED: m51_restored_victory_keeps_the_original_kick_deadline, E0599 (12.245s).
GREEN: four focused M51 durability tests, 29.645s including real PG execution
4.85s. Durability/dispatch clippy -D warnings:34.850s. No SQL/content changes.
M51 remains PARTIAL pending native pillar/portal projection and reset closure.

## M51: native spectator occupancy before reserving a pit

Entry/advance preflight now checks the positioned player census, not only
center/summon cells and the local match registry. Other players anywhere in the
source five-tile rectangle block entry; the entrant and other floors do not.
Source: Crystal data-global/lib/quests/svargrond_arena.lua:193-205,249-256.
RED: missing occupied_by_other predicate, E0599 (12.538s). GREEN: four arena
source tests, 2.829s; quests/dispatch clippy -D warnings, 26.975s. No SQL/content
changes or new numbers. M51 remains PARTIAL pending pillar/portal/reset wiring.
Engine-only pin-check passed in 154.617s; the committed bundle digest is unchanged.

## M51: native pillar/portal projection and exact-run reset

The committed fight/victory projection now updates the pinned pillar occurrence
between source Item1841 and Item5022, with its collision and client map facts in
one callback-owner mutation. The portal is bound/non-pickupable. Recovery is
idempotent; entry/advance, leaving, death/logout pruning and the existing timed
kick close the matching run's portal. A late old-run release cannot reset a
newer run. Missing/hidden pillars and ambiguous/foreign occurrences are refused.
Sources: Crystal201189f data-global/scripts/quests/svargrond_arena/
creaturescripts_arena_kill.lua:22-35; data-global/lib/quests/svargrond_arena.lua:
207-246,259-313. This uses the existing Channel map callback owner and SV15
committed match state; no protocol id, capability, mechanic ordinal or migration.
RED: missing projection method E0599, 126.907s. GREEN native map/collision test
m51_pillar_portal_view_and_collision_reset_together:21.475s including build.
World/dispatch clippy -D warnings:49.571s. Pin-check:157.579s, unchanged bundle.
M51 remains PARTIAL: native portal STEP admission, automatic death publication
and the full source pit reset (including movable items) are still unfinished.
Own M51 durability cases rerun:4 PASS, real PG4.81s (108.400s including rebuild).

## M51: durable native portal origin (migration2181)

Allocated2181_svargrond_native_step_origin.sql, in SRVENC's2180-2189 range.
Arena receipts retain the complete native-step binding beside its compact id.
Both client/native collisions and two native bindings with the same compact id
refuse replay. Existing client reward/arena writers keep their NULL origin.
NativeArenaStep is built from an actual translation snapshot at the source
pillar, binding exact actor, session, run, pit, scope generation, owner epoch and
movement revisions. The native writer accepts only credited phase2 Advance and
requires exact current unexpired pit custody before a new transition.
Source: Crystal201189f data-global/scripts/quests/svargrond_arena/
movements_arena_pit.lua:52-64. No protocol command/capability/ordinal allocation.
RED: missing origin helpers E0425,19.875s. GREEN:6 focused durability tests,
31.976s including build. Real PG native-origin case:5.662605238s; pit custody:
5.701863876s. Migration uniqueness/embedded-migrator test:14.968s PASS.
Durability/dispatch clippy -D warnings:28.990s. Pin-check:166.664s PASS.
M51 remains PARTIAL: native STEP ingress still needs to call this typed writer;
automatic death publication and full source pit reset also remain unfinished.

## M51: expose exact-run native portal admission

The map owner now admits action25200 only for a visible, passable, bound,
non-pickupable Item5022 projected by the matching run. Another run or a reset
pillar cannot impersonate its portal. Source: Crystal201189f
movements_arena_pit.lua:52-64 and creaturescripts_arena_kill.lua:22-35.
RED missing admission predicate E0599:93.285s; GREEN map/collision test:31.793s.
Changed world/dispatch/durability clippy -D warnings:41.711s. Formatting PASS;
engine-only derive-identity equals the committed pin identity. No SQL or numbers.
Native STEP caller and its full ten-pit PG case continue in the next slice.
