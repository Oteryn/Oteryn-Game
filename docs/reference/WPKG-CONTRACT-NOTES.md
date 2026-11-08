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

## WPKG — WORLDCHG-Q merged and focused validation complete (2026-11-07)

Merged source `ebc53bc6e` as merge commit `718b199fb`; follow-up `d9dacb214` repairs native-model generation order. Main's Fury/Nightmare layers and Flower/Stampede raid test branches are retained. The captured-wolf profile reuses main's canonical owner: 40 new native profiles, no removed or duplicate keys, and the existing wolf record is byte-semantically unchanged. All eleven lane-owned titles remain QUALIFIED (the catalogue total of twelve includes the existing Stampede).

Source conflicts were resolved in daily-state persistence/layer projection, boot loading and raid announcement fixtures. Derived content, manifests, callbacks, native providers, Grizzly room input pins, Prey model/reports and world pins were regenerated with repository tools. The Prey producer now writes generated loot admission before hashing its model input; its regression requires the generated closure to be visible. No generated file was hand-edited.

Focused validation (command wall durations; logs retained under `/tmp/wpkg-*` in this workspace):

- `worldchg-fmt`: PASS, 17.036s.
- `worldchg-check`: PASS, 27.052s.
- `worldchg-clippy`: PASS, 259.540s.
- `worldchg-quest-final`: PASS, 384.840s.
- `worldchg-native-check-final`: PASS, 83.119s.
- `worldchg-prey-tests`: PASS, 0.170s.
- `worldchg-pin-refresh-final`: PASS, 134.012s.
- `worldchg-pin-check-final`: PASS, 169.874s.
- `worldchg-content-integration`: PASS, 36.863s.
- `worldchg-content-world`: PASS, 86.877s.
- `worldchg-owner-pg`: PASS, 98.752s.
- `worldchg-dispatch-pg-focused`: PASS, 268.137s.

Content integration passed 123 tests (3.12s actual tests); world repository passed five (84.09s). All six owner PostgreSQL cases executed and passed (14.94s actual tests). The dispatch filter passed fourteen tests (193.16s actual tests), including thirteen native PostgreSQL cases with WORLDCHG-Q/RAIDS_GLOBAL execution markers. Case markers include Thawing51.776–71.725s, Voice54.880/68.648s, Grimvale45.743/57.326s, Hive66.131s, Benevola96.635s, Yasir102.670s and Yeo81.307s. The queued duplicate Cargo invocation was cancelled before any case executed; the actual run used six workers. No full PostgreSQL suite ran. The optional broader daily-map module was not run.

The fresh pin-refresh and pin-check bind digest `7d31c6eb0395cafda888178a0d5e204bad469be754430172a2f396c5e330daaa` (32,839,521 bytes). All 93 quest-authoring modules passed. Rust checks cover all targets/features of the eleven changed crates; Rust source remained unchanged after them. No PR is opened. WVAR follows next; its completed Fury Gates qualification must be composed with main's existing native layer/raid path.

## Coordinator migration registry — WPKG (2026-11-07)

WPKG owns 1330–1349. The five incoming sources contain 29 package-owned migration files, so related unmerged SQL is consolidated in original order; no accepted main migration is renumbered or edited. SQL statements are preserved. QENGINE2 PR3020 is still open at `efad3dee13e9cbc9652296726bf15828769c69d4`; its 0999–1006 content must remain canonical dependencies, never package variants. Shared QBASE files, if present, use the coordinator's 1200–1209 numbers and unchanged shared SQL.

Current WORLDCHG migration files:

- `apps/game-server/migrations/1330_world_change_owner.sql`
- `apps/game-server/migrations/1331_world_change_credit_counters.sql`
- `apps/game-server/migrations/1332_world_change_voice_support.sql`
- `apps/game-server/migrations/1333_world_change_barter_custody.sql`

1330 retains the World owner; 1331 retains credited counters. 1332 concatenates the gobbler cooldown and Voice phase migrations; 1333 concatenates Benevola custody and Yeo barter. Every statement remains in its original lane order. Exact filename references were updated. The repository duplicate-version check on the staged tree returned no duplicates. Focused PostgreSQL checks and regenerated pins follow this compiling checkpoint. Future lane migration groups will continue in ascending order inside this same package range.

Registry revalidation completed on the 1330–1333 allocation: all-target/all-feature
cargo check passed175.240s; all six focused owner PostgreSQL cases executed and
passed (12.37s test time,157.583s command). Tool-driven pin-refresh136.989s and
pin-check152.670s passed; digest remains7d31c6eb0395cafda888178a0d5e204bad469be754430172a2f396c5e330daaa.
The repository duplicate-migration check on HEAD returned no duplicate versions.

### WVAR compiling checkpoint: reset ownership and source materialization

The merged server/dispatch all-target/all-feature check passed (247.817s).
WORLD fixtures now explicitly commit only their current intended reset through
`record_reset_fixture`; future histories and boot/lazy reads remain inactive.
The main Furyosa regression chooses an actual Thais reset and retains stale,
exit-only, undated, firing and replay assertions. WVAR corpse import accepts
unadmitted legacy records without semantics while rejecting admitted conflicts
(five tests,0.526s). NAV keeps sealed pre-admission evidence (seven tests,24.406s).
Shared shop/potion input repinning permits only exact verified WVAR source
qualifier outputs from main9668; all other selected changes remain errors.
Generated content and focused PostgreSQL validation remain pending here.


## Integrated WVAR source contract notes

## WVAR spawn prerequisites

A gated spawn source opens after one sealed, committed death from each exact prerequisite home cell and creature identity. Repeated kills of one source, administrative removal, failed projection, and replay cannot replace another prerequisite. The existing spawn owner handles warning, occupancy, respawn, and activation rollback; no second scheduler is introduced. Prerequisites are local to the admitted Channel spawn set.

## WVAR variant projection and conservative source rules

Variant-only sectors use the same sorted tile stream and portal/marker binding as base sectors. A missing managed portal anchor refuses admission. Only bundles declaring the W1 capability receive these projections; exits remain available after closure through current authority checks.

Missing exact entrance geometry is represented by bounded, declared landing pads. Awash uses a separate dry return pad so a flooded entrance cannot strand a player; Down the Drain reserves dry entrance/return pads over the sourced surface water. Awash's202coal threshold is interpreted as the community total across three pumps. The exact source citations and per-title geometry assumptions are retained in the recipe's source/assumption fields.

## WVAR native creature and Item admission

Mamma Longlegs uses the retained1800HP/2200XP/outfit208 facts and the existing Giant Spider executable AI as the conservative archetype. Its new native profile invents no loot, Bestiary, Prey or Forge reward. The bounded corpse Item dependencies are admitted through the existing authoring importer. Sandfish13992 keeps its existing canonical weight660, stack100 and food regeneration660seconds; only the missing native provider binding is added. Source locators and hashes are recorded beside those native facts.

## WVAR camp, Spider Nest and illusion rules

Spider Nest entry arrives beside Mamma's live spawn cell and returns to a separate maintained surface pad. Decorative web12747 has no retained solid flag; the selected sealed variant therefore declares a bounded barrier through the existing map collision/passability owner. Three lifetime sealed credited Mamma kills deny new entry while the return remains available.

Bored uses three conservative adjacent illusion pads around Wyda because the retained source fixes the exact illusion identity/top-damage achievement but not an executable population count. The NPC home cell remains free. The existing seven-spawn Warpath recipe means six distinct guard homes before Bibby, using the already-pushed spawn prerequisite owner. Lumberjack's source50gold offer is interpreted as the player selling one piece of5901wood, independently of the Chip→Queen report.

## WVAR Lumberjack NPC contract

Retained TibiaWiki page76247/rev893906, `npc_conditions`:9–13 (body SHA256 `a31cf523648ff168e34f12a31429a05d8db42be4e7a0daa6df49dc6e32355ce2`) supplies the Chip→Queen Eloise order, Whistle-Blower, and a50gold wood sale with unspecified quantity. Wood5901 is Crystal `201189f871d23597b82caf88fd611454a2ddefb8` `data/items/items.xml:15919`, file SHA256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`.

The explicit conservative offer is that the player sells ONE wood5901 to Chip for50gold through the existing NPC trade owner. This is independent of the report and does not make50gold a quest reward. Conservative dialogue keywords are `wood`/`logging` at Chip and `chip`/`lumberjack` at Queen Eloise; replies are authored text, not purported donor quotations. Reporting requires the prior visit and grants Whistle-Blower once. It remains available after the daily change ends. Chip's authored home `[32418,31699,7]` is an explicitly inferred placement, checked against the current stored variant; wandering changes talk range, not source membership. New writes require a private context resolved from the real current NPC actor, session, lease, scope generation and active content. Existing matching Quest/trade receipts retain ordinary read-only replay after closure.

## Warpath repeat admission

The retained Warpath spoiler fixes24hours and restarts the clock on an early revisit. Entry therefore records an actual server movement-step Quest receipt: elapsed admission uses SetNow, while an early denied attempt uses a separate SetNow transition. A bounded palisade enclosure and separate return platform are conservative geometry for the retained Femor camp. The source inside roster remains two Berserkers, three Warlords and one Leader; only six distinct sealed guard deaths unlock Bibby through the existing spawn owner. Return never consumes or checks the entry clock.

## WVAR source-specific reward closure

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` fixes Wyda loot at `data-crystal/monster/raids/giant_spider_wyda.lua:67-70` (SHA256 `d64925aa3300e7bb6c8ec8abcfd815500624d344e4e2b411f8559624108993d6`): gold3031 and spider fangs8031 only. Bibby uses `data-crystal/monster/bosses/bibby_bloodbath.lua:72-90` (SHA256 `62bc99a21a214baaf6e1b0a8dba5ee595674c7baf15b0b3f952925ea3b58e5a2`), its17 genuine leaves; modern imported44612 is absent. These two source-specific tables replace the imported tables without promoting ordinary Giant Spider loot. Only six missing genuine leaves8031/7412/7395/817/3316/3265 become Native materializable Items.

The same pinned `data/items/items.xml` (SHA256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`) supplies Bibby corpse6008 at16475–16480 and its4148→4149→4147 chain at12738–12755: capacities10 on the first three, durations10/300/300/60seconds, and weights0/120000/120000/10000. Pinned appearances preserve the primary corpse immobility and the later stages being takeable. Existing Item decay/custody owns the chain; no new scheduler. The existing independent Crystal/Canary imbuement cross-check gains only butcher axe7412, whose two slots and five family tiers match both exact source pins; no existing witness row changes.

The six leaf definitions in that same exact XML are8031:21688–21692,7412:19911–19930,7395:19688–19691,817:2728–2740,3316:9014–9024, and3265:8282–8295. The canonical Item source facts feed the native loot admission for Bibby corpse6008 (materializable, capacity10); later corpse stages retain the existing Item decay owner.

## WVAR generic physical reward closure

`tools/content-schema/native-gameplay/wvar-reward-closure.json` is the reusable source receipt for all42 WVAR recipe/source creature keys:41 retained donor Lua definitions plus the explicitly authored Mamma template. It derives133 corpse-chain Items and337 actual corpse/loot dependencies from the exact Crystal201189f archive SHA256 `2b734db10c8c00e7a11a59f1cb49a786099959e98a622732b7fb80bcc8345b9d`. Every creature has its Lua file:line+SHA; every Item has its XML file:line+SHA and span digest. No raw source or map bytes are copied into the repository. The bounded supplemental keys are the four source Shadowthorn Elf types and the prepared-bucket Thornfire Wolf.

The generic reward projection admits physical/stack/container/absolute decay facts and supported loot charges. Equipment, combat, imbuement and use-effect systems retain their existing owners; this reward importer does not fill their unknown fields. Existing canonical Known facts remain authoritative. Actual source primary-corpse/loot edges alone enter the loot section; decay-only stages and future bucket operands do not become unrelated loot facts.

Our earlier16-slot Mamma/Giant Spider projection was a native projection, not the donor capacity. It is corrected to the source30/30/20 capacities. The existing shared corpse-container owner now supports the exact required maximum44, rather than clamping larger bodies: Hive-related bodies and Deepling Scout36, Spectre/Poacher40, and Insectoid Worker44. Migration1102 retains the installed receipt/lifecycle/nested/single-location guards and extends their shared bound, including retirement ordinals. The independent total loot-plan ceiling remains unchanged.

Canonical guard receipts are regenerated through their existing source capture functions; their captured record sets remain unchanged. The existing Prey producer is rerun because its model binds the exact loot bytes. There is no second counter, store, admission table or timer in this source closure. The shared dependency closure does not qualify any additional world-change title by itself.

## WVAR iceberg, Spirit Grounds, Hive and Poacher rules

Chakoya's 63 retained donor homes use the 2,118 bounded snow/ice tiles recorded in the authored recipe. Equal iceberg/open-water alternatives are conservative: open water removes owned upper-floor terrain and changes floor 7 to appearance 622. The retained [Chakoya page](https://tibia.fandom.com/index.php?curid=76213&oldid=887139), line 4 (SHA256 `16abb4b72959e6969dc11e24717b7cbf703df6d7cc9ed4a8935686a4ff6a2e68`), declares no event reward. Its line 6 (SHA256 `ff4d57a94bdf28759e74d23b6806a02e949c1c2c78983d805a24fd0804d4671e`) distinguishes Cartography 101 and incidental Fireproof Horn loot.

Spirit Grounds retains all three entrance coordinates and the shared destination. Twelve equal variants are the Cartesian product of three entrances and four populations. This preserves the retained [Spirit Grounds page](https://tibia.fandom.com/index.php?curid=76264&oldid=887160), line 17 (SHA256 `59b829915a0b21c494521fb1a351335f44708cadf75709a3dd009ead86a33940`): population and daily portal choice are independent; the objective and achievement are unknown. Four creature pads, a connected 49-cell cleared room floor and an adjacent return selector are conservative geometry. Separate maintained dry pads beside each surface entry prevent landing on the active entry. These escape surfaces persist without enabling a closed entry. No objective or achievement is invented.

Hive Outpost uses six Vandura source pads conservatively assigned to the six hive-born types. Daily 500-permille admission and 49:1 ordinary/Overseer weights are explicit assumptions. The optional Overseer occupies a seventh adjacent clear pad and preserves Kollos. The retained [Hive Outpost page](https://tibia.fandom.com/index.php?curid=76241&oldid=887146), line 6 (SHA256 `8abed8a9797761bfd4b722eb5b7b3cd547eb00ee93d028a54cc845f3ac0fc4ca`), establishes Vandura and the invasion, while leaving the reward quantity unknown. Exact donor homes are `world-monster.xml:51169,52098-52102` at Crystal commit `201189f871d23597b82caf88fd611454a2ddefb8`, SHA256 `f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e`.

Poacher Caves uses the same 12 Gloom/Ghost Wolf donor homes and 60-second source intervals for mutually exclusive Game/Poacher/Gloomy populations. The retained [Poacher Caves page](https://tibia.fandom.com/index.php?curid=76258&oldid=1175805), lines 17-18 (line SHA256 `5f657dd641977d669f56568d3f13ce11480418696a162e6817a8b9ff6ebaf0ea` and `86aede34bd487d35f366c6acd47a369d707214110e68b4b2ac083a153f359b3b`), describes the populations but supplies no executable threshold/order. Equal daily selection is the conservative rule. Exact homes are `world-monster.xml:63222,63329,63776,63891,63921,64194,64310,64313,90293,90811,90976,91588` at the same pinned Crystal commit and XML SHA256 above. No kill threshold is invented.

All decisions use the existing World/reset owner. All 63/4/6+1/12 native creature memberships use already-qualified native policies; no additional Creature or Item profile is introduced. Recipe source fields retain the full donor URLs, exact XML home lines and map SHA256 `c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85`.

## WVAR Thornfire reset and water stations

Thornfire uses the existing World/reset owner and exact slot15 recipes. The restored phase retains54 pinned Shadowthorn Elf homes on688 admitted tiles, including three Overseers; the burning phase retains24 Firestarters on634 tiles. The explicit conservative phase rule requires one sealed native death from every exact restored home and all three Overseer-release bits. Existing next-save admission maps counter stage1 to restored variant1 and stage2 to burning variant0; one burning save epoch then restores the normal roster. The phase has no random selection or boot activation. Individual repeated kills cannot substitute for another home.

Prepared bucket source facts remain bound to Crystal201189f `prepared_bucket.lua:3-83` (SHA256 `5e0d0253f913cb788a7fcbd6be41ad465b56ab1026c68226f80f79c31cefe01f`) and `thornfire_wolf.lua:3-18` (SHA256 `010270cc7a3ad1ecdd960b645d1013bc61f24feace4df38092295b91aca2b7cf`). Mixing11466 with2873 yields empty bucket12819; using bog water12721 fills it to12722. Each fire use transforms the filled state back to empty; filled water decays after one hour. Existing terrain restoration uses the first future save+8h or save+16h boundary, then the next reset after16h. Every extinguish increments the personal count; Firefighter grants only on the10% two-wolf branch when the count is at least500 (`prepared_bucket.lua:44-59`, same source SHA256 above); no guaranteed mount or unrelated event reward is invented. The retained Lua samples four nearby pads with replacement and permits `createMonster` failure. The explicit conservative placement rule requires two distinct currently available source pads atomically, otherwise the whole use refuses before debit; it does not claim exact donor sampling. Current Character/session/generation/lease, exact selected action membership, atomic Item custody, durable replay and native spawn/death receipts remain with their existing owners.


The final Thornfire admission extends the earlier42-key closure through the same producer to the existing M10 bog-derived Crystal Wolf:43 actor keys,42 retained Creature Lua definitions,137 corpse-chain Items and344 dependencies. Crystal `201189f871d23597b82caf88fd611454a2ddefb8` `data-crystal/monster/mammals/crystal_wolf.lua:32,78-85`, SHA256 `55815115e6b393f6e7cab39a69ee0c59fc0c87acf59dcf9ce1e0e33e4d6e7061`, supplies its six exact loot leaves and12058→12069→12070→12071 corpse chain (capacities10/10/10/leaf); the unrelated modern10310 loot edge is removed. The two bucket profiles remain physical/use operands rather than unrelated loot. Existing craft authoring admits nine operand profiles and130 recipes. Native Item payload is3728 rows/8,107,377 bytes; the official Prey producer was rerun against the final loot bytes (15 cases/776 creatures). Historical42-key PG receipts remain evidence for their recorded payload, and do not substitute for the separate final43-key Native admission proof.

### WVAR expanded compile gate

All15 changed crates passed all-target/all-feature `cargo check` (116.181s).
Two main test fixtures now provide WVAR's new optional bundle/variant fields;
their existing layer and fungus assertions are retained. Prey completed15 cases
for776 creatures against the merged source loot. Clippy, pins and PostgreSQL
remain pending at this compiling checkpoint.

### WVAR regenerated compiling checkpoint

All15 changed crates pass Clippy all targets/features with `-D warnings`
(285.315s). Native registration freshness passes84.339s. Quest-authoring ran
all93 modules in509.049s:92 passed; the sole stale Grizzly hash was then
regenerated and its test passed3.909s. Native/Prey, dependent shop/potion and
static imbuement evidence were regenerated with repository tooling. The new
bounded evidence refresh proves all34,111 canonical Items equal pinned main
plus verified WVAR admissions before invoking existing evidence producers;
257 imbuement tests pass84.997s in the prepared Python environment. Pin refresh
passes249.826s. Pin check and native PostgreSQL execution are still pending;
this checkpoint does not claim WVAR qualified in the package yet.


### WVAR reverted within its packaging window

Reverted merge7d24f6cb4 and its follow-ups832e65001,91f7fbf1f,892cef29d,
982d52b00. The runtime, authored/generated content, pins and migrations now
exactly match the previously green WORLDCHG registry checkpoint796d7b973;
only unioned historical notes/reports differ. WVAR's1334/1335 migrations are
absent. Source contract/report entries are retained as historical evidence.

WVAR could not complete focused qualification in60 minutes. Final red point:
the pin-check process was killed after139.659s and the dispatch PostgreSQL
binary build after279.814s when overlapping memory exceeded the16GiB limit
(cgroup oom_kill increased by2). No WVAR PostgreSQL case executed; no passing
pin check or WVAR qualification is claimed. Future heavy builds and pin
materialization must run serially. Its compile/Clippy, source producer,
Prey simulation, quest-authoring recovery and pin-refresh evidence remain
recorded above for recovery from the pushed history. Continue with WCONTRIB.

Rollback validation: wvar-revert-check PASS 117.904s; wvar-revert-fmt PASS 15.026s.


## Integrated WCONTRIB source contract notes

# WCONTRIB decisions

Horse Station follows the owner's three-hour-or-zero-horses activation rule and three-hour raid expiry. A late observation preserves the original deadline; observing after expiry restores the stable immediately.

This rule intentionally differs from the retained rental-triggered 24-hour raid: Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d `data-global/npc/palomino.lua:104-112,148-160`, SHA256 `595387918194d62554ba02e66c381e945fc065a0c0b3b2658ce773fe52f293da`. Native initial stables contain nine horses: `data-global/scripts/globalevents/spawn/horse_station_world_change.lua:1-9,18-22,39-43`, SHA256 `631b28ccc0120d2f8d9d74c1f9540b40989065b6785d7d4331f495389c92e07b`.

Dog Collar issuance uses an atomic gift: eligibility, earning and native backpack MINT commit together. A full backpack refuses without earning or setting the once flag. A successful collar remains in custody and exact replays remain valid after the disappearance ends; there is no unearned pending grant.

World live changes insert revision one once and advance existing rows with guarded UPDATEs. PostgreSQL executes BEFORE INSERT guards before an UPSERT conflict arm, so UPSERT is deliberately unsuitable for the immutable revision chain.

Steamship requires 200 shared Coal and opens only at the next save; each retiring epoch funds its following saved day. Junkar's retained destinations and prices remain the travel authority.

Swamp Fever uses an owner-authorized conservative threshold of 100 medicine pouches12517 to cure/prevent the following saved day's epidemic. The retained donor establishes the medicine and individual Ottokar exchange, but not a shared numeric cure threshold. The individual exchange remains separate.

Twisted Waters uses 1,000 corpse deposits per retiring epoch, advancing one pollution stage when met and recovering one stage otherwise. Yawno and explicit dirty-water use accept the final portable aged Swamp Troll corpse4178. The selected native lake water component is projected as clean/dirty/stinking; only a successful Shimmer catch starts the sourced personal 20-hour restriction. These deposit/recovery and lake appearance choices are conservative authored rules.

Rathleton uses twelve fixed ballot identities and one earned right per explicit cast. Unknown alternate-Monday parity is anchored to Monday2000-01-03UTC; the active epoch's save timestamp owns the period. Ties retain incumbents. Native missing entrance selectors are replaced by explicit Marvin passages to sourced dungeon/first-pipe destinations, checked against current promoted rank and elected dungeon inside the existing travel transaction. Rank admission also uses native Alaistar/Flint trades and citizen boat routes. Sources and exact hashes are recorded in the owned WORLDCHG-Q rows.

Horse returns restore one shared horse, bounded at nine; restoring all nine ends a raid early. Reduced stable cohorts use the first N members of the sourced nine-horse layout. Existing native raid scheduling, Channel target admission, actor cleanup and generation guards own every cohort.

Noodles uses the accepted92-permille saved-day disappearance rule and thirteen documented Thais roaming anchors. An escort belongs to one Character across Channels, preserves its collar, and can return only beside the actual King. The personal repeat interval is fourteen days. A return guarantees Dog Lady on the next saved day; the next reset releases any unfinished escort and selects the new day's disappearance. Dog Lady uses her existing dog presentation32, never an invented human appearance.

The shared NPC reward/access engine is reused from the quest-engine lane at5a6de28c142de185594217ce1e5cbc7f4b6e6e4e. Its unrelated binary map restoration is excluded. Required WCONTRIB NPCs are bound through the existing retained-source placement converter; missing legacy travel-service declarations use the native route catalogue and greeting adapter.

King `noodles` dialogue and collar use share the same World RETURN action. Spatial commits and receipt observation bind the original player/NPC context; ambiguous outcomes retain that context instead of reconstructing it after either actor moves.

Ottokar's personal exchange remains distinct from epidemic donations: one medicine pouch (12517) yields one belongings item (12413). Repeated custody-backed gifts use exact per-command claim identities; an exact replay never consumes or awards again. The hundredth personal delivery grants Doctor! Doctor! once; the exchange count can keep growing while the donor achievement progress itself saturates. Retained Crystal9f5a72c ottokar.lua:61-77 SHA256fe8c4a6740a75448c3af035f27c3b2783b9fe91ecad7ba4d255544003370a951; data/scripts/lib/register_achievements.lua:696-717 SHA25624ca189087f6403ab994720763b69ee04eab0397ff917ff7700edb9063d6de37. The belongings profile retains items.xml:30161-30163 weight3000, with the existing source XML hash cited in WORLDCHG-Q.

Alaistar's Commoner shop uses the retained glob-of-glooth21182 sale for125 gold: Crystal9f5a72c data-global/npc/alaistar.lua:48 SHA256632237084e0382dd959172a0f7d737ed747fad710a2cea9db51b7f849bd8c36c. The forward content repair restores the offer removed by the earlier unqualified-access filter; the current promoted-rank predicate now gates its real transaction.


### WCONTRIB compiling merge checkpoint

Sourcec8a8d97cf is integrated with WORLD behavior retained. Core server, dispatch
and World-change libraries check all features in30.531s. World-change
unit/component tests pass; twelve quest/cosmetic producer tests pass5.634s.
Main's14 outfit definitions, palette, fallbacks and8 mounts remain authoritative;
49 new physical looks produce63 unique outfit pairs. Legacy grants and new
canonical quest grants resolve to the same retained identities. Existing WORLD
callbacks receive empty new effect/projection fields. Skinning, current native
teleports, dynamic quest floor masks, main static layers and NPC dialogue digests
are combined with the new shared state, fishing and Quest participants.

Own migrations1336–1339 consolidate source1110–1115 in original SQL order.
0999 is the exact canonical QENGINE2 dependency at aad7ff6dd, with no own variant;
1000–1006 are not needed by this lane and are not imported. If3020 merges into
main later, consume main and compose the newer owner proofs then. Source notes
are unioned only here; no root CONTRACT-NOTES.md remains.

Full legacy/native regeneration, final pins and PostgreSQL qualification remain
pending. No package QUALIFIED claim is made at this compiling WIP checkpoint.


### WCONTRIB legacy and compatibility checks

Full legacy materialization and Rust inventory repinning pass567.363s; NPC
placement regeneration/check and converter tests pass0.147/0.151/0.266s.
All12 changed crates pass all-target/all-feature check51.015s. The complete
World owner test module now passes10 cases, including positive/negative main
static Fury requirements. Furyosa keeps main's independently admitted, persisted
static layer guard; full WVAR remains reverted. Node epoch, exit-only, undated
and active-layer checks are retained, and WORLD's negative fixture chooses
truly inactive Fury days.25 cosmetic compiler tests pass2.99s. Main's49
original item/base mappings and14 outfit records remain intact; additional
Quest mappings have the closed QENGINE provenance.

Native registration now orders WCONTRIB after the accepted ITEMHOLD and QENGINE
owners. Only exact matching custody/binding/admission facts may reuse14156 or
21182; their existing presentation, semantics and provenance are preserved.
New native/model outputs, final pins, Clippy and real PostgreSQL remain pending.

### WCONTRIB producer encoding and lint checkpoint

Clippy all12 changed crates/all targets/all features passes253.594s with
`-D warnings`. The Quest reward producer now uses the shared native manifest
encoder, keeping its output byte-identical to ordinary native regeneration.
Its12 tests pass5.827s and all25 cosmetic compiler tests pass2.879s. This also
retains main's bounded external cosmetics descriptor instead of embedding the
large source shards. Final native model, pins and PG remain pending.


### WCONTRIB generated content and pin checkpoint

- wcontrib-tree-regenerate-v3: PASS 717.331s
- wcontrib-grizzly-regenerate-final: PASS 4.009s
- wcontrib-fmt-final: PASS 14.912s
- wcontrib-pin-refresh: PASS 164.928s
- wcontrib-pin-check: PASS 177.482s

Native Item union:3612 existing records remain byte-semantically unchanged;
24 additions, zero removals, zero duplicates; total3636. This includes exact
preservation of the14156/21182 shared owners. Pin-check World digest is
`b096b258e9351ca8ea571e3e2a73a34d4883ddda590b6ea67c212f52c9cb4aeb`.
Real PostgreSQL execution and the final content tests remain pending here.


### WCONTRIB reverted: focused gameplay qualification exceeded60 minutes

Revert merge459652e8e and follow-upsff27026a1,d84d38315,ee31f0b58.
All contribution/Quest additions and own1336–1339 migrations are removed;
source reports/contracts and historical validation remain as evidence only.
The16 selected owner tests failed during schema setup in330.445s command /
52.06s test time: canonical0999 wrapped the old carried-USE arm, causing1333's
literal anchor to reject it. No gameplay case qualified and no native
WCONTRIB PG case executed. The corrected1333 composition preserves both
independent proof arms and their outer current-transaction predicate. A real
PostgreSQL18.6 replay of all325 composed migrations passes4.159s actual /
4.255s command with both proof arms present; embedded compilation also passes.
There is insufficient time to rebuild and qualify all gameplay cases within
the lane window, so the requested revert fallback applies.

Retain only independent main compatibility repairs: persisted static Fury
eligibility and its guarded fixtures/unit test, and the1333 migration-composition
repair for upcoming canonical QENGINE dependencies. Other runtime/content/pins
restore the prior green World package. Continue with WEVENTS.


Rollback validation after retaining independent fixes:
- wcontrib-revert-check: PASS 120.128s.
- wcontrib-revert-fmt: PASS 14.684s.
- wcontrib-revert-fury-unit-final: PASS 0.849s.
- wcontrib-revert-schema-pg-final: PASS 4.399s.
The first rollback unit run caught an accidentally inactive draw in the existing selected-day fixture. Its selected draw is restored; all10 cases now pass. The320-migration base replay requires the existing Benevola/transaction proof; canonical QENGINE is intentionally absent until the next dependency merge.


## Incoming WEVENTS source notes (validation pending)

# WEVENTS contract notes

Owner rule (2026-11-06): publish each tested mechanic slice immediately, at least
once every 30 minutes; bound any individual fact investigation to ten minutes.
Unrecoverable rules use explicit conservative policy rather than deferred work.

- Flower Month follows Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
  `data-global/scripts/raids/world_changes/flower_month.lua:1-65`, SHA256
  `dc1607c195712ed166dcda95423fba91821d0ea1b22b85a249fc9aab8d74b1bb`:
  four areas, 25 Dryads per wave, 250 shared placement attempts, 15-minute
  rotation and indefinite repetition until reset. This supersedes the wiki's
  five-wave account. June admission uses the committed reset snapshot, including
  June 30's overnight tail until the next Berlin 06:00 save.
- Mummy follows the same donor revision,
  `data-global/scripts/actions/worldchanges/the_mummys_curse/horestis_jars.lua:18-122`,
  SHA256 `6e47be40083c0ff174c454940382f891f96d7c304c999457e5165ab98d24751a`.
  The owner explicitly removes the duplicate delayed boss call: one Horestis
  entry per existing World-raid firing target. Executable wall delays are
  277000 ms for 1603 and 1200000 ms for 3514, despite the announcement text.
  Current B3 walls already contain 1603; the event projection manages both wall
  appearances at those four source positions so the opening actually opens.
- Chyllfroest includes May 1: April 1 inclusive through May 2 exclusive, from
  `data-global/scripts/quests/tinder_box_quest_chyllfroest/movements_prohibit_entry.lua:12-21`,
  SHA256 `72a5b54d8759917fd75072692cef0e01e67beb70305b5839d32e1135f2322f84`
  at the same Crystal revision. UTC is the explicit synchronized calendar rule;
  the donor does not pin its process timezone.
- Native startup selectors come from `data-global/startup/tables/item.lua:3087-3126`,
  SHA256 `677bffd2482d16a5c73633e3f7324c4dc16cf1e00fbb5368e56ce4b6d60402fd`
  at that revision. Tooling verifies all sixteen selectors against retained
  bytes and native B3 objects; no map binary rewrite is needed.

Detailed validation and subsequent event decisions belong in
`docs/architecture/reviews/WEVENTS-Q.md` and this task's WORLDCHG-Q rows.

## Spring into Life implementation policy

Recovered donor facts: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data/items/items.xml:33011-33052`, SHA256
`c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb`,
defines dragon nest 14757 and surprise nests 14750/14759.
`data/scripts/actions/items/surprise_bags.lua:6,17-27,31`, SHA256
`66ba74a2b736e0631776ac7674c21a6838ebd5ecd25d9804b7f9613fefe11e1e`,
uses 14759: eleven uniformly selected categories, with quantities uniformly
1–10 for candy and five coloured eggs, and quantity one for chocolate, hydra
egg, blue/red surprise bags and phoenix egg. The existing reward-bag owner
already implements this rule; reuse it rather than add another random grant.

The ten-minute investigation budget found no donor calendar, nest hatch clock
or merchant implementation. Conservative rules selected for implementation:
April 16 server save through April 23 server save, one hour to hatch an admitted
native nest, and one free participation reward per Character per season.
Reuse Rosemarie's existing source-bound merchant population during this window;
paid surprise nests cost one crystal coin each and cannot bypass participation
requirements for the free reward. These calendar/hatch/merchant choices are
Oteryn policy, not claims about donor code. April as the month is independently
corroborated by
https://github.com/s2ward/talesoftibia.com/blob/9bc90ae10ee2f0c6ea21063b4188a402e378e4bf/site/json/events.json .

## Shared lifecycle policy

The Spring map scan found six source dragon-nest objects, of which four are
inside house 3015. Only the two public objects at (32344,31604,6) and
(33977,32646,6) are eligible selectors; house objects remain excluded.

The reusable lifecycle model uses these explicitly authored fallback rules
pending the native bindings and admission proofs: Rise runs September 1 server
save through September 8 server save, with a 24-hour preparation stage, five
distinct incarnations credited within five minutes, a synchronized retry on
failure, then a final boss. These timings are Oteryn policy where retained
source bytes did not supply a complete scheduler. Kingsday continues hourly
waves after a victory during its selected day. Bank robbery checks once per
elapsed hour after reset at 10 per mille, consumes failed checks, and admits at
most one robbery per day. Insectoid restoration uses three shared fires and a
bucket holding at most three units; each successful extinguish spends one
unit atomically. Reset creates a new cycle. These are model rules, not claims
that the six remaining titles have completed native qualification.

## Devovorga's Essence price tiers

The retained recipe identifies Tentacle Piece `oteryn:item.tibia.i11666`,
Anmothra's two-piece offering and the selectable 2/5/8/12/16 tier counts:
`content/quests/definitions/quests-00200-00299.json:1`, SHA256
`422ff275d4b416d2a77acf960bf61fb88ae68b044ca738b45198b1ac9d803a8b`.
It cites the pinned article
https://tibia.fandom.com/index.php?curid=76216&oldid=845419
(body SHA256 `4f0767fb999b622c67eaef1d49a978c8a7502d122504176c51c71b3fd59d033e`)
and spoiler
https://tibia.fandom.com/index.php?curid=76217&oldid=944253
(body SHA256 `0e5d9ea11a467ee2aed7b1baf384992f9f76253729affa594d07973372388cd0`).
The engine's five incarnation slots use those counts and that item. Spring's
independently authored merchant price remains one crystal coin.

## Seasonal expiry and isolated test time

Expired active/dormant cycles transition to failure when the qualified event's
scheduler next runs; a bounded indexed page prevents an unbounded catch-up
transaction. Victory remains final. Current-cycle jar actions do not acquire
locks for older annual cycles. Fixture-only time overrides are scoped to a
Root and its clones, reject production Roots, and never change PostgreSQL time.

Spring public nest selectors are authored IDs 52601/52602, on the two verified
source objects (item 14098, donor items.xml:32647-32650, same SHA above).
Each successful native hatch rearms its public nest for one hour. A stale or
replayed placement ACK cannot rearm it twice. If every target Channel reports
failed placement, retry after an hour without awarding a hatch. The latest
successful hatch time preserves a participant's earned claim when joining
another nest. Superseded persistent hatchlings use the raid runner's LOST
cleanup; they do not accumulate across hourly nest generations. The merchant
reward/purchase tests seed completed-hatch prerequisites separately from the
scheduler/placement and actual-actor cases.

Rosemarie's native conversation reuses her retained greeting, farewell and
walk-away lines, rather than a placeholder: Crystal
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/npc/rosemarie.lua:51-53`, SHA256
`e96bd8db17c9fa763a6e38a7dffa1df0f3e02f09110f4b13b32accfecd58aa84`.
The shared quest-reward greeting registry supplies her conversation; the World
binding still admits or refuses each seasonal purchase and free reward.

Completed raid firing rows remain immutable. Migration 1125 records retirement
separately; current physical-effect reads and timed kill admission reject a
retired firing. Running firings may transition to LOST, while completed firings
retain their terminal state and append normal LOST cleanup effects. This also
prevents a prior successful hatch's retained actors from accumulating when its
next object revision fires.

## Kingsday implementation policy

Royal greeting words and reply follow Crystal
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/npc/king_tibianus.lua:482-491`, SHA256
`e6509e41311669a22f9c7ac6c1787fcf5b63120d06d802a1ad268e0adac9d9bd`:
"hail king" / "salutations king", the loyal-subject greeting and "How rude!"
walk-away. The default farewell follows the retained npc_handler.lua:82,
SHA256 `8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a`.
The retained normalized placement is (32311,32172,6):
`tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json:1`,
SHA256 `1f9b6ebbd79d92fbcb4d6e522c6dc30c95ad83ca6f4b0f5030ee81fb2f47635a`.

The pinned recipe explicitly ties Loyal Subject to respectful attendance,
not arena combat:
https://tibia.fandom.com/index.php?curid=76245&oldid=994341
(body SHA256 `0b67e7e6c7047759d08c7efec385a5d1ab30586518258f185cec9466cfa0cef3`).
A respectful greeting opens the native conversation; asking about the
celebration records attendance and atomically reserves its achievement claim.
The existing achievement definition is `oteryn:achievement/loyal_subject`.

No complete donor celebration scheduler or exhibition population was found
within the investigation budget. The conservative authored policy retains the
model's 24-per-mille daily activation, hourly exhibitions after the first hour,
and two Orc Warlords per exhibition. The Oteryn exhibition ring is the verified
public courtyard (32304..32308,32184..32188,7), all house 0, flags 0 and simple
ground 103, in `content/world/placements/region-z07-x126-y125.b3`, SHA256
`b7f4b09e880cc1637e2d056e058c37e8b2a77b614751333d4b89496a7fdfe037`.
These location/population/chance choices are explicitly Oteryn policy, not
claims that the retained royal dialogue supplies a raid. Previous exhibition
actors retire on the next firing or day reset. Arena waves continue after
attendance victory; a daily claim cannot repeat the account achievement.

## Bank Robbery implementation policy

The conservative native scope is Thais, Naji and Jesse the Wicked. The existing
hourly 10-per-mille draw is consumed once per hour after the first hour; one
successful robbery closes Naji's bank service until recovery or the next save.
This is explicit Oteryn policy where the retained bank dialogue supplies no
complete robbery scheduler. Other cities are not selected implicitly.

Jesse's existing native creature and guaranteed stolen-gold bag loot follow
Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
`data-otservbr-global/monster/bosses/jesse_the_wicked.lua:21-22,77`, SHA256
`08d8ca5871bf4b1bbd3123d940dc26193796bc70dce003f63c21bf0eec2825c2`:
https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/bosses/jesse_the_wicked.lua#L77
Recovery must consume item 13429 with a native MINT cause matching the actual
current-World/current-day robbery kill. Ordinary bags, previous-day bags and
foreign-World bags do not prove recovery. The bag debit, shared reopening and
quest reward are atomic. The pinned quest reward is ten platinum coins and
Honest Finder, per https://tibia.fandom.com/index.php?curid=76139&oldid=1082154
(body SHA256 `a865de3250f6abd2e432be0b265a88cb14e6bc202b07661af28f2f9cd35d5cc1`).

Naji's retained normalized placement is (32342,32229,7), in
`tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json:1`,
SHA256 `1f9b6ebbd79d92fbcb4d6e522c6dc30c95ad83ca6f4b0f5030ee81fb2f47635a`.
The authored escape alley (32336..32339,32233..32235,7) has simple ground 506,
house 0 and flags 0 in region-z07-x126-y125.b3, SHA256
`b7f4b09e880cc1637e2d056e058c37e8b2a77b614751333d4b89496a7fdfe037`.
The position is an explicit Oteryn fallback rather than an inferred donor raid.

The historical canonical bag weight is 5000, but retained Crystal
`data/items/items.xml:31463-31466` @ `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
SHA256 `c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb`,
and pinned Canary XML:31281-31284 both explicitly declare **500**. Per the owner
donor-wins rule, the active native Item profile uses 500. The historical
canonical import remains intact; the source override is explicit in the
generated qengine reward profile evidence. No other known physical fact is
overridden. Naji's registered `parseBank` callback is retained Crystal
`data-global/npc/naji.lua:61-80`, SHA256
`7752587c14e87d611edc01ee3723afb75fa08c2f29aded246ad458353ed7c1ac`.
A published event-owned bank service admits this existing native Bank owner;
profession labels and arbitrary donor callback text alone still grant no service.

Jesse's corpse chain also needed native admission. Retained Crystal
`data/items/items.xml:37349-37370` @ `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
SHA256 `c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb`,
agrees with pinned Canary XML:37154-37176: 18153 (20 slots, 10 s) → 18150
(20 slots, 300 s) → 18151 (10 slots, 300 s) → 18152 (60 s) → removal.
All four native profiles are Ground-only and reuse the existing durable
absolute-deadline/decay owner. Their legacy reference-only identities remain
unchanged. Native loot admission now derives the corpse capacities from those
exact profiles, alongside the stolen-gold bag's portable admission.

### WEVENTS: Greenshore restoration policy

Insectoid Invasion uses the existing 100-permille daily World selection. Its two
Scouts are reserved from ordinary activation and projected through the timed raid
owner at their retained positions (32208,32058,7), (32209,32060,7).
Evidence: `content/world/spawns/spawns-10000-11999.json:1`, SHA256
`a74b7da322bba5ebea20c772dae368d1337ec4caf9cd146f89b1bb9b8419b051`;
the source respawn interval is 90 seconds. Oteryn policy replaces the preceding
population at each synchronized 90-second slot, bounding the event to two Scouts.

No complete donor restoration script was retained. The conservative authored
rule is one shared bucket with capacity three and three shared fires: filling
adds one water unit, extinguishing consumes one, and extinguishing all three
ends the invasion. Full buckets and repeated/completed actions are no-ops.
The four native ground USE selectors and decorative, nonpickupable projections
are explicitly authored in `content/quests/world-event-object-policy.json`.
The selected public tiles are verified against
`content/world/placements/region-z07-x125-y125.b3:1`, SHA256
`268c6b75fad682c6d9baeadc6563c2cace27d8414caa66ca538b49687a6dc262`.
Victory removes the event population and projections; reset starts fresh state.
These restoration details are Oteryn policy, not claimed donor behavior.

### Insectoid Invasion implementation policy

The Greenshore shared restoration rule, native coordinates and source hashes are
specified above under [WEVENTS: Greenshore restoration policy](#wevents-greenshore-restoration-policy).

### Devovorga world event policy

Retained Crystal `data-global/npc/a_beautiful_girl.lua:1-58` at
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, SHA256
`2a4dd3dc8afca0ee526d223b827d773b2974ffd0aeb198ce5da7a0b583f33e61`,
contains NPC setup and a greeting, with no calendar, encounter fee or reward
implementation. The retained corpus and inspected public tree contain no complete
Devovorga World controller. The following calendar, mutation and location choices
are explicit conservative Oteryn rules, rather than claimed donor script facts.

Rise retains the existing September 1 Berlin 06:00 through September 8 Berlin
06:00 window and one-day preparation. Five distinct incarnations must fall within
five minutes of the first credited defeat. Bloom of Doom and Spawn of Devovorga
then precede a second synchronized incarnation round, Vulnerable Cocoon, and final
Devovorga. Expired incarnation attempts restart together; seasonal expiry fails
an unfinished event and preserves a completed victory. Daily reset readmits
uncredited members. Global stage completion retires superseded projections.

The authored public Vengoth clearing and separate paid encounter room are verified
against `content/world/placements/region-z07-x128-y123.b3:1`, SHA256
`865ee3aa9e0c4d592571b40b72c137a8b05a19f850e0b570974276ebbeb9a22d`.
The existing 16-permille daily Essence selection gates five entrances. Admission
consumes tentacle pieces 11666 in tiers 2/5/8/12/16 for
Anmothra/Chikhaton/Irahsae/Phrodomo/Teneshpar. Native encounters first bind the admitted generation's complete creature policy
table, including in a Channel with no previous creature spawn. Native encounter
preflight and a bounded room lease precede the fenced debit. Active calls are serialized against
other native room admissions. These authored USE entrances admit level-1 and
higher characters without an upper level bracket; this is an explicit Oteryn
admission choice in the absence of a complete donor encounter controller. The source recipe and pinned wiki citations for
these tiers remain in `content/quests/definitions/quests-00200-00299.json:1`.

Normalized actor facts and exact public source URLs/file lines/SHA256 are in
`content/quests/world-event-creature-policy.json`. Public Fencore source is
`fifth-empearl/fencore` @ `00484ac96d3379fd7f18a3a410e53c2ae31fc981`:
`data-otservbr-global/monster/devovorga.lua:5-25,83-90` SHA256
`32f3e3a4854a0e30c223bc58beb1a1d307128afcae17267e32bcf815ff3c1361`,
`phrodomo.lua:5-25,77-86` SHA256
`78ed2e396f95cb2d516f453bdf7d5b205f24c5a174355140e6eaaf3066481830`,
and `teneshpar.lua:5-24,83-92` SHA256
`d1faede5a81ca84fc06f0c3f6d37e61d8e0bc8d05bfe5d191ed8f61edc012b07`.
Anmothra and Irahsae literal health/default defensive facts come from
`soul4soul/converted-tibiawiki-monster-data` @
`a2cccf99fccc547e13720704f99cd0125f22f82f`,
`tfs revscriptsys/Anmothra.lua:6-22,87-88` SHA256
`f2fbfc4e6bd51d5c338d96abfd9c1a9bb24f7ea8a329ed79fe71f2a8f0f94711`
and `Irahsae.lua:6-22,87-88` SHA256
`a0ef65c05bf48aa5ead6f0fb8bd451e42ff2e4fad482d53f924ddac4867b2160`.
Their zero outfit IDs are incomplete conversion fields: the authored fallback
uses Chikhaton's presentation and Cave Rat physical attack. Mutation actors use
1000 HP, with Bloom/Cocoon passive. Other physical attacks use donor ranges or the
existing native nonlethal attack/skill approximation. These actors claim primary
physical combat, not complete donor spell parity. Null loot bindings explicitly
admit no ordinary corpse/loot/XP; fenced event rewards remain separate.

Rise's chosen level-160 reward is one Demon Backpack 9601 containing five
Tentacle Pieces, three blue giant shimmering pearls 281 and five Gold Ingots
9058, plus I Did My Part. This retains the selected high-level source package;
the spoiler's blue pearls win its base-page green-pearl conflict. Admission is
once per character/season, requires global victory and a native final-boss defeat
receipt, and uses the existing container MINT and account achievement owners.


The existing NPC reward proof in migration 0999 admitted only `command`, despite
its Rust owner accepting native selector rewards. Migration 1127 uses the same
closed `command`/`use`/`step` set without relaxing same-transaction receipt,
consumption, MINT or outfit evidence. Native reward USE remains a fenced Quest
and World transaction; it does not bypass the reward owner.


## WEVENTS packaging checkpoint — validation pending

Source 033791043 is being merged after the WVAR and WCONTRIB reverts. The retained WORLDCHG qualification set remains intact (12 titles); WEVENTS adds nine titles. Canonical QENGINE 0999 is consumed unchanged. Lane migrations are 1340–1344, with related source statements consolidated in original order; duplicate 1130 is omitted in favor of identical 1331. Real PostgreSQL 18.6 replay executed all 326 migrations in 4.211 seconds, preserving callback proof arms. Main cosmetics identities, palette, mounts and grants are unioned through producer tooling. Rust, pins, focused gameplay PG, content integration and authoring verification must complete before this lane is green.


### WEVENTS owner PostgreSQL proof (2026-11-07)

All four focused owner PostgreSQL tests executed and passed on PostgreSQL 18.6 (47.42 seconds test runtime). Actual logged subcase durations: Chyllfroest 14.163s, Flower 6.906s, Mummy 5.081s, Spring 5.669s, Kingsday 5.514s, Bank 4.818s, Insect binding 5.260s; insect restoration 14.279s, timed ownership/rewards/replay/calendar/reset 14.336s, Devovorga native stages 16.816s. The merged libraries previously passed all-feature Cargo check in 119.127s. The 95-module authoring run passed 94 modules, with only stale Grizzly-room hashes failing; regeneration and its focused test passed in 3.887s. Sixteen raid producer tests pass in 22.741s, four WEVENTS producer tests in 6.649s, twenty cosmetics compiler tests in 6.464s, and twelve reward producer tests passed within the authoring run.

This is still a compiling WIP: final native/tree generation, pins, all-target Clippy, native dispatch PG cases, content integration and final composed regressions are pending. No qualification is claimed from compilation alone.


### WEVENTS producer and all-target checkpoint

All changed crates passed all-target/all-feature Clippy with `-D warnings` in 191.696 seconds. Final native refresh passed in 87.972 seconds, and formatting check in 26.033 seconds. All 25 cosmetics compiler/catalogue tests passed in 2.751 seconds while preserving exact main subsets (14 canonical outfits, 49 original mappings, 8 mounts and 133 palette colours). Twelve additional event callbacks now regenerate from explicit source-preserving authoring inputs through qengine_access.py; closed event/action/member/cost/storage/reward validation matches runtime admission. Canonical main outfit keys are accepted by both Rust reward validators; suffix/bound checks remain enforced. Flower retains both repeat flags and shared-area admission.

Still WIP: pin refresh/check, seventeen native WEVENTS PostgreSQL cases, all-target Cargo check and final integration validation are pending.


### WEVENTS regenerated pin checkpoint

- `wevents-pin-refresh`: 249.855 seconds, passed.
- `wevents-pin-check`: 151.101 seconds, passed.
- `wevents-manifest-pin-refresh`: 156.128 seconds, passed.
- `wevents-manifest-pin-check`: 155.963 seconds, passed.
- `wevents-pin-tool-clippy`: 55.5 seconds, passed.
- `wevents-preprod-pin-test`: 0.743 seconds, passed.
- `wevents-fmt-final`: 14.631 seconds, passed.

Bundle `2756ec77e94d02ad062c1cb320c7545e598a39da8bd7e80a2b94342cef1fcf7b` reproduces. The pin tool now refreshes named preproduction fields even when the manifest and pin came from different merge parents; the existing manifest/pin equality test passes. Both fields were regenerated by the tool. Native dispatch PostgreSQL and final integration checks remain pending; this checkpoint is still WIP.

Current WPKG migration files:

- `1330_world_change_owner.sql`
- `1331_world_change_credit_counters.sql`
- `1332_world_change_voice_support.sql`
- `1333_world_change_barter_custody.sql`
- `1340_world_timed_events.sql`
- `1341_world_event_bindings.sql`
- `1342_world_event_native_credits.sql`
- `1343_world_event_window_expiry.sql`
- `1344_world_event_mint_provenance.sql`


### WEVENTS reverted at its 60-minute qualification cutoff

Revert merge3319b7ac6 and follow-ups4b6a0c711,c73e39924,7072c5ff9.
Remove all WEVENTS functionality, canonical dependency0999 and own1340–1344
migrations. Source reports and contracts remain as historical evidence only.
Four owner PostgreSQL cases passed (47.42s actual); all17 native cases executed
and failed (32.30s actual) at the common provisioning gate: combined63-outfit
authoring was702,687bytes, above the existing200,000-byte input bound. A separately bounded inline16MiB
format repaired provisioning without widening the Rust loader. Native nest and
spring USE cases then executed successfully (145.432s and160.592s), but bank,
Kingsday and spring purchases failed with missing callbacks. The source has
four event reward callbacks and three NPC greetings absent from its producer;
regeneration dropped these handwritten additions. Two native cases also retained
numeric1127 migration assertions, now corrected in the attempted rebuild to1344.
Complete focused gameplay qualification was not green within the window. No WEVENTS title is
qualified by the package. Earlier Clippy/pin/producer successes are historical.

Retain only the independent pin-refresh tooling repair: named preproduction
fields regenerate even if manifest and pin were retained from different merge
parents. The existing manifest equality test and strict pin-tool Clippy passed.
Continue with SRV9; regenerate and validate final bundle pins after composition.


## SRV9 merge checkpoint — qualification pending

Source17d0a9f83 merges after the explicit WVAR/WCONTRIB/WEVENTS reverts. Preserve WORLDCHG Yeo admission and source-name templates while adding S10 healing/travel/blessings, fluid subtypes, full charges and reciprocal marriage. The Yeo request explicitly retains None subtype/charge defaults. Migrations1082/1083 are combined in1345;1084,1085,1086 become1346,1347,1348 using git mv and updated filename references. All324 migrations replayed on PostgreSQL18.6 in4.597s actual with the WORLDCHG proof arms preserved. All changed libraries pass all-feature Cargo check; full legacy/native regeneration, pins, strict all-target checks and focused PostgreSQL are pending. No new qualification is claimed from compilation alone.


### SRV9 all-target checkpoint

All changed crates passed strict Clippy for all targets/features in266.089s. The all-feature library check passed185s including queued build time. Format check passed. Legacy regeneration is preserving the main corpse, cheese, consumable and ranged-source custody phases; native/tree generation, pins, focused PostgreSQL and final integration checks remain pending. No tracked qlane-meta/ or .qengine/ bookkeeping remains.


### SRV9 generated-content checkpoint

- `srv9-legacy-generation`: 736.52s, passed.
- `srv9-tree-native`: 510.106s, passed.
- `srv9-native-fresh-check`: 110.699s, passed.
- `srv9-native-precompile`: 398.836s, passed.

The native dispatch test binary compiles; this is not PostgreSQL execution proof. Generated legacy/tree/native files are reproduced by repository tooling, including preserved WORLDCHG and main custody inputs. Pin refresh/check, focused PostgreSQL and final integration validation remain pending.


### Owner standing rule: full dependent-crate qualification before another push

The freshly fetched origin/main is bbb3e06d0 (four new main commits). Compose it locally before the full unfiltered nextest matrix; no push until all failures are collected, grouped, repaired and the complete dependent set reruns green. The first reverse-normal-dependency closure contains22crates (recorded outside the repository). SRV9 pin-check passed in173.413s. The focused native command did not execute tests: compilation exhausted disk space. Completed task caches were removed, restoring10GiB; no source or prepared dependency cache was deleted. This local checkpoint is not PostgreSQL qualification.


### Latest-main composition

Merged origin/main bbb3e06d0 (MAINRED1107, SRVWLD, CLHUD and mechanics audit updates). The sole conflict was the old root contract deletion versus main's rename; preserve main's docs/reference/SRVWLD-CONTRACT-NOTES.md and keep no root CONTRACT-NOTES.md. No source lane branch or main branch is modified. Regenerate the reverse-dependent test closure for the composed tree and run the owner-mandated full nextest matrix before any push.

### QENGINE2 main composition — qualification pending

Owner cancelled the SRV9 cutoff: retain SRV9 and fix the entire reverse-dependent matrix before one push. The first executable full run was superseded by the owner's immediate #3020 main-merge direction: 1,985 passed, five running cases received SIGINT, and 6,782 remained unrun (2,216.067s test execution; 2,503.031s including build). This is partial execution evidence, not a green full run.

Merge origin/main `1ae39c7fb9bef09c3cd50d16410374f4134b1676`. Canonical migrations 0999–1006 are byte-identical to main; this exact main tree has no 1200–1205 SQL files, and WPKG adds no QBASE copy. WPKG adds forward migration `1349_world_change_qengine_composition.sql` to preserve both canonical value-only reward receipts and the closed Benevola receipt-only handover, with same-transaction carried-consumption proof. Do not modify canonical migrations or previously published WPKG migration contents.

Resolve quest-writer participants by retaining canonical reward/native admissions plus WORLDCHG's closed Benevola participant; retain both callback projection and native-source teleport behavior, both NPC revision inputs, and both sets of source-name templates. WORLDCHG callback constructors adopt the canonical empty effects default and projection parameter. All generated content and pins are regenerated using repository tooling. The expanded owner-mandated normal reverse-dependency closure is 34 crates; complete replacement nextest/PG, strict Clippy, format and pin evidence remain pending.

All 335 SQL migrations replayed successfully on PostgreSQL18.6 in 5.151s actual (5.849s command wall time), preserving both carried USE and QENGINE reward proof arms. This replay proves schema composition only, not lane gameplay qualification.


## QENGINE2 report retained from merged main

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


Owner registry correction: QENGINE2/QBASE on main occupies 0999–1008, including 1006_quest_owned_base_predicates, 1007_quest_carried_payment_reapplication and 1008_quest_carried_membership_projection. All ten canonical SQL files are byte-identical to origin/main; zero 1200–1205 copies are present. WPKG-only migrations remain 1330–1333 and 1345–1349.

### Composed-main pre-nextest checks

The merged tree passes all-target/all-feature Cargo check (393.275s), strict all-target/all-feature Clippy with `-D warnings` (303.853s), format check (15.251s), native-provider freshness (108.063s), all 95 quest-authoring modules (478.344s), marriage producer tests (0.393s), cosmetics authoring (4.388s), and WORLDCHG authoring (17.247s). Final repository-tool pin refresh passes in25.679s and pin-check in27.926s. The staged-tree duplicate-migration script prints no duplicates; the root contract and tracked lane bookkeeping are absent. The replacement full 34-crate nextest/PG matrix is still pending; these checks alone do not authorize a push under the owner's prevention rule.

Current WPKG-only SQL files:
- `1330_world_change_owner.sql`
- `1331_world_change_credit_counters.sql`
- `1332_world_change_voice_support.sql`
- `1333_world_change_barter_custody.sql`
- `1345_s10_blessing_vendors.sql`
- `1346_npc_trade_fluid_subtypes.sql`
- `1347_npc_trade_full_charges.sql`
- `1348_npc_marriage.sql`
- `1349_world_change_qengine_composition.sql`


### Owner speed-first rule — final WPKG local qualification

The owner explicitly replaced the full reverse-dependency nextest push gate: run own mechanic/PG cases, workspace-wide all-target compilation, changed-crate Clippy, fmt, duplicate migration checks and tooling regeneration; CI owns the full suites and returns one combined findings list. The full 34-crate run was stopped during compilation as directed (no tests executed). No full-suite success is claimed. No test was weakened, newly ignored, skipped in code or deleted to obtain these results. Focused command filters select only the required mechanic/content checks.

Final dispositions: WORLDCHG-Q and SRV9 retained; WVAR, WCONTRIB and WEVENTS remain explicitly reverted with the reasons recorded above. The owner cancelled the SRV9 cutoff, and its full source behavior is retained. Main QENGINE2/QBASE 0999–1008 are byte-identical to origin/main 1ae39c7fb; no 1200–1205 copies, root contract file or tracked lane bookkeeping remain.

The focused PostgreSQL regression reproduced RED before1349: the pre-fix WORLDCHG shape rejected a canonical zero-track QENGINE reward receipt. After1349 both canonical reward and closed Benevola shapes passed, while unrelated empty receipts remained rejected. All335 migrations replayed; actual PostgreSQL red→green proof took4.793s. Separate trigger probes rejected proofless canonical rewards and proofless Benevola handovers. Native Benevola then passed the real atomic wolf debit/shared-credit/replay/refusal route in83.039s.

Local validation on the composed source tree:

| Check | Result / duration |
| --- | --- |
| cargo check --workspace --all-targets --locked --keep-going | PASS,377.114s command wall |
| changed crates: all-target/all-feature check | PASS,393.275s |
| changed crates: all-target/all-feature Clippy -D warnings | PASS,303.853s |
| cargo fmt --all --check | PASS,15.251s |
| final tooling pin-refresh / pin-check | PASS,25.679s /27.926s |
| quest-authoring unit suite | PASS,95modules,478.344s |
| native-provider freshness | PASS,108.063s |
| marriage / cosmetics / WORLDCHG authoring | PASS,0.393s /4.388s /17.247s |
| owner-native-pg | PASS,29tests,504.782s actual test execution |
| owner-server-pg | PASS,12tests,38.481s actual test execution |
| owner-content-integration | PASS,132tests,292.769s actual test execution |
| owner-world-change-units | PASS,44tests,1.559s actual test execution |

Native/server commands preserve successful stdout and report real PostgreSQL completion markers (`POSTGRES_CASE_COMPLETED` and `WORLDCHG-Q PG EXECUTED`), including SRV9 fluids/charges/marriage, counter concurrency/rollback/replay, Hive permit/egress, Fire Earth/Fire Feathered, Stampede, Yasir and Yeo. Tests below include short non-PG unit cases; durations are nextest execution times, separate from compilation.

| Focused group | Case | Seconds |
| --- | --- | ---: |
| owner-native-pg | `gameplay_transport::item_use::fluid_map_tests::srv9b_native_fluid_namespace_matches_shop_craft_and_drinking` | 0.011 |
| owner-native-pg | `gameplay_transport::kill_reward::tests::worldchg_native_death_uses_carrier_floor_not_overlay_floor` | 0.013 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::map_harvest::worldchg_thawing_stale_epoch_refuses_without_value` | 41.649 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::map_harvest::worldchg_thawing_ordinary_flower_at_daily_coordinate_is_not_day_gated` | 41.676 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::map_harvest::worldchg_thawing_inactive_daily_source_refuses_without_value` | 41.710 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::map_harvest::worldchg_thawing_daily_added_use_yield_replay_restart` | 41.716 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::raid_live::worldchg_postgres_stampede_map_runtime_e2e` | 44.520 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_blessings::s10_blessings_enforced_pvp_postgres_refuses_purchase` | 81.179 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_blessings::s10_blessings_postgres_nonpremium_and_phoenix_stages` | 81.238 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_bank::s10_eva_postgres_bank_deposit_and_withdraw` | 81.307 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_blessings::s10_twist_postgres_accepts_amulet_carried_in_backpack` | 84.449 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_rashid::s10_weekday_rollover_moves_live_rashid_on_postgres` | 5.697 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_healing::s10_healing_postgres_restores_only_emergency_health` | 80.993 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_rashid::s10_rashid_wandering_postgres_shop_payment_and_replay` | 80.953 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_voices::s10_valindara_postgres_source_voice_reaches_chat` | 44.470 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_travel::s10_travel_postgres_source_keywords_and_free_arrival` | 84.981 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_wandering::s10_wandering_postgres_moves_within_source_radius_and_pauses_for_customers` | 81.195 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::srv9b_fluids::srv9b_charges_postgres_source_purchase_and_replay` | 85.964 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::srv9b_fluids::srv9b_fluids_postgres_buy_sell_subtype_fence_and_replay` | 85.499 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::srv9b_marriage::srv9b_marriage_postgres_native_lynda_service` | 85.829 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::s10_promotions::s10_promotions_postgres_registered_sources_charge_and_replay` | 251.690 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::terrain_tool::worldchg_voice_active_gobbler_wire_and_replay` | 45.218 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::terrain_tool::worldchg_voice_inactive_gobbler_refuses_without_writes` | 43.879 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::worldchg_grimvale::worldchg_grimvale_final_native_wave_projects_actual_source_portal` | 41.833 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::worldchg_grimvale::worldchg_grimvale_native_entry_requires_current_moon_without_quest_writes` | 45.169 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::worldchg_hive_egress::worldchg_hive_native_gate_closed_stage_and_missing_scent_allow_egress_only` | 43.999 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::worldchg_benevola::worldchg_benevola_native_yes_debits_one_and_credits_ten_atomically` | 83.039 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::worldchg_trader::worldchg_trader_postgres_yasir_current_day_ship_and_receipt_gate` | 80.107 |
| owner-native-pg | `gameplay_transport::monk_save::postgres_tests::worldchg_yeo::worldchg_yeo_native_repeatable_ten_shrimp_one_gloo_atomic_replay` | 80.074 |
| owner-server-pg | `node::serve::channel_boot::worldchg_bootstrap_tests::worldchg_bootstrap_uninitialized_epoch_has_no_activation` | 0.009 |
| owner-server-pg | `combat::death_reward::tests::worldchg_actual_corpse_plan_still_refuses_thirty_one_entries` | 0.009 |
| owner-server-pg | `lifted_tests_auto::worldchg_stampede_content::worldchg_stampede_content_qualifies_native_wave_region_and_achievement` | 0.057 |
| owner-server-pg | `node::raid_scheduler::tests::world_src_furyosa_production_candidates_require_active_epoch_layer` | 6.451 |
| owner-server-pg | `worldchg_counter::worldchg_postgres_shared_kill_counter_atomic_replay_and_save` | 6.408 |
| owner-server-pg | `lifted_tests_auto::world_src_flower_postgres::world_src_flower_postgres_repeats_donor_cycle_without_duplicate_firing` | 14.761 |
| owner-server-pg | `worldchg_fire_earth::worldchg_postgres_fire_earth_fifty_active_kills_grant_once` | 8.364 |
| owner-server-pg | `worldchg_fire_feathered::worldchg_postgres_fire_feathered_guardian_completion_waits_for_quara_reset` | 8.360 |
| owner-server-pg | `worldchg_stampede::worldchg_postgres_stampede_daily_gate_reaches_existing_raid_runner` | 7.939 |
| owner-server-pg | `worldchg_owner::worldchg_postgres_persistence_reset_and_atomic_contribution` | 7.984 |
| owner-server-pg | `worldchg_hive_shared::worldchg_postgres_hive_completed_task_and_cycle_reset_are_one_receipt` | 8.276 |
| owner-server-pg | `combat::death_reward::tests::worldchg_embedded_ff_inputs_have_real_loot_rows_and_bounded_seeded_plans` | 38.461 |


## QUEST COLLECTOR — ROUND 1

Owner speed instructions supersede earlier qualification gates for this round: only `cargo check --workspace --all-targets` and `cargo fmt --check`; no local PG runs, Clippy or test suites. CI with heavy-tests owns full validation. Latest integrated main: `87957a7fa774d2979ae708688a199d8024b5122c`.

Merged source heads (pending final compilation):
- qworld: `c16eee476adb36d8f905a885b921e56f069ceb6d` (merge `63d20ac6edccb3d275cf4a4ca185e2a5c03f6a69`).
- qnpcfx: `a3ebe0bcf77750943b2193b316380b935b7bf0f7` (merge `b209ea0b6abfa6552469b9edc48c6e865ad744c6`).

Skipped/reverted source heads:

SKIPPED qengine2 `5c01deabb33b448fe7de795a3f9271477af05ea6`: cargo check E0428/E0119/E0753 duplicate NativeQuestDeath; merge reverted with mainline 1. Files: `crates/gameplay/durability/src/durability/quest_state/native_kills.rs`.

SKIPPED npcfix `b12367d4ef50ae1e03267c158a834ae5f3ad5302`: 55 conflicts across shared owners and producers cannot be resolved safely in five minutes. Files: `apps/game-server/tests/content_npc_catalogue.rs`, `apps/game-server/tests/content_world_project_repository.rs`, `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs`, `content/cosmetics/outfits/authoring.json`, `content/cosmetics/outfits/native-cosmetics.json`, `content/creatures/definitions/spell-native-profiles.json`, `content/creatures/prey-slot-values.json`, `content/dialogues/quests/shared-reward-greetings.json`, `content/items/definitions/items-16500-16999.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/taxonomy/items.json`, `content/npcs/definitions/npcs-00000-00499.json`, `content/npcs/definitions/npcs-00500-00999.json`, `content/npcs/definitions/npcs-01000-01281.json`, `content/quests/shared-reward-callbacks.json`, `content/quests/shared-reward-offers.json`, `content/services/bank/index.json`, `content/services/blessings/index.json`, `content/services/promotion/index.json`, `content/spells.manifest.json`, `content/world/npc-placements/index.json`, `content/world/pins/oteryn.identity.json`, `content/world/pins/oteryn.json`, `content/world/placements/index.json`, `crates/gameplay/content/src/content/npc_catalogue/service.rs`, `crates/gameplay/content/src/content/npc_catalogue/travel.rs`, `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs`, `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/durability/src/durability/npc_trade.rs`, `crates/gameplay/durability/src/durability/npc_trade/sell_hireling/tests.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/contents.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/world/src/map/facts.rs`, `crates/gameplay/world/src/map/floor.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `tools/content-migration/qengine_reward_item_profiles.py`, `tools/content-migration/register_spell_families.py`, `tools/content-migration/test_monster_seven_spell_overlay.py`, `tools/content-migration/test_world_project_v2_to_tree.py`, `tools/content-migration/validate_world_project_v2_to_tree.py`, `tools/content-migration/world_project_v2_to_tree.py`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`, `tools/content-schema/npc-authoring/npc_placement_scope.json`, `tools/content-schema/npc-authoring/test_convert_placements.py`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_qengine_rewards.py`.

SKIPPED questpc `6c09e0365110db6683f938dd08d5d584744cbf37`: conflict surface exceeds safe five-minute reconciliation. Files: `apps/game-server/tests/integration_rest/world_reward_claims_binding.rs`, `content/interactions/reward_claims/reward-claim-variants-00000-00099.json`, `content/interactions/reward_claims/reward-claims-00000-00099.json`, `content/items/definitions/items-17500-17999.json`, `content/items/definitions/items-18000-18499.json`, `content/items/definitions/items-20000-20499.json`, `content/items/definitions/spell-native-profiles.json`, `content/quests/definitions/quests-00000-00099.json`, `content/quests/definitions/quests-00100-00199.json`, `content/quests/definitions/quests-00200-00299.json`, `content/quests/grizzly-boss-rooms.json`, `content/quests/missions/completion-candidate.json`, `content/quests/missions/quest-state-completion-candidate.json`, `content/quests/missions/quest-state.json`, `content/spells.manifest.json`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/quests/src/quest/world_callbacks.rs`, `docs/agents/evidence/quest-claim-qualification.json`, `docs/repository/QUEST_CALLBACK_COVERAGE.json`, `tools/content-migration/test_quest_callback_profiles.py`, `tools/content-schema/quest-authoring/check_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/quest_tree_authoring.py`, `tools/content-schema/quest-authoring/samples/binding_packets/index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/npc/quest-npc-dialogue-links.json`, `tools/content-schema/quest-authoring/samples/binding_packets/recipes/supplementary-index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/chosen-reward-item-candidates.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/perquest-352.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/requirement-choices.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-holds.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-only-105.json`, `tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json`, `tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/crosswalk.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/dialogue.json.gz`, `tools/content-schema/quest-authoring/samples/donor-source/components248/summary.json`, `tools/content-schema/quest-authoring/samples/donor-source/coverage.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-dependencies.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-index.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc.json.gz`, `tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json`, `tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/reward-only-completion/candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json`, `tools/content-schema/quest-authoring/samples/v2-export/declarations.json`, `tools/content-schema/quest-authoring/samples/v2-export/report.json`, `tools/content-schema/quest-authoring/test_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/test_quest_completion_matrix.py`, `tools/content-schema/reward-claim-authoring/container_native.py`, `tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json`, `tools/content-schema/reward-claim-authoring/test_container_native.py`, `tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/test_variant_stack_serialization.py`.

SKIPPED questpc8 `e70393b99336f248cb49614d7b707d1a82ca80ee`: conflict surface exceeds safe five-minute reconciliation. Files: `apps/game-server/src/node/serve/bundle_npcs.rs`, `apps/game-server/tests/integration_rest/world_reward_claims_binding.rs`, `content/cosmetics/outfits/authoring.json`, `content/cosmetics/outfits/native-cosmetics.json`, `content/creatures/prey-slot-values.json`, `content/creatures/prey-tiers.json`, `content/dialogues/quests/shared-reward-greetings.json`, `content/interactions/reward_claims/reward-claim-variants-00000-00099.json`, `content/interactions/reward_claims/reward-claims-00000-00099.json`, `content/items/definitions/items-16500-16999.json`, `content/items/definitions/items-17500-17999.json`, `content/items/definitions/items-18000-18499.json`, `content/items/definitions/items-20000-20499.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/taxonomy/items.json`, `content/quests/definitions/quests-00000-00099.json`, `content/quests/definitions/quests-00100-00199.json`, `content/quests/definitions/quests-00200-00299.json`, `content/quests/grizzly-boss-rooms.json`, `content/quests/missions/completion-candidate.json`, `content/quests/missions/quest-state-completion-candidate.json`, `content/quests/missions/quest-state.json`, `content/quests/shared-reward-callbacks.json`, `content/quests/shared-reward-offers.json`, `content/spells.manifest.json`, `content/world/npc-placements/index.json`, `content/world/pins/oteryn.json`, `crates/gameplay/content/src/content/npc_catalogue/service.rs`, `crates/gameplay/content/src/content/world_reward_claims.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/actions.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/unnatural_selection.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/quest_world_callback.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/contents.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/world_callbacks.rs`, `crates/gameplay/world/src/map/facts.rs`, `crates/gameplay/world/src/map/facts/quest_callbacks.rs`, `crates/gameplay/world/src/map/floor.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/agents/evidence/quest-claim-qualification.json`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `docs/repository/QUEST_CALLBACK_COVERAGE.json`, `tools/content-migration/register_spell_families.py`, `tools/content-migration/test_monster_seven_spell_overlay.py`, `tools/content-migration/test_quest_callback_profiles.py`, `tools/content-schema/native-gameplay/build_full_spell_artifact.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/npc-authoring/convert_placements.py`, `tools/content-schema/npc-authoring/npc_placement_scope.json`, `tools/content-schema/quest-authoring/check_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/quest_tree_authoring.py`, `tools/content-schema/quest-authoring/samples/binding_packets/index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/npc/quest-npc-dialogue-links.json`, `tools/content-schema/quest-authoring/samples/binding_packets/recipes/supplementary-index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/chosen-reward-item-candidates.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/perquest-352.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/requirement-choices.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-holds.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-only-105.json`, `tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json`, `tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/crosswalk.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/dialogue.json.gz`, `tools/content-schema/quest-authoring/samples/donor-source/components248/summary.json`, `tools/content-schema/quest-authoring/samples/donor-source/coverage.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-dependencies.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-index.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc.json.gz`, `tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json`, `tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/reward-only-completion/candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json`, `tools/content-schema/quest-authoring/samples/v2-export/declarations.json`, `tools/content-schema/quest-authoring/samples/v2-export/report.json`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/test_quest_completion_matrix.py`, `tools/content-schema/reward-claim-authoring/container_native.py`, `tools/content-schema/reward-claim-authoring/reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json`, `tools/content-schema/reward-claim-authoring/test_container_native.py`, `tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/test_variant_stack_serialization.py`.

SKIPPED questpc7 `d58add72dde9dff485e365fb5a594e5c215d02d4`: conflict surface exceeds safe five-minute reconciliation. Files: `apps/game-server/src/node/serve/bundle_npcs.rs`, `apps/game-server/tests/integration_rest/world_reward_claims_binding.rs`, `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs`, `content/cosmetics/outfits/authoring.json`, `content/cosmetics/outfits/native-cosmetics.json`, `content/creatures/prey-slot-values.json`, `content/dialogues/quests/shared-reward-greetings.json`, `content/interactions/reward_claims/reward-claim-variants-00000-00099.json`, `content/interactions/reward_claims/reward-claims-00000-00099.json`, `content/items/definitions/items-16500-16999.json`, `content/items/definitions/items-17500-17999.json`, `content/items/definitions/items-18000-18499.json`, `content/items/definitions/items-20000-20499.json`, `content/items/definitions/spell-native-profiles.json`, `content/quests/definitions/quests-00000-00099.json`, `content/quests/definitions/quests-00100-00199.json`, `content/quests/definitions/quests-00200-00299.json`, `content/quests/grizzly-boss-rooms.json`, `content/quests/missions/completion-candidate.json`, `content/quests/missions/quest-state-completion-candidate.json`, `content/quests/missions/quest-state.json`, `content/quests/shared-outfit-callbacks.json`, `content/quests/shared-reward-callbacks.json`, `content/quests/shared-reward-offers.json`, `content/spells.manifest.json`, `content/world/npc-placements/index.json`, `content/world/pins/oteryn.json`, `content/world/placements/index.json`, `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/quest_world_callback.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/contents.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/world_callbacks.rs`, `crates/world-bundle/src/sector.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/agents/evidence/quest-claim-qualification.json`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `docs/repository/QUEST_CALLBACK_COVERAGE.json`, `tools/content-migration/qengine_reward_item_profiles.py`, `tools/content-migration/register_spell_families.py`, `tools/content-migration/test_quest_callback_profiles.py`, `tools/content-schema/native-gameplay/build_full_spell_artifact.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`, `tools/content-schema/npc-authoring/npc_placement_scope.json`, `tools/content-schema/quest-authoring/check_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/quest-authoring/quest_batch4.py`, `tools/content-schema/quest-authoring/quest_tree_authoring.py`, `tools/content-schema/quest-authoring/samples/binding_packets/index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/npc/quest-npc-dialogue-links.json`, `tools/content-schema/quest-authoring/samples/binding_packets/recipes/supplementary-index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/chosen-reward-item-candidates.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/perquest-352.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/requirement-choices.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-holds.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-only-105.json`, `tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json`, `tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/crosswalk.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/dialogue.json.gz`, `tools/content-schema/quest-authoring/samples/donor-source/components248/summary.json`, `tools/content-schema/quest-authoring/samples/donor-source/coverage.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-dependencies.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-index.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc.json.gz`, `tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json`, `tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/reward-only-completion/candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json`, `tools/content-schema/quest-authoring/samples/v2-export/declarations.json`, `tools/content-schema/quest-authoring/samples/v2-export/report.json`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_qengine_rewards.py`, `tools/content-schema/quest-authoring/test_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/test_quest_completion_matrix.py`, `tools/content-schema/reward-claim-authoring/container_native.py`, `tools/content-schema/reward-claim-authoring/reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json`, `tools/content-schema/reward-claim-authoring/test_container_native.py`, `tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/test_variant_stack_serialization.py`.

SKIPPED questl2 `581bcd1bdca65f6ba0642abec8cb25bf3c262cc4`: conflict surface exceeds safe five-minute reconciliation. Files: `apps/game-server/tests/integration_rest/world_reward_claims_binding.rs`, `content/cosmetics/outfits/authoring.json`, `content/cosmetics/outfits/native-cosmetics.json`, `content/creatures/prey-slot-values.json`, `content/dialogues/quests/shared-reward-greetings.json`, `content/encounters/raids/index.json`, `content/interactions/reward_claims/reward-claim-variants-00000-00099.json`, `content/interactions/reward_claims/reward-claims-00000-00099.json`, `content/items/definitions/items-16500-16999.json`, `content/items/definitions/items-17500-17999.json`, `content/items/definitions/items-18000-18499.json`, `content/items/definitions/items-20000-20499.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/taxonomy/items.json`, `content/quests/definitions/quests-00000-00099.json`, `content/quests/definitions/quests-00100-00199.json`, `content/quests/definitions/quests-00200-00299.json`, `content/quests/grizzly-boss-rooms.json`, `content/quests/missions/completion-candidate.json`, `content/quests/missions/quest-state-completion-candidate.json`, `content/quests/missions/quest-state.json`, `content/quests/shared-reward-callbacks.json`, `content/quests/shared-reward-offers.json`, `content/spells.manifest.json`, `content/world/npc-placements/index.json`, `content/world/pins/oteryn.json`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_rewards.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/quest_world_callback.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/contents.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/mod.rs`, `crates/gameplay/quests/src/quest/reward_callbacks.rs`, `crates/gameplay/social/src/interaction/chest_use.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/agents/evidence/quest-claim-qualification.json`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `docs/repository/QUEST_CALLBACK_COVERAGE.json`, `tools/content-migration/qengine_reward_item_profiles.py`, `tools/content-migration/register_spell_families.py`, `tools/content-migration/test_quest_callback_profiles.py`, `tools/content-schema/native-gameplay/build_full_spell_artifact.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`, `tools/content-schema/npc-authoring/npc_placement_scope.json`, `tools/content-schema/quest-authoring/check_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/quest-authoring/quest_tree_authoring.py`, `tools/content-schema/quest-authoring/samples/binding_packets/index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/npc/quest-npc-dialogue-links.json`, `tools/content-schema/quest-authoring/samples/binding_packets/recipes/supplementary-index.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/chosen-reward-item-candidates.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/perquest-352.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/requirement-choices.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-holds.json`, `tools/content-schema/quest-authoring/samples/binding_packets/rewards/reward-only-105.json`, `tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json`, `tools/content-schema/quest-authoring/samples/completion-matrix/all373.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/crosswalk.json`, `tools/content-schema/quest-authoring/samples/donor-source/components248/dialogue.json.gz`, `tools/content-schema/quest-authoring/samples/donor-source/components248/summary.json`, `tools/content-schema/quest-authoring/samples/donor-source/coverage.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-dependencies.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc-index.json`, `tools/content-schema/quest-authoring/samples/donor-source/npc.json.gz`, `tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json`, `tools/content-schema/quest-authoring/samples/server-completion/collect-claim-candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/reward-only-completion/candidates.json`, `tools/content-schema/quest-authoring/samples/server-completion/use-trigger-candidates.json`, `tools/content-schema/quest-authoring/samples/v2-export/declarations.json`, `tools/content-schema/quest-authoring/samples/v2-export/report.json`, `tools/content-schema/quest-authoring/shared-reward-callbacks.md`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_qengine_rewards.py`, `tools/content-schema/quest-authoring/test_quest_claim_qualification.py`, `tools/content-schema/quest-authoring/test_quest_completion_matrix.py`, `tools/content-schema/reward-claim-authoring/container_native.py`, `tools/content-schema/reward-claim-authoring/reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json`, `tools/content-schema/reward-claim-authoring/test_container_native.py`, `tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/test_variant_stack_serialization.py`.

SKIPPED questpc5 `da3c129374c411120e1088e5cbe7c60a0bc2c97e`: conflict surface exceeds safe five-minute reconciliation. Files: `content/creatures/prey-slot-values.json`, `content/creatures/prey-tiers.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/spells.manifest.json`, `content/world/pins/oteryn.json`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/contents.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `tools/content-migration/register_spell_families.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_qengine_rewards.py`.

SKIPPED questpc4 `8bf3338eca7e86708c7ae4a87493043b0976e831`: conflict surface exceeds safe five-minute reconciliation. Files: `apps/game-server/src/node/serve/channel_boot.rs`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/spells.manifest.json`, `content/world/pins/oteryn.json`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/map_harvest.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/loader.rs`, `crates/gameplay/quests/src/quest/reward_callbacks.rs`, `crates/gameplay/world/src/map/floor.rs`, `deploy/synology-game/preprod-topology.toml`, `tools/content-migration/qengine_reward_item_profiles.py`, `tools/content-migration/register_spell_families.py`, `tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`, `tools/content-schema/quest-authoring/shared-reward-callbacks.md`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_qengine_rewards.py`, `tools/content-schema/world-authoring/convert_world_base.py`.

SKIPPED questpc2 `26e5a8f0b396cc0a1a67ed3476780a88aa5be06b`: conflict surface exceeds safe five-minute reconciliation. Files: `content/cosmetics/outfits/authoring.json`, `content/cosmetics/outfits/native-cosmetics.json`, `content/creatures/prey-slot-values.json`, `content/creatures/prey-tiers.json`, `content/encounters/raids/catalogue.json`, `content/encounters/raids/index.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/spells.manifest.json`, `content/world/pins/oteryn.json`, `crates/gameplay/content/src/content/creature_reward.rs`, `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs`, `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/item_mint_audit/tests/oneof_wire_tests.rs`, `crates/gameplay/durability/src/durability/npc_trade.rs`, `crates/gameplay/durability/src/durability/npc_trade/hireling.rs`, `crates/gameplay/durability/src/durability/npc_trade/tests.rs`, `crates/gameplay/durability/src/durability/npc_trade_audit.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/world_callbacks.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `tools/content-migration/register_spell_families.py`, `tools/content-migration/test_monster_seven_spell_overlay.py`, `tools/content-migration/test_quest_callback_profiles.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/world-authoring/convert_world_base.py`, `tools/content-schema/world-authoring/quest_access_bindings.py`.

SKIPPED questpc3 `06d1bda324884db38eb4f46f1812274a159c7775`: conflict surface exceeds safe five-minute reconciliation. Files: `apps/game-server/src/node/serve/daily_world_changes.rs`, `content/creatures/prey-slot-values.json`, `content/dialogues/quests/shared-reward-greetings.json`, `content/encounters/raids/index.json`, `content/interactions/reward_claims/reward-claim-variants-00000-00099.json`, `content/interactions/reward_claims/reward-claims-00000-00099.json`, `content/items/definitions/items-17500-17999.json`, `content/items/definitions/items-18000-18499.json`, `content/items/definitions/items-20000-20499.json`, `content/items/definitions/items-21000-21499.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/taxonomy/items.json`, `content/quests/shared-reward-callbacks.json`, `content/quests/shared-reward-offers.json`, `content/services/blessings/index.json`, `content/spells.manifest.json`, `content/world/npc-placements/index.json`, `content/world/pins/oteryn.json`, `content/world/placements/index.json`, `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/raid_live.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/character_appearance.rs`, `crates/gameplay/durability/src/durability/item_craft.rs`, `crates/gameplay/durability/src/durability/npc_trade.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/quest_world_callback.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/contents.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/world_callbacks.rs`, `crates/gameplay/world-change/src/daily.rs`, `crates/gameplay/world/src/map/facts.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `docs/repository/QUEST_CALLBACK_COVERAGE.json`, `tools/content-migration/test_quest_callback_profiles.py`, `tools/content-migration/validate_world_project_v2_to_tree.py`, `tools/content-migration/world_project_v2_to_tree.py`, `tools/content-schema/native-gameplay/condition-potion-profiles.json`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/npc-authoring/npc_placement_scope.json`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/reward-claim-authoring/container_native.py`, `tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json`, `tools/content-schema/reward-claim-authoring/test_container_native.py`, `tools/content-schema/reward-claim-authoring/test_reward_claim_variant_authoring.py`, `tools/content-schema/reward-claim-authoring/test_variant_stack_serialization.py`, `tools/content-schema/world-authoring/samples/world-base-capture-v1.json`.

qengine2 was reverted using mainline parent1 after the first workspace compilation failed on duplicated NativeQuestDeath definitions/methods. Main’s canonical engine remains. The two remaining lane merges preserve existing WPKG counters, the three QWORLD durable physical activations, QNPCFX programme receipts and the exact eleven-track Father acceptance. Canonical main migrations 0999–1008 remain unchanged. Native generated providers were refreshed with repository tooling; this source state is not claimed to have executed PostgreSQL tests.


### Round 1 reduced compilation candidate (owner PUSH NOW gate)

QENGINE2 merge ee73812f3 was reverted with mainline1 in ac1ba1e83 after E0428/E0119/E0753 duplicated NativeQuestDeath definitions/methods. QWORLD merge63d20ac6e was reverted with mainline1 in4f98c61f2 after E0063 missing world_change in the existing worldchg_fire_feathered and worldchg_hive_shared test constructors. No source lane branch was changed. The earlier provisional list is superseded: only QNPCFX a3ebe0bcf77750943b2193b316380b935b7bf0f7 remains in this round. Main87957a7fa is integrated. The other nine lanes remain skipped for their recorded five-minute conflict surfaces. The canonical main shared engine and all previously qualified WPKG lanes remain.

Per the final owner speed instructions, local PG, unit suites, Clippy and further verification are explicitly deferred to CI. Only workspace all-target compilation and fmt are push gates. Native providers were regenerated using repository tooling after the engine revert; legacy World package inputs are unchanged. The exact QNPCFX Father eleven-track predicate is retained alongside the WPKG receipt-only and main reward-only arms in unmerged package migration1349; main migration files are unchanged. No new title qualification is claimed.


### Round 1 publication gate

The final retained tree passes `cargo check --workspace --all-targets --locked --keep-going` in268.786s and `cargo fmt --all --check` (exit0). Retained: QNPCFX a3ebe0bcf77750943b2193b316380b935b7bf0f7. Skipped/reverted: QENGINE2, NPCFIX, QUESTPC, QUESTPC8, QUESTPC7, QUESTL2, QUESTPC5, QUESTPC4, QUESTPC2, QUESTPC3, QWORLD. No local PG runs, full test suites, or Clippy were required by the final round1 speed gate. CI qualification remains with the coordinator's heavy-tests PR.


## QUEST COLLECTOR — ROUND 2

Owner scope: clean source merges, cargo check --workspace --all-targets and fmt; do not wait for PR3030 CI or run local suites. Main `669d5af5261a60d347167b7c880a4498aebe72ed` (CATALOG #3025) is merged in `0e25200e5948e1566f798894d3540324b712a523`.

Merged qnpcfx `57ca1d89a24668e37c1dfc7a0baa9f1923cd5361` in `f3cdfddc4ec448ec3fbcf8296fcbd7cf5a9c8457`. source clean; sole derived Grizzly hash regenerated with repo tool.

The sole QNPCFX conflict was the generated Grizzly input hash; `grizzly_rooms.py` regenerated it without changing source quest logic. Main’s generated SOURCE receipt retained its new compiler metadata and the callback proof/approval hash was refreshed through `regenerate_content.refresh_quest_callback_proof`.

PR3030 early failure folded into this round: `ValueError: stale binding evidence: npc/quest-npc-dialogue-links.json`. Ran `python3 tools/content-schema/quest-authoring/quest_binding_authoring.py` after the merges; it regenerated dialogue links, recipe supplementary evidence and the binding index in17.316s. Generator validation passed310 identity/stage joins and6628 exact JSON pointers. This does not claim full quest-authoring-suite qualification.

SKIPPED qengine2 `e5850715e4f28ac951d3e9f18466e528055a5157`: merge conflicts; round 2 accepts clean merges only. Files: `apps/game-server/migrations/1550_reward_claim_additional_items.sql`, `apps/game-server/migrations/1551_complete_cosmetic_piece_recipes.sql`, `content/cosmetics/outfits/authoring.json`, `content/cosmetics/outfits/native-cosmetics.json`, `content/dialogues/quests/shared-reward-greetings.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/quests/shared-reward-callbacks.json`, `content/quests/shared-reward-offers.json`, `content/spells.manifest.json`, `content/world/pins/oteryn.json`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/item_cosmetics/positive.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_access.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_rewards.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/rotten_blood_progression.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/soul_war.rs`, `crates/gameplay/durability/src/durability/consume_transform.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs`, `crates/gameplay/quests/src/quest/reward_callbacks.rs`, `deploy/synology-game/preprod-topology.toml`, `qlane-meta/qengine2/CONTRACT-NOTES.md`, `qlane-meta/qengine2/QUEST-STATUS.json`, `qlane-meta/qengine2/RESUME.md`, `tools/content-migration/qengine_reward_item_profiles.py`, `tools/content-migration/register_spell_families.py`, `tools/content-schema/cosmetics-authoring/cosmetics.schema.json`, `tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_qengine_rewards.py`.

SKIPPED questpc8 `f0bf18a7ca52da22a418c01bbf96aac1ccb9728f`: merge conflicts; round 2 accepts clean merges only. Files: `apps/game-server/src/node/serve/bundle_npcs.rs`, `content/creatures/prey-slot-values.json`, `content/creatures/prey-tiers.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/spells.manifest.json`, `content/world/pins/oteryn.json`, `crates/gameplay/content/src/content/npc_catalogue/service.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/npc_rules.rs`, `crates/gameplay/quests/src/quest/npc_rules/validate.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `tools/content-migration/register_spell_families.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`.

SKIPPED questl2 `ac1fd2427499e1da0746c4f1e92873084df571db`: merge conflicts; round 2 accepts clean merges only. Files: `apps/game-server/src/node/serve/bundle_npcs.rs`, `content/creatures/prey-slot-values.json`, `content/creatures/prey-tiers.json`, `content/dialogues/quests/shared-reward-greetings.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/quests/grizzly-boss-rooms.json`, `content/quests/shared-reward-offers.json`, `content/spells.manifest.json`, `content/world/npc-placements/index.json`, `content/world/pins/oteryn.json`, `crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_rewards.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/character_revision_sequencer.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/quest_world_callback.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint.rs`, `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/loader.rs`, `crates/gameplay/quests/src/quest/npc_rules.rs`, `crates/gameplay/quests/src/quest/reward_callbacks.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `tools/content-migration/qengine_reward_item_profiles.py`, `tools/content-migration/register_spell_families.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`, `tools/content-schema/npc-authoring/npc_placement_scope.json`, `tools/content-schema/quest-authoring/qengine_rewards.py`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`, `tools/content-schema/quest-authoring/shared-reward-callbacks.md`, `tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json`, `tools/content-schema/quest-authoring/test_qengine_rewards.py`.

SKIPPED questpc2 `c8b42aba3cd95ecb5d2997f95ec047bc76fcdcf0`: merge conflicts; round 2 accepts clean merges only. Files: `content/creatures/prey-slot-values.json`, `content/creatures/prey-tiers.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/spells.manifest.json`, `content/world/pins/oteryn.json`, `crates/gameplay/content/src/content/creature_reward.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_access.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/character_appearance/quest_grants.rs`, `crates/gameplay/durability/src/durability/character_revision_sequencer.rs`, `crates/gameplay/durability/src/durability/npc_trade.rs`, `crates/gameplay/durability/src/durability/npc_trade/hireling.rs`, `crates/gameplay/durability/src/durability/npc_trade/tests.rs`, `crates/gameplay/durability/src/durability/npc_trade_audit.rs`, `crates/gameplay/durability/src/durability/quest_state/writer.rs`, `crates/gameplay/durability/src/durability/quest_world_callback.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/quests/src/quest/world_callbacks.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `tools/content-migration/register_spell_families.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/world-authoring/convert_world_base.py`.

SKIPPED questpc3 `d98761117ec6e9bf7e164e489e1b1dfd5ad82a75`: merge conflicts; round 2 accepts clean merges only. Files: `apps/game-server/src/node/serve/daily_world_changes.rs`, `content/creatures/prey-slot-values.json`, `content/items/definitions/spell-native-profiles.json`, `content/items/definitions/srx1-item-updates.json`, `content/services/blessings/index.json`, `content/spells.manifest.json`, `content/world/pins/oteryn.json`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/raid_live.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs`, `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs`, `crates/gameplay/durability/src/durability/npc_trade.rs`, `crates/gameplay/npc/src/npc_talk.rs`, `crates/gameplay/world-change/src/daily.rs`, `crates/gameplay/world/src/map/facts.rs`, `deploy/synology-game/preprod-topology.toml`, `docs/balance/PREY-SLOT-VALUE-2026-11-01.md`, `docs/balance/PREY-TIME-SIM-2026-11-01.md`, `tools/content-migration/test_world_project_v2_to_tree.py`, `tools/content-schema/native-gameplay/loot-tables.json`, `tools/content-schema/native-gameplay/prey-model-results.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json`, `tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json`.

The fetched QUESTPC8 branch had advanced beyond owner-reported0fd3f01d to f0bf18a7; the latest pushed head was attempted. No source lane or main branch was changed, no force/rebase was used, and no PR was opened.

Round 2 publication gate: workspace all-target check passed (259.190s command wall); cargo fmt --all --check passed (exit0). The reported PR3030 stale binding evidence was regenerated after the QNPCFX and CATALOG merges. No additional combined CI failure list arrived before this publication. Full CI and PG qualification are not claimed.

## PR3030 main synchronization and combined CI repair (2026-11-07)

Merged main 1138720b1 into the collector. Resolved channel boot, daily-world-change helpers, and the world-change dependency conflict by retaining the WORLDCHG durable day/counter projection and overlaying QWORLD's three persisted physical activation decisions. Epoch-zero boot remains inactive. Added the optional world-change fields to existing native constructors, including all three world-change authority test constructors; the QWORLD decision restore initializes only the unrelated WPKG projection fields to inactive defaults.

PR3030 PG partition 4: SRV9 marriage `prepare` unconditionally swallowed Lynda's separately qualified addon conversation, returned an ordinary success reply, and never committed the next quest reward source. Marriage now owns only explicit marriage prompts or a pending marriage conversation; other input reaches the existing bounded reward handler. No reward expectation or PG assertion was reduced.

PR3030 unit partition 3: registered SRV9's real fenced marriage mint/debit outbox site in the exact Type2 insert inventory; attached the retained source witnesses to explicitly authored WORLDCHG native progress tracks; extended the exact item-profile cohort accounting by 40 WORLDCHG additions (41 source profiles less previously admitted12369) and two SRV9 wedding items, with materializability and zero-speed assertions for every source profile. Yasir/Orockle exclusion fixtures now use unqualified merchant Alesar, while explicitly checking Sam's independent QENGINE2 reward admission has no scripted dialogue keywords. Sam cannot remain an exclusion fixture when QENGINE2's bounded reward tests require his presence.

Regenerated native providers/Grizzly receipts, callback proof, and pins with repository tooling. The staged-tree duplicate-migration check has no duplicates. No lane was dropped and canonical QENGINE2/QBASE migrations remain unchanged. Focused execution and final publication-gate results follow below.

The complete CI list added three independent integration repairs. The CATALOG Npc field inventory fixture was regenerated with `oteryn-game-platform-catalog content/world --inventory`; only the Npc fingerprint changed, and the existing explicit public/unsupported projection remains the authority. The quest tree's four affected registry/shard files were regenerated by quest_tree_authoring after binding evidence regeneration. Native output replay now counts and explicitly requires WORLDCHG's loot-tables output (44 total), keeping byte-for-byte comparisons of all outputs.

The old Si Ariki achievement fixture called the generic NPC-trade entry without a reset-selected Yasir placement. It now installs a natively drawn Ankrahmun trader day, uses commit_yasir_trade at its source position, and also asserts the generic entry rejects the same request; all once-only and replay achievement assertions remain.

Royal Costume reproduced `early eof` locally; PostgreSQL recorded transaction timeout at COMMIT inside game_item_nested_entry_proven. The owner then directed reuse of main e1ca3e00 (#3024), so the uncommitted WPKG proof-order migration and its build-registration change were removed in favor of that main fix. No WPKG Royal Costume implementation is retained.

Main e1ca3e003 (#3024) merged cleanly in11db584a2; all Royal Costume changes now come from main. Its apply_input API adds a SpellLanePermit check; the retained Benevola caller now supplies that permit and fence. QNPCFX's separate, already fenced NPC programme participant supplies its existing private NpcItemPermit through a37-line local input-authority adapter; both paths retain the Channel-scope requirement and the spell path retains main's check_channel. No new reward/trigger/quest feature was added.

Publication checkpoint under the owner's one-push PREPUSH-2 exception: cargo check --workspace --all-targets --locked --keep-going passed on main e1ca3e003 plus all retained repairs (Cargo build duration6m15s); cargo fmt --all --check passed in15.516s. Native regeneration after the main merge passed in88.276s. Content-tree/Item+Mount test runner passed in444.130s, direct equivalence validation passed in154.734s, regenerated quest-tree check passed in101.299s, and pin refresh passed in68.773s. Canonical migrations are unchanged; the removed1334 experiment is not present. Pending after this checkpoint: complete changed-crate unit suites and fresh merged-tree PG reruns for Royal Costume, mage outfit, Si Ariki/Yasir, and the retained native NPC/value participants. Prior focused repairs for audit inventory, native source witnesses, speed profiles, Yasir/Orockle closure, and public view each passed locally; these are not represented as full PREPUSH-2 suite completion.

## Main synchronization: QNPCFX f641d8d6c / PR3027

Merged the owner's main f641d8d6c, including its VIP replay fixture fix; no local VIP implementation was added. Four generated conflicts (native Item profiles, SRX1 supplement, spell manifest and Grizzly rooms) were regenerated with repository tools. Automatic additive merges duplicated the existing NPC consumption function, catalogue extension/legacy-default helpers, module registration and venison assertions; retained each existing collector implementation once. Main's shared report is the previous report plus ten new lines, so taking that superset preserves every entry without duplicating the full report.

Regenerated native providers/rooms and the content tree from the retained canonical World; regenerated the Raid registration after the raw conversion. All generated outputs match the pre-merge collector's bytes, preserving SRV9's two Bank, ten Blessing and seven Promotion records. The only executable diff against the preceding collector head is main's VIP PostgreSQL replay test. Pin refresh completed in27.513s. Current validation runs and durations will be recorded with publication; no test-suite result is claimed while those runs remain active.

## PR3030 combined donor and player-trade repair

Refreshed the pinned NPC donor capture and replayed the documented quest-authoring dependency order with repository generators, including definition witnesses, binding packets, state lowering, completion matrix and rollout backlog. No generated artifact was hand-edited. The exact workflow checks, donor and unit-test parts passed in234.063s,558.197s and425.362s respectively (96 Python unit modules). Final binding and completion-matrix freshness checks passed in17.505s and0.580s.

The traded item profiles match main byte-for-byte. Trade acceptance instead collapsed transient single-holder preflight unavailability into permanent Rejected. Its read-only preflight now uses the existing bounded command admission retry budget, preserving classified refusals and all original fences and writes. A new real-PostgreSQL regression proves recovery after holder return, a successful query, rejection after persistent unavailability, and unchanged Stale classification. CI supplied the failing scenario; the original scenario passed locally before the repair in87.111s, so no local red reproduction is claimed.

The final selected nextest run passed284/284 tests in394.101s: all281 quest-crate unit tests plus the original player-trade PostgreSQL scenario (97.623s), new holder regression (12.499s; measured holder exercise858.496ms, two attempts), and main's VIP replay scenario (12.572s). Earlier merged-main PG reruns also passed mage outfit98.266s and Royal Costume162.102s; those precede the final donor regeneration. No tests were weakened, skipped or deleted.

Final workspace/all-target compilation passed in427.704s, fmt in14.795s, pin-refresh in27.292s and pin-check in29.732s. Main f641d8d6c is included; no new repair was made to the owner's known main quest_world_callback compile regression. Full dispatch unit-suite completion remains pending under the owner's PREPUSH-2 cadence fallback; its earlier run was superseded by regenerated embedded quest inputs. The focused PostgreSQL cases and full quest unit suite above completed on the final source tree.

## PR3030 main3032 synchronization and recurring trade rejection

Merged main47db34f09 (#3032) in818dd5583. Resolved the callback conflict by retaining exactly one input-row helper, the collector's apply_input, with its existing typed spell/NPC authority adapter. All three input callers use that helper; no apply_input_rows duplicate remains. The merge's executable tree equals the preceding1e71d7e22 tree. Canonical migration files are unchanged.

The1e71d7e22 CI trade failure demonstrates that the prior Accept-only retry was incomplete. Offer still converted transient single-holder read failures to permanent Rejected, including trade-pair/fence admission, offer observation and backpack reads. The new real-wire PostgreSQL contention case holds the sole ready holder during Offer, returns it after50ms, and continues the complete original trade journey. Before the Offer repair it reproduced command78 / Offer / payload[8,12], Rejected versus Accepted, in69.978s (360.076s including build/queue). Offer now preserves typed transient errors through the existing command retry budget. Only unavailable reads retry; classified refusals, rate limits, fences, inventory checks, cancellation and the eventual publication retain their original behavior. No item/shop/NPC profiles were changed.

A separate load run reached the fixture's deliberate-contention setup and returned RootUnavailable because it requested its semantic pass before the previous connection return completed. That setup now waits through the same existing bounded readiness mechanism before holding its pass. The during-contention equality assertion and every swap, replay, cancellation, reservation and refusal assertion remain intact. Failure diagnostics now include the decoded intent to identify future command refusals precisely.

Durability validation passed715/715 cases with PG enabled (489.380s command wall;277.904s test run). Before the Offer repair, the merged-main focused PG run also passed original trade85.647s, Benevola89.570s and holder recovery9.072s; these are baseline evidence, not the final Offer-repair verdict. Final repair validation is recorded below after completion.

Final Offer-repair PG validation passed4/4 in107.813s of test execution (235.069s command wall): original two-session atomic swap/replay/cancellation107.809s, the new complete Offer-contention journey102.729s, Benevola106.758s and bounded holder recovery12.687s. This establishes a real-wire red-to-green Offer regression, not merely a helper unit test. Workspace/all-target compilation passed193.370s on the repaired source; fmt passed15.694s. Repository pin-refresh passed28.226s, binding evidence regeneration18.364s and completion matrix regeneration0.580s, all without generated content changes. Pin-check previously passed29.423s on these identical content inputs. The full dispatch route-less unit run uses the normal absent-PG environment; the four real PG cases above are separately proven with the configured database.

Publication uses the owner's PREPUSH-2 cadence fallback: the complete dispatch route-less unit command remains active (at least316 cases passed, no failures at checkpoint), while the full715-case durability suite, four real PG cases, final workspace check, generated-file tooling and fmt have completed successfully. This is not a claim that the entire dispatch unit run has finished.

## Collector round2: mechanical-merge eligibility

Started wpkg2 from1e71d7e22, paused for the owner-requested PR3030 main/CI repair, then fast-forwarded it to the repaired wpkg head c833ddf5d before resuming. No lane implementation was rewritten or partially cherry-picked. All six branch merge attempts were aborted; none met the owner’s mechanical-conflict limit. No migration renumbering is necessary because no lane migration entered this round.

- SKIPPED questpc3 fcd4bdc0c04f3994246fb650bc784096bf7f62d8: 28 conflicted paths, including 9 Rust files; conflicts cross runtime owners/quest behavior rather than registration-only edits. Representative files: apps/game-server/src/node/serve/daily_world_changes.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/raid_live.rs, crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs, crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs.
- SKIPPED questpc2 16d3f9a3fcb6bf8e42b23f817e8abad0255f2d6b: 34 conflicted paths, including 15 Rust files; conflicts cross runtime owners/quest behavior rather than registration-only edits. Representative files: crates/gameplay/content/src/content/creature_reward.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_access.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs, crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs.
- SKIPPED questl2 85ea4486d2fa2fe91bfc80ad91492b1db47eb746: 58 conflicted paths, including 13 Rust files; conflicts cross runtime owners/quest behavior rather than registration-only edits. Representative files: crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/qengine_rewards.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs, crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs.
- SKIPPED questpc8 2b8157325309a51ff6e26036a4cdd82cdc85c27a: 30 conflicted paths, including 9 Rust files; conflicts cross runtime owners/quest behavior rather than registration-only edits. Representative files: apps/game-server/src/node/serve/bundle_npcs.rs, crates/gameplay/content/src/content/npc_catalogue/service.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs.
- SKIPPED questpc7 9ecb830e6b7051e8931e99c4b85bd3664d27be4c: 46 conflicted paths, including 8 Rust files; conflicts cross runtime owners/quest behavior rather than registration-only edits. Representative files: apps/game-server/src/node/serve/bundle_npcs.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs, crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs, crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs.
- SKIPPED npcfix 98f67f0c7346c04c6292db1b7b4877295a5dc22e: 39 conflicted paths, including 10 Rust files; conflicts cross runtime owners/quest behavior rather than registration-only edits. Representative files: apps/game-server/tests/content_npc_catalogue.rs, apps/game-server/tests/content_world_project_repository.rs, crates/gameplay/content/src/content/npc_catalogue/service.rs, crates/gameplay/content/src/content/npc_catalogue/travel.rs.

Merged lanes: none. Existing completed collector work is preserved through the new wpkg base. This round’s only additional change is this merge-eligibility record. Unit/PG evidence for c833ddf5d is above; no newly merged lane has PG cases to run.

Round2 publication validation: workspace/all-target compilation passed432.493s; fmt21.789s; pin-refresh40.871s; binding evidence regeneration25.318s; completion matrix regeneration1.540s; staged-tree duplicate-migration check produced no duplicates. No changed crate or newly merged lane requires additional tests relative to the repaired wpkg base because this round adds only the skip record. The full dispatch route-less validation started for that base remains active, separately from the four completed real PG cases. Correction: the final full-log scan found two earlier failures (converter_facts_match_content_charges and quest_chat_greetings_use_the_same_source_state_as_npc_commands); the publication checkpoint had at least706 passed and two failed, not zero failures. The entire failure set is still being collected before a grouped repair.

The two interrupted-worktree failures were not reproduced on the restored collector tree: converter_facts_match_content_charges passed1.827s and quest_chat_greetings_use_the_same_source_state_as_npc_commands passed48.234s, using the same compiled dispatch binary as the final PG validation. Both tests load content from the live repository: the first saw invalid JSON during the npcfix checkout, and the second saw npcfix's postman_unit_price field while running against the collector parser. Attempting a merge during these reads invalidated their input snapshot. No product or assertion change was made for those failures. The full dispatch run remains pending and must not be reported as a green full-suite verdict. Future merge eligibility attempts must finish before any runtime-content test run starts.

Validation-record correction gate: workspace/all-target check passed101.793s, fmt20.657s, pin-refresh53.462s, binding evidence45.232s and completion matrix0.829s. No generated artifact changed. Main remains included; the staged duplicate-migration check is clean. The only delta from00c4605dd is the accurate pending-test record and stable-tree recheck evidence.

## Round2 synchronization after PR3030 merged

Merged origin/main78294c081 with merge commit635bf4e34. The additive driver duplicated main's entire2738-line report prefix; verified that the collector report is its exact superset and retained that superset once. Runtime, authoring inputs, content and migration bytes equal merged main. The fresh pre-synchronization heads of questpc3, questpc2, questl2, questpc8 and both alternates were inspected; their remaining conflicts crossed quest callbacks, NPC trade/value admission or state writers, so no quest implementation was imported.

The owner then routed questpc3, questpc2, questl2 and questpc8 to merge78294c081 themselves and freeze. Collector admission now waits for each owner's MERGED-MAIN report and frozen head; the old skip decisions are historical and do not substitute for admitting those refreshed batches. Current count is0/4, not READY. No lane migration was added or renumbered.

Main-only publication validation: cargo check --workspace --all-targets --locked --keep-going passed158.914s; cargo fmt --all --check15.543s; repository pin-refresh30.358s, binding regeneration20.535s and completion matrix0.731s, without generated content changes. The real PostgreSQL two-session atomic trade/replay/cancellation case passed85.526s using the existing compiled binary whose Rust/content inputs equal this tree. No crate changes relative to merged main require new unit tests for this synchronization. The earlier full dispatch run finished2681 passed /2 failures /18 excluded by its default selection in1494.470s; its two live-content-overlap failures passed the stable-tree rechecks already recorded above, so it is not presented as a clean full-suite verdict. No content-dependent test overlaps a merge in this round.

## Round2 partial publication: frozen QUESTPC8

Collected the owner's frozen questpc8 head8639b16b158d39e36bf08d98f2a4dd998a4e2c5e in mergec0295bedf. It merged cleanly. The incoming Rust, apps, tooling and content initially matched that frozen head exactly; no quest logic was rewritten. Moved its entire report to docs/reference/QUESTPC8-CONTRACT-NOTES.md and removed all583 remaining lane-bookkeeping files. No qlane-meta or .qengine files remain tracked.

New migration files, all lane-owned and additive, with no changes to existing main migrations:1300_questpc8_custody_and_npc_proofs.sql;1301_questpc8_reward_backpack_outputs.sql;1302_questpc8_world_reward_receipts.sql;1303_questpc8_state_operation_validation.sql;1304_questpc8_occurrence_reward_mint.sql;1305_questpc8_forge_profile_admission.sql;1306_questpc8_father_reward_amount.sql;1307_questpc8_canonical_father_values.sql. Duplicate-migration checks pass.

Regenerated native providers, equipment/audit/room inventories, binding evidence and completion matrix with repo tooling. This exposed stale witnesses in the frozen packet: four canonical-definition hashes and reward-claim classifications. Replayed the downstream authoring tools in dependency order; the generated runtime mission changed only four authoring_sources SHA256 values, preserving every transition and reward. The complete workflow check list initially found one remaining stale reward-claim variant migration packet; regenerated it with reward_claim_variant_migration.py and reran the unchanged complete34-check workflow successfully in237.811s. The three focused authoring unit modules passed24 tests in3.324s. No generated artifact was hand-edited and no source approval or assertion was relaxed.

Collector evidence: native regeneration356.574s; binding17.913s; matrix0.628s; downstream141.213s including compiler wait; final pin-refresh26.099s and pin-check28.191s. The first all-target/all-feature clippy attempt exhausted disk; removed inactive caches and reran with CARGO_BUILD_JOBS=2 and CARGO_INCREMENTAL=0. All13 changed crates passed strict clippy in302.745s. Final restored-snapshot workspace/all-target check passed308.387s and fmt22.793s. The prior workspace check on these runtime bytes passed51.744s as well.

Mandatory pre-push fetch found new mainb3864f14a (#3031). Its merge had only four generated Prey conflicts: content/creatures/prey-slot-values.json, both PREY2026-11-01 reports and tools/content-schema/native-gameplay/prey-model-results.json. The repo generator rebuilt source profiles in356.025s then refused PREY_MODEL_STALE: run prey_difficulty.py --simulate. A fresh simulation plus rebuild could not complete within the owner's main-merge window, so aborted that merge and restored the already checked snapshot before publication. Mainb3864f14a synchronization remains pending; no model rate or input hash was manufactured. This is the explicitly allowed abort-main/publish-last-compiling-state fallback.

The owner freeze record for8639b16b1 proves8/8 native PG routes in365.597s and323 Quest units in36.05s on the lane. Collector Rust unit and fresh native-PG reruns are pending, not claimed as local passes, under PREPUSH-2's publication-cadence exception. The collector's complete Quest unit run is queued/running; the other changed-crate units and8PG routes remain to execute after it. Current round contains1/4 lanes (questpc8); questpc3, questpc2 and questl2 still require their green frozen delivery. This partial branch is not READY for a PR.

Post-publication completion: the collector Quest unit command passed323/323 in36.679s of test execution (96.138s command wall), with zero skipped tests. This replaces its pending status above; the eight collector native-PG routes and remaining12changed-crate unit runs are still pending. Corrected the pin-refresh duration to the recorded26.099s; runtime and generated inputs are unchanged from the validated/published snapshot.

## Collected validation and strict callback-coverage repair

The collector's eight native PostgreSQL routes passed8/8 in253.032s of execution (662.203s including build): Father legacy88.982s, Father fresh88.996s, complete Father journey91.017s, Rathleton128.583s, Winterlight89.415s, Secret Library93.273s, Venore132.704s and Wrath124.445s. Explicit completion markers were captured. These execute the unchanged production implementation collected from questpc8; the following repair changes only a conformance test.

The entire remaining12-crate unit run completed5499 tests in2230.875s execution (2613.333s command wall):5498passed, exactly one failure,30excluded by the default selection. The failure was quest_callbacks_binding_and_coverage_require_exact_source_witnesses: its manifest correctly contains334 supported claims, but the legacy once-only binder reports328. The other six are the dedicated Falcon24-hour random-cycle sources, routed by use_falcon_chest and backed by claim_falcon_chest, not ordinary once claims. No reward or admission implementation is missing and no engine feature was added.

Preserved all four original source-binding perturbations and the complete supported-set equality. The test now compares the disjoint union of actual once-binder candidates and the production Falcon selector registry, asserts exactly six cycle sources, and additionally asserts that every timed source remains excluded by the generic binder as LeftOut::Variant. This prevents accidental once-only handling or duplicate ownership; no claim was removed from coverage and no test/status/assertion was skipped or downgraded. The source-witness contract and exact-six/seven-rewards selector tests both passed (2/2,0.275s execution;284.393s including build).

Repository equipment, audit and room census regeneration passed0.425s/0.026s/0.023s; binding18.455s, matrix0.643s and pin-refresh24.942s all leave generated bytes unchanged. Duplicate migrations remain clear. Current-source workspace/all-target check passed203.190s, dispatch all-target/all-feature clippy with warnings denied223.168s, and fmt15.319s. The complete same12-crate unit set has been restarted with --no-fail-fast; its rerun remains pending under the owner's PREPUSH-2 cadence fallback. The unchanged Quest cohort's prior323/323 result and8/8 real-PG result are not represented as the still-running full rerun's outcome.

Origin/main has advanced to6f61ac15d (including PUBPROJb3864f14a and the physical-item-lane fix). The previously recorded abort-main/publish-last-compiling-state fallback remains in effect while its required real Prey simulation/native rebuild is pending. This repair push does not claim that newer main is merged. Round remains1/4, not READY; the other three frozen green lane deliveries remain required.

## Round2 publication: current main and collected QUESTPC8

Merged main6f61ac15d,98d6ae8ce,47baa7bb8 and c77d436d8 with normal merge commits. The latest fetch confirms c77d436d8 is an ancestor. Preserved main's key-33 input-row authority, SOULPIT rewards and QREADY native providers, together with the frozen QUESTPC8 implementation. There is one input-row helper and no duplicate implementation. No source lane, protected main, migration already on main or quest outside the collected packet was rewritten. Current packet still contains only frozen QUESTPC8; this is not a four-lane READY claim.

Resolved generated conflicts with the repository resolver, then regenerated providers, manifests, Item indexes, completion matrix, room evidence, equipment/Type2/room inventories, equipment cohort pins and bundle pins. The provider-order repair runs native Item closure after final reward-claim authoring: otherwise six complete container-recipe Items (2905,2992,3424,3466,3602,6498) are omitted. All six now exist, and the independent generated equipment union/cohort count agrees at3920. No generated content or pin was hand-edited.

Main's unconditional duplicate-selector guard conflicted with collected stateful stations. A small new selector_gates helper permits only same-quest stations whose storage intervals prove them mutually exclusive. Existing production candidate selection and transaction gates remain unchanged. Overlapping and cross-quest aliases still fail with the original error; the new positive/negative test and existing duplicate-alias regression passed. Main's newer QREADY cohorts and seven later QENGINE2 profiles are excluded from the immutable3663-record census base rather than double-counted.

Rebuilt binding/mission/completion/reward candidate evidence and final donor capture in dependency order (binding source crosswalk before donor). All34 unchanged workflow checks passed; the98 authoring unit modules and donor freshness check passed. Corrected authoring tests retain all true SOURCE holds and exact qualification witnesses; explicit false compiler metadata no longer makes every row appear changed, all162 pending ledger rows are checked, and qualified admissions retain their precise PG evidence. After c77d436d8, the11 QREADY authoring tests and14 completion/proof/runtime tests also passed (4.959s and0.585s test execution).

Rechecked the NPC catalogue:4/4 tests passed in50.57s, including399 retained raw services. The executable model's stale390-service/11095-offer pins predated main's S10 services and subtype restoration. Added tools/contracts/refresh_npc_service_counts.py to capture the actual retained model and regenerate every existing strict count assertion together. Its --check passed with400 services,12565 offers,1282 NPCs,195 routes and446 generated-reply NPCs. All held-category assertions remain exact (NonGoldCurrency31, SellPriceAboveCoinCapacity17, TimedCountMismatch7; the other five categories0). No NPC admission, price, quantity or service behavior was changed by this counter repair.

PostgreSQL18.6 proof:10/10 focused cases passed, with database-start and completion markers, in341.880s test execution. Individual nextest durations:
- l6_reward_container_preserves_mead_and_exact_replay: 59.609s.
- quest_catalogue_chests_exact_fluid_subtypes_use_replay_and_refusal: 104.204s.
- catalogue_npc_father_fresh_acceptance_is_one_eleven_track_transaction: 104.285s.
- fathers_burden_eight_handins_cape_experience_and_replay: 111.190s.
- catalogue_npc_father_legacy_hash_preserves_defaults_and_restart_receipts: 92.837s.
- questpc8_secret_library_raft_research_fiddle_gail_oberon_and_access: 101.695s.
- hero_of_rathleton_pipe_greeting_commits_access_once_and_replays: 136.081s.
- questpc8_venore_native_daily_circuit_and_exact_repeat_boundary: 147.661s.
- questpc8_winterlight_native_disguise_four_fires_queen_and_base_outfit: 108.028s.
- questpc8_wrath_native_opening_exact_materials_crate_and_replay: 135.893s.

The focused unit recheck passed69/69 in57.023s execution, including all eight substantive failures recorded before the repair, callback disjointness/overlap cases, catalogue boot and the full raw-attribute equipment-speed audit. Successful command wall durations (including any cargo lock wait): native regeneration 369.282s; pin-refresh 27.928s; pin-check 35.375s; workspace/all-targets 264.857s; fmt 21.525s; all13 strict Clippy 422.817s; new NPC capture target Clippy 131.724s; focused unit command 192.218s; PG command 399.784s; NPC count check command 461.804s.

The full13-crate unit run was interrupted for the owner's PREPUSH-2 cadence exception: it had1667 passes and eight substantive failures (plus eight interrupted workers), with4368 tests not yet run. Those eight failures are covered by the69-test green recheck above, but the full changed-crate unit remainder is PENDING, not claimed green. CI runs the full suites. An initial PG listing failed because cleanup removed an already-cached obsolete test path; rebuilt the missing binaries, then the complete10-case PG retry passed. A pin-check invocation initially omitted its required output directory; the corrected exact command passed as recorded above. No test was weakened, skipped by a new exclusion, deleted or marked qualified without proof.

Duplicate-migration checks on HEAD and the worktree pass. The only migrations added relative to c77d436d8 remain the unchanged QUESTPC8 files1300–1307 listed earlier in this report; none needed renumbering. WPKG's historical owned numbers1330–1333 and1345–1349 are already on main; this synchronization adds no WPKG-owned SQL. No root CONTRACT-NOTES.md, qlane-meta or .qengine files remain tracked.
