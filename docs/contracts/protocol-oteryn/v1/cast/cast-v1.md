# S17 Cast V1 candidate contract

Owning design: S17-CAST-DESIGN, accepted section 6, revision
956a1cdff6188381b25419251654f63a8b19536f (PR #3000).
Capabilities 289 VIEW, 290 CONTROL, 291 CHAT and 292 CONSENT are **unoffered**.
SPECTATE1 capability 33, commands 34/35, domain 25 and family 15 are unchanged.
Player controls use commands 502 CONTROL, 503 CONSENT and 505 MODERATE.
504 CHAT / kind 10 CasterReply is reserved; caster replies are not implemented.
Domain 104 carries replacement scenes; domain 103 carries live caster chat. No client UI is implemented here.

## Message

`crates/protocol-oteryn/src/cast.rs` is the executable schema. Canonical protobuf
subset: fields are required, emitted in order even at zero, with minimal varints;
unknown/repeated/out-of-order fields, trailing bytes and invalid enum/range values
are refused. Each variant starts with field 1 (varint kind).

| Kind | Message | Subsequent fields, in order |
|---|---|---|
| 1 / 2 | Start / Update | 2 revision, 3 Platform access revision, 4 delay 0–300, 5 cap 1–50, 6 password-required boolean |
| 3 | Stop | 2 expected revision |
| 4 / 5 | ConsentSet / ConsentState | 2 revision, 3 allow-shown boolean |
| 6 | Join | 2 one-time observer material, 1–4096 bytes |
| 7 / 8 | Leave / Keepalive | none |
| 9 / 10 | Chat / CasterReply | 2 plain UTF-8 text, 1–256 bytes |
| 11 | Moderate | 2 expected revision, 3 opaque 16-byte subject, 4 action (1 mute, 2 unmute, 3 kick), 5 nonzero 16-byte idempotency key |
| 12 | State | 2 revision, 3 positive epoch, 4 delay, 5 viewer count 0–50 |
| 13 | Slate | 2 positive epoch, 3 positive sequence |
| 14 | Clear | 2 positive epoch |
| 15 | Viewer ChatLine | 2 epoch, 3 sequence, 4 opaque handle (16 bytes), 5 text |
| 16 | Live CasterChatLine | 2 handle, 3 text |
| 17 | End | none; generic, no fresh cast content |
| 19 | CasterChatEmpty | none; playing-caster initial snapshot only |
| 18 | Scene | 2 epoch, 3 sequence, repeated 4 tiles then repeated 5 actors |
| 20 | CasterState | 2 revision, 3 epoch, 4 delay, 5 viewer count, 6 opaque 16-byte cast handle; owner only |

Caster control fields never constitute Platform access proof. Game must resolve
the access revision from the trusted Platform policy owner. Reusable passwords
never reach Game. Observer material must be purpose/audience/node/account/cast/
epoch/access-revision/expiry bound, single use and globally reserved before Game
admission. Only the online, mutually authenticated Platform consume/renew adapter creates
production `VerifiedAdmission` values. A ticket is not a character-login credential.

Observer envelope: required varint field 1 = **16**, field 2 = generation,
length-delimited field 4 = Message. Generation zero permits only Join. After
admission the generation is positive. Observer client messages are exclusively
Join, Leave, Keepalive and Chat; gameplay, control, consent, reply and unknown
opcodes are refused before dispatch. Observer server messages exclude live
CasterChatLine, ConsentState and CasterState. Family 16 has no legacy/player fallback.
The envelope limit is 16432 bytes; its message limit is 16384 bytes.

CHAT requires VIEW for an observer or CONTROL for a playing caster. Observers
cannot select CONTROL/CONSENT. Playing-caster controls require normal selected-capability negotiation;
observer VIEW/CHAT selection is bound to the purpose-separated account profile
and checked against Game offers before admission. Codec support never activates
an offer. The native listener routes family 16 before player bootstrap and never creates
a character actor. Password verification, global quotas and worker routing are
composed; client consumers are a later task.

## Scene

Each tile submessage has varint fields 1 x, 2 y, 3 public item type (all u16).
Each actor has 1 cast-local handle (u32), 2 x, 3 y, 4 public appearance (u16),
5 public health percentage (0–100). All submessage fields are required in order.
At most 512 tiles and 128 actors, no opaque player payloads. Scene replaces the
whole world view atomically. No names, account/character IDs, speech, inventory,
private UI, room chat, exact vitals, labels or unknown presentation subtypes.

Missing/stale bystander consent yields a whole-view slate. Explicit start is
the caster's own consent; other characters default off. The trusted committed
interest set excludes hidden/off-screen actors. Consent withdrawal purges queued
scenes, immediately clears the viewer scene and requires a fresh eligible
baseline. Release rechecks revision/freshness. Final synchronous nonblocking
writes serialize with the consent barrier under the cast owner, outside the tick.

The phase-1 native producer emits neutral numeric player glyphs with coarse
health and no map tiles, NPCs or creatures. This intentionally bounded producer
is not a full game renderer; client presentation is a later task. It runs at
one committed capture per second outside database/network waits.

All viewer content, including chat and metadata, follows selected D (default 0).
Accepted viewer chat reaches the separate caster queue live even at D=300.
Delay changes reset epoch and clear queues. Access rotation ends observers.
Stop, kick, timeout, logout/death/scope/session/lease/connection changes purge
queues and release reservations. Casting is allowed in competitive areas.

## Composition and rollout

Production remains off pending client consumers and measured node/worker/egress
qualification. `OTERYN_CAST_ENABLED` defaults false and enabling it requires
explicit node capacity, HTTPS Platform URL, protected client identity and pinned
Platform CA. TLS 1.3, no redirects/proxy, bounded requests and online consumption
are mandatory. All four capability offers remain false, independently gating
the native command path. Competitive areas do not prohibit casting.

The companion Platform 0968 implementation accepts an existing account-only
native admission ticket through the authenticated Gateway cast-watch route.
No character selection is accepted. It verifies optional Argon2id passwords,
reserves at most two sessions per account across casts and fifty per cast under
account-then-policy locks, and issues a hashed, single-use `ocv1.` ticket.
Consumption binds account, node, cast, epoch, access revision and selected chat.
Renewal rechecks current account security generation and all policy bindings.
Five-second global reservations have conservative local deadlines and online
renewal every two seconds; a keepalive alone never extends a reservation.
Leave, kick, expiry, stop and access rotation release or invalidate admission.
Password changes belong to the authenticated caster account; Game receives no
reusable password. A password-required new cast stays closed until a verifier
is installed. Policy announcements alone cannot remove an installed password.

Viewer chat has per-account and room limits, plain-text bounds and selected-chat
checks. It reaches the playing caster live through domain 103; viewer copies
follow D. Moderation uses opaque handles from those live chat lines. Mute affects
the account in that cast; kick removes one session, without a permanent ban.

Global consent withdrawal uses the 0966 durable invalidation/worker-lease barrier
and capture permits documented in `S17_CAST_CONSENT_PERSISTENCE.md`. Revocation
clears buffered and shown scenes before false commits, including delayed frames
on other workers. Missing, stale, expired or conflicting facts fail closed.

The Game audit adapter (0967) writes registered type-10 security envelopes with
UUIDv7 identities and P30D expiry. The existing typed Character/item outboxes
require unrelated receipts and retention tuples, so they cannot hold these facts.
Node possession/currentness and stream ownership are independently checked.
Incarnation/ordinal prefixes reject gaps and conflicting replay; acknowledgements
are constructed only after commit. Cast/observer terminal slots are reserved
before starts/joins. Viewer delivery waits for its audit prefix; a storage error
or invalid receipt closes casts, purges queues and retains reserved terminals.
Consent writes audit atomically with the preference. Runtime cannot delete audit;
Control has a bounded expiry-only routine. Export/GM composition is still pending.
